use super::*;

impl Lowerer {
    pub(super) fn lower_call_expr(
        &mut self,
        callee: ast::Expr,
        type_args: Vec<ast::TypeExpr>,
        args: Vec<ast::Expr>,
        expected_ret_ty: Option<&HirTypeRef>,
        span: Span,
    ) -> RR<HirExpr> {
        let callee_span = callee.span;
        if let ast::ExprKind::Name(name) = &callee.kind
            && let Some(generic_sym) =
                self.resolve_generic_call(name, &type_args, &args, expected_ret_ty, span)?
        {
            let hargs = self.lower_call_args(args)?;
            return Ok(HirExpr::Call(HirCall {
                callee: Box::new(HirExpr::Global(generic_sym, callee_span)),
                args: hargs,
                span,
            }));
        }
        if let ast::ExprKind::Field { base, name } = &callee.kind
            && !type_args.is_empty()
            && let ast::ExprKind::Name(trait_name) = &base.kind
        {
            match self.resolve_trait_assoc_const_call(trait_name, name, &type_args, &args, span)? {
                TraitAssocConstResolution::Concrete(sym) => {
                    return Ok(HirExpr::Call(HirCall {
                        callee: Box::new(HirExpr::Global(sym, callee_span)),
                        args: Vec::new(),
                        span,
                    }));
                }
                TraitAssocConstResolution::GenericBound => {
                    let c = self.lower_expr(callee)?;
                    return Ok(HirExpr::Call(HirCall {
                        callee: Box::new(c),
                        args: Vec::new(),
                        span,
                    }));
                }
                TraitAssocConstResolution::NotAssocConst => {}
            }
            match self
                .resolve_trait_static_method_call(trait_name, name, &type_args, &args, span)?
            {
                TraitStaticMethodResolution::Concrete(sym) => {
                    let hargs = self.lower_call_args(args)?;
                    return Ok(HirExpr::Call(HirCall {
                        callee: Box::new(HirExpr::Global(sym, callee_span)),
                        args: hargs,
                        span,
                    }));
                }
                TraitStaticMethodResolution::GenericBound => {
                    let c = self.lower_expr(callee)?;
                    let hargs = self.lower_call_args(args)?;
                    return Ok(HirExpr::Call(HirCall {
                        callee: Box::new(c),
                        args: hargs,
                        span,
                    }));
                }
                TraitStaticMethodResolution::NotStaticMethod => {}
            }
        }
        if !type_args.is_empty() {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                "explicit type arguments are only supported on generic function calls".to_string(),
            )
            .at(span));
        }
        if let ast::ExprKind::Field { base, name } = &callee.kind
            && let Some(method_sym) = self.resolve_receiver_method_call(base, name, span)?
        {
            let mut hargs = Vec::with_capacity(args.len() + 1);
            hargs.push(HirArg::Pos(self.lower_expr((**base).clone())?));
            hargs.extend(self.lower_call_args(args)?);
            return Ok(HirExpr::Call(HirCall {
                callee: Box::new(HirExpr::Global(method_sym, callee_span)),
                args: hargs,
                span,
            }));
        }
        if let Some((trait_name, method_name)) = Self::trait_method_callee(&callee)
            && let Some(trait_method_sym) =
                self.resolve_trait_call(&trait_name, &method_name, &args, span)?
        {
            let hargs = self.lower_call_args(args)?;
            return Ok(HirExpr::Call(HirCall {
                callee: Box::new(HirExpr::Global(trait_method_sym, callee_span)),
                args: hargs,
                span,
            }));
        }
        let dotted_callee = Self::dotted_name_from_expr(&callee);
        let c = if let Some(dotted) = dotted_callee.filter(|d| self.root_is_unbound_for_dotted(d)) {
            self.lower_dotted_ref(&dotted, callee_span)
        } else {
            self.lower_expr(callee)?
        };
        let hargs = self.lower_call_args(args)?;
        Ok(HirExpr::Call(HirCall {
            callee: Box::new(c),
            args: hargs,
            span,
        }))
    }
}
