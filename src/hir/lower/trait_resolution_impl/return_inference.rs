use super::*;
impl Lowerer {
    pub(crate) fn trait_method_callee(callee: &ast::Expr) -> Option<(String, String)> {
        let ast::ExprKind::Field { base, name } = &callee.kind else {
            return None;
        };
        let ast::ExprKind::Name(trait_name) = &base.kind else {
            return None;
        };
        Some((trait_name.clone(), name.clone()))
    }
    pub(crate) fn trait_receiver_expr(args: &[ast::Expr]) -> Option<&ast::Expr> {
        let first = args.first()?;
        match &first.kind {
            ast::ExprKind::NamedArg { name, value } if name == "self" => Some(value),
            _ => Some(first),
        }
    }
    pub(crate) fn trait_type_of_ast_expr(&self, expr: &ast::Expr) -> Option<HirTypeRef> {
        match &expr.kind {
            ast::ExprKind::Name(name) => {
                let local = self.lookup(name)?;
                self.local_trait_types.get(&local).cloned()
            }
            ast::ExprKind::Lit(ast::Lit::Int(_)) => Some(HirTypeRef::Named("int".to_string())),
            ast::ExprKind::Lit(ast::Lit::Float(_)) => Some(HirTypeRef::Named("float".to_string())),
            ast::ExprKind::Lit(ast::Lit::Bool(_)) => Some(HirTypeRef::Named("bool".to_string())),
            ast::ExprKind::Lit(ast::Lit::Str(_)) => Some(HirTypeRef::Named("str".to_string())),
            ast::ExprKind::Lit(ast::Lit::Null) => Some(HirTypeRef::Named("null".to_string())),
            ast::ExprKind::Binary { op, lhs, .. } => {
                let (trait_name, method_name) = Self::operator_trait_for_binop(*op)?;
                let lhs_ty = self.trait_type_of_ast_expr(lhs)?;
                self.trait_method_return_type_for_receiver(&lhs_ty, trait_name, method_name)
            }
            ast::ExprKind::Unary { op, rhs } => {
                let (trait_name, method_name) = Self::operator_trait_for_unop(*op)?;
                let rhs_ty = self.trait_type_of_ast_expr(rhs)?;
                self.trait_method_return_type_for_receiver(&rhs_ty, trait_name, method_name)
            }
            ast::ExprKind::Call {
                callee,
                type_args,
                args,
            } => self.trait_type_of_call_expr(callee, type_args, args),
            _ => None,
        }
    }
    pub(crate) fn trait_type_of_call_expr(
        &self,
        callee: &ast::Expr,
        type_args: &[ast::TypeExpr],
        args: &[ast::Expr],
    ) -> Option<HirTypeRef> {
        if let ast::ExprKind::Name(name) = &callee.kind {
            let info = self.generic_fns.get(name)?;
            let subst = self
                .infer_generic_subst(&info.decl, type_args, args, None, callee.span)
                .ok()?;
            let ret_ty = info.decl.ret_ty_hint.clone()?;
            let ret_ty = Self::substitute_type_expr(ret_ty, &subst);
            return Some(Self::ast_type_ref(&ret_ty));
        }

        if let Some((trait_name, method_name)) = Self::trait_method_callee(callee)
            && self.trait_defs.contains_key(&trait_name)
            && let Some(receiver) = Self::trait_receiver_expr(args)
        {
            let receiver_ty = self.trait_type_of_ast_expr(receiver)?;
            return self.trait_method_return_type_for_receiver(
                &receiver_ty,
                &trait_name,
                &method_name,
            );
        }

        if let ast::ExprKind::Field { base, name } = &callee.kind {
            let receiver_ty = self.trait_type_of_ast_expr(base)?;
            return self.trait_method_return_type_for_receiver(&receiver_ty, "", name);
        }

        None
    }
    pub(crate) fn trait_method_return_type_for_receiver(
        &self,
        receiver_ty: &HirTypeRef,
        explicit_trait_name: &str,
        method_name: &str,
    ) -> Option<HirTypeRef> {
        if !explicit_trait_name.is_empty() {
            return self.trait_method_return_type(explicit_trait_name, method_name, receiver_ty);
        }

        if let Some(type_key) = self.current_generic_ref_key(receiver_ty) {
            let candidates = self.type_param_method_bound_candidates(&type_key, method_name);
            return match candidates.as_slice() {
                [trait_name] => self.trait_method_return_type(trait_name, method_name, receiver_ty),
                _ => None,
            };
        }

        if self.type_ref_contains_current_type_param(receiver_ty) {
            return None;
        }

        let receiver_key = receiver_ty.key();
        let mut candidates = self
            .trait_impls
            .iter()
            .filter_map(|((trait_name, for_ty), impl_info)| {
                (for_ty == &receiver_key && impl_info.method_symbols.contains_key(method_name))
                    .then_some(trait_name.clone())
            })
            .collect::<Vec<_>>();
        candidates.sort();
        candidates.dedup();
        match candidates.as_slice() {
            [trait_name] => self.trait_method_return_type(trait_name, method_name, receiver_ty),
            _ => None,
        }
    }
    pub(crate) fn trait_method_return_type(
        &self,
        trait_name: &str,
        method_name: &str,
        receiver_ty: &HirTypeRef,
    ) -> Option<HirTypeRef> {
        let ret_ty = self.trait_method_return_type_expr(trait_name, method_name)?;
        Some(Self::substitute_self_type_ref(
            Self::ast_type_ref(&ret_ty),
            receiver_ty,
        ))
    }
    pub(crate) fn trait_method_return_type_expr(
        &self,
        trait_name: &str,
        method_name: &str,
    ) -> Option<ast::TypeExpr> {
        let mut stack = vec![trait_name.to_string()];
        let mut seen = FxHashSet::default();
        while let Some(name) = stack.pop() {
            if !seen.insert(name.clone()) {
                continue;
            }
            let Some(info) = self.trait_defs.get(&name) else {
                continue;
            };
            if let Some(method) = info
                .decl
                .methods
                .iter()
                .find(|method| method.name == method_name)
            {
                return method.ret_ty_hint.clone();
            }
            stack.extend(info.decl.supertraits.iter().cloned());
        }
        None
    }
    pub(crate) fn substitute_self_type_ref(ty: HirTypeRef, receiver_ty: &HirTypeRef) -> HirTypeRef {
        match ty {
            HirTypeRef::Named(name) if name == "Self" => receiver_ty.clone(),
            HirTypeRef::Named(name) => HirTypeRef::Named(name),
            HirTypeRef::Generic { base, args } => HirTypeRef::Generic {
                base,
                args: args
                    .into_iter()
                    .map(|arg| Self::substitute_self_type_ref(arg, receiver_ty))
                    .collect(),
            },
        }
    }
}
