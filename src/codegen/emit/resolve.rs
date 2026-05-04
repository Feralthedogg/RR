use super::*;

impl RBackend {
    pub(crate) fn single_positional_call_arg(
        values: &[Value],
        val_id: usize,
        callee_name: &str,
    ) -> Option<usize> {
        let ValueKind::Call {
            callee,
            args,
            names,
        } = &values.get(val_id)?.kind
        else {
            return None;
        };
        if callee != callee_name || args.len() != 1 {
            return None;
        }
        if names
            .first()
            .and_then(std::option::Option::as_ref)
            .is_some()
        {
            return None;
        }
        Some(args[0])
    }

    pub(crate) fn negated_single_positional_call_arg(
        values: &[Value],
        val_id: usize,
        callee_name: &str,
    ) -> Option<usize> {
        let ValueKind::Unary {
            op: UnaryOp::Not,
            rhs,
        } = &values.get(val_id)?.kind
        else {
            return None;
        };
        Self::single_positional_call_arg(values, *rhs, callee_name)
    }

    pub(crate) fn eq_zero_operand(values: &[Value], val_id: usize) -> Option<usize> {
        let ValueKind::Binary {
            op: BinOp::Eq,
            lhs,
            rhs,
        } = &values.get(val_id)?.kind
        else {
            return None;
        };
        match (&values.get(*lhs)?.kind, &values.get(*rhs)?.kind) {
            (ValueKind::Const(Lit::Int(0)), _) => Some(*rhs),
            (ValueKind::Const(Lit::Float(v)), _) if *v == 0.0 => Some(*rhs),
            (_, ValueKind::Const(Lit::Int(0))) => Some(*lhs),
            (_, ValueKind::Const(Lit::Float(v))) if *v == 0.0 => Some(*lhs),
            _ => None,
        }
    }

    pub(crate) fn try_simplify_same_var_non_finite_guard(
        &self,
        lhs: usize,
        rhs: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        for (is_na_side, not_finite_side) in [(lhs, rhs), (rhs, lhs)] {
            let na_arg = Self::single_positional_call_arg(values, is_na_side, "is.na")?;
            let finite_arg =
                Self::negated_single_positional_call_arg(values, not_finite_side, "is.finite")?;
            let na_expr = self.resolve_preferred_plain_symbol_expr(na_arg, values, params);
            let finite_expr = self.resolve_preferred_plain_symbol_expr(finite_arg, values, params);
            if na_expr == finite_expr {
                return Some(format!("!(is.finite({finite_expr}))"));
            }
        }
        None
    }

    pub(crate) fn try_simplify_not_finite_or_zero_guard(
        &self,
        lhs: usize,
        rhs: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        for (not_finite_side, zero_side) in [(lhs, rhs), (rhs, lhs)] {
            let finite_arg =
                Self::negated_single_positional_call_arg(values, not_finite_side, "is.finite")?;
            let zero_arg = Self::eq_zero_operand(values, zero_side)?;
            let finite_expr = self.resolve_preferred_plain_symbol_expr(finite_arg, values, params);
            let zero_expr = self.resolve_preferred_plain_symbol_expr(zero_arg, values, params);
            if finite_expr == zero_expr {
                return Some(format!(
                    "(!(is.finite({finite_expr})) | ({zero_expr} == 0))"
                ));
            }
        }
        None
    }

    pub(crate) fn named_mutable_base_expr(
        val_id: usize,
        values: &[Value],
        value_bindings: &FxHashMap<usize, (String, u64)>,
        var_versions: &FxHashMap<String, u64>,
    ) -> Option<String> {
        if let Some((var, version)) = value_bindings.get(&val_id)
            && var_versions.get(var).copied().unwrap_or(0) == *version
        {
            return Some(var.clone());
        }
        match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Load { .. } | ValueKind::Param { .. }) => {
                values.get(val_id).and_then(|v| v.origin_var.clone())
            }
            _ => None,
        }
    }

    pub(crate) fn resolve_val(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
        prefer_expr: bool,
    ) -> String {
        let val = &values[val_id];

        if !prefer_expr
            && let ValueKind::Load { var } = &val.kind
            && self
                .loop_analysis
                .active_scalar_loop_indices
                .iter()
                .rev()
                .any(|ctx| ctx.var == *var)
        {
            return var.clone();
        }

        if !prefer_expr && Self::should_prefer_stale_var_over_expr(val) {
            if let Some(origin_var) = self.resolve_stale_origin_var(val_id, val, values) {
                return origin_var;
            }
            if let Some(origin_var) = self.resolve_stale_fresh_clone_var(val_id, val, values) {
                return origin_var;
            }
        }

        if !prefer_expr && let Some(bound) = self.resolve_bound_value(val_id) {
            return bound;
        }

        let should_use_name = !prefer_expr
            && val.origin_var.is_some()
            && matches!(val.kind, ValueKind::Load { .. } | ValueKind::Param { .. });
        if should_use_name && let Some(origin_var) = &val.origin_var {
            return origin_var.clone();
        }

        match &val.kind {
            ValueKind::Const(lit) => self.emit_lit_with_value(lit, val),
            ValueKind::Phi { .. } => {
                "rr_fail(\"RR.InternalError\", \"ICE9001\", \"phi reached codegen\", \"codegen\")"
                    .to_string()
            }
            ValueKind::Param { index } => self.resolve_param(*index, params),
            ValueKind::RecordLit { fields } => {
                let rendered = fields
                    .iter()
                    .map(|(name, value)| {
                        format!(
                            "{name} = {}",
                            self.resolve_preferred_plain_symbol_expr(*value, values, params)
                        )
                    })
                    .collect::<Vec<_>>();
                format!("list({})", rendered.join(", "))
            }
            ValueKind::FieldGet { base, field } => {
                let base = self.resolve_preferred_plain_symbol_expr(*base, values, params);
                format!(r#"{base}[["{field}"]]"#)
            }
            ValueKind::FieldSet { base, field, value } => {
                let base = self.resolve_preferred_plain_symbol_expr(*base, values, params);
                let value = self.resolve_preferred_plain_symbol_expr(*value, values, params);
                format!(r#"rr_field_set({base}, "{field}", {value})"#)
            }
            ValueKind::Binary { op, lhs, rhs } => {
                self.resolve_binary_expr(val, *op, *lhs, *rhs, values, params)
            }
            ValueKind::Unary { op, rhs } => self.resolve_unary_expr(*op, *rhs, values, params),
            ValueKind::Call {
                callee,
                args,
                names,
            } => self.resolve_call_expr(val, callee, args, names, values, params),
            ValueKind::Intrinsic { op, args } => {
                self.resolve_intrinsic_expr(*op, args, values, params)
            }
            ValueKind::Len { base } => {
                format!(
                    "length({})",
                    self.resolve_preferred_plain_symbol_expr(*base, values, params)
                )
            }
            ValueKind::Range { start, end } => {
                format!(
                    "{}:{}",
                    self.resolve_preferred_plain_symbol_expr(*start, values, params),
                    self.resolve_preferred_plain_symbol_expr(*end, values, params)
                )
            }
            ValueKind::Indices { base } => {
                format!(
                    "(seq_along({}) - 1L)",
                    self.resolve_preferred_plain_symbol_expr(*base, values, params)
                )
            }
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => self.resolve_index1d_expr(
                *base,
                *idx,
                index_emit::IndexReadSafety {
                    bounds_safe: *is_safe,
                    na_safe: *is_na_safe,
                },
                values,
                params,
            ),
            ValueKind::Index2D { base, r, c } => {
                self.resolve_index2d_expr(*base, *r, *c, values, params)
            }
            ValueKind::Index3D { base, i, j, k } => {
                self.resolve_index3d_expr(*base, *i, *j, *k, values, params)
            }
            ValueKind::Load { var } => self
                .resolve_readonly_arg_alias_name(var, values)
                .unwrap_or_else(|| var.clone()),
            ValueKind::RSymbol { name } => name.clone(),
        }
    }

    pub(crate) fn resolve_param(&self, index: usize, params: &[String]) -> String {
        if index < params.len() {
            params[index].clone()
        } else {
            format!(".p{}", index)
        }
    }

    pub(crate) fn resolve_binary_expr(
        &self,
        val: &Value,
        op: BinOp,
        lhs: usize,
        rhs: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        let mut l = self.resolve_preferred_plain_symbol_expr(lhs, values, params);
        let mut r = self.resolve_preferred_plain_symbol_expr(rhs, values, params);
        if let Some(bound_l) = self.resolve_bound_value(lhs)
            && Self::is_plain_symbol_expr(bound_l.as_str())
            && !bound_l.starts_with('.')
        {
            l = bound_l;
        }
        if let Some(bound_r) = self.resolve_bound_value(rhs)
            && Self::is_plain_symbol_expr(bound_r.as_str())
            && !bound_r.starts_with('.')
        {
            r = bound_r;
        }
        if let Some(origin_var) = self.resolve_live_const_origin_var(lhs, values) {
            l = origin_var;
        }
        if let Some(origin_var) = self.resolve_live_const_origin_var(rhs, values) {
            r = origin_var;
        }
        if matches!(op, BinOp::Mul | BinOp::Div | BinOp::Mod) {
            if let Some(origin_var) = values[lhs].origin_var.as_deref()
                && matches!(values[lhs].kind, ValueKind::Const(_))
                && r == origin_var
            {
                l = origin_var.to_string();
            }
            if let Some(origin_var) = values[rhs].origin_var.as_deref()
                && matches!(values[rhs].kind, ValueKind::Const(_))
                && l == origin_var
            {
                r = origin_var.to_string();
            }
        }
        if matches!(op, BinOp::Or) {
            if let Some(simplified) =
                self.try_simplify_same_var_non_finite_guard(lhs, rhs, values, params)
            {
                return simplified;
            }
            if let Some(simplified) =
                self.try_simplify_not_finite_or_zero_guard(lhs, rhs, values, params)
            {
                return simplified;
            }
        }
        if matches!(op, BinOp::Add)
            && (matches!(values[lhs].kind, ValueKind::Const(Lit::Str(_)))
                || matches!(values[rhs].kind, ValueKind::Const(Lit::Str(_))))
        {
            return format!("paste0({}, {})", l, r);
        }
        let ty = val.value_ty;
        if self.analysis.direct_builtin_vector_math
            && ty.shape == ShapeTy::Vector
            && ty.prim == PrimTy::Double
        {
            return format!("({} {} {})", l, Self::binary_op_str(op), r);
        }
        if ty.shape == ShapeTy::Vector && ty.prim == PrimTy::Double {
            match op {
                BinOp::Add => return format!("rr_parallel_vec_add_f64({}, {})", l, r),
                BinOp::Sub => return format!("rr_parallel_vec_sub_f64({}, {})", l, r),
                BinOp::Mul => return format!("rr_parallel_vec_mul_f64({}, {})", l, r),
                BinOp::Div => return format!("rr_parallel_vec_div_f64({}, {})", l, r),
                _ => {}
            }
        }
        format!("({} {} {})", l, Self::binary_op_str(op), r)
    }

    pub(crate) fn resolve_live_const_origin_var(
        &self,
        val_id: usize,
        values: &[Value],
    ) -> Option<String> {
        let val = values.get(val_id)?;
        let ValueKind::Const(_) = &val.kind else {
            return None;
        };
        let origin_var = val.origin_var.as_ref()?;
        let (bound_val_id, version) = *self.value_tracker.var_value_bindings.get(origin_var)?;
        if self.current_var_version(origin_var) != version {
            return None;
        }
        let bound = values.get(bound_val_id)?;
        if bound.kind == val.kind {
            return Some(origin_var.clone());
        }
        None
    }

    pub(crate) fn resolve_unary_expr(
        &self,
        op: UnaryOp,
        rhs: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        if matches!(op, UnaryOp::Not)
            && let Some(arg) = Self::single_positional_call_arg(values, rhs, "is.finite")
        {
            let rendered = self.resolve_preferred_plain_symbol_expr(arg, values, params);
            return format!("!(is.finite({rendered}))");
        }
        if matches!(op, UnaryOp::Neg) {
            match values.get(rhs).map(|value| &value.kind) {
                Some(ValueKind::Const(Lit::Int(v))) => {
                    if let Some(negated) = v.checked_neg() {
                        return format!("{negated}L");
                    }
                }
                Some(ValueKind::Const(Lit::Float(v))) => {
                    return self.emit_float_lit(-v);
                }
                _ => {}
            }
        }
        let r = self.resolve_preferred_plain_symbol_expr(rhs, values, params);
        format!("({}({}))", Self::unary_op_str(op), r)
    }

    pub(crate) fn resolve_call_expr(
        &self,
        val: &Value,
        callee: &str,
        args: &[usize],
        names: &[Option<String>],
        values: &[Value],
        params: &[String],
    ) -> String {
        if matches!(callee, "rr_index1_read" | "rr_index1_read_floor")
            && (args.len() == 2 || args.len() == 3)
            && names.iter().take(2).all(std::option::Option::is_none)
            && self.can_elide_index_expr(args[1], values, params)
        {
            let base = self.resolve_preferred_plain_symbol_expr(args[0], values, params);
            let idx = self.resolve_preferred_plain_symbol_expr(args[1], values, params);
            return format!("{}[{}]", base, idx);
        }
        if callee == "rr_index1_write"
            && (args.len() == 1 || args.len() == 2)
            && names
                .first()
                .and_then(std::option::Option::as_ref)
                .is_none()
            && self.can_elide_index_expr(args[0], values, params)
        {
            return self.resolve_preferred_plain_symbol_expr(args[0], values, params);
        }
        if matches!(callee, "rr_index1_read_vec" | "rr_index1_read_vec_floor")
            && args.len() >= 2
            && names.iter().take(2).all(std::option::Option::is_none)
        {
            let base = args[0];
            let idx = args[1];
            if let Some(end_expr) = self.known_full_end_expr_for_value(base, values, params)
                && self.value_is_one_based_full_range_alias(
                    idx,
                    end_expr.as_str(),
                    values,
                    params,
                    &mut FxHashSet::default(),
                )
            {
                return self.resolve_preferred_plain_symbol_expr(base, values, params);
            }
        }
        if let Some((base, idx)) = Self::floor_index_read_components(callee, args, names, values) {
            if let Some(end_expr) = self.known_full_end_expr_for_value(base, values, params)
                && self.value_is_one_based_full_range_alias(
                    idx,
                    end_expr.as_str(),
                    values,
                    params,
                    &mut FxHashSet::default(),
                )
            {
                return self.resolve_preferred_plain_symbol_expr(base, values, params);
            }
            let b = self.resolve_preferred_plain_symbol_expr(base, values, params);
            let i = self.resolve_preferred_plain_symbol_expr(idx, values, params);
            return format!("rr_index1_read_idx({}, {}, \"index\")", b, i);
        }
        if callee == "rr_named_list"
            && names.iter().all(Option::is_none)
            && args.len().is_multiple_of(2)
        {
            let mut fields = Vec::new();
            let mut ok = true;
            for pair in args.chunks(2) {
                match values.get(pair[0]).map(|value| &value.kind) {
                    Some(ValueKind::Const(Lit::Str(name))) => {
                        fields.push(format!(
                            "{} = {}",
                            name,
                            self.resolve_preferred_plain_symbol_expr(pair[1], values, params)
                        ));
                    }
                    _ => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                return format!("list({})", fields.join(", "));
            }
        }
        if callee == "rr_field_get"
            && args.len() == 2
            && names.iter().all(Option::is_none)
            && let Some(ValueKind::Const(Lit::Str(name))) =
                values.get(args[1]).map(|value| &value.kind)
        {
            let base = self.resolve_preferred_plain_symbol_expr(args[0], values, params);
            return format!(r#"{base}[["{name}"]]"#);
        }
        if Self::can_elide_identity_floor_call(callee, args, names, values) {
            return self.resolve_preferred_plain_symbol_expr(args[0], values, params);
        }
        if !self.analysis.direct_builtin_vector_math
            && val.value_ty.shape == ShapeTy::Vector
            && val.value_ty.prim == PrimTy::Double
            && names.iter().all(Option::is_none)
        {
            let resolved: Vec<String> = args
                .iter()
                .map(|arg| self.resolve_preferred_plain_symbol_expr(*arg, values, params))
                .collect();
            match (callee, resolved.as_slice()) {
                ("abs", [arg]) => return format!("rr_intrinsic_vec_abs_f64({arg})"),
                ("log", [arg]) => return format!("rr_intrinsic_vec_log_f64({arg})"),
                ("sqrt", [arg]) => return format!("rr_intrinsic_vec_sqrt_f64({arg})"),
                ("pmax", [lhs, rhs]) => return format!("rr_intrinsic_vec_pmax_f64({lhs}, {rhs})"),
                ("pmin", [lhs, rhs]) => return format!("rr_intrinsic_vec_pmin_f64({lhs}, {rhs})"),
                _ => {}
            }
        }
        if callee == "rr_idx_cube_vec_i" && args.len() == 4 && names.iter().all(Option::is_none) {
            let rendered_args = [
                self.resolve_rr_idx_cube_vec_arg_expr(args[0], values, params),
                self.resolve_rr_idx_cube_vec_arg_expr(args[1], values, params),
                self.resolve_rr_idx_cube_vec_arg_expr(args[2], values, params),
                self.resolve_preferred_plain_symbol_expr(args[3], values, params),
            ];
            return format!(
                "rr_idx_cube_vec_i({}, {}, {}, {})",
                rendered_args[0], rendered_args[1], rendered_args[2], rendered_args[3]
            );
        }
        let arg_list = self.build_named_arg_list(args, names, values, params);
        let rendered_callee = Self::emitted_callee_name(callee);
        format!("{}({})", rendered_callee, arg_list)
    }

    pub(crate) fn resolve_rr_idx_cube_vec_arg_expr(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        self.try_resolve_singleton_replace_expr(val_id, values, params)
            .or_else(|| {
                self.try_render_singleton_assign_call_with_scalar_rhs(val_id, values, params)
            })
            .unwrap_or_else(|| self.resolve_preferred_plain_symbol_expr(val_id, values, params))
    }

    pub(crate) fn try_resolve_singleton_replace_expr(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        let ValueKind::Call { callee, args, .. } = &values.get(val_id)?.kind else {
            return None;
        };
        if *callee != "rr_assign_slice" || args.len() < 4 {
            return None;
        }
        let start_expr =
            self.resolve_bound_temp_expr(args[1], values, params, &mut FxHashSet::default());
        let end_expr =
            self.resolve_bound_temp_expr(args[2], values, params, &mut FxHashSet::default());
        if start_expr != end_expr {
            return None;
        }
        let boundary_ok = self.value_is_known_one(args[1], values)
            || self.value_is_full_dest_end(
                args[0],
                args[2],
                values,
                params,
                &mut FxHashSet::default(),
            )
            || self
                .resolve_named_mutable_base_var(args[0], values, params)
                .is_some_and(|base_var| {
                    self.whole_dest_end_matches_known_var(
                        base_var.as_str(),
                        args[2],
                        values,
                        params,
                    )
                });
        if !boundary_ok {
            return None;
        }
        let scalar = self.resolve_singleton_assign_scalar_expr(args[3], values, params)?;
        let base = self.resolve_bound_temp_expr(args[0], values, params, &mut FxHashSet::default());
        Some(format!("replace({}, {}, {})", base, start_expr, scalar))
    }

    pub(crate) fn resolve_singleton_assign_scalar_expr(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Call { callee, args, .. })
                if *callee == "rep.int"
                    && args.len() >= 2
                    && self.value_is_known_one(args[1], values) =>
            {
                if self.can_reuse_live_expr_alias(args[0], values) {
                    let preferred = self.resolve_preferred_live_expr_alias(args[0], values, params);
                    if Self::is_plain_symbol_expr(preferred.as_str()) && !preferred.starts_with('.')
                    {
                        return Some(preferred);
                    }
                }
                Some(self.resolve_bound_temp_expr(
                    args[0],
                    values,
                    params,
                    &mut FxHashSet::default(),
                ))
            }
            _ if self.value_is_scalar_shape(val_id, values) => {
                if matches!(
                    values.get(val_id).map(|value| &value.kind),
                    Some(
                        ValueKind::Binary { .. }
                            | ValueKind::Unary { .. }
                            | ValueKind::Call { .. }
                            | ValueKind::FieldGet { .. }
                            | ValueKind::Len { .. }
                    )
                ) && self.can_reuse_live_expr_alias(val_id, values)
                {
                    let preferred = self.resolve_preferred_live_expr_alias(val_id, values, params);
                    if Self::is_plain_symbol_expr(preferred.as_str()) && !preferred.starts_with('.')
                    {
                        return Some(preferred);
                    }
                }
                Some(self.resolve_bound_temp_expr(
                    val_id,
                    values,
                    params,
                    &mut FxHashSet::default(),
                ))
            }
            _ => None,
        }
    }

    pub(crate) fn try_render_singleton_assign_call_with_scalar_rhs(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        let ValueKind::Call { callee, args, .. } = &values.get(val_id)?.kind else {
            return None;
        };
        if *callee != "rr_assign_slice" || args.len() < 4 {
            return None;
        }
        let start_expr = self.resolve_preferred_plain_symbol_expr(args[1], values, params);
        let end_expr = self.resolve_preferred_plain_symbol_expr(args[2], values, params);
        if start_expr != end_expr {
            return None;
        }
        let scalar = self.resolve_singleton_assign_scalar_expr(args[3], values, params)?;
        let base = self.resolve_bound_temp_expr(args[0], values, params, &mut FxHashSet::default());
        let start_expr =
            self.resolve_bound_temp_expr(args[1], values, params, &mut FxHashSet::default());
        let end_expr =
            self.resolve_bound_temp_expr(args[2], values, params, &mut FxHashSet::default());
        Some(format!(
            "rr_assign_slice({}, {}, {}, {})",
            base, start_expr, end_expr, scalar
        ))
    }

    pub(crate) fn resolve_intrinsic_expr(
        &self,
        op: IntrinsicOp,
        args: &[usize],
        values: &[Value],
        params: &[String],
    ) -> String {
        let has_matrix_arg = args
            .iter()
            .any(|arg| values[*arg].value_ty.shape == ShapeTy::Matrix);
        if self.analysis.direct_builtin_vector_math && !has_matrix_arg {
            let resolved: Vec<String> = args
                .iter()
                .map(|arg| self.resolve_preferred_plain_symbol_expr(*arg, values, params))
                .collect();
            return match op {
                IntrinsicOp::VecAddF64 => format!("({} + {})", resolved[0], resolved[1]),
                IntrinsicOp::VecSubF64 => format!("({} - {})", resolved[0], resolved[1]),
                IntrinsicOp::VecMulF64 => format!("({} * {})", resolved[0], resolved[1]),
                IntrinsicOp::VecDivF64 => format!("({} / {})", resolved[0], resolved[1]),
                IntrinsicOp::VecAbsF64 => format!("abs({})", resolved[0]),
                IntrinsicOp::VecLogF64 => format!("log({})", resolved[0]),
                IntrinsicOp::VecSqrtF64 => format!("sqrt({})", resolved[0]),
                IntrinsicOp::VecPmaxF64 => format!("pmax({}, {})", resolved[0], resolved[1]),
                IntrinsicOp::VecPminF64 => format!("pmin({}, {})", resolved[0], resolved[1]),
                IntrinsicOp::VecSumF64 => format!("sum({})", resolved[0]),
                IntrinsicOp::VecMeanF64 => format!("mean({})", resolved[0]),
            };
        }
        if has_matrix_arg {
            let resolved: Vec<String> = args
                .iter()
                .map(|arg| self.resolve_preferred_plain_symbol_expr(*arg, values, params))
                .collect();
            return match op {
                IntrinsicOp::VecAddF64 => format!("({} + {})", resolved[0], resolved[1]),
                IntrinsicOp::VecSubF64 => format!("({} - {})", resolved[0], resolved[1]),
                IntrinsicOp::VecMulF64 => format!("({} * {})", resolved[0], resolved[1]),
                IntrinsicOp::VecDivF64 => format!("({} / {})", resolved[0], resolved[1]),
                IntrinsicOp::VecAbsF64 => format!("abs({})", resolved[0]),
                IntrinsicOp::VecLogF64 => format!("log({})", resolved[0]),
                IntrinsicOp::VecSqrtF64 => format!("sqrt({})", resolved[0]),
                IntrinsicOp::VecPmaxF64 => format!("pmax({}, {})", resolved[0], resolved[1]),
                IntrinsicOp::VecPminF64 => format!("pmin({}, {})", resolved[0], resolved[1]),
                IntrinsicOp::VecSumF64 => format!("sum({})", resolved[0]),
                IntrinsicOp::VecMeanF64 => format!("mean({})", resolved[0]),
            };
        }
        let arg_list = self.build_plain_arg_list(args, values, params);
        format!("{}({})", Self::intrinsic_helper(op), arg_list)
    }
}
