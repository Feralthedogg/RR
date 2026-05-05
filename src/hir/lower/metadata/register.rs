use super::*;
pub(crate) struct ImplRegistrationPlan {
    pub(crate) trait_info: TraitDeclInfo,
    pub(crate) for_ty: HirTypeRef,
    pub(crate) impl_key: (String, String),
    pub(crate) is_generic_impl: bool,
    pub(crate) is_public_impl: bool,
    pub(crate) header: TraitImplHeader,
}

impl Lowerer {
    pub(crate) fn register_trait_decls(&mut self, stmts: &[ast::Stmt]) -> RR<()> {
        for stmt in stmts {
            if let ast::StmtKind::TraitDecl(decl) = &stmt.kind {
                if self.trait_defs.contains_key(&decl.name) {
                    return Err(RRException::new(
                        "RR.SemanticError",
                        RRCode::E1002,
                        Stage::Lower,
                        format!("duplicate trait declaration '{}'", decl.name),
                    )
                    .at(stmt.span));
                }
                let mut method_names = FxHashSet::default();
                let mut assoc_item_names = FxHashSet::default();
                for method in &decl.methods {
                    if !method_names.insert(method.name.clone()) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "duplicate method '{}' in trait '{}'",
                                method.name, decl.name
                            ),
                        )
                        .at(method.span));
                    }
                    if !assoc_item_names.insert(method.name.clone()) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "duplicate associated item '{}' in trait '{}'",
                                method.name, decl.name
                            ),
                        )
                        .at(method.span));
                    }
                }
                let mut assoc_type_names = FxHashSet::default();
                for assoc_ty in &decl.assoc_types {
                    if !assoc_type_names.insert(assoc_ty.name.clone()) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "duplicate associated type '{}' in trait '{}'",
                                assoc_ty.name, decl.name
                            ),
                        )
                        .at(assoc_ty.span));
                    }
                    if !assoc_item_names.insert(assoc_ty.name.clone()) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "duplicate associated item '{}' in trait '{}'",
                                assoc_ty.name, decl.name
                            ),
                        )
                        .at(assoc_ty.span));
                    }
                }
                let mut assoc_const_names = FxHashSet::default();
                for assoc_const in &decl.assoc_consts {
                    if !assoc_const_names.insert(assoc_const.name.clone()) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "duplicate associated const '{}' in trait '{}'",
                                assoc_const.name, decl.name
                            ),
                        )
                        .at(assoc_const.span));
                    }
                    if !assoc_item_names.insert(assoc_const.name.clone()) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "duplicate associated item '{}' in trait '{}'",
                                assoc_const.name, decl.name
                            ),
                        )
                        .at(assoc_const.span));
                    }
                }
                for supertrait in &decl.supertraits {
                    if supertrait == &decl.name {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!("trait '{}' cannot list itself as a supertrait", decl.name),
                        )
                        .at(stmt.span));
                    }
                    if !self.trait_defs.contains_key(supertrait) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "trait '{}' references unknown supertrait '{}'",
                                decl.name, supertrait
                            ),
                        )
                        .at(stmt.span));
                    }
                }
                self.trait_defs
                    .insert(decl.name.clone(), TraitDeclInfo { decl: decl.clone() });
            }
        }
        Ok(())
    }
    pub(crate) fn register_generic_fn_decls(&mut self, stmts: &[ast::Stmt]) -> RR<()> {
        for stmt in stmts {
            let fndecl = match &stmt.kind {
                ast::StmtKind::FnDecl {
                    name,
                    type_params,
                    params,
                    ret_ty_hint,
                    where_bounds,
                    body,
                } if !type_params.is_empty() => ast::FnDecl {
                    name: name.clone(),
                    type_params: type_params.clone(),
                    params: params.clone(),
                    ret_ty_hint: ret_ty_hint.clone(),
                    where_bounds: where_bounds.clone(),
                    body: body.clone(),
                    public: false,
                },
                ast::StmtKind::Export(fndecl) if !fndecl.type_params.is_empty() => fndecl.clone(),
                _ => continue,
            };
            if self.generic_fns.contains_key(&fndecl.name) {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!("duplicate generic function declaration '{}'", fndecl.name),
                )
                .at(stmt.span));
            }
            self.generic_fns
                .insert(fndecl.name.clone(), GenericFnInfo { decl: fndecl });
        }
        Ok(())
    }
    pub(crate) fn register_impl_decls(&mut self, stmts: &[ast::Stmt]) -> RR<()> {
        for stmt in stmts {
            if let ast::StmtKind::ImplDecl(decl) = &stmt.kind {
                self.register_impl_decl(stmt.span, decl)?;
            }
        }
        Ok(())
    }

    pub(crate) fn register_impl_decl(&mut self, span: Span, decl: &ast::ImplDecl) -> RR<()> {
        let plan = self.prepare_impl_registration(span, decl)?;
        if decl.negative {
            self.register_negative_impl_decl(span, decl, plan)
        } else {
            self.register_positive_impl_decl(span, decl, plan)
        }
    }

    pub(crate) fn prepare_impl_registration(
        &self,
        span: Span,
        decl: &ast::ImplDecl,
    ) -> RR<ImplRegistrationPlan> {
        let Some(trait_info) = self.trait_defs.get(&decl.trait_name).cloned() else {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!("impl references unknown trait '{}'", decl.trait_name),
            )
            .at(span));
        };
        let for_ty = Self::ast_type_ref(&decl.for_ty);
        let impl_key = (decl.trait_name.clone(), for_ty.key());
        let impl_type_params: FxHashSet<String> = decl.type_params.iter().cloned().collect();
        let is_generic_impl = !decl.type_params.is_empty()
            || Self::type_ref_contains_type_param(&for_ty, &impl_type_params);
        let is_public_impl = decl.public && trait_info.decl.public;
        let header = TraitImplHeader {
            trait_name: decl.trait_name.clone(),
            for_ty: for_ty.clone(),
            type_params: decl.type_params.clone(),
            public: is_public_impl,
            span,
        };
        Ok(ImplRegistrationPlan {
            trait_info,
            for_ty,
            impl_key,
            is_generic_impl,
            is_public_impl,
            header,
        })
    }

    pub(crate) fn register_negative_impl_decl(
        &mut self,
        span: Span,
        decl: &ast::ImplDecl,
        plan: ImplRegistrationPlan,
    ) -> RR<()> {
        Self::validate_negative_impl_body_empty(span, decl, &plan.for_ty)?;
        self.validate_negative_impl_overlaps(span, decl, &plan.header)?;
        self.negative_trait_impls.push(plan.header);
        Ok(())
    }

    pub(crate) fn validate_negative_impl_body_empty(
        span: Span,
        decl: &ast::ImplDecl,
        for_ty: &HirTypeRef,
    ) -> RR<()> {
        if decl.methods.is_empty() && decl.assoc_types.is_empty() && decl.assoc_consts.is_empty() {
            return Ok(());
        }
        Err(RRException::new(
            "RR.SemanticError",
            RRCode::E1002,
            Stage::Lower,
            format!(
                "negative impl of trait '{}' for '{}' cannot define methods, associated types, or associated consts",
                decl.trait_name,
                for_ty.key()
            ),
        )
        .at(span))
    }

    pub(crate) fn validate_negative_impl_overlaps(
        &self,
        span: Span,
        decl: &ast::ImplDecl,
        header: &TraitImplHeader,
    ) -> RR<()> {
        for existing in &self.negative_trait_impls {
            if trait_impl_patterns_overlap(existing, header) {
                return Err(Self::overlapping_negative_impl_error(
                    span, decl, header, existing,
                ));
            }
        }
        for positive in self.positive_impl_headers(span) {
            if trait_impl_patterns_overlap(&positive, header)
                && !trait_impl_is_more_specific(header, &positive)
            {
                return Err(Self::negative_positive_conflict_error(
                    span, decl, header, &positive,
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn positive_impl_headers(&self, span: Span) -> Vec<TraitImplHeader> {
        let concrete =
            self.trait_impls
                .iter()
                .map(|((trait_name, _), impl_info)| TraitImplHeader {
                    trait_name: trait_name.clone(),
                    for_ty: impl_info.for_ty.clone(),
                    type_params: Vec::new(),
                    public: impl_info.public,
                    span,
                });
        let generic = self.generic_trait_impls.iter().map(|info| TraitImplHeader {
            trait_name: info.decl.trait_name.clone(),
            for_ty: info.for_ty.clone(),
            type_params: info.decl.type_params.clone(),
            public: info.decl.public,
            span,
        });
        concrete.chain(generic).collect()
    }

    pub(crate) fn register_positive_impl_decl(
        &mut self,
        span: Span,
        decl: &ast::ImplDecl,
        plan: ImplRegistrationPlan,
    ) -> RR<()> {
        self.validate_positive_impl_against_negative(span, decl, &plan.header)?;
        self.validate_positive_impl_solver_overlap(span, plan.header.clone())?;
        self.validate_duplicate_impl(span, decl, &plan)?;

        let methods_by_name = Self::collect_impl_methods_by_name(decl)?;
        let assoc_types_by_name = Self::collect_impl_assoc_types_by_name(decl)?;
        Self::validate_impl_assoc_types(span, decl, &plan, &assoc_types_by_name)?;
        let assoc_consts_by_name = Self::collect_impl_assoc_consts_by_name(decl)?;
        Self::validate_impl_assoc_consts(
            span,
            decl,
            &plan,
            &assoc_types_by_name,
            &assoc_consts_by_name,
        )?;
        let method_symbols = self.build_impl_method_symbols(
            span,
            decl,
            &plan,
            &assoc_types_by_name,
            &methods_by_name,
        )?;
        let const_symbols = Self::build_impl_const_symbols(decl, &plan);
        Self::validate_extra_impl_methods(decl, &plan.trait_info)?;
        self.store_trait_impl(
            decl,
            plan,
            assoc_types_by_name,
            method_symbols,
            const_symbols,
        );
        Ok(())
    }

    pub(crate) fn validate_positive_impl_against_negative(
        &self,
        span: Span,
        decl: &ast::ImplDecl,
        header: &TraitImplHeader,
    ) -> RR<()> {
        for negative in &self.negative_trait_impls {
            if trait_impl_patterns_overlap(negative, header)
                && !trait_impl_is_more_specific(negative, header)
            {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "impl of trait '{}' for '{}' conflicts with negative impl for '{}'",
                        decl.trait_name,
                        header.for_ty.key(),
                        negative.for_ty.key()
                    ),
                )
                .at(span));
            }
        }
        Ok(())
    }

    pub(crate) fn validate_positive_impl_solver_overlap(
        &self,
        span: Span,
        header: TraitImplHeader,
    ) -> RR<()> {
        let mut solver = TraitSolver::new();
        for positive in self.positive_impl_headers(span) {
            solver.add_impl(positive)?;
        }
        solver.add_impl(header)
    }

    pub(crate) fn validate_duplicate_impl(
        &self,
        span: Span,
        decl: &ast::ImplDecl,
        plan: &ImplRegistrationPlan,
    ) -> RR<()> {
        if !plan.is_generic_impl && self.trait_impls.contains_key(&plan.impl_key) {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "duplicate impl of trait '{}' for '{}'",
                    decl.trait_name,
                    plan.for_ty.key()
                ),
            )
            .at(span));
        }
        if plan.is_generic_impl
            && self.generic_trait_impls.iter().any(|info| {
                info.decl.trait_name == decl.trait_name && info.for_ty.key() == plan.for_ty.key()
            })
        {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "duplicate generic impl of trait '{}' for '{}'",
                    decl.trait_name,
                    plan.for_ty.key()
                ),
            )
            .at(span));
        }
        Ok(())
    }

    pub(crate) fn collect_impl_methods_by_name(
        decl: &ast::ImplDecl,
    ) -> RR<FxHashMap<String, &ast::FnDecl>> {
        let mut methods_by_name = FxHashMap::default();
        for method in &decl.methods {
            if methods_by_name
                .insert(method.name.clone(), method)
                .is_some()
            {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "duplicate impl method '{}' for trait '{}'",
                        method.name, decl.trait_name
                    ),
                )
                .at(method.body.span));
            }
        }
        Ok(methods_by_name)
    }

    pub(crate) fn collect_impl_assoc_types_by_name(
        decl: &ast::ImplDecl,
    ) -> RR<FxHashMap<String, HirTypeRef>> {
        let mut assoc_types_by_name = FxHashMap::default();
        for assoc_ty in &decl.assoc_types {
            if assoc_types_by_name
                .insert(assoc_ty.name.clone(), Self::ast_type_ref(&assoc_ty.ty))
                .is_some()
            {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "duplicate associated type '{}' in impl of trait '{}'",
                        assoc_ty.name, decl.trait_name
                    ),
                )
                .at(assoc_ty.span));
            }
        }
        Ok(assoc_types_by_name)
    }

    pub(crate) fn validate_impl_assoc_types(
        span: Span,
        decl: &ast::ImplDecl,
        plan: &ImplRegistrationPlan,
        assoc_types_by_name: &FxHashMap<String, HirTypeRef>,
    ) -> RR<()> {
        for assoc_ty in &plan.trait_info.decl.assoc_types {
            if !Self::impl_assoc_type_satisfies_trait_decl(assoc_types_by_name, assoc_ty) {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "impl of trait '{}' for '{}' is missing associated type '{}'",
                        decl.trait_name,
                        plan.for_ty.key(),
                        assoc_ty.name
                    ),
                )
                .at(span));
            }
        }
        Ok(())
    }

    pub(crate) fn collect_impl_assoc_consts_by_name(
        decl: &ast::ImplDecl,
    ) -> RR<FxHashMap<String, &ast::ImplAssocConst>> {
        let mut assoc_consts_by_name = FxHashMap::default();
        for assoc_const in &decl.assoc_consts {
            if assoc_consts_by_name
                .insert(assoc_const.name.clone(), assoc_const)
                .is_some()
            {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "duplicate associated const '{}' in impl of trait '{}'",
                        assoc_const.name, decl.trait_name
                    ),
                )
                .at(assoc_const.span));
            }
        }
        Ok(assoc_consts_by_name)
    }

    pub(crate) fn validate_impl_assoc_consts(
        span: Span,
        decl: &ast::ImplDecl,
        plan: &ImplRegistrationPlan,
        assoc_types_by_name: &FxHashMap<String, HirTypeRef>,
        assoc_consts_by_name: &FxHashMap<String, &ast::ImplAssocConst>,
    ) -> RR<()> {
        for assoc_const in &plan.trait_info.decl.assoc_consts {
            match assoc_consts_by_name.get(&assoc_const.name) {
                Some(impl_const) => {
                    let trait_ty = Some(Self::ast_type_ref(&assoc_const.ty_hint));
                    let impl_ty = Some(Self::ast_type_ref(&impl_const.ty_hint));
                    if !Self::type_ref_matches_trait_sig(
                        &impl_ty,
                        &trait_ty,
                        &plan.for_ty,
                        assoc_types_by_name,
                    ) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "impl associated const '{}.{}' type does not match trait signature",
                                decl.trait_name, assoc_const.name
                            ),
                        )
                        .at(impl_const.span));
                    }
                }
                None if assoc_const.default.is_some() => {}
                None => {
                    return Err(RRException::new(
                        "RR.SemanticError",
                        RRCode::E1002,
                        Stage::Lower,
                        format!(
                            "impl of trait '{}' for '{}' is missing associated const '{}'",
                            decl.trait_name,
                            plan.for_ty.key(),
                            assoc_const.name
                        ),
                    )
                    .at(span));
                }
            }
        }
        Self::validate_extra_impl_assoc_consts(decl, &plan.trait_info)
    }

    pub(crate) fn validate_extra_impl_assoc_consts(
        decl: &ast::ImplDecl,
        trait_info: &TraitDeclInfo,
    ) -> RR<()> {
        for impl_const in &decl.assoc_consts {
            if !trait_info
                .decl
                .assoc_consts
                .iter()
                .any(|assoc_const| assoc_const.name == impl_const.name)
            {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "impl associated const '{}' is not declared by trait '{}'",
                        impl_const.name, decl.trait_name
                    ),
                )
                .at(impl_const.span));
            }
        }
        Ok(())
    }

    pub(crate) fn build_impl_method_symbols(
        &self,
        span: Span,
        decl: &ast::ImplDecl,
        plan: &ImplRegistrationPlan,
        assoc_types_by_name: &FxHashMap<String, HirTypeRef>,
        methods_by_name: &FxHashMap<String, &ast::FnDecl>,
    ) -> RR<FxHashMap<String, String>> {
        let mut method_symbols = FxHashMap::default();
        for trait_method in &plan.trait_info.decl.methods {
            let impl_method = methods_by_name.get(&trait_method.name);
            if impl_method.is_none() && trait_method.default_body.is_none() {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "impl of trait '{}' for '{}' is missing method '{}'",
                        decl.trait_name,
                        plan.for_ty.key(),
                        trait_method.name
                    ),
                )
                .at(span));
            };
            if let Some(impl_method) = impl_method {
                self.validate_impl_method_signature(
                    &decl.trait_name,
                    &plan.for_ty,
                    assoc_types_by_name,
                    trait_method,
                    impl_method,
                )?;
            }
            let mangled = trait_names::trait_method_mangle(
                &decl.trait_name,
                &plan.for_ty,
                &trait_method.name,
            );
            method_symbols.insert(trait_method.name.clone(), mangled);
        }
        Ok(method_symbols)
    }

    pub(crate) fn build_impl_const_symbols(
        decl: &ast::ImplDecl,
        plan: &ImplRegistrationPlan,
    ) -> FxHashMap<String, String> {
        let mut const_symbols = FxHashMap::default();
        for trait_const in &plan.trait_info.decl.assoc_consts {
            let mangled =
                trait_names::trait_const_mangle(&decl.trait_name, &plan.for_ty, &trait_const.name);
            const_symbols.insert(trait_const.name.clone(), mangled);
        }
        const_symbols
    }

    pub(crate) fn validate_extra_impl_methods(
        decl: &ast::ImplDecl,
        trait_info: &TraitDeclInfo,
    ) -> RR<()> {
        for impl_method in &decl.methods {
            if !trait_info
                .decl
                .methods
                .iter()
                .any(|method| method.name == impl_method.name)
            {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "impl method '{}' is not declared by trait '{}'",
                        impl_method.name, decl.trait_name
                    ),
                )
                .at(impl_method.body.span));
            }
        }
        Ok(())
    }

    pub(crate) fn store_trait_impl(
        &mut self,
        decl: &ast::ImplDecl,
        plan: ImplRegistrationPlan,
        assoc_types_by_name: FxHashMap<String, HirTypeRef>,
        method_symbols: FxHashMap<String, String>,
        const_symbols: FxHashMap<String, String>,
    ) {
        if plan.is_generic_impl {
            self.generic_trait_impls.push(GenericTraitImplInfo {
                decl: decl.clone(),
                for_ty: plan.for_ty,
            });
        } else {
            self.trait_impls.insert(
                plan.impl_key,
                TraitImplInfo {
                    trait_name: decl.trait_name.clone(),
                    for_ty: plan.for_ty,
                    assoc_types: assoc_types_by_name,
                    method_symbols,
                    const_symbols,
                    public: plan.is_public_impl,
                },
            );
        }
    }

    pub(crate) fn overlapping_negative_impl_error(
        span: Span,
        decl: &ast::ImplDecl,
        header: &TraitImplHeader,
        existing: &TraitImplHeader,
    ) -> RRException {
        RRException::new(
            "RR.SemanticError",
            RRCode::E1002,
            Stage::Lower,
            format!(
                "overlapping negative impl of trait '{}' for '{}' conflicts with existing negative impl for '{}'",
                decl.trait_name,
                header.for_ty.key(),
                existing.for_ty.key()
            ),
        )
        .at(span)
    }

    pub(crate) fn negative_positive_conflict_error(
        span: Span,
        decl: &ast::ImplDecl,
        header: &TraitImplHeader,
        positive: &TraitImplHeader,
    ) -> RRException {
        RRException::new(
            "RR.SemanticError",
            RRCode::E1002,
            Stage::Lower,
            format!(
                "negative impl of trait '{}' for '{}' conflicts with existing positive impl for '{}'",
                decl.trait_name,
                header.for_ty.key(),
                positive.for_ty.key()
            ),
        )
        .at(span)
    }
}
