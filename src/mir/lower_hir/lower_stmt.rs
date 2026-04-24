impl<'a> MirLowerer<'a> {
    fn lower_stmt(&mut self, stmt: hir::HirStmt) -> RR<()> {
        match stmt {
            hir::HirStmt::Let {
                local, init, span, ..
            } => {
                let val = if let Some(e) = init {
                    self.lower_expr(e)?
                } else {
                    self.add_null_val(span) // Default init
                };
                self.write_var(local, val);
            }
            hir::HirStmt::Assign {
                target,
                value,
                span,
            } => {
                let v = self.lower_expr(value)?;
                match target {
                    hir::HirLValue::Local(l) => self.write_var(l, v),
                    hir::HirLValue::Index { base, index } => {
                        let base_id = self.lower_expr(base)?;
                        let mut ids = Vec::with_capacity(index.len());
                        for idx_expr in index {
                            ids.push(self.lower_expr(idx_expr)?);
                        }
                        match ids.as_slice() {
                            [idx] => {
                                self.fn_ir.blocks[self.curr_block].instrs.push(
                                    Instr::StoreIndex1D {
                                        base: base_id,
                                        idx: *idx,
                                        val: v,
                                        is_safe: false,
                                        is_na_safe: false,
                                        is_vector: false,
                                        span,
                                    },
                                );
                            }
                            [r, c] => {
                                self.fn_ir.blocks[self.curr_block].instrs.push(
                                    Instr::StoreIndex2D {
                                        base: base_id,
                                        r: *r,
                                        c: *c,
                                        val: v,
                                        span,
                                    },
                                );
                            }
                            [i, j, k] => {
                                self.fn_ir.blocks[self.curr_block].instrs.push(
                                    Instr::StoreIndex3D {
                                        base: base_id,
                                        i: *i,
                                        j: *j,
                                        k: *k,
                                        val: v,
                                        span,
                                    },
                                );
                            }
                            _ => {
                                return Err(crate::error::RRException::new(
                                    "RR.SemanticError",
                                    crate::error::RRCode::E1002,
                                    crate::error::Stage::Mir,
                                    "Only 1D/2D/3D indexing is supported",
                                ));
                            }
                        }
                    }
                    hir::HirLValue::Field { base, name } => {
                        let field_name = self
                            .symbols
                            .get(&name)
                            .cloned()
                            .unwrap_or_else(|| format!("field_{}", name.0));
                        let base_clone = base.clone();
                        let base_id = self.lower_expr(base)?;
                        let set_id = self.add_value(
                            ValueKind::FieldSet {
                                base: base_id,
                                field: field_name,
                                value: v,
                            },
                            span,
                        );
                        match base_clone {
                            hir::HirExpr::Local(lid) => {
                                self.write_var(lid, set_id);
                            }
                            hir::HirExpr::Global(sym, _) => {
                                if let Some(dst_name) = self.symbols.get(&sym).cloned() {
                                    self.fn_ir.blocks[self.curr_block]
                                        .instrs
                                        .push(Instr::Assign {
                                            dst: dst_name,
                                            src: set_id,
                                            span,
                                        });
                                } else {
                                    self.fn_ir.blocks[self.curr_block]
                                        .instrs
                                        .push(Instr::Eval { val: set_id, span });
                                }
                            }
                            _ => {
                                // Fallback: preserve side effect when base isn't a writable symbol.
                                self.fn_ir.blocks[self.curr_block]
                                    .instrs
                                    .push(Instr::Eval { val: set_id, span });
                            }
                        }
                    }
                }
            }
            hir::HirStmt::Expr { expr, .. } => {
                self.lower_expr(expr)?;
            }
            hir::HirStmt::Return { value, span: _span } => {
                let v = if let Some(e) = value {
                    Some(self.lower_expr(e)?)
                } else {
                    None
                };
                self.terminate_and_detach(Terminator::Return(v));
            }
            hir::HirStmt::If {
                cond,
                then_blk,
                else_blk,
                span: _span,
            } => {
                let cond_val = self.lower_expr(cond)?;
                let pre_if_bb = self.curr_block;

                let then_bb = self.fn_ir.add_block();
                let else_bb = self.fn_ir.add_block();
                let join_bb = self.fn_ir.add_block();

                self.terminate(Terminator::If {
                    cond: cond_val,
                    then_bb,
                    else_bb,
                });

                // Then branch
                self.add_pred(then_bb, pre_if_bb);
                self.curr_block = then_bb;
                self.seal_block(then_bb)?;
                self.lower_block_effects(then_blk)?;
                if !self.is_terminated(self.curr_block) {
                    self.add_pred(join_bb, self.curr_block);
                    self.terminate(Terminator::Goto(join_bb));
                }

                // Else branch
                self.add_pred(else_bb, pre_if_bb);
                self.curr_block = else_bb;
                self.seal_block(else_bb)?;
                if let Some(eb) = else_blk {
                    self.lower_block_effects(eb)?;
                }
                if !self.is_terminated(self.curr_block) {
                    self.add_pred(join_bb, self.curr_block);
                    self.terminate(Terminator::Goto(join_bb));
                }

                self.curr_block = join_bb;
                self.seal_block(join_bb)?;
            }
            hir::HirStmt::While {
                cond,
                body,
                span: _span,
            } => {
                let header_bb = self.fn_ir.add_block();
                let body_bb = self.fn_ir.add_block();
                let exit_bb = self.fn_ir.add_block();

                self.add_pred(header_bb, self.curr_block);
                self.terminate(Terminator::Goto(header_bb));

                self.curr_block = header_bb;
                let cond_val = self.lower_expr(cond)?;
                self.terminate(Terminator::If {
                    cond: cond_val,
                    then_bb: body_bb,
                    else_bb: exit_bb,
                });

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
                let curr_reachable = self
                    .preds
                    .get(&self.curr_block)
                    .map(|ps| !ps.is_empty())
                    .unwrap_or(false);
                if !self.is_terminated(self.curr_block) && curr_reachable {
                    self.add_pred(header_bb, self.curr_block);
                    self.terminate(Terminator::Goto(header_bb));
                }

                self.seal_block(header_bb)?;
                self.add_pred(exit_bb, header_bb);
                self.curr_block = exit_bb;
                self.seal_block(exit_bb)?;
            }
            hir::HirStmt::For { iter, body, span } => {
                self.lower_for(iter, body, span)?;
            }
            hir::HirStmt::Break { span } => {
                if let Some(targets) = self.loop_stack.last().copied() {
                    self.add_pred(targets.break_bb, self.curr_block);
                    self.terminate_and_detach(Terminator::Goto(targets.break_bb));
                } else {
                    return Err(crate::error::RRException::new(
                        "RR.SemanticError",
                        crate::error::RRCode::E1002,
                        crate::error::Stage::Mir,
                        "break used outside of a loop".to_string(),
                    )
                    .at(span));
                }
            }
            hir::HirStmt::Next { span } => {
                if let Some(targets) = self.loop_stack.last().copied() {
                    if let Some((var, iv)) = targets.continue_step {
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
                    self.add_pred(targets.continue_bb, self.curr_block);
                    self.terminate_and_detach(Terminator::Goto(targets.continue_bb));
                } else {
                    return Err(crate::error::RRException::new(
                        "RR.SemanticError",
                        crate::error::RRCode::E1002,
                        crate::error::Stage::Mir,
                        "next used outside of a loop".to_string(),
                    )
                    .at(span));
                }
            }
        }
        Ok(())
    }
}
