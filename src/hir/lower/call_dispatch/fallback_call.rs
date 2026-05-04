use super::*;

impl Lowerer {
    pub(super) fn lower_regular_or_dotted_call(
        &mut self,
        callee: ast::Expr,
        args: Vec<ast::Expr>,
        span: Span,
        callee_span: Span,
    ) -> RR<HirExpr> {
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
