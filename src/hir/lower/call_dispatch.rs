use super::*;

mod fallback_call;
mod generic_call;
mod receiver_call;
mod trait_static_call;

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
        if let Some(call) = self.lower_generic_function_call(
            &callee,
            &type_args,
            &args,
            expected_ret_ty,
            span,
            callee_span,
        )? {
            return Ok(call);
        }
        if let Some(call) =
            self.lower_trait_associated_call(&callee, &type_args, &args, span, callee_span)?
        {
            return Ok(call);
        }
        self.reject_unsupported_explicit_type_args(&type_args, span)?;
        if let Some(call) =
            self.lower_static_receiver_method_call(&callee, &args, span, callee_span)?
        {
            return Ok(call);
        }
        if let Some(call) =
            self.lower_generic_bound_receiver_call(&callee, &args, span, callee_span)?
        {
            return Ok(call);
        }
        if let Some(err) = self.unresolved_receiver_method_error(&callee, span) {
            return Err(err);
        }
        if let Some(call) =
            self.lower_explicit_trait_method_call(&callee, &args, span, callee_span)?
        {
            return Ok(call);
        }
        self.lower_regular_or_dotted_call(callee, args, span, callee_span)
    }

    fn reject_unsupported_explicit_type_args(
        &self,
        type_args: &[ast::TypeExpr],
        span: Span,
    ) -> RR<()> {
        if type_args.is_empty() {
            return Ok(());
        }
        Err(RRException::new(
            "RR.SemanticError",
            RRCode::E1002,
            Stage::Lower,
            "explicit type arguments are only supported on generic function calls".to_string(),
        )
        .at(span))
    }
}
