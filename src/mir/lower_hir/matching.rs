use super::*;

impl<'a> MirLowerer<'a> {
    pub(crate) fn lower_match_expr(
        &mut self,
        scrut: hir::HirExpr,
        arms: Vec<hir::HirMatchArm>,
    ) -> RR<ValueId> {
        let span = Span::default();
        if arms.is_empty() {
            return Err(crate::error::RRException::new(
                "RR.SemanticError",
                crate::error::RRCode::E3001,
                crate::error::Stage::Lower,
                "match expression requires at least one arm",
            )
            .at(span)
            .note("Add at least one match arm, typically ending with `_ => ...`."));
        }
        if !Self::match_has_final_unguarded_catch_all(&arms) {
            return Err(crate::error::RRException::new(
                "RR.SemanticError",
                crate::error::RRCode::E3001,
                crate::error::Stage::Lower,
                "non-exhaustive match requires an unguarded catch-all arm",
            )
            .at(span)
            .note("Add `_ => ...` (or an unguarded binding arm) as the final fallback."));
        }
        let scrut_val = self.lower_expr(scrut)?;

        let merge_bb = self.fn_ir.add_block();
        let mut arm_results: Vec<(ValueId, BlockId)> = Vec::new();
        let mut test_bb = self.curr_block;
        let arm_len = arms.len();

        for (i, arm) in arms.into_iter().enumerate() {
            self.curr_block = test_bb;
            let arm_bb = self.fn_ir.add_block();
            let is_final_catch_all = i + 1 == arm_len && Self::is_unguarded_catch_all_arm(&arm);
            let fail_bb = if let Some(guard_expr) = arm.guard {
                let cond = self.lower_match_pat_cond(scrut_val, &arm.pat, arm.span)?;
                let guard_bb = self.fn_ir.add_block();
                let fail_bb = self.fn_ir.add_block();
                self.terminate(Terminator::If {
                    cond,
                    then_bb: guard_bb,
                    else_bb: fail_bb,
                });
                self.add_pred(guard_bb, test_bb);
                self.add_pred(fail_bb, test_bb);

                self.curr_block = guard_bb;
                self.seal_block(guard_bb)?;
                self.bind_match_pattern(scrut_val, &arm.pat, arm.span)?;
                let guard_val = self.lower_expr(guard_expr)?;
                self.terminate(Terminator::If {
                    cond: guard_val,
                    then_bb: arm_bb,
                    else_bb: fail_bb,
                });
                self.add_pred(arm_bb, guard_bb);
                self.add_pred(fail_bb, guard_bb);
                Some(fail_bb)
            } else if is_final_catch_all {
                self.terminate(Terminator::Goto(arm_bb));
                self.add_pred(arm_bb, test_bb);
                None
            } else {
                let cond = self.lower_match_pat_cond(scrut_val, &arm.pat, arm.span)?;
                let fail_bb = self.fn_ir.add_block();
                self.terminate(Terminator::If {
                    cond,
                    then_bb: arm_bb,
                    else_bb: fail_bb,
                });
                self.add_pred(arm_bb, test_bb);
                self.add_pred(fail_bb, test_bb);
                Some(fail_bb)
            };

            self.curr_block = arm_bb;
            self.seal_block(arm_bb)?;
            self.bind_match_pattern(scrut_val, &arm.pat, arm.span)?;
            let arm_val = self.lower_expr(arm.body)?;
            let arm_end_bb = self.curr_block;
            if !self.is_terminated(self.curr_block) {
                self.add_pred(merge_bb, self.curr_block);
                self.terminate(Terminator::Goto(merge_bb));
            }
            arm_results.push((arm_val, arm_end_bb));

            if let Some(fail_bb) = fail_bb {
                test_bb = fail_bb;
                self.seal_block(fail_bb)?;
                if i + 1 == arm_len {
                    self.curr_block = test_bb;
                }
            }
        }

        self.curr_block = merge_bb;
        self.seal_block(merge_bb)?;
        let phi = self.add_value(ValueKind::Phi { args: arm_results }, span);
        if let Some(v) = self.fn_ir.values.get_mut(phi) {
            v.phi_block = Some(merge_bb);
        }
        Ok(phi)
    }

    pub(crate) fn match_has_final_unguarded_catch_all(arms: &[hir::HirMatchArm]) -> bool {
        arms.last()
            .map(Self::is_unguarded_catch_all_arm)
            .unwrap_or(false)
    }

    pub(crate) fn is_unguarded_catch_all_arm(arm: &hir::HirMatchArm) -> bool {
        arm.guard.is_none() && matches!(&arm.pat, hir::HirPat::Wild | hir::HirPat::Bind { .. })
    }

    pub(crate) fn lower_match_pat_cond(
        &mut self,
        scrut: ValueId,
        pat: &hir::HirPat,
        span: Span,
    ) -> RR<ValueId> {
        match pat {
            hir::HirPat::Wild | hir::HirPat::Bind { .. } => Ok(self.add_bool_val(true, span)),
            hir::HirPat::Lit(l) => {
                let rhs = self.add_value(ValueKind::Const(Self::hir_lit_to_lit(l)), span);
                Ok(self.add_value(
                    ValueKind::Binary {
                        op: BinOp::Eq,
                        lhs: scrut,
                        rhs,
                    },
                    span,
                ))
            }
            hir::HirPat::Or(pats) => {
                if pats.is_empty() {
                    return Ok(self.add_value(ValueKind::Const(Lit::Bool(false)), span));
                }
                let mut cond = self.lower_match_pat_cond(scrut, &pats[0], span)?;
                for p in pats.iter().skip(1) {
                    let rhs = self.lower_match_pat_cond(scrut, p, span)?;
                    cond = self.add_bin_bool(BinOp::Or, cond, rhs, span);
                }
                Ok(cond)
            }
            hir::HirPat::List { items, rest } => {
                let is_list_matchable =
                    self.add_call_value("rr_list_pattern_matchable", vec![scrut], span);
                let len = self.add_value(ValueKind::Len { base: scrut }, span);
                let expected = self.add_int_val(items.len() as i64, span);
                let len_cond = if rest.is_some() {
                    self.add_bin_bool(BinOp::Ge, len, expected, span)
                } else {
                    self.add_bin_bool(BinOp::Eq, len, expected, span)
                };
                let mut cond = self.add_bin_bool(BinOp::And, is_list_matchable, len_cond, span);

                for (i, item_pat) in items.iter().enumerate() {
                    let idx = self.add_int_val((i + 1) as i64, span);
                    let elem = self.add_value(
                        ValueKind::Index1D {
                            base: scrut,
                            idx,
                            is_safe: true,
                            is_na_safe: true,
                        },
                        span,
                    );
                    let elem_cond = self.lower_match_pat_cond(elem, item_pat, span)?;
                    cond = self.add_bin_bool(BinOp::And, cond, elem_cond, span);
                }
                Ok(cond)
            }
            hir::HirPat::Record { fields } => {
                let mut cond = self.add_bool_val(true, span);
                for (field, subpat) in fields {
                    let field_name = self.symbol_name(*field);
                    let field_name_val =
                        self.add_value(ValueKind::Const(Lit::Str(field_name)), span);
                    let exists =
                        self.add_call_value("rr_field_exists", vec![scrut, field_name_val], span);
                    cond = self.add_bin_bool(BinOp::And, cond, exists, span);

                    let field_val =
                        self.add_call_value("rr_field_get", vec![scrut, field_name_val], span);
                    let field_cond = self.lower_match_pat_cond(field_val, subpat, span)?;
                    cond = self.add_bin_bool(BinOp::And, cond, field_cond, span);
                }
                Ok(cond)
            }
        }
    }

    pub(crate) fn bind_match_pattern(
        &mut self,
        scrut: ValueId,
        pat: &hir::HirPat,
        span: Span,
    ) -> RR<()> {
        match pat {
            hir::HirPat::Bind { local, .. } => {
                self.write_var(*local, scrut);
                Ok(())
            }
            hir::HirPat::Or(_) | hir::HirPat::Wild | hir::HirPat::Lit(_) => Ok(()),
            hir::HirPat::List { items, rest } => {
                for (i, item_pat) in items.iter().enumerate() {
                    let idx = self.add_int_val((i + 1) as i64, span);
                    let elem = self.add_value(
                        ValueKind::Index1D {
                            base: scrut,
                            idx,
                            is_safe: true,
                            is_na_safe: true,
                        },
                        span,
                    );
                    self.bind_match_pattern(elem, item_pat, span)?;
                }
                if let Some((_, rest_local)) = rest {
                    let start_idx = self.add_int_val((items.len() + 1) as i64, span);
                    let tail = self.add_call_value("rr_list_rest", vec![scrut, start_idx], span);
                    self.write_var(*rest_local, tail);
                }
                Ok(())
            }
            hir::HirPat::Record { fields } => {
                for (field, subpat) in fields {
                    let field_name = self.symbol_name(*field);
                    let field_name_val =
                        self.add_value(ValueKind::Const(Lit::Str(field_name)), span);
                    let field_val =
                        self.add_call_value("rr_field_get", vec![scrut, field_name_val], span);
                    self.bind_match_pattern(field_val, subpat, span)?;
                }
                Ok(())
            }
        }
    }

    pub(crate) fn hir_lit_to_lit(l: &hir::HirLit) -> Lit {
        match l {
            hir::HirLit::Int(i) => Lit::Int(*i),
            hir::HirLit::Double(f) => Lit::Float(*f),
            hir::HirLit::Char(s) => Lit::Str(s.clone()),
            hir::HirLit::Bool(b) => Lit::Bool(*b),
            hir::HirLit::NA => Lit::Na,
            hir::HirLit::Null => Lit::Null,
        }
    }

    pub(crate) fn unique_param_local_name(
        &self,
        param_name: &str,
        local_id: hir::LocalId,
    ) -> String {
        let mut candidate = format!(".arg_{}", param_name);
        if self.var_names.values().any(|n| n == &candidate) {
            candidate = format!(".arg_{}_{}", param_name, local_id.0);
        }
        candidate
    }
}
