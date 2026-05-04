use super::*;
impl Lowerer {
    pub(crate) fn apply_implicit_tail_return(mut body: HirBlock) -> HirBlock {
        if body
            .stmts
            .iter()
            .any(|s| matches!(s, HirStmt::Return { .. }))
        {
            return body;
        }
        if let Some(last) = body.stmts.pop() {
            match last {
                HirStmt::Expr { expr, span } => {
                    body.stmts.push(HirStmt::Return {
                        value: Some(expr),
                        span,
                    });
                }
                other => body.stmts.push(other),
            }
        }
        body
    }
    pub(crate) fn resolve_or_declare_local_for_assign(
        &mut self,
        name: &str,
        span: Span,
    ) -> RR<LocalId> {
        if let Some(lid) = self.lookup(name) {
            return Ok(lid);
        }
        if self.strict_let {
            let mut err = RRException::new(
                "RR.SemanticError",
                RRCode::E1001,
                Stage::Lower,
                format!("assignment to undeclared variable '{}'", name),
            )
            .at(span)
            .note("Declare it first with `let` before assignment.");
            if let Some(suggestion) = did_you_mean(
                name,
                self.scopes.iter().flat_map(|scope| scope.keys().cloned()),
            ) {
                err = err.help(suggestion);
            }
            return Err(err);
        }
        let lid = self.declare_local(name);
        if self.warn_implicit_decl {
            let where_msg = if span.start_line > 0 {
                format!("{}:{}", span.start_line, span.start_col)
            } else {
                "unknown".to_string()
            };
            self.warnings.push(format!(
                "implicit declaration via assignment: '{}' at {} (treated as `let {} = ...;`). Use an explicit lowering policy to forbid or allow this legacy behavior.",
                name, where_msg, name
            ));
        }
        Ok(lid)
    }
    pub(crate) fn lower_block(&mut self, block: ast::Block) -> RR<HirBlock> {
        let mut stmts = Vec::new();
        for s in block.stmts {
            stmts.push(self.lower_stmt(s)?);
        }
        Ok(HirBlock {
            stmts,
            span: block.span,
        })
    }
    pub(crate) fn lower_stmt(&mut self, stmt: ast::Stmt) -> RR<HirStmt> {
        match stmt.kind {
            ast::StmtKind::Let {
                name,
                ty_hint,
                init,
            } => {
                let dyn_trait = ty_hint.as_ref().and_then(Self::dyn_trait_name);
                let init_trait_ty = init
                    .as_ref()
                    .and_then(|expr| self.trait_type_of_ast_expr(expr));
                let expected_ty = if dyn_trait.is_some() {
                    init_trait_ty.clone()
                } else {
                    ty_hint.as_ref().map(Self::ast_type_ref)
                };
                let val = if let Some(e) = init {
                    Some(self.lower_expr_with_expected(e, expected_ty.as_ref())?)
                } else {
                    None
                };
                let lid = self.declare_local(&name);
                if let Some(trait_name) = dyn_trait {
                    if let Some(concrete_ty) = init_trait_ty.clone() {
                        if !self.ensure_trait_impl_for_type(trait_name, &concrete_ty, stmt.span)? {
                            return Err(RRException::new(
                                "RR.SemanticError",
                                RRCode::E1002,
                                Stage::Lower,
                                format!(
                                    "dyn trait binding requires '{}' to implement '{}'",
                                    concrete_ty.key(),
                                    trait_name
                                ),
                            )
                            .at(stmt.span));
                        }
                        self.local_trait_types.insert(lid, concrete_ty);
                    } else if let Some(ty_hint) = &ty_hint {
                        self.local_trait_types
                            .insert(lid, Self::ast_type_ref(ty_hint));
                    }
                } else if let Some(ty_hint) = &ty_hint {
                    self.local_trait_types
                        .insert(lid, Self::ast_type_ref(ty_hint));
                } else if let Some(inferred_ty) = init_trait_ty {
                    self.local_trait_types.insert(lid, inferred_ty);
                }
                let sym = self.intern_symbol(&name);
                if self.scopes.len() == 1
                    && let Some(HirExpr::Global(global_sym, _)) = &val
                    && self
                        .symbols
                        .get(global_sym)
                        .map(|s| s.starts_with("__lambda_"))
                        .unwrap_or(false)
                {
                    self.global_fn_aliases.insert(name.clone(), *global_sym);
                }
                Ok(HirStmt::Let {
                    local: lid,
                    name: sym,
                    ty: ty_hint.as_ref().and_then(Self::parse_type_hint_expr),
                    init: val,
                    span: stmt.span,
                })
            }
            ast::StmtKind::Assign { target, value } => {
                let lhs = self.lower_lvalue(target)?;
                let rhs = self.lower_expr(value)?;
                if self.scopes.len() == 1
                    && let HirLValue::Local(_lid) = &lhs
                    && let HirExpr::Global(global_sym, _) = &rhs
                    && self
                        .symbols
                        .get(global_sym)
                        .map(|s| s.starts_with("__lambda_"))
                        .unwrap_or(false)
                    && let Some(name) = self.local_name_of_lvalue(&lhs)
                {
                    self.global_fn_aliases.insert(name, *global_sym);
                }
                Ok(HirStmt::Assign {
                    target: lhs,
                    value: rhs,
                    span: stmt.span,
                })
            }
            ast::StmtKind::If {
                cond,
                then_blk,
                else_blk,
            } => {
                let c = self.lower_expr(cond)?;
                self.enter_scope();
                let t = self.lower_block(then_blk)?;
                self.exit_scope();
                let e = if let Some(blk) = else_blk {
                    self.enter_scope();
                    let lowered = self.lower_block(blk)?;
                    self.exit_scope();
                    Some(lowered)
                } else {
                    None
                };
                Ok(HirStmt::If {
                    cond: c,
                    then_blk: t,
                    else_blk: e,
                    span: stmt.span,
                })
            }
            ast::StmtKind::While { cond, body } => {
                let c = self.lower_expr(cond)?;
                self.enter_scope();
                let b = self.lower_block(body)?;
                self.exit_scope();
                Ok(HirStmt::While {
                    cond: c,
                    body: b,
                    span: stmt.span,
                })
            }
            ast::StmtKind::For { var, iter, body } => {
                let iter_expr = self.lower_expr(iter)?;
                self.enter_scope();
                let lid = self.declare_local(&var);

                // Canonicalize known iterator forms for better downstream optimization.
                let iter_kind = match iter_expr {
                    HirExpr::Range { start, end } => HirForIter::Range {
                        var: lid,
                        start: *start,
                        end: *end,
                        inclusive: true,
                    },
                    HirExpr::Call(call) => {
                        let one_arg = call.args.len() == 1;
                        match (&*call.callee, one_arg) {
                            (HirExpr::Global(sym, _), true) => {
                                let name = self.symbols.get(sym).cloned().unwrap_or_default();
                                let arg_expr = match call.args[0].clone() {
                                    HirArg::Pos(e) => e,
                                    HirArg::Named { value, .. } => value,
                                };
                                if name == "seq_len" {
                                    HirForIter::SeqLen {
                                        var: lid,
                                        len: arg_expr,
                                    }
                                } else if name == "seq_along" {
                                    HirForIter::SeqAlong {
                                        var: lid,
                                        xs: arg_expr,
                                    }
                                } else {
                                    HirForIter::SeqAlong {
                                        var: lid,
                                        xs: HirExpr::Call(call),
                                    }
                                }
                            }
                            _ => HirForIter::SeqAlong {
                                var: lid,
                                xs: HirExpr::Call(call),
                            },
                        }
                    }
                    other => HirForIter::SeqAlong {
                        var: lid,
                        xs: other,
                    },
                };

                let body_hir = self.lower_block(body)?;
                self.exit_scope();
                Ok(HirStmt::For {
                    iter: iter_kind,
                    body: body_hir,
                    span: stmt.span,
                })
            }
            ast::StmtKind::Return { value } => {
                let v = if let Some(e) = value {
                    Some(self.lower_expr(e)?)
                } else {
                    None
                };
                Ok(HirStmt::Return {
                    value: v,
                    span: stmt.span,
                })
            }
            ast::StmtKind::Break => Ok(HirStmt::Break { span: stmt.span }),
            ast::StmtKind::Next => Ok(HirStmt::Next { span: stmt.span }),
            ast::StmtKind::UnsafeRBlock { code, read_only } => Ok(HirStmt::UnsafeRBlock {
                code,
                read_only,
                span: stmt.span,
            }),
            ast::StmtKind::ExprStmt { expr } => Ok(HirStmt::Expr {
                expr: self.lower_expr(expr)?,
                span: stmt.span,
            }),
            _ => Err(RRException::new(
                "Feature.NotImpl",
                RRCode::E3001,
                Stage::Lower,
                "Stmt kind not supported".to_string(),
            )),
        }
    }
    pub(crate) fn lower_pattern(&mut self, pat: ast::Pattern) -> RR<HirPat> {
        match pat.kind {
            ast::PatternKind::Wild => Ok(HirPat::Wild),
            ast::PatternKind::Lit(l) => {
                let hl = match l {
                    ast::Lit::Int(i) => HirLit::Int(i),
                    ast::Lit::Float(f) => HirLit::Double(f),
                    ast::Lit::Str(s) => HirLit::Char(s),
                    ast::Lit::Bool(b) => HirLit::Bool(b),
                    ast::Lit::Na => HirLit::NA,
                    ast::Lit::Null => HirLit::Null,
                };
                Ok(HirPat::Lit(hl))
            }
            ast::PatternKind::Bind(n) => {
                let lid = self.declare_local(&n);
                let sym = self.intern_symbol(&n);
                Ok(HirPat::Bind {
                    name: sym,
                    local: lid,
                })
            }
            ast::PatternKind::List { items, rest } => {
                let mut hitems = Vec::new();
                for i in items {
                    hitems.push(self.lower_pattern(i)?);
                }

                let hrest = if let Some(n) = rest {
                    let lid = self.declare_local(&n);
                    let sym = self.intern_symbol(&n);
                    Some((sym, lid))
                } else {
                    None
                };
                Ok(HirPat::List {
                    items: hitems,
                    rest: hrest,
                })
            }
            ast::PatternKind::Record { fields } => {
                let mut hfields = Vec::new();
                for (name, p) in fields {
                    let sym = self.intern_symbol(&name);
                    let hp = self.lower_pattern(p)?;
                    hfields.push((sym, hp));
                }
                Ok(HirPat::Record { fields: hfields })
            }
        }
    }
    pub(crate) fn lower_lvalue(&mut self, lval: ast::LValue) -> RR<HirLValue> {
        let lv_span = lval.span;
        match lval.kind {
            ast::LValueKind::Name(n) => {
                let lid = self.resolve_or_declare_local_for_assign(&n, lv_span)?;
                Ok(HirLValue::Local(lid))
            }
            ast::LValueKind::Index { base, idx } => {
                let b = self.lower_expr(base)?;
                let mut indices = Vec::new();
                for i in idx {
                    indices.push(self.lower_expr(i)?);
                }
                Ok(HirLValue::Index {
                    base: b,
                    index: indices,
                })
            }
            ast::LValueKind::Field { base, name } => {
                if let Some(dotted) = Self::dotted_name_from_field(&base, &name)
                    .filter(|d| self.root_is_unbound_for_dotted(d))
                {
                    let lid = self.resolve_or_declare_local_for_assign(&dotted, lv_span)?;
                    return Ok(HirLValue::Local(lid));
                }
                let b = self.lower_expr(base)?;
                let sym = self.intern_symbol(&name);
                Ok(HirLValue::Field { base: b, name: sym })
            }
        }
    }
    pub(crate) fn local_name_of_lvalue(&self, lval: &HirLValue) -> Option<String> {
        match lval {
            HirLValue::Local(id) => self.local_names.get(id).cloned(),
            _ => None,
        }
    }
}
