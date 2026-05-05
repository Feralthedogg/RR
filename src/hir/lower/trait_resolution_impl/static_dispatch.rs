use super::*;
impl Lowerer {
    pub(crate) fn resolve_trait_assoc_const_call(
        &mut self,
        trait_name: &str,
        const_name: &str,
        type_args: &[ast::TypeExpr],
        args: &[ast::Expr],
        span: Span,
    ) -> RR<TraitAssocConstResolution> {
        let Some(trait_info) = self.trait_defs.get(trait_name) else {
            return Ok(TraitAssocConstResolution::NotAssocConst);
        };
        if !trait_info
            .decl
            .assoc_consts
            .iter()
            .any(|assoc_const| assoc_const.name == const_name)
        {
            return Ok(TraitAssocConstResolution::NotAssocConst);
        }
        if !args.is_empty() {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "associated const '{}.{}' does not accept call arguments",
                    trait_name, const_name
                ),
            )
            .at(span));
        }
        if type_args.len() != 1 {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "associated const '{}.{}' requires exactly one explicit receiver type argument",
                    trait_name, const_name
                ),
            )
            .at(span));
        }
        let receiver_ty = Self::ast_type_ref(&type_args[0]);
        if let Some(type_key) = self.current_generic_ref_key(&receiver_ty) {
            if self.generic_ref_has_trait_bound(&type_key, trait_name) {
                return Ok(TraitAssocConstResolution::GenericBound);
            }
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "generic associated const '{}.{}' requires bound `{}: {}`",
                    trait_name, const_name, type_key, trait_name
                ),
            )
            .at(span));
        }
        let receiver_key = receiver_ty.key();
        if !self.ensure_trait_impl_for_type(trait_name, &receiver_ty, span)? {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "no impl of trait '{}' for associated const receiver type '{}'",
                    trait_name, receiver_key
                ),
            )
            .at(span));
        }
        let Some(mangled) = self
            .trait_impls
            .get(&(trait_name.to_string(), receiver_key.clone()))
            .and_then(|impl_info| impl_info.const_symbols.get(const_name))
            .cloned()
        else {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "impl of trait '{}' for '{}' has no associated const '{}'",
                    trait_name, receiver_key, const_name
                ),
            )
            .at(span));
        };
        Ok(TraitAssocConstResolution::Concrete(
            self.intern_symbol(&mangled),
        ))
    }
    pub(crate) fn resolve_trait_static_method_call(
        &mut self,
        trait_name: &str,
        method_name: &str,
        type_args: &[ast::TypeExpr],
        args: &[ast::Expr],
        span: Span,
    ) -> RR<TraitStaticMethodResolution> {
        let Some(trait_info) = self.trait_defs.get(trait_name) else {
            return Ok(TraitStaticMethodResolution::NotStaticMethod);
        };
        let Some(method) = trait_info
            .decl
            .methods
            .iter()
            .find(|method| method.name == method_name)
        else {
            return Ok(TraitStaticMethodResolution::NotStaticMethod);
        };
        if type_args.len() != 1 {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "static trait method '{}.{}' requires exactly one explicit receiver type argument",
                    trait_name, method_name
                ),
            )
            .at(span));
        }
        if method
            .params
            .first()
            .is_some_and(|param| param.name == "self")
            && args.is_empty()
        {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "trait method '{}.{}' requires a receiver argument",
                    trait_name, method_name
                ),
            )
            .at(span));
        }

        let receiver_ty = Self::ast_type_ref(&type_args[0]);
        if let Some(type_key) = self.current_generic_ref_key(&receiver_ty) {
            if self.generic_ref_has_trait_bound(&type_key, trait_name) {
                return Ok(TraitStaticMethodResolution::GenericBound);
            }
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "generic static trait method '{}.{}' requires bound `{}: {}`",
                    trait_name, method_name, type_key, trait_name
                ),
            )
            .at(span));
        }

        let receiver_key = receiver_ty.key();
        if !self.ensure_trait_impl_for_type(trait_name, &receiver_ty, span)? {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "no impl of trait '{}' for static method receiver type '{}'",
                    trait_name, receiver_key
                ),
            )
            .at(span));
        }
        let Some(mangled) = self
            .trait_impls
            .get(&(trait_name.to_string(), receiver_key.clone()))
            .and_then(|impl_info| impl_info.method_symbols.get(method_name))
            .cloned()
        else {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "impl of trait '{}' for '{}' has no method '{}'",
                    trait_name, receiver_key, method_name
                ),
            )
            .at(span));
        };
        Ok(TraitStaticMethodResolution::Concrete(
            self.intern_symbol(&mangled),
        ))
    }
    pub(crate) fn receiver_method_candidates(
        &mut self,
        receiver_ty: &HirTypeRef,
        method_name: &str,
        span: Span,
    ) -> RR<Vec<(String, String)>> {
        for trait_name in self
            .generic_trait_impls
            .iter()
            .filter(|info| {
                info.decl
                    .methods
                    .iter()
                    .any(|method| method.name == method_name)
                    && self
                        .trait_defs
                        .get(&info.decl.trait_name)
                        .is_some_and(|trait_info| {
                            trait_info
                                .decl
                                .methods
                                .iter()
                                .any(|method| method.name == method_name)
                        })
            })
            .map(|info| info.decl.trait_name.clone())
            .collect::<Vec<_>>()
        {
            self.ensure_trait_impl_for_type(&trait_name, receiver_ty, span)?;
        }
        let receiver_key = receiver_ty.key();
        let mut candidates = Vec::new();
        for ((trait_name, for_ty), impl_info) in &self.trait_impls {
            if for_ty != &receiver_key {
                continue;
            }
            if let Some(mangled) = impl_info.method_symbols.get(method_name) {
                candidates.push((trait_name.clone(), mangled.clone()));
            }
        }
        candidates.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(candidates)
    }
    pub(crate) fn resolve_receiver_method_call(
        &mut self,
        receiver: &ast::Expr,
        method_name: &str,
        span: Span,
    ) -> RR<Option<SymbolId>> {
        let Some(receiver_ty) = self.trait_type_of_ast_expr(receiver) else {
            return Ok(None);
        };
        if let Some(type_key) = self.current_generic_ref_key(&receiver_ty) {
            let candidates = self.type_param_method_bound_candidates(&type_key, method_name);
            return match candidates.as_slice() {
                [] => Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "generic receiver type '{}' uses method '{}' without a matching trait bound",
                        type_key, method_name
                    ),
                )
                .at(span)
                .note(format!(
                    "Add a bound such as `where {}: TraitWith{}` before using this method.",
                    type_key,
                    method_name
                        .chars()
                        .next()
                        .map(|ch| ch.to_uppercase().collect::<String>())
                        .unwrap_or_default()
                ))),
                [_] => Ok(None),
                _ => Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "ambiguous generic trait method '{}.{}'; bounds [{}] all provide it. Use explicit Trait.method(receiver, ...) syntax",
                        type_key,
                        method_name,
                        candidates.join(", ")
                    ),
                )
                .at(span)),
            };
        }
        if self.type_ref_contains_current_type_param(&receiver_ty) {
            return Ok(None);
        }
        let candidates = self.receiver_method_candidates(&receiver_ty, method_name, span)?;
        match candidates.as_slice() {
            [] => Ok(None),
            [(_, mangled)] => Ok(Some(self.intern_symbol(mangled))),
            _ => {
                let trait_names = candidates
                    .iter()
                    .map(|(trait_name, _)| trait_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "ambiguous trait method '{}.{}' for receiver type '{}'; candidates are [{}]. Use explicit Trait.method(receiver, ...) syntax",
                        receiver_ty.key(),
                        method_name,
                        receiver_ty.key(),
                        trait_names
                    ),
                )
                .at(span))
            }
        }
    }
    pub(crate) fn resolve_trait_call(
        &mut self,
        trait_name: &str,
        method_name: &str,
        args: &[ast::Expr],
        span: Span,
    ) -> RR<Option<SymbolId>> {
        let Some(trait_info) = self.trait_defs.get(trait_name) else {
            return Ok(None);
        };
        if !trait_info
            .decl
            .methods
            .iter()
            .any(|method| method.name == method_name)
        {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!("trait '{}' has no method '{}'", trait_name, method_name),
            )
            .at(span));
        }
        let Some(receiver) = Self::trait_receiver_expr(args) else {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "trait method '{}.{}' requires a receiver argument",
                    trait_name, method_name
                ),
            )
            .at(span));
        };
        let Some(receiver_ty) = self.trait_type_of_ast_expr(receiver) else {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "trait method '{}.{}' requires a receiver with an explicit static type hint",
                    trait_name, method_name
                ),
            )
            .at(receiver.span));
        };
        if let Some(type_key) = self.current_generic_ref_key(&receiver_ty) {
            if self.generic_ref_has_trait_bound(&type_key, trait_name) {
                return Ok(None);
            }
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "generic trait call '{}.{}' requires bound `{}: {}`",
                    trait_name, method_name, type_key, trait_name
                ),
            )
            .at(receiver.span));
        }
        let receiver_key = receiver_ty.key();
        if !self.ensure_trait_impl_for_type(trait_name, &receiver_ty, receiver.span)? {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "no impl of trait '{}' for receiver type '{}'",
                    trait_name, receiver_key
                ),
            )
            .at(receiver.span));
        }
        let Some(impl_info) = self
            .trait_impls
            .get(&(trait_name.to_string(), receiver_key.clone()))
        else {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "no impl of trait '{}' for receiver type '{}'",
                    trait_name, receiver_key
                ),
            )
            .at(receiver.span));
        };
        let Some(mangled) = impl_info.method_symbols.get(method_name).cloned() else {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "impl of trait '{}' for '{}' has no method '{}'",
                    trait_name, receiver_key, method_name
                ),
            )
            .at(span));
        };
        Ok(Some(self.intern_symbol(&mangled)))
    }
}
