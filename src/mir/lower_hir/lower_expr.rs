impl<'a> MirLowerer<'a> {
    fn lower_expr(&mut self, expr: hir::HirExpr) -> RR<ValueId> {
        // println!("DEBUG: Lowering Expr: {:?}", expr);
        match expr {
            hir::HirExpr::Lit(l) => {
                let al = match l {
                    hir::HirLit::Int(i) => Lit::Int(i),
                    hir::HirLit::Double(f) => Lit::Float(f),
                    hir::HirLit::Char(s) => Lit::Str(s),
                    hir::HirLit::Bool(b) => Lit::Bool(b),
                    hir::HirLit::NA => Lit::Na,
                    hir::HirLit::Null => Lit::Null,
                };
                Ok(self.add_value(ValueKind::Const(al), Span::default()))
            }
            hir::HirExpr::Local(l) => self.read_var(l, self.curr_block),
            hir::HirExpr::Global(sym, span) => {
                let raw_name = self
                    .symbols
                    .get(&sym)
                    .cloned()
                    .unwrap_or_else(|| format!("Sym_{}", sym.0));
                if self.in_tidy_mask() && Self::should_lower_as_tidy_symbol(&raw_name) {
                    return Ok(self.add_value(
                        ValueKind::RSymbol {
                            name: raw_name.clone(),
                        },
                        span,
                    ));
                }
                let name = if self.known_functions.contains_key(&raw_name) {
                    format!("Sym_{}", sym.0)
                } else {
                    raw_name
                };
                Ok(self.add_value_with_name(
                    ValueKind::Load { var: name.clone() },
                    span,
                    Some(name),
                ))
            }
            hir::HirExpr::Unary { op, expr } => {
                let rhs = self.lower_expr(*expr)?;
                let op = match op {
                    hir::HirUnOp::Not => crate::syntax::ast::UnaryOp::Not,
                    hir::HirUnOp::Neg => crate::syntax::ast::UnaryOp::Neg,
                };
                Ok(self.add_value(ValueKind::Unary { op, rhs }, Span::default()))
            }
            hir::HirExpr::Binary { op, lhs, rhs } => {
                let l = self.lower_expr(*lhs)?;
                let r = self.lower_expr(*rhs)?;
                let op = self.map_binop(op);
                Ok(self.add_value(ValueKind::Binary { op, lhs: l, rhs: r }, Span::default()))
            }
            hir::HirExpr::Field { base, name } => {
                let b = self.lower_expr(*base)?;
                let field_name = self
                    .symbols
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| format!("field_{}", name.0));
                Ok(self.add_value(
                    ValueKind::FieldGet {
                        base: b,
                        field: field_name,
                    },
                    Span::default(),
                ))
            }
            hir::HirExpr::Index { base, index } => {
                let span = Span::default();
                let base_id = self.lower_expr(*base)?;
                let mut ids = Vec::with_capacity(index.len());
                for idx_expr in index {
                    ids.push(self.lower_expr(idx_expr)?);
                }
                match ids.as_slice() {
                    [idx] => Ok(self.add_value(
                        ValueKind::Index1D {
                            base: base_id,
                            idx: *idx,
                            is_safe: false,
                            is_na_safe: false,
                        },
                        span,
                    )),
                    [r, c] => Ok(self.add_value(
                        ValueKind::Index2D {
                            base: base_id,
                            r: *r,
                            c: *c,
                        },
                        span,
                    )),
                    [i, j, k] => Ok(self.add_value(
                        ValueKind::Index3D {
                            base: base_id,
                            i: *i,
                            j: *j,
                            k: *k,
                        },
                        span,
                    )),
                    _ => Err(crate::error::RRException::new(
                        "RR.SemanticError",
                        crate::error::RRCode::E1002,
                        crate::error::Stage::Mir,
                        "Only 1D/2D/3D indexing is supported",
                    )),
                }
            }
            hir::HirExpr::Block(blk) => self.lower_block(blk),

            hir::HirExpr::Call(hir::HirCall { callee, args, span }) => {
                let tidy_mask_args = match callee.as_ref() {
                    hir::HirExpr::Global(sym, _) => self
                        .symbols
                        .get(sym)
                        .is_some_and(|name| Self::is_tidy_data_mask_call(name)),
                    _ => false,
                };
                let mut v_args = Vec::new();
                let mut arg_names: Vec<Option<String>> = Vec::new();
                for arg in args {
                    match arg {
                        hir::HirArg::Pos(e) => {
                            let lowered = if tidy_mask_args {
                                self.with_tidy_mask(|lowerer| lowerer.lower_expr(e))?
                            } else {
                                self.lower_expr(e)?
                            };
                            v_args.push(lowered);
                            arg_names.push(None);
                        }
                        hir::HirArg::Named { name, value } => {
                            let lowered = if tidy_mask_args {
                                self.with_tidy_mask(|lowerer| lowerer.lower_expr(value))?
                            } else {
                                self.lower_expr(value)?
                            };
                            v_args.push(lowered);
                            let n = self
                                .symbols
                                .get(&name)
                                .cloned()
                                .unwrap_or_else(|| format!("arg_{}", name.0));
                            arg_names.push(Some(n));
                        }
                    }
                }

                match callee.as_ref() {
                    hir::HirExpr::Global(sym, _) => {
                        if let Some(name) = self.symbols.get(sym) {
                            if name.starts_with("rr_") {
                                return Ok(self.add_value(
                                    ValueKind::Call {
                                        callee: name.clone(),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                ));
                            }
                            if Self::allow_user_builtin_shadowing(name)
                                && let Some(expected) = self.known_functions.get(name)
                            {
                                let _ = expected;
                                return Ok(self.add_value(
                                    ValueKind::Call {
                                        callee: format!("Sym_{}", sym.0),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                ));
                            }
                            if name == "length" {
                                if v_args.len() != 1 {
                                    return Err(crate::error::RRException::new(
                                        "RR.SemanticError",
                                        crate::error::RRCode::E1002,
                                        crate::error::Stage::Mir,
                                        format!(
                                            "builtin '{}' expects 1 argument, got {}",
                                            name,
                                            v_args.len()
                                        ),
                                    )
                                    .at(span));
                                }
                                return Ok(self.add_value(ValueKind::Len { base: v_args[0] }, span));
                            }
                            // Known builtins should keep their original names.
                            if matches!(
                                name.as_str(),
                                "seq_along"
                                    | "seq_len"
                                    | "c"
                                    | "list"
                                    | "sum"
                                    | "mean"
                                    | "var"
                                    | "prod"
                                    | "min"
                                    | "max"
                                    | "abs"
                                    | "sqrt"
                                    | "sin"
                                    | "cos"
                                    | "tan"
                                    | "asin"
                                    | "acos"
                                    | "atan"
                                    | "atan2"
                                    | "sinh"
                                    | "cosh"
                                    | "tanh"
                                    | "log"
                                    | "log10"
                                    | "log2"
                                    | "exp"
                                    | "sign"
                                    | "gamma"
                                    | "lgamma"
                                    | "floor"
                                    | "ceiling"
                                    | "trunc"
                                    | "round"
                                    | "pmax"
                                    | "pmin"
                                    | "print"
                                    | "paste"
                                    | "paste0"
                                    | "sprintf"
                                    | "cat"
                                    | "names"
                                    | "rownames"
                                    | "colnames"
                                    | "sort"
                                    | "order"
                                    | "match"
                                    | "unique"
                                    | "duplicated"
                                    | "anyDuplicated"
                                    | "any"
                                    | "all"
                                    | "which"
                                    | "is.na"
                                    | "is.finite"
                                    | "numeric"
                                    | "character"
                                    | "logical"
                                    | "integer"
                                    | "double"
                                    | "rep"
                                    | "rep.int"
                                    | "vector"
                                    | "matrix"
                                    | "dim"
                                    | "dimnames"
                                    | "nrow"
                                    | "ncol"
                                    | "colSums"
                                    | "rowSums"
                                    | "crossprod"
                                    | "tcrossprod"
                                    | "t"
                                    | "diag"
                                    | "rbind"
                                    | "cbind"
                            ) {
                                return Ok(self.add_value(
                                    ValueKind::Call {
                                        callee: name.clone(),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                ));
                            }
                            if Self::is_dynamic_fallback_builtin(name) {
                                self.fn_ir
                                    .mark_hybrid_interop(Self::hybrid_interop_reason(name));
                                return Ok(self.add_value(
                                    ValueKind::Call {
                                        callee: name.clone(),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                ));
                            }
                            if Self::is_namespaced_r_call(name) {
                                if !Self::is_supported_package_call(name) {
                                    self.fn_ir.mark_opaque_interop_reason(
                                        Self::opaque_package_reason(name),
                                    );
                                }
                                return Ok(self.add_value(
                                    ValueKind::Call {
                                        callee: name.clone(),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                ));
                            }
                            if self.in_tidy_mask() && Self::is_tidy_helper_call(name) {
                                if !Self::is_supported_tidy_helper_call(name) {
                                    self.fn_ir.mark_opaque_interop_reason(
                                        Self::opaque_tidy_helper_reason(name),
                                    );
                                }
                                return Ok(self.add_value(
                                    ValueKind::Call {
                                        callee: name.clone(),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                ));
                            }
                            if let Some(expected) = self.known_functions.get(name) {
                                let _ = expected;
                                return Ok(self.add_value(
                                    ValueKind::Call {
                                        callee: format!("Sym_{}", sym.0),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                ));
                            }
                            let mut err = crate::error::RRException::new(
                                "RR.SemanticError",
                                crate::error::RRCode::E1001,
                                crate::error::Stage::Mir,
                                format!("undefined function '{}'", name),
                            )
                            .at(span)
                            .note("Define or import the function before calling it.");
                            if let Some(suggestion) = self.suggest_function_name(name) {
                                err = err.help(suggestion);
                            }
                            return Err(err);
                        }
                        Err(crate::error::RRException::new(
                            "RR.SemanticError",
                            crate::error::RRCode::E1001,
                            crate::error::Stage::Mir,
                            "invalid unresolved callee symbol".to_string(),
                        )
                        .at(span))
                    }
                    _ => {
                        let callee_val = self.lower_expr(callee.as_ref().clone())?;
                        let mut dyn_args = Vec::with_capacity(v_args.len() + 1);
                        dyn_args.push(callee_val);
                        dyn_args.extend(v_args);
                        let mut dyn_names = Vec::with_capacity(arg_names.len() + 1);
                        dyn_names.push(None);
                        dyn_names.extend(arg_names);
                        Ok(self.add_value(
                            ValueKind::Call {
                                callee: "rr_call_closure".to_string(),
                                args: dyn_args,
                                names: dyn_names,
                            },
                            span,
                        ))
                    }
                }
            }
            hir::HirExpr::IfExpr {
                cond,
                then_expr,
                else_expr,
            } => {
                let cond_val = self.lower_expr(*cond)?;

                let then_bb = self.fn_ir.add_block();
                let else_bb = self.fn_ir.add_block();
                let merge_bb = self.fn_ir.add_block();

                self.add_pred(then_bb, self.curr_block);
                self.add_pred(else_bb, self.curr_block);

                self.terminate(Terminator::If {
                    cond: cond_val,
                    then_bb,
                    else_bb,
                });

                // Then Branch
                self.curr_block = then_bb;
                // Seal Then? Only 1 pred.
                self.seal_block(then_bb)?;
                let then_val = self.lower_expr(*then_expr)?;
                if !self.is_terminated(then_bb) {
                    self.add_pred(merge_bb, self.curr_block);
                    self.terminate(Terminator::Goto(merge_bb));
                }
                let then_end_bb = self.curr_block;

                // Else Branch
                self.curr_block = else_bb;
                self.seal_block(else_bb)?;
                let else_val = self.lower_expr(*else_expr)?;
                if !self.is_terminated(else_bb) {
                    self.add_pred(merge_bb, self.curr_block);
                    self.terminate(Terminator::Goto(merge_bb));
                }
                let else_end_bb = self.curr_block;

                // Merge Branch
                self.curr_block = merge_bb;
                self.seal_block(merge_bb)?;

                // Phi for result value
                let phi_val = self.add_value(
                    ValueKind::Phi {
                        args: vec![(then_val, then_end_bb), (else_val, else_end_bb)],
                    },
                    Span::default(),
                );
                if let Some(v) = self.fn_ir.values.get_mut(phi_val) {
                    v.phi_block = Some(merge_bb);
                }

                Ok(phi_val)
            }
            hir::HirExpr::VectorLit(elems) => {
                let mut vals = Vec::new();
                for e in elems {
                    vals.push(self.lower_expr(e)?);
                }
                // Lower vector literals via R's `c(...)` constructor.
                let names = vec![None; vals.len()];
                Ok(self.add_value(
                    ValueKind::Call {
                        callee: "c".to_string(),
                        args: vals,
                        names,
                    },
                    Span::default(),
                ))
            }
            hir::HirExpr::ListLit(fields) => {
                let mut vals = Vec::new();
                for (sym, e) in fields {
                    let field = self
                        .symbols
                        .get(&sym)
                        .cloned()
                        .unwrap_or_else(|| format!("field_{}", sym.0));
                    vals.push((field, self.lower_expr(e)?));
                }
                Ok(self.add_value(ValueKind::RecordLit { fields: vals }, Span::default()))
            }
            hir::HirExpr::Range { start, end } => {
                let s = self.lower_expr(*start)?;
                let e = self.lower_expr(*end)?;
                Ok(self.add_value(ValueKind::Range { start: s, end: e }, Span::default()))
            }
            hir::HirExpr::Try(inner) => {
                // RR v1: try-postfix lowers to value propagation in MIR.
                // Runtime error propagation is still handled by R semantics.
                self.lower_expr(*inner)
            }
            hir::HirExpr::Match { scrut, arms } => self.lower_match_expr(*scrut, arms),
            hir::HirExpr::Column(name) => {
                Ok(self.add_value(ValueKind::RSymbol { name }, Span::default()))
            }
            hir::HirExpr::Unquote(e) => self.lower_expr(*e),
            _ => Err(crate::error::RRException::new(
                "RR.SemanticError",
                crate::error::RRCode::E1002,
                crate::error::Stage::Mir,
                format!("unsupported expression in MIR lowering: {:?}", expr),
            )
            .at(Span::default())
            .push_frame("mir::lower_hir::lower_expr/1", Some(Span::default()))),
        }
    }
}
