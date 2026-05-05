use super::*;

impl Lowerer {
    pub(super) fn lower_static_receiver_method_call(
        &mut self,
        callee: &ast::Expr,
        args: &[ast::Expr],
        span: Span,
        callee_span: Span,
    ) -> RR<Option<HirExpr>> {
        let ast::ExprKind::Field { base, name } = &callee.kind else {
            return Ok(None);
        };
        let Some(method_sym) = self.resolve_receiver_method_call(base, name, span)? else {
            return Ok(None);
        };
        let mut hargs = Vec::with_capacity(args.len() + 1);
        hargs.push(HirArg::Pos(self.lower_expr((**base).clone())?));
        hargs.extend(self.lower_call_args(args.to_vec())?);
        Ok(Some(HirExpr::Call(HirCall {
            callee: Box::new(HirExpr::Global(method_sym, callee_span)),
            args: hargs,
            span,
        })))
    }

    pub(super) fn lower_generic_bound_receiver_call(
        &mut self,
        callee: &ast::Expr,
        args: &[ast::Expr],
        span: Span,
        callee_span: Span,
    ) -> RR<Option<HirExpr>> {
        let ast::ExprKind::Field { base, name } = &callee.kind else {
            return Ok(None);
        };
        if !self.is_unresolved_trait_receiver_method(base, name) {
            return Ok(None);
        }
        let receiver_ty = self.trait_type_of_ast_expr(base);
        if !receiver_ty
            .as_ref()
            .is_some_and(|receiver_ty| self.type_ref_contains_current_type_param(receiver_ty))
        {
            return Ok(None);
        }
        let dotted_callee = Self::dotted_name_from_expr(callee);
        let c = if let Some(dotted) = dotted_callee.filter(|d| self.root_is_unbound_for_dotted(d)) {
            self.lower_dotted_ref(&dotted, callee_span)
        } else {
            self.lower_expr(callee.clone())?
        };
        let hargs = self.lower_call_args(args.to_vec())?;
        Ok(Some(HirExpr::Call(HirCall {
            callee: Box::new(c),
            args: hargs,
            span,
        })))
    }

    pub(super) fn unresolved_receiver_method_error(
        &self,
        callee: &ast::Expr,
        span: Span,
    ) -> Option<RRException> {
        let ast::ExprKind::Field { base, name } = &callee.kind else {
            return None;
        };
        if !self.is_unresolved_trait_receiver_method(base, name) {
            return None;
        }
        let receiver_ty = self.trait_type_of_ast_expr(base);
        let err = if let Some(receiver_ty) = receiver_ty {
            RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "cannot resolve receiver method '{}' for receiver type '{}'",
                    name,
                    receiver_ty.key()
                ),
            )
        } else {
            RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "receiver method '{}' requires a receiver with an explicit static type hint",
                    name
                ),
            )
        };
        Some(
            err.at(span)
                .label(
                    DiagnosticLabelKind::Use,
                    base.span,
                    "receiver expression has no known static trait dispatch target",
                )
                .note(
                    "RR receiver methods are statically dispatched. If the receiver type or bound is missing, `.method(...)` cannot be resolved as a trait call.",
                )
                .fix(
                    "add a receiver type hint, add the matching `where T: Trait` bound, or use explicit `Trait.method(receiver, ...)` syntax",
                ),
        )
    }

    pub(super) fn is_unresolved_trait_receiver_method(&self, base: &ast::Expr, name: &str) -> bool {
        self.any_trait_has_method(name)
            && !Self::dotted_name_from_field(base, name)
                .is_some_and(|dotted| self.root_is_unbound_for_dotted(&dotted))
    }
}
