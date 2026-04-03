use super::*;

impl RBackend {
    pub(super) fn try_emit_full_range_conditional_loop_sequence(
        &mut self,
        items: &[StructuredBlock],
        fn_ir: &FnIR,
    ) -> Option<usize> {
        if items.len() < 2 {
            return None;
        }
        let StructuredBlock::BasicBlock(init_bb) = items.first()? else {
            return None;
        };
        let StructuredBlock::Loop {
            header: _,
            cond,
            continue_on_true,
            body,
        } = items.get(1)?
        else {
            return None;
        };
        if !continue_on_true {
            return None;
        }
        let init_block = &fn_ir.blocks[*init_bb];
        let [
            Instr::Assign {
                dst: idx_var, src, ..
            },
        ] = init_block.instrs.as_slice()
        else {
            return None;
        };
        if !is_recognized_loop_index_name(idx_var) || !self.value_is_known_one(*src, &fn_ir.values)
        {
            return None;
        }
        if items[2..]
            .iter()
            .any(|item| self.structured_uses_var(item, fn_ir, idx_var))
        {
            return None;
        }
        let (guard_var, end_val) = self.extract_full_range_loop_guard(*cond, idx_var, fn_ir)?;
        if guard_var != *idx_var {
            return None;
        }
        let (branch_cond, then_bb, else_bb, incr_bb) =
            self.extract_conditional_loop_shape(body.as_ref())?;
        let (dest_var, then_val) =
            self.extract_conditional_loop_store(then_bb, idx_var, end_val, fn_ir)?;
        let (else_dest_var, else_val) =
            self.extract_conditional_loop_store(else_bb, idx_var, end_val, fn_ir)?;
        if dest_var != else_dest_var || !self.loop_increment_matches(incr_bb, idx_var, fn_ir) {
            return None;
        }

        let cond_expr = self.resolve_full_range_loop_expr(
            branch_cond,
            idx_var,
            &fn_ir.values,
            &fn_ir.params,
            &mut FxHashSet::default(),
        )?;
        let then_expr = self.resolve_full_range_loop_expr(
            then_val,
            idx_var,
            &fn_ir.values,
            &fn_ir.params,
            &mut FxHashSet::default(),
        )?;
        let else_expr = self.resolve_full_range_loop_expr(
            else_val,
            idx_var,
            &fn_ir.values,
            &fn_ir.params,
            &mut FxHashSet::default(),
        )?;

        let cond_span = fn_ir.values[branch_cond].span;
        self.emit_mark(cond_span, Some("loop-vector-ifelse"));
        self.record_span(cond_span);
        self.write_stmt(&format!(
            "{dest_var} <- ifelse(({cond_expr}), {then_expr}, {else_expr})"
        ));
        self.note_var_write(&dest_var);
        self.loop_analysis
            .recent_whole_assign_bases
            .insert(dest_var);
        self.value_tracker.last_assigned_value_ids.clear();
        Some(2)
    }

    pub(super) fn extract_full_range_loop_guard(
        &self,
        cond: usize,
        expected_idx_var: &str,
        fn_ir: &FnIR,
    ) -> Option<(String, usize)> {
        let ValueKind::Binary { op, lhs, rhs } = fn_ir.values.get(cond)?.kind else {
            return None;
        };
        match op {
            BinOp::Le => {
                let idx_var = self.extract_loop_index_var(lhs, &fn_ir.values)?;
                if idx_var == expected_idx_var {
                    Some((idx_var, rhs))
                } else {
                    None
                }
            }
            BinOp::Ge => {
                let idx_var = self.extract_loop_index_var(rhs, &fn_ir.values)?;
                if idx_var == expected_idx_var {
                    Some((idx_var, lhs))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(super) fn extract_loop_index_var(
        &self,
        value_id: usize,
        values: &[Value],
    ) -> Option<String> {
        match values.get(value_id).map(|v| &v.kind) {
            Some(ValueKind::Load { var }) if is_recognized_loop_index_name(var) => {
                Some(var.clone())
            }
            Some(ValueKind::Load { var }) => self
                .resolve_bound_value_id(var)
                .and_then(|bound| self.extract_loop_index_var(bound, values)),
            _ => None,
        }
    }

    pub(super) fn extract_conditional_loop_shape(
        &self,
        body: &StructuredBlock,
    ) -> Option<(usize, usize, usize, usize)> {
        let StructuredBlock::Sequence(items) = body else {
            return None;
        };
        let [
            StructuredBlock::If {
                cond,
                then_body,
                else_body,
            },
            StructuredBlock::BasicBlock(incr_bb),
            StructuredBlock::Next,
        ] = items.as_slice()
        else {
            return None;
        };
        let then_bb = self.single_basic_block(then_body.as_ref())?;
        let else_bb = self.single_basic_block(else_body.as_ref()?.as_ref())?;
        Some((*cond, then_bb, else_bb, *incr_bb))
    }

    pub(super) fn single_basic_block(&self, node: &StructuredBlock) -> Option<usize> {
        match node {
            StructuredBlock::BasicBlock(bb) => Some(*bb),
            StructuredBlock::Sequence(items) if items.len() == 1 => {
                self.single_basic_block(&items[0])
            }
            _ => None,
        }
    }

    pub(super) fn extract_conditional_loop_store(
        &self,
        bb: usize,
        idx_var: &str,
        end_val: usize,
        fn_ir: &FnIR,
    ) -> Option<(String, usize)> {
        let block = &fn_ir.blocks[bb];
        let [Instr::StoreIndex1D { base, idx, val, .. }] = block.instrs.as_slice() else {
            return None;
        };
        if !self.value_matches_loop_index(*idx, idx_var, &fn_ir.values, &mut FxHashSet::default()) {
            return None;
        }
        if !self.value_is_full_dest_end(
            *base,
            end_val,
            &fn_ir.values,
            &fn_ir.params,
            &mut FxHashSet::default(),
        ) {
            return None;
        }
        let dest_var = Self::named_mutable_base_expr(
            *base,
            &fn_ir.values,
            &self.value_tracker.value_bindings,
            &self.value_tracker.var_versions,
        )?;
        Some((dest_var, *val))
    }

    pub(super) fn value_matches_loop_index(
        &self,
        value_id: usize,
        idx_var: &str,
        values: &[Value],
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        if !seen.insert(value_id) {
            return false;
        }
        match values.get(value_id).map(|v| &v.kind) {
            Some(ValueKind::Load { var }) if var == idx_var => true,
            Some(ValueKind::Load { var }) => self
                .resolve_bound_value_id(var)
                .is_some_and(|bound| self.value_matches_loop_index(bound, idx_var, values, seen)),
            _ => false,
        }
    }

    pub(super) fn loop_increment_matches(&self, bb: usize, idx_var: &str, fn_ir: &FnIR) -> bool {
        let block = &fn_ir.blocks[bb];
        let [Instr::Assign { dst, src, .. }] = block.instrs.as_slice() else {
            return false;
        };
        if dst != idx_var {
            return false;
        }
        match fn_ir.values.get(*src).map(|v| &v.kind) {
            Some(ValueKind::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
            }) => {
                (self.value_matches_loop_index(
                    *lhs,
                    idx_var,
                    &fn_ir.values,
                    &mut FxHashSet::default(),
                ) && self.value_is_known_one(*rhs, &fn_ir.values))
                    || (self.value_matches_loop_index(
                        *rhs,
                        idx_var,
                        &fn_ir.values,
                        &mut FxHashSet::default(),
                    ) && self.value_is_known_one(*lhs, &fn_ir.values))
            }
            _ => false,
        }
    }

    pub(super) fn known_small_positive_scalar(
        &self,
        value_id: usize,
        values: &[Value],
    ) -> Option<i64> {
        let value = values.get(value_id)?;
        if value.facts.has(Facts::INT_SCALAR | Facts::NON_NA) && value.facts.interval.min >= 1 {
            let max = value.facts.interval.max;
            let min = value.facts.interval.min;
            if min == max {
                return Some(min);
            }
        }
        match &value.kind {
            ValueKind::Const(Lit::Int(i)) if *i >= 1 => Some(*i),
            ValueKind::Const(Lit::Float(f))
                if f.is_finite()
                    && (*f - f.trunc()).abs() < f64::EPSILON
                    && *f >= 1.0
                    && *f <= i64::MAX as f64 =>
            {
                Some(*f as i64)
            }
            _ => None,
        }
    }

    pub(super) fn extract_scalar_loop_index_context_from_init_bb(
        &self,
        init_bb: usize,
        cond: usize,
        fn_ir: &FnIR,
    ) -> Option<ActiveScalarLoopIndex> {
        let block = &fn_ir.blocks[init_bb];
        let ValueKind::Binary { op, lhs, rhs } = fn_ir.values.get(cond)?.kind else {
            return None;
        };
        let (idx_var, cmp) = match op {
            BinOp::Lt => (
                self.extract_loop_index_var(lhs, &fn_ir.values)?,
                ScalarLoopCmp::Lt,
            ),
            BinOp::Le => (
                self.extract_loop_index_var(lhs, &fn_ir.values)?,
                ScalarLoopCmp::Le,
            ),
            BinOp::Gt => (
                self.extract_loop_index_var(rhs, &fn_ir.values)?,
                ScalarLoopCmp::Lt,
            ),
            BinOp::Ge => (
                self.extract_loop_index_var(rhs, &fn_ir.values)?,
                ScalarLoopCmp::Le,
            ),
            _ => return None,
        };

        let start_min = block
            .instrs
            .iter()
            .filter_map(|instr| match instr {
                Instr::Assign { dst, src, .. } if *dst == idx_var => {
                    self.known_small_positive_scalar(*src, &fn_ir.values)
                }
                _ => None,
            })
            .next()?;

        Some(ActiveScalarLoopIndex {
            var: idx_var,
            start_min,
            cmp,
        })
    }

    pub(super) fn extract_scalar_loop_index_context_from_live_binding(
        &self,
        cond: usize,
        fn_ir: &FnIR,
    ) -> Option<ActiveScalarLoopIndex> {
        let ValueKind::Binary { op, lhs, rhs } = fn_ir.values.get(cond)?.kind else {
            return None;
        };
        let (idx_var, cmp) = match op {
            BinOp::Lt => (
                self.extract_loop_index_var(lhs, &fn_ir.values)?,
                ScalarLoopCmp::Lt,
            ),
            BinOp::Le => (
                self.extract_loop_index_var(lhs, &fn_ir.values)?,
                ScalarLoopCmp::Le,
            ),
            BinOp::Gt => (
                self.extract_loop_index_var(rhs, &fn_ir.values)?,
                ScalarLoopCmp::Lt,
            ),
            BinOp::Ge => (
                self.extract_loop_index_var(rhs, &fn_ir.values)?,
                ScalarLoopCmp::Le,
            ),
            _ => return None,
        };
        let bound = self.resolve_bound_value_id(&idx_var)?;
        let start_min = self.known_small_positive_scalar(bound, &fn_ir.values)?;
        Some(ActiveScalarLoopIndex {
            var: idx_var,
            start_min,
            cmp,
        })
    }

    pub(super) fn generated_loop_index_var_from_header(
        &self,
        header: usize,
        fn_ir: &FnIR,
    ) -> Option<String> {
        self.generated_loop_var_from_block(header, fn_ir)
    }

    pub(super) fn generated_loop_var_from_block(&self, bb: usize, fn_ir: &FnIR) -> Option<String> {
        fn_ir
            .blocks
            .get(bb)?
            .instrs
            .iter()
            .find_map(|instr| match instr {
                Instr::Assign { dst, .. } if is_generated_poly_loop_var_name(dst) => {
                    Some(dst.clone())
                }
                _ => None,
            })
    }

    pub(super) fn loop_index_offset(
        &self,
        value_id: usize,
        ctx: &ActiveScalarLoopIndex,
        values: &[Value],
        seen: &mut FxHashSet<usize>,
    ) -> Option<i64> {
        if !seen.insert(value_id) {
            return None;
        }
        match values.get(value_id).map(|v| &v.kind)? {
            ValueKind::Load { var } if var == &ctx.var => Some(0),
            ValueKind::Load { var } => self
                .resolve_bound_value_id(var)
                .and_then(|bound| self.loop_index_offset(bound, ctx, values, seen)),
            ValueKind::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
            } => {
                if let Some(base) = self.loop_index_offset(*lhs, ctx, values, seen)
                    && let Some(delta) = self.known_small_positive_scalar(*rhs, values)
                {
                    return Some(base.saturating_add(delta));
                }
                if let Some(base) = self.loop_index_offset(*rhs, ctx, values, seen)
                    && let Some(delta) = self.known_small_positive_scalar(*lhs, values)
                {
                    return Some(base.saturating_add(delta));
                }
                None
            }
            ValueKind::Binary {
                op: BinOp::Sub,
                lhs,
                rhs,
            } => {
                let base = self.loop_index_offset(*lhs, ctx, values, seen)?;
                let delta = self.known_small_positive_scalar(*rhs, values)?;
                Some(base.saturating_sub(delta))
            }
            _ => None,
        }
    }

    pub(super) fn loop_context_allows_offset(ctx: &ActiveScalarLoopIndex, offset: i64) -> bool {
        if ctx.start_min.saturating_add(offset) < 1 {
            return false;
        }
        if offset <= 0 {
            return true;
        }
        matches!(ctx.cmp, ScalarLoopCmp::Lt) && offset <= 1
    }

    pub(super) fn rendered_loop_index_offset(
        expr: &str,
        ctx: &ActiveScalarLoopIndex,
    ) -> Option<i64> {
        let mut compact = expr
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>();
        if compact == ctx.var {
            return Some(0);
        }
        if compact.starts_with('(') && compact.ends_with(')') {
            compact = compact[1..compact.len() - 1].to_string();
        }
        let minus_one = format!("{}-1", ctx.var);
        if compact == minus_one || compact == format!("{minus_one}L") {
            return Some(-1);
        }
        let plus_one = format!("{}+1", ctx.var);
        if compact == plus_one || compact == format!("{plus_one}L") {
            return Some(1);
        }
        None
    }

    pub(super) fn resolve_full_range_loop_expr(
        &self,
        val_id: usize,
        idx_var: &str,
        values: &[Value],
        params: &[String],
        seen: &mut FxHashSet<usize>,
    ) -> Option<String> {
        if !seen.insert(val_id) {
            return None;
        }
        let value = values.get(val_id)?;
        match values.get(val_id).map(|v| &v.kind)? {
            ValueKind::Const(lit) => Some(self.emit_lit(lit)),
            ValueKind::Param { index } => Some(self.resolve_param(*index, params)),
            ValueKind::Load { var } if var == idx_var => None,
            ValueKind::Load { var } if var.starts_with('.') => self
                .resolve_bound_value_id(var)
                .filter(|bound| *bound != val_id)
                .and_then(|bound| {
                    self.resolve_full_range_loop_expr(bound, idx_var, values, params, seen)
                }),
            ValueKind::Load { var } => Some(var.clone()),
            ValueKind::Binary { op, lhs, rhs } => {
                let l = self.resolve_full_range_loop_expr(*lhs, idx_var, values, params, seen)?;
                let r = self.resolve_full_range_loop_expr(*rhs, idx_var, values, params, seen)?;
                if matches!(
                    op,
                    BinOp::Eq
                        | BinOp::Ne
                        | BinOp::Lt
                        | BinOp::Le
                        | BinOp::Gt
                        | BinOp::Ge
                        | BinOp::Add
                        | BinOp::Sub
                        | BinOp::Mul
                        | BinOp::Div
                        | BinOp::Mod
                        | BinOp::And
                        | BinOp::Or
                ) {
                    Some(format!("({l} {} {r})", Self::binary_op_str(*op)))
                } else {
                    None
                }
            }
            ValueKind::Unary { op, rhs } => {
                let r = self.resolve_full_range_loop_expr(*rhs, idx_var, values, params, seen)?;
                Some(format!("({}({}))", Self::unary_op_str(*op), r))
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } => {
                if callee.starts_with("rr_") || names.iter().any(|name| name.is_some()) {
                    return None;
                }
                let rendered_args: Option<Vec<String>> = args
                    .iter()
                    .map(|arg| {
                        self.resolve_full_range_loop_expr(*arg, idx_var, values, params, seen)
                    })
                    .collect();
                let rendered_args = rendered_args?;
                if !self.analysis.direct_builtin_vector_math
                    && value.value_ty.shape == ShapeTy::Vector
                    && value.value_ty.prim == PrimTy::Double
                {
                    match (callee.as_str(), rendered_args.as_slice()) {
                        ("abs", [arg]) => return Some(format!("rr_intrinsic_vec_abs_f64({arg})")),
                        ("log", [arg]) => return Some(format!("rr_intrinsic_vec_log_f64({arg})")),
                        ("sqrt", [arg]) => {
                            return Some(format!("rr_intrinsic_vec_sqrt_f64({arg})"));
                        }
                        ("pmax", [lhs, rhs]) => {
                            return Some(format!("rr_intrinsic_vec_pmax_f64({lhs}, {rhs})"));
                        }
                        ("pmin", [lhs, rhs]) => {
                            return Some(format!("rr_intrinsic_vec_pmin_f64({lhs}, {rhs})"));
                        }
                        _ => {}
                    }
                }
                let rendered_callee = Self::emitted_callee_name(callee);
                Some(format!("{}({})", rendered_callee, rendered_args.join(", ")))
            }
            ValueKind::Intrinsic { op, args } => {
                let rendered_args: Option<Vec<String>> = args
                    .iter()
                    .map(|arg| {
                        self.resolve_full_range_loop_expr(*arg, idx_var, values, params, seen)
                    })
                    .collect();
                let rendered_args = rendered_args?;
                if !self.analysis.direct_builtin_vector_math {
                    match (op, rendered_args.as_slice()) {
                        (IntrinsicOp::VecAbsF64, [arg]) => {
                            return Some(format!("rr_intrinsic_vec_abs_f64({arg})"));
                        }
                        (IntrinsicOp::VecLogF64, [arg]) => {
                            return Some(format!("rr_intrinsic_vec_log_f64({arg})"));
                        }
                        (IntrinsicOp::VecSqrtF64, [arg]) => {
                            return Some(format!("rr_intrinsic_vec_sqrt_f64({arg})"));
                        }
                        (IntrinsicOp::VecPmaxF64, [lhs, rhs]) => {
                            return Some(format!("rr_intrinsic_vec_pmax_f64({lhs}, {rhs})"));
                        }
                        (IntrinsicOp::VecPminF64, [lhs, rhs]) => {
                            return Some(format!("rr_intrinsic_vec_pmin_f64({lhs}, {rhs})"));
                        }
                        _ => {}
                    }
                }
                match (op, rendered_args.as_slice()) {
                    (IntrinsicOp::VecAddF64, [lhs, rhs]) => Some(format!("({lhs} + {rhs})")),
                    (IntrinsicOp::VecSubF64, [lhs, rhs]) => Some(format!("({lhs} - {rhs})")),
                    (IntrinsicOp::VecMulF64, [lhs, rhs]) => Some(format!("({lhs} * {rhs})")),
                    (IntrinsicOp::VecDivF64, [lhs, rhs]) => Some(format!("({lhs} / {rhs})")),
                    (IntrinsicOp::VecAbsF64, [arg]) => Some(format!("abs({arg})")),
                    (IntrinsicOp::VecLogF64, [arg]) => Some(format!("log({arg})")),
                    (IntrinsicOp::VecSqrtF64, [arg]) => Some(format!("sqrt({arg})")),
                    (IntrinsicOp::VecPmaxF64, [lhs, rhs]) => Some(format!("pmax({lhs}, {rhs})")),
                    (IntrinsicOp::VecPminF64, [lhs, rhs]) => Some(format!("pmin({lhs}, {rhs})")),
                    _ => None,
                }
            }
            ValueKind::Index1D { base, idx, .. } => {
                if self.value_matches_loop_index(*idx, idx_var, values, &mut FxHashSet::default()) {
                    Some(self.resolve_read_base(*base, values, params))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(super) fn structured_uses_var(
        &self,
        node: &StructuredBlock,
        fn_ir: &FnIR,
        var: &str,
    ) -> bool {
        match node {
            StructuredBlock::Sequence(items) => items
                .iter()
                .any(|item| self.structured_uses_var(item, fn_ir, var)),
            StructuredBlock::BasicBlock(bb) => {
                let block = &fn_ir.blocks[*bb];
                block.instrs.iter().any(|instr| match instr {
                    Instr::Assign { dst, src, .. } => {
                        dst == var
                            || self.value_mentions_var(
                                *src,
                                &fn_ir.values,
                                var,
                                &mut FxHashSet::default(),
                            )
                    }
                    Instr::Eval { val, .. } => {
                        self.value_mentions_var(*val, &fn_ir.values, var, &mut FxHashSet::default())
                    }
                    Instr::StoreIndex1D { base, idx, val, .. } => {
                        self.value_mentions_var(
                            *base,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *idx,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *val,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        )
                    }
                    Instr::StoreIndex2D {
                        base, r, c, val, ..
                    } => {
                        self.value_mentions_var(
                            *base,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *r,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *c,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *val,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        )
                    }
                    Instr::StoreIndex3D {
                        base, i, j, k, val, ..
                    } => {
                        self.value_mentions_var(
                            *base,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *i,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *j,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *k,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        ) || self.value_mentions_var(
                            *val,
                            &fn_ir.values,
                            var,
                            &mut FxHashSet::default(),
                        )
                    }
                }) || match block.term {
                    Terminator::If { cond, .. } => {
                        self.value_mentions_var(cond, &fn_ir.values, var, &mut FxHashSet::default())
                    }
                    Terminator::Return(Some(val)) => {
                        self.value_mentions_var(val, &fn_ir.values, var, &mut FxHashSet::default())
                    }
                    _ => false,
                }
            }
            StructuredBlock::If {
                cond,
                then_body,
                else_body,
            } => {
                self.value_mentions_var(*cond, &fn_ir.values, var, &mut FxHashSet::default())
                    || self.structured_uses_var(then_body, fn_ir, var)
                    || else_body
                        .as_ref()
                        .is_some_and(|body| self.structured_uses_var(body, fn_ir, var))
            }
            StructuredBlock::Loop { cond, body, .. } => {
                self.value_mentions_var(*cond, &fn_ir.values, var, &mut FxHashSet::default())
                    || self.structured_uses_var(body, fn_ir, var)
            }
            StructuredBlock::Return(Some(val)) => {
                self.value_mentions_var(*val, &fn_ir.values, var, &mut FxHashSet::default())
            }
            StructuredBlock::Break | StructuredBlock::Next | StructuredBlock::Return(None) => false,
        }
    }

    pub(super) fn value_mentions_var(
        &self,
        value_id: usize,
        values: &[Value],
        var: &str,
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        if !seen.insert(value_id) {
            return false;
        }
        match values.get(value_id).map(|v| &v.kind) {
            Some(ValueKind::Load { var: load_var }) => load_var == var,
            Some(ValueKind::Phi { args }) => args
                .iter()
                .any(|(arg, _)| self.value_mentions_var(*arg, values, var, seen)),
            Some(ValueKind::Len { base }) | Some(ValueKind::Indices { base }) => {
                self.value_mentions_var(*base, values, var, seen)
            }
            Some(ValueKind::Range { start, end }) => {
                self.value_mentions_var(*start, values, var, seen)
                    || self.value_mentions_var(*end, values, var, seen)
            }
            Some(ValueKind::Binary { lhs, rhs, .. }) => {
                self.value_mentions_var(*lhs, values, var, seen)
                    || self.value_mentions_var(*rhs, values, var, seen)
            }
            Some(ValueKind::Unary { rhs, .. }) => self.value_mentions_var(*rhs, values, var, seen),
            Some(ValueKind::Call { args, .. }) | Some(ValueKind::Intrinsic { args, .. }) => args
                .iter()
                .any(|arg| self.value_mentions_var(*arg, values, var, seen)),
            Some(ValueKind::Index1D { base, idx, .. }) => {
                self.value_mentions_var(*base, values, var, seen)
                    || self.value_mentions_var(*idx, values, var, seen)
            }
            Some(ValueKind::Index2D { base, r, c }) => {
                self.value_mentions_var(*base, values, var, seen)
                    || self.value_mentions_var(*r, values, var, seen)
                    || self.value_mentions_var(*c, values, var, seen)
            }
            Some(ValueKind::Index3D { base, i, j, k }) => {
                self.value_mentions_var(*base, values, var, seen)
                    || self.value_mentions_var(*i, values, var, seen)
                    || self.value_mentions_var(*j, values, var, seen)
                    || self.value_mentions_var(*k, values, var, seen)
            }
            _ => false,
        }
    }
}
