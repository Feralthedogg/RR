use super::*;
impl Lowerer {
    pub(crate) fn lower_expr(&mut self, expr: ast::Expr) -> RR<HirExpr> {
        self.lower_expr_inner(expr, None)
    }
    pub(crate) fn lower_expr_with_expected(
        &mut self,
        expr: ast::Expr,
        expected_ret_ty: Option<&HirTypeRef>,
    ) -> RR<HirExpr> {
        self.lower_expr_inner(expr, expected_ret_ty)
    }
    pub(crate) fn lower_expr_inner(
        &mut self,
        expr: ast::Expr,
        expected_ret_ty: Option<&HirTypeRef>,
    ) -> RR<HirExpr> {
        match expr.kind {
            ast::ExprKind::Lit(l) => {
                let hl = match l {
                    ast::Lit::Int(i) => HirLit::Int(i),
                    ast::Lit::Float(f) => HirLit::Double(f),
                    ast::Lit::Str(s) => HirLit::Char(s),
                    ast::Lit::Bool(b) => HirLit::Bool(b),
                    ast::Lit::Na => HirLit::NA,
                    ast::Lit::Null => HirLit::Null,
                };
                Ok(HirExpr::Lit(hl))
            }
            ast::ExprKind::Name(n) => {
                if let Some(lid) = self.lookup(&n) {
                    Ok(HirExpr::Local(lid))
                } else if let Some(sym) = self.global_fn_aliases.get(&n).copied() {
                    Ok(HirExpr::Global(sym, expr.span))
                } else if let Some(sym) = self.r_import_aliases.get(&n).copied() {
                    Ok(HirExpr::Global(sym, expr.span))
                } else {
                    Ok(HirExpr::Global(self.intern_symbol(&n), expr.span))
                }
            }
            ast::ExprKind::Binary { op, lhs, rhs } => {
                if let Some((trait_name, method_name)) = Self::operator_trait_for_binop(op)
                    && let Some(lhs_ty) = self.trait_type_of_ast_expr(&lhs)
                    && let Some(type_key) = self.current_generic_ref_key(&lhs_ty)
                    && !self.generic_ref_has_trait_bound(&type_key, trait_name)
                {
                    return Err(RRException::new(
                        "RR.SemanticError",
                        RRCode::E1002,
                        Stage::Lower,
                        format!(
                            "generic operator '{}' requires bound `{}: {}`",
                            method_name, type_key, trait_name
                        ),
                    )
                    .at(expr.span));
                }
                if let Some((trait_name, method_name)) = Self::operator_trait_for_binop(op)
                    && let Some(lhs_ty) = self.trait_type_of_ast_expr(&lhs)
                    && let Some(method_sym) = self.trait_impl_method_for_type(
                        trait_name,
                        method_name,
                        &lhs_ty,
                        expr.span,
                    )?
                {
                    let hargs = vec![
                        HirArg::Pos(self.lower_expr(*lhs)?),
                        HirArg::Pos(self.lower_expr(*rhs)?),
                    ];
                    return Ok(HirExpr::Call(HirCall {
                        callee: Box::new(HirExpr::Global(method_sym, expr.span)),
                        args: hargs,
                        span: expr.span,
                    }));
                }
                let hop = Self::hir_binop(op);
                Ok(HirExpr::Binary {
                    op: hop,
                    lhs: Box::new(self.lower_expr(*lhs)?),
                    rhs: Box::new(self.lower_expr(*rhs)?),
                })
            }
            ast::ExprKind::Formula { lhs, rhs } => {
                if let Some(lhs) = lhs {
                    self.lower_formula_binary_expr(*lhs, *rhs, expr.span)
                } else {
                    self.lower_formula_unary_expr(*rhs, expr.span)
                }
            }
            ast::ExprKind::Unary { op, rhs } => {
                if matches!(op, ast::UnaryOp::Formula) {
                    return self.lower_formula_unary_expr(*rhs, expr.span);
                }
                if let Some((trait_name, method_name)) = Self::operator_trait_for_unop(op)
                    && let Some(rhs_ty) = self.trait_type_of_ast_expr(&rhs)
                    && let Some(type_key) = self.current_generic_ref_key(&rhs_ty)
                    && !self.generic_ref_has_trait_bound(&type_key, trait_name)
                {
                    return Err(RRException::new(
                        "RR.SemanticError",
                        RRCode::E1002,
                        Stage::Lower,
                        format!(
                            "generic operator '{}' requires bound `{}: {}`",
                            method_name, type_key, trait_name
                        ),
                    )
                    .at(expr.span));
                }
                if let Some((trait_name, method_name)) = Self::operator_trait_for_unop(op)
                    && let Some(rhs_ty) = self.trait_type_of_ast_expr(&rhs)
                    && let Some(method_sym) = self.trait_impl_method_for_type(
                        trait_name,
                        method_name,
                        &rhs_ty,
                        expr.span,
                    )?
                {
                    let hargs = vec![HirArg::Pos(self.lower_expr(*rhs)?)];
                    return Ok(HirExpr::Call(HirCall {
                        callee: Box::new(HirExpr::Global(method_sym, expr.span)),
                        args: hargs,
                        span: expr.span,
                    }));
                }
                let hop = match op {
                    ast::UnaryOp::Not => HirUnOp::Not,
                    ast::UnaryOp::Neg => HirUnOp::Neg,
                    ast::UnaryOp::Formula => {
                        return Err(InternalCompilerError::new(
                            Stage::Lower,
                            "formula unary reached HIR expression lowering after desugaring",
                        )
                        .at(expr.span)
                        .into_exception());
                    }
                };
                Ok(HirExpr::Unary {
                    op: hop,
                    expr: Box::new(self.lower_expr(*rhs)?),
                })
            }
            ast::ExprKind::Lambda {
                params,
                ret_ty_hint,
                body,
            } => self.lower_lambda_expr(params, ret_ty_hint, body, expr.span),
            ast::ExprKind::Call {
                callee,
                type_args,
                args,
            } => self.lower_call_expr(*callee, type_args, args, expected_ret_ty, expr.span),
            ast::ExprKind::Pipe { lhs, rhs_call } => {
                let lhs_h = self.lower_expr(*lhs)?;
                match rhs_call.kind {
                    ast::ExprKind::Call {
                        callee,
                        type_args: _,
                        args,
                    } => {
                        let c = self.lower_expr(*callee)?;
                        let mut hargs = Vec::with_capacity(args.len() + 1);
                        hargs.push(HirArg::Pos(lhs_h));
                        for a in args {
                            match a.kind {
                                ast::ExprKind::NamedArg { name, value } => {
                                    let sym = self.intern_symbol(&name);
                                    hargs.push(HirArg::Named {
                                        name: sym,
                                        value: self.lower_expr(*value)?,
                                    });
                                }
                                _ => hargs.push(HirArg::Pos(self.lower_expr(a)?)),
                            }
                        }
                        Ok(HirExpr::Call(HirCall {
                            callee: Box::new(c),
                            args: hargs,
                            span: expr.span,
                        }))
                    }
                    ast::ExprKind::Try { expr: inner } => match inner.kind {
                        ast::ExprKind::Call {
                            callee,
                            type_args: _,
                            args,
                        } => {
                            let c = self.lower_expr(*callee)?;
                            let mut hargs = Vec::with_capacity(args.len() + 1);
                            hargs.push(HirArg::Pos(lhs_h));
                            for a in args {
                                match a.kind {
                                    ast::ExprKind::NamedArg { name, value } => {
                                        let sym = self.intern_symbol(&name);
                                        hargs.push(HirArg::Named {
                                            name: sym,
                                            value: self.lower_expr(*value)?,
                                        });
                                    }
                                    _ => hargs.push(HirArg::Pos(self.lower_expr(a)?)),
                                }
                            }
                            let call = HirExpr::Call(HirCall {
                                callee: Box::new(c),
                                args: hargs,
                                span: expr.span,
                            });
                            Ok(HirExpr::Try(Box::new(call)))
                        }
                        _ => Err(RRException::new(
                            "RR.ParseError",
                            RRCode::E0001,
                            Stage::Lower,
                            "RHS of |> must be call or call?".to_string(),
                        )),
                    },
                    _ => Err(RRException::new(
                        "RR.ParseError",
                        RRCode::E0001,
                        Stage::Lower,
                        "RHS of |> must be call".to_string(),
                    )),
                }
            }
            ast::ExprKind::Field { base, name } => {
                if let Some(dotted) = Self::dotted_name_from_field(&base, &name)
                    .filter(|d| self.root_is_unbound_for_dotted(d))
                {
                    return Ok(self.lower_dotted_ref(&dotted, expr.span));
                }
                let b = self.lower_expr(*base)?;
                let sym = self.intern_symbol(&name);
                Ok(HirExpr::Field {
                    base: Box::new(b),
                    name: sym,
                })
            }
            // v6 features
            ast::ExprKind::Match { scrutinee, arms } => {
                let s = self.lower_expr(*scrutinee)?;
                let mut harms = Vec::new();
                for arm in arms {
                    self.enter_scope(); // Arm scope
                    let pat = self.lower_pattern(arm.pat)?;
                    let guard = if let Some(g) = arm.guard {
                        Some(self.lower_expr(*g)?)
                    } else {
                        None
                    };
                    let body = self.lower_expr(*arm.body)?;
                    self.exit_scope();

                    harms.push(HirMatchArm {
                        pat,
                        guard,
                        body,
                        span: arm.span,
                    });
                }
                Ok(HirExpr::Match {
                    scrut: Box::new(s),
                    arms: harms,
                })
            }
            ast::ExprKind::Try { expr: e } => Ok(HirExpr::Try(Box::new(self.lower_expr(*e)?))),
            ast::ExprKind::Column(n) => Ok(HirExpr::Column(n)),
            ast::ExprKind::ColRef(n) => Ok(HirExpr::Column(n)),
            ast::ExprKind::Unquote(e) => {
                let inner = self.lower_expr(*e)?;
                Ok(HirExpr::Unquote(Box::new(inner)))
            }
            ast::ExprKind::Index { base, idx } => {
                if let Some(base_ty) = self.trait_type_of_ast_expr(&base) {
                    if let Some(type_key) = self.current_generic_ref_key(&base_ty)
                        && !self.generic_ref_has_trait_bound(&type_key, "Index")
                    {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "generic index operation requires bound `{}: Index`",
                                type_key
                            ),
                        )
                        .at(expr.span));
                    }
                    if let Some(method_sym) =
                        self.trait_impl_method_for_type("Index", "index", &base_ty, expr.span)?
                    {
                        let mut hargs = Vec::with_capacity(idx.len() + 1);
                        hargs.push(HirArg::Pos(self.lower_expr(*base)?));
                        for i in idx {
                            hargs.push(HirArg::Pos(self.lower_expr(i)?));
                        }
                        return Ok(HirExpr::Call(HirCall {
                            callee: Box::new(HirExpr::Global(method_sym, expr.span)),
                            args: hargs,
                            span: expr.span,
                        }));
                    }
                }
                let b = self.lower_expr(*base)?;
                let mut indices = Vec::new();
                for i in idx {
                    indices.push(self.lower_expr(i)?);
                }
                Ok(HirExpr::Index {
                    base: Box::new(b),
                    index: indices,
                })
            }
            ast::ExprKind::Range { a, b } => {
                let start = self.lower_expr(*a)?;
                let end = self.lower_expr(*b)?;
                Ok(HirExpr::Range {
                    start: Box::new(start),
                    end: Box::new(end),
                })
            }
            ast::ExprKind::VectorLit(elems) => {
                let mut helems = Vec::new();
                for e in elems {
                    helems.push(self.lower_expr(e)?);
                }
                Ok(HirExpr::VectorLit(helems))
            }
            ast::ExprKind::RecordLit(fields) => {
                let mut hfields = Vec::new();
                for (k, v) in fields {
                    let sym = self.intern_symbol(&k);
                    hfields.push((sym, self.lower_expr(v)?));
                }
                Ok(HirExpr::ListLit(hfields))
            }
            _ => Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!("unsupported expression in HIR lowering: {:?}", expr.kind),
            )
            .at(expr.span)
            .push_frame("hir::lower::lower_expr/1", Some(expr.span))),
        }
    }
}
