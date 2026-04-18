use super::*;

impl RBackend {
    fn structured_contains_loop(node: &StructuredBlock) -> bool {
        match node {
            StructuredBlock::Sequence(items) => items.iter().any(Self::structured_contains_loop),
            StructuredBlock::If {
                then_body,
                else_body,
                ..
            } => {
                Self::structured_contains_loop(then_body)
                    || else_body
                        .as_deref()
                        .is_some_and(Self::structured_contains_loop)
            }
            StructuredBlock::Loop { .. } => true,
            StructuredBlock::BasicBlock(_)
            | StructuredBlock::Break
            | StructuredBlock::Next
            | StructuredBlock::Return(_) => false,
        }
    }

    pub(super) fn emit_structured(
        &mut self,
        node: &StructuredBlock,
        fn_ir: &FnIR,
    ) -> Result<(), crate::error::RRException> {
        match node {
            StructuredBlock::Sequence(items) => {
                let mut idx = 0usize;
                while idx < items.len() {
                    if let Some(consumed) =
                        self.try_emit_full_range_conditional_loop_sequence(&items[idx..], fn_ir)
                    {
                        idx += consumed;
                        continue;
                    }
                    if idx + 1 < items.len()
                        && let StructuredBlock::BasicBlock(init_bb) = &items[idx]
                        && let StructuredBlock::Loop {
                            cond,
                            continue_on_true,
                            ..
                        } = &items[idx + 1]
                        && *continue_on_true
                        && let Some(ctx) = self
                            .extract_scalar_loop_index_context_from_init_bb(*init_bb, *cond, fn_ir)
                            .or_else(|| {
                                self.extract_scalar_loop_index_context_from_live_binding(
                                    *cond, fn_ir,
                                )
                            })
                    {
                        let fallback_loop_var = self.generated_loop_var_from_block(*init_bb, fn_ir);
                        self.emit_structured(&items[idx], fn_ir)?;
                        self.loop_analysis.active_scalar_loop_indices.push(ctx);
                        if let Some(fallback_loop_var) = fallback_loop_var.clone() {
                            self.loop_analysis
                                .active_loop_fallback_vars
                                .push(fallback_loop_var);
                        }
                        self.emit_structured(&items[idx + 1], fn_ir)?;
                        if fallback_loop_var.is_some() {
                            self.loop_analysis.active_loop_fallback_vars.pop();
                        }
                        self.loop_analysis.active_scalar_loop_indices.pop();
                        idx += 2;
                        continue;
                    }
                    if idx + 1 < items.len()
                        && let StructuredBlock::BasicBlock(init_bb) = &items[idx]
                        && let StructuredBlock::Loop {
                            continue_on_true, ..
                        } = &items[idx + 1]
                        && *continue_on_true
                        && let Some(fallback_loop_var) =
                            self.generated_loop_var_from_block(*init_bb, fn_ir)
                    {
                        self.emit_structured(&items[idx], fn_ir)?;
                        self.loop_analysis
                            .active_loop_fallback_vars
                            .push(fallback_loop_var);
                        self.emit_structured(&items[idx + 1], fn_ir)?;
                        self.loop_analysis.active_loop_fallback_vars.pop();
                        idx += 2;
                        continue;
                    }
                    self.emit_structured(&items[idx], fn_ir)?;
                    idx += 1;
                }
            }
            StructuredBlock::BasicBlock(bid) => {
                let blk = &fn_ir.blocks[*bid];
                for instr in &blk.instrs {
                    self.emit_instr(instr, &fn_ir.values, &fn_ir.params)?;
                }
            }
            StructuredBlock::If {
                cond,
                then_body,
                else_body,
            } => {
                let snapshot = self.begin_branch_snapshot();
                let pre_if_known_full_end_exprs = self.loop_analysis.known_full_end_exprs.clone();

                let cond_span = fn_ir.values[*cond].span;
                self.emit_mark(cond_span, Some("if"));
                self.record_span(cond_span);
                let c = self.resolve_cond(*cond, &fn_ir.values, &fn_ir.params);
                self.write_stmt(&format!("if ({}) {{", c));
                self.indent += 1;
                self.emit_structured(then_body, fn_ir)?;
                self.indent -= 1;
                let then_var_versions = self.value_tracker.var_versions.clone();
                let then_var_value_bindings = self.value_tracker.var_value_bindings.clone();
                let then_last_assigned = self.value_tracker.last_assigned_value_ids.clone();
                let then_known_full_end_exprs = self.loop_analysis.known_full_end_exprs.clone();
                if let Some(else_body) = else_body {
                    self.rollback_branch_snapshot(snapshot);
                    self.loop_analysis.known_full_end_exprs = pre_if_known_full_end_exprs.clone();
                    self.write_stmt("} else {");
                    self.indent += 1;
                    self.emit_structured(else_body, fn_ir)?;
                    self.indent -= 1;
                    let else_var_versions = self.value_tracker.var_versions.clone();
                    let else_var_value_bindings = self.value_tracker.var_value_bindings.clone();
                    let else_last_assigned = self.value_tracker.last_assigned_value_ids.clone();
                    let else_known_full_end_exprs = self.loop_analysis.known_full_end_exprs.clone();
                    self.write_stmt("}");
                    self.rollback_branch_snapshot(snapshot);
                    self.loop_analysis.known_full_end_exprs = pre_if_known_full_end_exprs.clone();
                    self.join_branch_var_value_bindings(
                        &then_var_versions,
                        &then_var_value_bindings,
                        &else_var_versions,
                        &else_var_value_bindings,
                    );
                    self.join_branch_last_assigned_values(&then_last_assigned, &else_last_assigned);
                    self.join_branch_known_full_end_exprs(
                        &pre_if_known_full_end_exprs,
                        &then_known_full_end_exprs,
                        &else_known_full_end_exprs,
                    );
                } else {
                    self.write_stmt("}");
                    self.rollback_branch_snapshot(snapshot);
                    self.loop_analysis.known_full_end_exprs = pre_if_known_full_end_exprs;
                }

                self.end_branch_snapshot();
                self.value_tracker.value_bindings.clear();
                self.loop_analysis.recent_whole_assign_bases.clear();
            }
            StructuredBlock::Loop {
                header,
                cond,
                continue_on_true,
                body,
            } => {
                let pre_loop_value_bindings = self.value_tracker.value_bindings.clone();
                let pre_loop_var_value_bindings = self.value_tracker.var_value_bindings.clone();
                let mut loop_mutated_vars = FxHashSet::default();
                Self::collect_mutated_vars(body, fn_ir, &mut loop_mutated_vars);
                let body_mutated_vars = loop_mutated_vars.clone();
                for instr in &fn_ir.blocks[*header].instrs {
                    match instr {
                        Instr::Assign { dst, .. } => {
                            loop_mutated_vars.insert(dst.clone());
                        }
                        Instr::StoreIndex1D { base, .. }
                        | Instr::StoreIndex2D { base, .. }
                        | Instr::StoreIndex3D { base, .. } => {
                            if let Some(var) = Self::named_written_base(*base, &fn_ir.values) {
                                loop_mutated_vars.insert(var);
                            }
                        }
                        Instr::Eval { .. } => {}
                    }
                }
                let pre_loop_known_full_end_exprs = loop_mutated_vars
                    .iter()
                    .filter_map(|var| {
                        self.known_full_end_expr_for_var(var)
                            .map(|expr| (var.clone(), expr.to_string()))
                    })
                    .collect::<FxHashMap<_, _>>();
                self.invalidate_var_bindings(loop_mutated_vars.iter());
                self.loop_analysis
                    .active_loop_known_full_end_exprs
                    .push(pre_loop_known_full_end_exprs.clone());
                self.loop_analysis
                    .active_loop_mutated_vars
                    .push(loop_mutated_vars.clone());
                let scalar_loop_ctx = self
                    .extract_scalar_loop_index_context_from_init_bb(*header, *cond, fn_ir)
                    .or_else(|| {
                        self.extract_scalar_loop_index_context_from_live_binding(*cond, fn_ir)
                    });
                let current_loop_idx_var = match fn_ir.values.get(*cond).map(|v| &v.kind) {
                    Some(ValueKind::Binary {
                        op: BinOp::Le, lhs, ..
                    }) => self.extract_loop_index_var(*lhs, &fn_ir.values),
                    Some(ValueKind::Binary {
                        op: BinOp::Ge, rhs, ..
                    }) => self.extract_loop_index_var(*rhs, &fn_ir.values),
                    _ => None,
                }
                .or_else(|| scalar_loop_ctx.as_ref().map(|ctx| ctx.var.clone()))
                .or_else(|| self.generated_loop_index_var_from_header(*header, fn_ir));
                if !Self::structured_contains_loop(body) {
                    self.emit_loop_invariant_scalar_hoists(
                        *header,
                        *cond,
                        body.as_ref(),
                        fn_ir,
                        &loop_mutated_vars,
                        current_loop_idx_var.as_deref(),
                    );
                }
                if let Some(ctx) = scalar_loop_ctx.clone() {
                    self.loop_analysis.active_scalar_loop_indices.push(ctx);
                }
                self.write_stmt("repeat {");
                self.indent += 1;

                let blk = &fn_ir.blocks[*header];
                for instr in &blk.instrs {
                    self.emit_instr(instr, &fn_ir.values, &fn_ir.params)?;
                }

                let cond_span = fn_ir.values[*cond].span;
                self.emit_mark(cond_span, Some("loop-cond"));
                self.record_span(cond_span);
                let c = self.resolve_cond(*cond, &fn_ir.values, &fn_ir.params);
                if *continue_on_true {
                    self.write_stmt(&format!("if (!{}) break", c));
                } else {
                    self.write_stmt(&format!("if ({}) break", c));
                }
                self.emit_structured(body, fn_ir)?;
                let fallback_idx_var = match fn_ir.values.get(*cond).map(|v| &v.kind) {
                    Some(ValueKind::Binary {
                        op: BinOp::Le, lhs, ..
                    }) => self.extract_loop_index_var(*lhs, &fn_ir.values),
                    Some(ValueKind::Binary {
                        op: BinOp::Ge, rhs, ..
                    }) => self.extract_loop_index_var(*rhs, &fn_ir.values),
                    _ => None,
                }
                .or_else(|| scalar_loop_ctx.as_ref().map(|ctx| ctx.var.clone()))
                .or_else(|| self.loop_analysis.active_loop_fallback_vars.last().cloned())
                .or_else(|| self.generated_loop_index_var_from_header(*header, fn_ir));
                if let Some(idx_var) = fallback_idx_var
                    && !body_mutated_vars.contains(&idx_var)
                {
                    self.write_stmt(&format!("{idx_var} <- ({idx_var} + 1L)"));
                }

                self.indent -= 1;
                self.write_stmt("}");

                self.value_tracker.value_bindings = pre_loop_value_bindings;
                self.value_tracker.var_value_bindings = pre_loop_var_value_bindings;
                self.value_tracker.last_assigned_value_ids.clear();
                self.invalidate_var_bindings(loop_mutated_vars.iter());
                for var in &loop_mutated_vars {
                    self.loop_analysis.known_full_end_exprs.remove(var);
                }
                self.loop_analysis
                    .known_full_end_exprs
                    .extend(pre_loop_known_full_end_exprs);
                self.loop_analysis.recent_whole_assign_bases.clear();
                self.loop_analysis.active_loop_known_full_end_exprs.pop();
                self.loop_analysis.active_loop_mutated_vars.pop();
                if scalar_loop_ctx.is_some() {
                    self.loop_analysis.active_scalar_loop_indices.pop();
                }
            }
            StructuredBlock::Break => {
                self.write_stmt("break");
            }
            StructuredBlock::Next => {
                self.write_stmt("next");
            }
            StructuredBlock::Return(v) => match v {
                Some(val) => {
                    if std::env::var_os("RR_DEBUG_RETURN").is_some() {
                        eprintln!(
                            "RR_DEBUG_RETURN fn={} val={} kind={:?} bound={:?} stale={:?}",
                            fn_ir.name,
                            val,
                            fn_ir.values[*val].kind,
                            self.resolve_bound_value(*val),
                            self.resolve_stale_origin_var(*val, &fn_ir.values[*val], &fn_ir.values)
                        );
                    }
                    if let ValueKind::Call {
                        callee,
                        args,
                        names,
                    } = &fn_ir.values[*val].kind
                        && callee == "rr_assign_slice"
                        && !args.is_empty()
                        && let Some(base_var) = Self::named_mutable_base_expr(
                            args[0],
                            &fn_ir.values,
                            &self.value_tracker.value_bindings,
                            &self.value_tracker.var_versions,
                        )
                    {
                        if self.resolve_bound_value(*val).as_deref() == Some(base_var.as_str()) {
                            self.write_stmt(&format!("return({base_var})"));
                            return Ok(());
                        }
                        let call_expr = self.resolve_call_expr(
                            &fn_ir.values[*val],
                            callee,
                            args,
                            names,
                            &fn_ir.values,
                            &fn_ir.params,
                        );
                        self.write_stmt(&format!("{base_var} <- {call_expr}"));
                        self.write_stmt(&format!("return({base_var})"));
                        return Ok(());
                    }
                    if let Some(bound) = self.resolve_bound_value(*val) {
                        self.write_stmt(&format!("return({bound})"));
                        return Ok(());
                    }
                    let r = self.resolve_val(*val, &fn_ir.values, &fn_ir.params, false);
                    self.write_stmt(&format!("return({})", r));
                }
                None => self.write_stmt("return(NULL)"),
            },
        }
        Ok(())
    }
}
