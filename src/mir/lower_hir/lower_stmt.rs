use super::*;
impl<'a> MirLowerer<'a> {
    pub(crate) fn lower_stmt(&mut self, stmt: hir::HirStmt) -> RR<()> {
        match stmt {
            hir::HirStmt::Let {
                local,
                ty,
                init,
                span,
                ..
            } => self.lower_let_stmt(local, ty, init, span)?,
            hir::HirStmt::Assign {
                target,
                value,
                span,
            } => self.lower_assign_stmt(target, value, span)?,
            hir::HirStmt::Expr { expr, .. } => {
                self.lower_expr(expr)?;
            }
            hir::HirStmt::UnsafeRBlock {
                code,
                read_only,
                span,
            } => self.lower_unsafe_r_block(code, read_only, span),
            hir::HirStmt::Return { value, span: _span } => self.lower_return_stmt(value)?,
            hir::HirStmt::If {
                cond,
                then_blk,
                else_blk,
                span: _span,
            } => self.lower_if_stmt(cond, then_blk, else_blk)?,
            hir::HirStmt::While { cond, body, .. } => self.lower_while_stmt(cond, body)?,
            hir::HirStmt::For { iter, body, span } => {
                self.lower_for(iter, body, span)?;
            }
            hir::HirStmt::Break { span } => self.lower_break_stmt(span)?,
            hir::HirStmt::Next { span } => self.lower_next_stmt(span)?,
        }
        Ok(())
    }

    pub(crate) fn lower_let_stmt(
        &mut self,
        local: hir::LocalId,
        ty: Option<hir::Ty>,
        init: Option<hir::HirExpr>,
        span: Span,
    ) -> RR<()> {
        let val = if let Some(expr) = init {
            self.lower_expr(expr)?
        } else {
            self.add_null_val(span)
        };
        if let Some(ty) = ty.as_ref() {
            self.apply_let_type_hint(val, ty);
        }
        self.write_var(local, val);
        Ok(())
    }

    pub(crate) fn apply_let_type_hint(&mut self, val: ValueId, ty: &hir::Ty) {
        let Some(value) = self.fn_ir.values.get_mut(val) else {
            return;
        };
        let hinted_ty = hir_ty_to_type_state(ty);
        value.value_ty = value.value_ty.join(hinted_ty);
        if value.value_term.is_any() {
            value.value_term = hir_ty_to_type_term_with_symbols(ty, self.symbols);
        }
    }

    pub(crate) fn lower_assign_stmt(
        &mut self,
        target: hir::HirLValue,
        value: hir::HirExpr,
        span: Span,
    ) -> RR<()> {
        let value = self.lower_expr(value)?;
        match target {
            hir::HirLValue::Local(local) => self.write_var(local, value),
            hir::HirLValue::Index { base, index } => {
                self.lower_index_assignment(base, index, value, span)?;
            }
            hir::HirLValue::Field { base, name } => {
                self.lower_field_assignment(base, name, value, span)?;
            }
        }
        Ok(())
    }

    pub(crate) fn lower_index_assignment(
        &mut self,
        base: hir::HirExpr,
        index: Vec<hir::HirExpr>,
        value: ValueId,
        span: Span,
    ) -> RR<()> {
        let (base_id, base_local) = match base {
            hir::HirExpr::Local(local) => (
                self.mutable_local_base_for_lvalue(local, span)?,
                Some(local),
            ),
            other => (self.lower_expr(other)?, None),
        };
        let index_ids = self.lower_index_operands(index)?;
        self.emit_index_store(base_id, &index_ids, value, span)?;
        if let Some(local) = base_local {
            self.mark_local_mutated_after_index_store(local, base_id, span);
        }
        Ok(())
    }

    pub(crate) fn lower_index_operands(&mut self, index: Vec<hir::HirExpr>) -> RR<Vec<ValueId>> {
        let mut ids = Vec::with_capacity(index.len());
        for idx_expr in index {
            ids.push(self.lower_expr(idx_expr)?);
        }
        Ok(ids)
    }

    pub(crate) fn emit_index_store(
        &mut self,
        base: ValueId,
        index_ids: &[ValueId],
        value: ValueId,
        span: Span,
    ) -> RR<()> {
        let instr = match index_ids {
            [idx] => Instr::StoreIndex1D {
                base,
                idx: *idx,
                val: value,
                is_safe: false,
                is_na_safe: false,
                is_vector: false,
                span,
            },
            [r, c] => Instr::StoreIndex2D {
                base,
                r: *r,
                c: *c,
                val: value,
                span,
            },
            [i, j, k] => Instr::StoreIndex3D {
                base,
                i: *i,
                j: *j,
                k: *k,
                val: value,
                span,
            },
            _ => return Err(Self::unsupported_index_store_error()),
        };
        self.fn_ir.blocks[self.curr_block].instrs.push(instr);
        Ok(())
    }

    pub(crate) fn unsupported_index_store_error() -> crate::error::RRException {
        crate::error::RRException::new(
            "RR.SemanticError",
            crate::error::RRCode::E1002,
            crate::error::Stage::Mir,
            "Only 1D/2D/3D indexing is supported",
        )
    }

    pub(crate) fn lower_field_assignment(
        &mut self,
        base: hir::HirExpr,
        name: hir::SymbolId,
        value: ValueId,
        span: Span,
    ) -> RR<()> {
        let field = self
            .symbols
            .get(&name)
            .cloned()
            .unwrap_or_else(|| format!("field_{}", name.0));
        let base_clone = base.clone();
        let base = self.lower_expr(base)?;
        let set_id = self.add_value(ValueKind::FieldSet { base, field, value }, span);
        self.store_field_assignment_result(base_clone, set_id, span);
        Ok(())
    }

    pub(crate) fn store_field_assignment_result(
        &mut self,
        base: hir::HirExpr,
        set_id: ValueId,
        span: Span,
    ) {
        match base {
            hir::HirExpr::Local(local) => self.write_var(local, set_id),
            hir::HirExpr::Global(sym, _) => {
                if let Some(dst) = self.symbols.get(&sym).cloned() {
                    self.fn_ir.blocks[self.curr_block]
                        .instrs
                        .push(Instr::Assign {
                            dst,
                            src: set_id,
                            span,
                        });
                } else {
                    self.emit_eval(set_id, span);
                }
            }
            _ => self.emit_eval(set_id, span),
        }
    }

    pub(crate) fn emit_eval(&mut self, val: ValueId, span: Span) {
        self.fn_ir.blocks[self.curr_block]
            .instrs
            .push(Instr::Eval { val, span });
    }

    pub(crate) fn lower_unsafe_r_block(&mut self, code: String, read_only: bool, span: Span) {
        if !read_only {
            self.unsafe_r_seen = true;
            self.fn_ir.mark_opaque_interop(
                "unsafe R block requires conservative optimization".to_string(),
            );
        }
        self.fn_ir.blocks[self.curr_block]
            .instrs
            .push(Instr::UnsafeRBlock {
                code,
                read_only,
                span,
            });
        if !read_only {
            self.invalidate_defs_after_unsafe_r(span);
        }
    }

    pub(crate) fn lower_return_stmt(&mut self, value: Option<hir::HirExpr>) -> RR<()> {
        let value = if let Some(expr) = value {
            Some(self.lower_expr(expr)?)
        } else {
            None
        };
        self.terminate_and_detach(Terminator::Return(value));
        Ok(())
    }

    pub(crate) fn lower_if_stmt(
        &mut self,
        cond: hir::HirExpr,
        then_blk: hir::HirBlock,
        else_blk: Option<hir::HirBlock>,
    ) -> RR<()> {
        let cond = self.lower_expr(cond)?;
        let pre_if_bb = self.curr_block;
        let then_bb = self.fn_ir.add_block();
        let else_bb = self.fn_ir.add_block();
        let join_bb = self.fn_ir.add_block();

        self.terminate(Terminator::If {
            cond,
            then_bb,
            else_bb,
        });
        self.lower_if_stmt_branch(then_bb, pre_if_bb, join_bb, Some(then_blk))?;
        self.lower_if_stmt_branch(else_bb, pre_if_bb, join_bb, else_blk)?;
        self.curr_block = join_bb;
        self.seal_block(join_bb)
    }

    pub(crate) fn lower_if_stmt_branch(
        &mut self,
        branch_bb: BlockId,
        pred_bb: BlockId,
        join_bb: BlockId,
        block: Option<hir::HirBlock>,
    ) -> RR<()> {
        self.add_pred(branch_bb, pred_bb);
        self.curr_block = branch_bb;
        self.seal_block(branch_bb)?;
        if let Some(block) = block {
            self.lower_block_effects(block)?;
        }
        if !self.is_terminated(self.curr_block) {
            self.add_pred(join_bb, self.curr_block);
            self.terminate(Terminator::Goto(join_bb));
        }
        Ok(())
    }

    pub(crate) fn lower_while_stmt(&mut self, cond: hir::HirExpr, body: hir::HirBlock) -> RR<()> {
        let header_bb = self.fn_ir.add_block();
        let body_bb = self.fn_ir.add_block();
        let exit_bb = self.fn_ir.add_block();

        self.add_pred(header_bb, self.curr_block);
        self.terminate(Terminator::Goto(header_bb));
        self.lower_while_condition(header_bb, body_bb, exit_bb, cond)?;
        self.lower_while_body(header_bb, body_bb, exit_bb, body)?;
        self.seal_block(header_bb)?;
        self.add_pred(exit_bb, header_bb);
        self.curr_block = exit_bb;
        self.seal_block(exit_bb)
    }

    pub(crate) fn lower_while_condition(
        &mut self,
        header_bb: BlockId,
        body_bb: BlockId,
        exit_bb: BlockId,
        cond: hir::HirExpr,
    ) -> RR<()> {
        self.curr_block = header_bb;
        let cond = self.lower_expr(cond)?;
        self.terminate(Terminator::If {
            cond,
            then_bb: body_bb,
            else_bb: exit_bb,
        });
        Ok(())
    }

    pub(crate) fn lower_while_body(
        &mut self,
        header_bb: BlockId,
        body_bb: BlockId,
        exit_bb: BlockId,
        body: hir::HirBlock,
    ) -> RR<()> {
        self.add_pred(body_bb, header_bb);
        self.curr_block = body_bb;
        self.seal_block(body_bb)?;
        self.loop_stack.push(LoopTargets {
            break_bb: exit_bb,
            continue_bb: header_bb,
            continue_step: None,
        });
        self.lower_block_effects(body)?;
        self.loop_stack.pop();
        if self.current_block_is_reachable() && !self.is_terminated(self.curr_block) {
            self.add_pred(header_bb, self.curr_block);
            self.terminate(Terminator::Goto(header_bb));
        }
        Ok(())
    }

    pub(crate) fn current_block_is_reachable(&self) -> bool {
        self.preds
            .get(&self.curr_block)
            .map(|preds| !preds.is_empty())
            .unwrap_or(false)
    }

    pub(crate) fn lower_break_stmt(&mut self, span: Span) -> RR<()> {
        let Some(targets) = self.loop_stack.last().copied() else {
            return Err(Self::loop_control_outside_loop_error("break", span));
        };
        self.add_pred(targets.break_bb, self.curr_block);
        self.terminate_and_detach(Terminator::Goto(targets.break_bb));
        Ok(())
    }

    pub(crate) fn lower_next_stmt(&mut self, span: Span) -> RR<()> {
        let Some(targets) = self.loop_stack.last().copied() else {
            return Err(Self::loop_control_outside_loop_error("next", span));
        };
        self.apply_continue_step(targets.continue_step, span);
        self.add_pred(targets.continue_bb, self.curr_block);
        self.terminate_and_detach(Terminator::Goto(targets.continue_bb));
        Ok(())
    }

    pub(crate) fn apply_continue_step(
        &mut self,
        continue_step: Option<(hir::LocalId, ValueId)>,
        span: Span,
    ) {
        let Some((var, iv)) = continue_step else {
            return;
        };
        let one = self.add_int_val(1, span);
        let next_iv = self.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: iv,
                rhs: one,
            },
            span,
        );
        self.write_var(var, next_iv);
    }

    pub(crate) fn loop_control_outside_loop_error(
        keyword: &str,
        span: Span,
    ) -> crate::error::RRException {
        crate::error::RRException::new(
            "RR.SemanticError",
            crate::error::RRCode::E1002,
            crate::error::Stage::Mir,
            format!("{} used outside of a loop", keyword),
        )
        .at(span)
    }
}
