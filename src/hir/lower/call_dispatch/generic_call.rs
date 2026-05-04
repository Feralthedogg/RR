use super::*;

impl Lowerer {
    pub(super) fn lower_generic_function_call(
        &mut self,
        callee: &ast::Expr,
        type_args: &[ast::TypeExpr],
        args: &[ast::Expr],
        expected_ret_ty: Option<&HirTypeRef>,
        span: Span,
        callee_span: Span,
    ) -> RR<Option<HirExpr>> {
        let ast::ExprKind::Name(name) = &callee.kind else {
            return Ok(None);
        };
        let Some(generic_sym) =
            self.resolve_generic_call(name, type_args, args, expected_ret_ty, span)?
        else {
            return Ok(None);
        };
        let hargs = self.lower_call_args(args.to_vec())?;
        Ok(Some(HirExpr::Call(HirCall {
            callee: Box::new(HirExpr::Global(generic_sym, callee_span)),
            args: hargs,
            span,
        })))
    }
}
