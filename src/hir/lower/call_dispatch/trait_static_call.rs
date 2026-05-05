use super::*;

impl Lowerer {
    pub(super) fn lower_trait_associated_call(
        &mut self,
        callee: &ast::Expr,
        type_args: &[ast::TypeExpr],
        args: &[ast::Expr],
        span: Span,
        callee_span: Span,
    ) -> RR<Option<HirExpr>> {
        if type_args.is_empty() {
            return Ok(None);
        }
        let ast::ExprKind::Field { base, name } = &callee.kind else {
            return Ok(None);
        };
        let ast::ExprKind::Name(trait_name) = &base.kind else {
            return Ok(None);
        };
        match self.resolve_trait_assoc_const_call(trait_name, name, type_args, args, span)? {
            TraitAssocConstResolution::Concrete(sym) => {
                return Ok(Some(HirExpr::Call(HirCall {
                    callee: Box::new(HirExpr::Global(sym, callee_span)),
                    args: Vec::new(),
                    span,
                })));
            }
            TraitAssocConstResolution::GenericBound => {
                let c = self.lower_expr(callee.clone())?;
                return Ok(Some(HirExpr::Call(HirCall {
                    callee: Box::new(c),
                    args: Vec::new(),
                    span,
                })));
            }
            TraitAssocConstResolution::NotAssocConst => {}
        }
        match self.resolve_trait_static_method_call(trait_name, name, type_args, args, span)? {
            TraitStaticMethodResolution::Concrete(sym) => {
                let hargs = self.lower_call_args(args.to_vec())?;
                Ok(Some(HirExpr::Call(HirCall {
                    callee: Box::new(HirExpr::Global(sym, callee_span)),
                    args: hargs,
                    span,
                })))
            }
            TraitStaticMethodResolution::GenericBound => {
                let c = self.lower_expr(callee.clone())?;
                let hargs = self.lower_call_args(args.to_vec())?;
                Ok(Some(HirExpr::Call(HirCall {
                    callee: Box::new(c),
                    args: hargs,
                    span,
                })))
            }
            TraitStaticMethodResolution::NotStaticMethod => Ok(None),
        }
    }

    pub(super) fn lower_explicit_trait_method_call(
        &mut self,
        callee: &ast::Expr,
        args: &[ast::Expr],
        span: Span,
        callee_span: Span,
    ) -> RR<Option<HirExpr>> {
        let Some((trait_name, method_name)) = Self::trait_method_callee(callee) else {
            return Ok(None);
        };
        let Some(trait_method_sym) =
            self.resolve_trait_call(&trait_name, &method_name, args, span)?
        else {
            return Ok(None);
        };
        let hargs = self.lower_call_args(args.to_vec())?;
        Ok(Some(HirExpr::Call(HirCall {
            callee: Box::new(HirExpr::Global(trait_method_sym, callee_span)),
            args: hargs,
            span,
        })))
    }
}
