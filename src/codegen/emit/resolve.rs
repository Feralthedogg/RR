use super::*;

impl RBackend {
    pub(super) fn named_mutable_base_expr(
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

    pub(super) fn resolve_val(
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
                            self.resolve_val(*value, values, params, false)
                        )
                    })
                    .collect::<Vec<_>>();
                format!("list({})", rendered.join(", "))
            }
            ValueKind::FieldGet { base, field } => {
                let base = self.resolve_val(*base, values, params, false);
                format!(r#"{base}[["{field}"]]"#)
            }
            ValueKind::FieldSet { base, field, value } => {
                let base = self.resolve_val(*base, values, params, false);
                let value = self.resolve_val(*value, values, params, false);
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
                format!("length({})", self.resolve_val(*base, values, params, false))
            }
            ValueKind::Range { start, end } => {
                format!(
                    "{}:{}",
                    self.resolve_val(*start, values, params, false),
                    self.resolve_val(*end, values, params, false)
                )
            }
            ValueKind::Indices { base } => {
                format!(
                    "(seq_along({}) - 1L)",
                    self.resolve_val(*base, values, params, false)
                )
            }
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => self.resolve_index1d_expr(*base, *idx, *is_safe, *is_na_safe, values, params),
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

    pub(super) fn resolve_param(&self, index: usize, params: &[String]) -> String {
        if index < params.len() {
            params[index].clone()
        } else {
            format!(".p{}", index)
        }
    }

    pub(super) fn resolve_binary_expr(
        &self,
        val: &Value,
        op: BinOp,
        lhs: usize,
        rhs: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        let mut l = self.resolve_val(lhs, values, params, false);
        let mut r = self.resolve_val(rhs, values, params, false);
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

    pub(super) fn resolve_live_const_origin_var(
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

    pub(super) fn resolve_unary_expr(
        &self,
        op: UnaryOp,
        rhs: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
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
        let r = self.resolve_val(rhs, values, params, false);
        format!("({}({}))", Self::unary_op_str(op), r)
    }

    pub(super) fn resolve_call_expr(
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
            let base = self.resolve_val(args[0], values, params, false);
            let idx = self.resolve_val(args[1], values, params, false);
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
            return self.resolve_val(args[0], values, params, false);
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
                return self.resolve_val(base, values, params, false);
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
                return self.resolve_val(base, values, params, false);
            }
            let b = self.resolve_val(base, values, params, false);
            let i = self.resolve_val(idx, values, params, false);
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
                            self.resolve_val(pair[1], values, params, false)
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
            let base = self.resolve_val(args[0], values, params, false);
            return format!(r#"{base}[["{name}"]]"#);
        }
        if Self::can_elide_identity_floor_call(callee, args, names, values) {
            return self.resolve_val(args[0], values, params, false);
        }
        if !self.analysis.direct_builtin_vector_math
            && val.value_ty.shape == ShapeTy::Vector
            && val.value_ty.prim == PrimTy::Double
            && names.iter().all(Option::is_none)
        {
            let resolved: Vec<String> = args
                .iter()
                .map(|arg| self.resolve_val(*arg, values, params, false))
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
                self.resolve_val(args[3], values, params, false),
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

    pub(super) fn resolve_rr_idx_cube_vec_arg_expr(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        self.try_resolve_singleton_replace_expr(val_id, values, params)
            .or_else(|| {
                self.try_render_singleton_assign_call_with_scalar_rhs(val_id, values, params)
            })
            .unwrap_or_else(|| self.resolve_val(val_id, values, params, false))
    }

    pub(super) fn try_resolve_singleton_replace_expr(
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

    pub(super) fn resolve_singleton_assign_scalar_expr(
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
                Some(self.resolve_bound_temp_expr(
                    args[0],
                    values,
                    params,
                    &mut FxHashSet::default(),
                ))
            }
            _ if self.value_is_scalar_shape(val_id, values) => Some(self.resolve_bound_temp_expr(
                val_id,
                values,
                params,
                &mut FxHashSet::default(),
            )),
            _ => None,
        }
    }

    pub(super) fn try_render_singleton_assign_call_with_scalar_rhs(
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
        let start_expr = self.resolve_val(args[1], values, params, false);
        let end_expr = self.resolve_val(args[2], values, params, false);
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

    pub(super) fn resolve_intrinsic_expr(
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
                .map(|arg| self.resolve_val(*arg, values, params, false))
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
                .map(|arg| self.resolve_val(*arg, values, params, false))
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
