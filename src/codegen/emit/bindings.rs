use super::*;

impl RBackend {
    pub(crate) fn current_var_version(&self, var: &str) -> u64 {
        *self.value_tracker.var_versions.get(var).unwrap_or(&0)
    }

    pub(crate) fn resolve_raw_generated_loop_expr(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
        seen: &mut FxHashSet<usize>,
    ) -> String {
        if !seen.insert(val_id) {
            return self.resolve_val(val_id, values, params, true);
        }
        match &values[val_id].kind {
            ValueKind::Const(lit) => self.emit_lit_with_value(lit, &values[val_id]),
            ValueKind::Param { index } => self.resolve_param(*index, params),
            ValueKind::Load { var } => var.clone(),
            ValueKind::Binary { op, lhs, rhs } => format!(
                "({} {} {})",
                self.resolve_raw_generated_loop_expr(*lhs, values, params, seen),
                Self::binary_op_str(*op),
                self.resolve_raw_generated_loop_expr(*rhs, values, params, seen)
            ),
            ValueKind::Unary { op, rhs } => {
                let rhs = self.resolve_raw_generated_loop_expr(*rhs, values, params, seen);
                match op {
                    UnaryOp::Neg => format!("(-{rhs})"),
                    UnaryOp::Not => format!("(!{rhs})"),
                    UnaryOp::Formula => format!("(~{rhs})"),
                }
            }
            _ => self.resolve_val(val_id, values, params, true),
        }
    }

    pub(crate) fn note_var_write(&mut self, var: &str) {
        let next = self.current_var_version(var) + 1;
        self.log_var_version_change(var);
        self.value_tracker
            .var_versions
            .insert(var.to_string(), next);
    }

    pub(crate) fn bind_value_to_var(&mut self, val_id: usize, var: &str) {
        let version = self.current_var_version(var);
        self.log_value_binding_change(val_id);
        self.value_tracker
            .value_bindings
            .insert(val_id, (var.to_string(), version));
    }

    pub(crate) fn bind_var_to_value(&mut self, var: &str, val_id: usize) {
        let version = self.current_var_version(var);
        self.log_var_value_binding_change(var);
        self.value_tracker
            .var_value_bindings
            .insert(var.to_string(), (val_id, version));
    }

    pub(crate) fn resolve_bound_value(&self, val_id: usize) -> Option<String> {
        if let Some((var, version)) = self.value_tracker.value_bindings.get(&val_id)
            && self.current_var_version(var) == *version
        {
            return Some(var.clone());
        }
        None
    }

    pub(crate) fn resolve_bound_value_id(&self, var: &str) -> Option<usize> {
        self.value_tracker
            .var_value_bindings
            .get(var)
            .filter(|(_, version)| self.current_var_version(var) == *version)
            .map(|(val_id, _)| *val_id)
    }

    pub(crate) fn can_elide_index_expr(
        &self,
        idx: usize,
        values: &[Value],
        params: &[String],
    ) -> bool {
        if Self::can_elide_index_wrapper(idx, values) {
            return true;
        }
        for ctx in self.loop_analysis.active_scalar_loop_indices.iter().rev() {
            if self
                .loop_index_offset(idx, ctx, values, &mut FxHashSet::default())
                .is_some_and(|offset| Self::loop_context_allows_offset(ctx, offset))
            {
                return true;
            }
        }
        let rendered = self.resolve_val(idx, values, params, false);
        for ctx in self.loop_analysis.active_scalar_loop_indices.iter().rev() {
            if Self::rendered_loop_index_offset(&rendered, ctx)
                .is_some_and(|offset| Self::loop_context_allows_offset(ctx, offset))
            {
                return true;
            }
        }
        self.resolve_bound_value_id(&rendered)
            .is_some_and(|bound| Self::can_elide_index_wrapper(bound, values))
    }

    pub(crate) fn resolve_temp_bound_value_id(&self, var: &str) -> Option<usize> {
        self.resolve_bound_value_id(var).or_else(|| {
            (var.starts_with(".__rr_cse_") || var.starts_with(".tachyon_exprmap"))
                .then(|| {
                    self.value_tracker
                        .var_value_bindings
                        .get(var)
                        .map(|(val_id, _)| *val_id)
                })
                .flatten()
        })
    }

    pub(crate) fn resolve_readonly_arg_alias_name(
        &self,
        var: &str,
        values: &[Value],
    ) -> Option<String> {
        let stripped = var.strip_prefix(".arg_")?;
        if stripped.is_empty() || self.current_var_version(var) > 1 {
            return None;
        }
        let bound = self.resolve_temp_bound_value_id(var)?;
        matches!(
            values.get(bound).map(|v| &v.kind),
            Some(ValueKind::Param { .. })
        )
        .then(|| stripped.to_string())
    }

    pub(crate) fn rewrite_live_readonly_arg_aliases(
        &self,
        expr: String,
        values: &[Value],
    ) -> String {
        let mut out = expr;
        let mut aliases: Vec<(String, String)> = self
            .value_tracker
            .var_value_bindings
            .keys()
            .filter_map(|var| {
                self.resolve_readonly_arg_alias_name(var, values)
                    .map(|alias| (var.clone(), alias))
            })
            .collect();
        aliases.sort_by_key(|(lhs, _)| std::cmp::Reverse(lhs.len()));
        for (from, to) in aliases {
            let Some(re) = compile_regex(format!(r"\b{}\b", regex::escape(&from))) else {
                continue;
            };
            out = re.replace_all(&out, to.as_str()).to_string();
        }
        out
    }

    pub(crate) fn known_full_end_expr_for_var(&self, var: &str) -> Option<&str> {
        self.loop_analysis
            .known_full_end_exprs
            .get(var)
            .map(String::as_str)
            .or_else(|| {
                self.loop_analysis
                    .active_loop_known_full_end_exprs
                    .iter()
                    .rev()
                    .find_map(|frame| frame.get(var).map(String::as_str))
            })
    }

    pub(crate) fn remember_known_full_end_expr(
        &mut self,
        var: &str,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) {
        if let Some(end_expr) = self.known_full_end_expr_for_value(val_id, values, params) {
            self.loop_analysis
                .known_full_end_exprs
                .insert(var.to_string(), end_expr.clone());
            if let Some(sym) = values.get(val_id).and_then(|value| value.value_ty.len_sym) {
                self.loop_analysis.len_sym_end_exprs.insert(sym, end_expr);
            }
        } else {
            self.loop_analysis.known_full_end_exprs.remove(var);
        }
    }

    pub(crate) fn known_full_end_expr_for_value(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        self.known_full_end_expr_for_value_impl(val_id, values, params, &mut FxHashSet::default())
    }

    pub(crate) fn resolve_known_full_end_expr_with_seen(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
        seen: &mut FxHashSet<usize>,
    ) -> Option<String> {
        self.known_full_end_expr_for_value_impl(val_id, values, params, seen)
            .or_else(|| {
                let rendered =
                    self.resolve_bound_temp_expr(val_id, values, params, &mut FxHashSet::default());
                (!rendered.is_empty()).then_some(rendered)
            })
    }

    pub(crate) fn known_full_end_expr_for_value_impl(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
        seen: &mut FxHashSet<usize>,
    ) -> Option<String> {
        if !seen.insert(val_id) {
            return None;
        }
        let value = values.get(val_id)?;
        match &value.kind {
            ValueKind::Param { index } => self
                .analysis
                .current_seq_len_param_end_slots
                .get(index)
                .map(|end_index| self.resolve_param(*end_index, params))
                .or_else(|| Some(self.resolve_param(*index, params))),
            ValueKind::Load { var } => self
                .resolve_bound_value_id(var)
                .and_then(|bound| {
                    self.known_full_end_expr_for_value_impl(bound, values, params, seen)
                })
                .or_else(|| self.known_full_end_expr_for_var(var).map(str::to_string)),
            ValueKind::Len { base } => {
                self.known_full_end_expr_for_value_impl(*base, values, params, seen)
            }
            ValueKind::Call { callee, args, .. }
                if self
                    .fresh_allocation_len_arg_index(callee.as_str(), args, values)
                    .is_some() =>
            {
                let len_idx = self
                    .fresh_allocation_len_arg_index(callee.as_str(), args, values)
                    .unwrap_or(0);
                self.resolve_known_full_end_expr_with_seen(args[len_idx], values, params, seen)
            }
            ValueKind::Call { callee, args, .. }
                if callee == "rr_assign_slice" && args.len() >= 4 =>
            {
                if self.value_is_known_one(args[1], values) {
                    self.resolve_known_full_end_expr_with_seen(args[2], values, params, seen)
                } else {
                    None
                }
            }
            ValueKind::Call { callee, args, .. }
                if callee == "rr_call_map_slice_auto" && args.len() >= 7 =>
            {
                if self.value_is_known_one(args[1], values) {
                    self.resolve_known_full_end_expr_with_seen(args[2], values, params, seen)
                } else {
                    None
                }
            }
            ValueKind::Call { callee, args, .. }
                if callee == "rr_call_map_whole_auto" && !args.is_empty() =>
            {
                Self::named_mutable_base_expr(
                    args[0],
                    values,
                    &self.value_tracker.value_bindings,
                    &self.value_tracker.var_versions,
                )
                .and_then(|var| {
                    self.known_full_end_expr_for_var(var.as_str())
                        .map(str::to_string)
                })
            }
            ValueKind::Binary { lhs, rhs, .. } if !self.value_is_scalar_shape(val_id, values) => {
                let lhs_end = self.known_full_end_expr_for_value_impl(*lhs, values, params, seen);
                let rhs_end = self.known_full_end_expr_for_value_impl(*rhs, values, params, seen);
                self.merge_known_full_end_exprs(lhs_end, rhs_end, *lhs, *rhs, values)
            }
            ValueKind::Unary { rhs, .. } if !self.value_is_scalar_shape(val_id, values) => {
                self.known_full_end_expr_for_value_impl(*rhs, values, params, seen)
            }
            ValueKind::Intrinsic { op, args } if !self.value_is_scalar_shape(val_id, values) => {
                match (op, args.as_slice()) {
                    (
                        IntrinsicOp::VecAddF64
                        | IntrinsicOp::VecSubF64
                        | IntrinsicOp::VecMulF64
                        | IntrinsicOp::VecDivF64
                        | IntrinsicOp::VecPmaxF64
                        | IntrinsicOp::VecPminF64,
                        [lhs, rhs],
                    ) => {
                        let lhs_end =
                            self.known_full_end_expr_for_value_impl(*lhs, values, params, seen);
                        let rhs_end =
                            self.known_full_end_expr_for_value_impl(*rhs, values, params, seen);
                        self.merge_known_full_end_exprs(lhs_end, rhs_end, *lhs, *rhs, values)
                    }
                    (
                        IntrinsicOp::VecAbsF64
                        | IntrinsicOp::VecLogF64
                        | IntrinsicOp::VecSqrtF64
                        | IntrinsicOp::VecSumF64
                        | IntrinsicOp::VecMeanF64,
                        [arg],
                    ) => self.known_full_end_expr_for_value_impl(*arg, values, params, seen),
                    _ => None,
                }
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } if names.iter().all(|name| name.is_none())
                && !self.value_is_scalar_shape(val_id, values) =>
            {
                match (callee.as_str(), args.as_slice()) {
                    ("abs" | "log" | "sqrt" | "floor" | "ceiling" | "trunc", [arg]) => {
                        self.known_full_end_expr_for_value_impl(*arg, values, params, seen)
                    }
                    ("pmax" | "pmin", [lhs, rhs]) => {
                        let lhs_end =
                            self.known_full_end_expr_for_value_impl(*lhs, values, params, seen);
                        let rhs_end =
                            self.known_full_end_expr_for_value_impl(*rhs, values, params, seen);
                        self.merge_known_full_end_exprs(lhs_end, rhs_end, *lhs, *rhs, values)
                    }
                    _ => value
                        .value_ty
                        .len_sym
                        .and_then(|sym| self.loop_analysis.len_sym_end_exprs.get(&sym).cloned()),
                }
            }
            _ => value
                .value_ty
                .len_sym
                .and_then(|sym| self.loop_analysis.len_sym_end_exprs.get(&sym).cloned()),
        }
    }

    pub(crate) fn resolve_known_full_end_expr(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        self.known_full_end_expr_for_value_impl(val_id, values, params, &mut FxHashSet::default())
            .or_else(|| {
                let rendered =
                    self.resolve_bound_temp_expr(val_id, values, params, &mut FxHashSet::default());
                (!rendered.is_empty()).then_some(rendered)
            })
    }

    pub(crate) fn fresh_allocation_len_arg_index(
        &self,
        callee: &str,
        args: &[usize],
        values: &[Value],
    ) -> Option<usize> {
        let argc = args.len();
        match callee {
            "numeric" | "seq_len" => Some(0),
            "rep.int" if argc >= 2 => Some(1),
            "vector" if argc >= 2 => Some(1),
            "vector" if argc >= 1 => Some(0),
            _ if self.analysis.known_fresh_result_calls.contains(callee)
                && argc == 3
                && self.value_can_be_allocator_scalar_arg(args[0], values)
                && self.value_can_be_allocator_scalar_arg(args[1], values)
                && matches!(self.const_int_value(args[2], values), Some(tag) if (0..=4).contains(&tag)) =>
            {
                Some(0)
            }
            _ => None,
        }
    }

    pub(crate) fn merge_known_full_end_exprs(
        &self,
        lhs_end: Option<String>,
        rhs_end: Option<String>,
        lhs: usize,
        rhs: usize,
        values: &[Value],
    ) -> Option<String> {
        match (lhs_end, rhs_end) {
            (Some(lhs_end), Some(rhs_end)) if lhs_end == rhs_end => Some(lhs_end),
            (Some(lhs_end), None)
                if !self.value_is_scalar_shape(lhs, values)
                    && self.value_is_scalar_shape(rhs, values) =>
            {
                Some(lhs_end)
            }
            (None, Some(rhs_end))
                if self.value_is_scalar_shape(lhs, values)
                    && !self.value_is_scalar_shape(rhs, values) =>
            {
                Some(rhs_end)
            }
            _ => None,
        }
    }

    pub(crate) fn value_is_scalar_shape(&self, value_id: usize, values: &[Value]) -> bool {
        values.get(value_id).is_some_and(|value| {
            value.value_ty.shape == ShapeTy::Scalar
                || value.facts.has(Facts::INT_SCALAR)
                || value.facts.has(Facts::BOOL_SCALAR)
                || matches!(value.kind, ValueKind::Const(_) | ValueKind::Len { .. })
        })
    }

    pub(crate) fn value_can_be_allocator_scalar_arg(
        &self,
        value_id: usize,
        values: &[Value],
    ) -> bool {
        values.get(value_id).is_some_and(|value| {
            if self.value_is_scalar_shape(value_id, values) {
                return true;
            }
            !matches!(value.value_ty.shape, ShapeTy::Vector | ShapeTy::Matrix)
                && !matches!(
                    value.value_term,
                    TypeTerm::Vector(_)
                        | TypeTerm::VectorLen(_, _)
                        | TypeTerm::Matrix(_)
                        | TypeTerm::MatrixDim(_, _, _)
                        | TypeTerm::ArrayDim(_, _)
                        | TypeTerm::DataFrame(_)
                        | TypeTerm::DataFrameNamed(_)
                        | TypeTerm::NamedList(_)
                        | TypeTerm::List(_)
                        | TypeTerm::Boxed(_)
                        | TypeTerm::Union(_)
                )
        })
    }

    pub(crate) fn whole_dest_end_matches_known_var(
        &self,
        var: &str,
        end: usize,
        values: &[Value],
        params: &[String],
    ) -> bool {
        assign::whole_dest_end_matches_known_var(self, var, end, values, params)
    }

    pub(crate) fn known_full_end_bound_for_var(&self, var: &str, values: &[Value]) -> Option<i64> {
        assign::known_full_end_bound_for_var(self, var, values)
    }

    pub(crate) fn known_full_end_bound_for_value(
        &self,
        val_id: usize,
        values: &[Value],
    ) -> Option<i64> {
        match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Call { callee, args, .. })
                if self
                    .fresh_allocation_len_arg_index(callee.as_str(), args, values)
                    .is_some() =>
            {
                let len_idx = self
                    .fresh_allocation_len_arg_index(callee.as_str(), args, values)
                    .unwrap_or(0);
                self.const_index_int_value(args[len_idx], values)
            }
            Some(ValueKind::Call { callee, args, .. })
                if callee == "rr_assign_slice" && args.len() >= 4 =>
            {
                self.const_index_int_value(args[2], values)
            }
            Some(ValueKind::Call { callee, args, .. })
                if callee == "rr_call_map_slice_auto" && args.len() >= 7 =>
            {
                self.const_index_int_value(args[2], values)
            }
            Some(ValueKind::Load { var }) => self
                .resolve_bound_value_id(var)
                .and_then(|bound| self.known_full_end_bound_for_value(bound, values)),
            _ => None,
        }
    }

    pub(crate) fn invalidate_var_binding(&mut self, var: &str) {
        self.loop_analysis.recent_whole_assign_bases.remove(var);
    }

    pub(crate) fn invalidate_alias_bindings_depending_on_var(
        &mut self,
        var: &str,
        values: &[Value],
    ) {
        let stale_values = self
            .value_tracker
            .value_bindings
            .keys()
            .copied()
            .filter(|val_id| self.value_mentions_written_var(*val_id, values, var))
            .collect::<Vec<_>>();
        for val_id in stale_values {
            self.log_value_binding_change(val_id);
            self.value_tracker.value_bindings.remove(&val_id);
        }

        let stale_vars = self
            .value_tracker
            .var_value_bindings
            .iter()
            .filter(|(_, (val_id, _))| self.value_mentions_written_var(*val_id, values, var))
            .map(|(bound_var, _)| bound_var.clone())
            .collect::<Vec<_>>();
        for bound_var in stale_vars {
            self.log_var_value_binding_change(&bound_var);
            self.value_tracker.var_value_bindings.remove(&bound_var);
        }

        let stale_last_assigned = self
            .value_tracker
            .last_assigned_value_ids
            .iter()
            .filter(|(_, val_id)| self.value_mentions_written_var(**val_id, values, var))
            .map(|(bound_var, _)| bound_var.clone())
            .collect::<Vec<_>>();
        for bound_var in stale_last_assigned {
            self.log_last_assigned_value_change(&bound_var);
            self.value_tracker
                .last_assigned_value_ids
                .remove(&bound_var);
        }

        self.invalidate_var_binding(var);
    }

    pub(crate) fn invalidate_alias_bindings_depending_on_vars<'a, I>(
        &mut self,
        vars: I,
        values: &[Value],
    ) where
        I: IntoIterator<Item = &'a String>,
    {
        for var in vars {
            self.invalidate_alias_bindings_depending_on_var(var, values);
        }
    }

    pub(crate) fn clear_expression_alias_bindings(&mut self) {
        for val_id in self
            .value_tracker
            .value_bindings
            .keys()
            .copied()
            .collect::<Vec<_>>()
        {
            self.log_value_binding_change(val_id);
        }
        for var in self
            .value_tracker
            .var_value_bindings
            .keys()
            .cloned()
            .collect::<Vec<_>>()
        {
            self.log_var_value_binding_change(&var);
        }
        for var in self
            .value_tracker
            .last_assigned_value_ids
            .keys()
            .cloned()
            .collect::<Vec<_>>()
        {
            self.log_last_assigned_value_change(&var);
        }

        self.value_tracker.value_bindings.clear();
        self.value_tracker.var_value_bindings.clear();
        self.value_tracker.last_assigned_value_ids.clear();
    }

    pub(crate) fn value_mentions_written_var(
        &self,
        val_id: usize,
        values: &[Value],
        var: &str,
    ) -> bool {
        let mut stack = vec![val_id];
        let mut seen = FxHashSet::default();
        while let Some(next) = stack.pop() {
            if !seen.insert(next) {
                continue;
            }
            let Some(value) = values.get(next) else {
                continue;
            };
            match &value.kind {
                ValueKind::Load { var: load_var } if load_var == var => return true,
                ValueKind::Param { .. } if value.origin_var.as_deref() == Some(var) => {
                    return true;
                }
                _ => {}
            }
            stack.extend(value_dependencies(&value.kind));
        }
        false
    }

    pub(crate) fn invalidate_var_bindings<'a, I>(&mut self, vars: I)
    where
        I: IntoIterator<Item = &'a String>,
    {
        for var in vars {
            self.invalidate_var_binding(var);
        }
    }

    pub(crate) fn resolve_stale_origin_var(
        &self,
        val_id: usize,
        val: &Value,
        _values: &[Value],
    ) -> Option<String> {
        let is_self_update_call =
            matches!(&val.kind, ValueKind::Call { callee, .. } if callee == "rr_assign_slice");
        if let Some((bound_var, version)) = self.value_tracker.value_bindings.get(&val_id) {
            let current_version = self.current_var_version(bound_var);
            if *version != current_version {
                if is_self_update_call {
                    return None;
                }
                return Some(bound_var.clone());
            }
        }

        let origin_var = val.origin_var.as_ref()?;
        let current_version = self.current_var_version(origin_var);

        if let Some((current_val_id, version)) =
            self.value_tracker.var_value_bindings.get(origin_var)
            && *version == current_version
            && *current_val_id != val_id
        {
            if is_self_update_call {
                return None;
            }
            return Some(origin_var.clone());
        }

        if !is_self_update_call && current_version > 0 && self.is_fresh_mutable_aggregate_value(val)
        {
            return Some(origin_var.clone());
        }

        None
    }

    pub(crate) fn resolve_stale_fresh_clone_var(
        &self,
        val_id: usize,
        val: &Value,
        values: &[Value],
    ) -> Option<String> {
        if val.origin_var.is_some() || !self.is_fresh_mutable_aggregate_value(val) {
            return None;
        }
        let mut best: Option<(&str, usize)> = None;
        for (other_val_id, (var, version)) in &self.value_tracker.value_bindings {
            if *other_val_id == val_id {
                continue;
            }
            if self.current_var_version(var) == *version {
                continue;
            }
            let Some(other) = values.get(*other_val_id) else {
                continue;
            };
            if other.kind == val.kind {
                match best {
                    None => best = Some((var.as_str(), *other_val_id)),
                    Some((best_var, best_id))
                        if (var.as_str(), *other_val_id) < (best_var, best_id) =>
                    {
                        best = Some((var.as_str(), *other_val_id));
                    }
                    Some(_) => {}
                }
            }
        }
        best.map(|(var, _)| var.to_string())
    }

    pub(crate) fn call_is_known_fresh_allocation(&self, callee: &str) -> bool {
        matches!(
            callee,
            "rep.int" | "numeric" | "vector" | "matrix" | "seq_len" | "seq_along"
        ) || self.analysis.known_fresh_result_calls.contains(callee)
    }

    pub(crate) fn is_fresh_mutable_aggregate_value(&self, val: &Value) -> bool {
        matches!(
            &val.kind,
            ValueKind::Call { callee, .. }
                if self.call_is_known_fresh_allocation(callee)
        )
    }

    pub(crate) fn should_prefer_stale_var_over_expr(val: &Value) -> bool {
        !matches!(val.value_ty.shape, ShapeTy::Scalar)
            || matches!(
                val.value_term,
                TypeTerm::Any
                    | TypeTerm::Vector(_)
                    | TypeTerm::VectorLen(_, _)
                    | TypeTerm::Matrix(_)
                    | TypeTerm::MatrixDim(_, _, _)
                    | TypeTerm::ArrayDim(_, _)
                    | TypeTerm::DataFrame(_)
                    | TypeTerm::DataFrameNamed(_)
                    | TypeTerm::NamedList(_)
                    | TypeTerm::List(_)
                    | TypeTerm::Boxed(_)
                    | TypeTerm::Option(_)
                    | TypeTerm::Union(_)
            )
    }

    pub(crate) fn bump_base_version_if_named(&mut self, base: usize, values: &[Value]) {
        if let Some(var) = values[base].origin_var.as_ref() {
            self.note_var_write(var);
        }
    }

    pub(crate) fn resolve_mutable_base(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        if let Some(origin_var) = values[val_id].origin_var.as_ref() {
            return origin_var.clone();
        }
        if let Some(bound) = self.resolve_bound_value(val_id) {
            return bound;
        }
        self.resolve_val(val_id, values, params, false)
    }

    pub(crate) fn resolve_read_base(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        if let Some(bound) = self.resolve_bound_value(val_id) {
            return bound;
        }
        if let ValueKind::Call { callee, .. } = &values[val_id].kind
            && callee.contains("::")
            && let Some(origin_var) = values[val_id].origin_var.as_ref()
        {
            return origin_var.clone();
        }
        self.resolve_val(val_id, values, params, false)
    }
}
