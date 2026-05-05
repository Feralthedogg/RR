use super::*;
impl Lowerer {
    pub(crate) fn infer_generic_type_from_hir_pattern(
        type_params: &FxHashSet<String>,
        pattern: &HirTypeRef,
        actual: &HirTypeRef,
        subst: &mut FxHashMap<String, HirTypeRef>,
        span: Span,
    ) -> RR<bool> {
        crate::typeck::trait_solver::infer_trait_type_subst(
            type_params,
            pattern,
            actual,
            subst,
            span,
        )
    }
    pub(crate) fn validate_trait_bounds_for_subst(
        &mut self,
        bounds: &[ast::TraitBound],
        subst: &FxHashMap<String, HirTypeRef>,
        span: Span,
    ) -> RR<()> {
        let subst = self.subst_with_assoc_type_projections(bounds, subst, span)?;
        for bound in bounds {
            let Some(concrete_ty) = subst.get(&bound.type_name) else {
                if Self::type_projection_parts(&bound.type_name).is_some() {
                    return Err(RRException::new(
                        "RR.SemanticError",
                        RRCode::E1002,
                        Stage::Lower,
                        format!(
                            "cannot resolve associated type projection '{}' in generic bounds",
                            bound.type_name
                        ),
                    )
                    .at(span));
                }
                continue;
            };
            for trait_name in &bound.trait_names {
                if !self.ensure_trait_impl_for_type(trait_name, concrete_ty, span)? {
                    return Err(RRException::new(
                        "RR.SemanticError",
                        RRCode::E1002,
                        Stage::Lower,
                        format!(
                            "generic bound '{}' requires trait '{}' for '{}', but no impl was found",
                            bound.type_name,
                            trait_name,
                            concrete_ty.key()
                        ),
                    )
                    .at(span));
                }
            }
        }
        Ok(())
    }
    pub(crate) fn ensure_trait_impl_for_type(
        &mut self,
        trait_name: &str,
        receiver_ty: &HirTypeRef,
        span: Span,
    ) -> RR<bool> {
        for negative in &self.negative_trait_impls {
            if negative.trait_name != trait_name {
                continue;
            }
            let mut subst = FxHashMap::default();
            if Self::infer_generic_type_from_hir_pattern(
                &negative.type_param_set(),
                &negative.for_ty,
                receiver_ty,
                &mut subst,
                span,
            )? {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "negative impl explicitly prevents trait '{}' for '{}'",
                        trait_name,
                        receiver_ty.key()
                    ),
                )
                .at(span));
            }
        }
        let key = (trait_name.to_string(), receiver_ty.key());
        if self.trait_impls.contains_key(&key) {
            return self.ensure_supertrait_impls_for_type(trait_name, receiver_ty, span);
        }

        let mut matches = Vec::new();
        for info in self.generic_trait_impls.iter().cloned() {
            if info.decl.trait_name != trait_name {
                continue;
            }
            let type_params: FxHashSet<String> = info.decl.type_params.iter().cloned().collect();
            let mut subst = FxHashMap::default();
            if Self::infer_generic_type_from_hir_pattern(
                &type_params,
                &info.for_ty,
                receiver_ty,
                &mut subst,
                span,
            )? {
                matches.push((info, subst));
            }
        }
        match matches.len() {
            0 => Ok(false),
            _ => {
                let best_indices = (0..matches.len())
                    .filter(|candidate_idx| {
                        let candidate_header = TraitImplHeader {
                            trait_name: matches[*candidate_idx].0.decl.trait_name.clone(),
                            for_ty: matches[*candidate_idx].0.for_ty.clone(),
                            type_params: matches[*candidate_idx].0.decl.type_params.clone(),
                            public: matches[*candidate_idx].0.decl.public,
                            span,
                        };
                        !(0..matches.len()).any(|other_idx| {
                            if other_idx == *candidate_idx {
                                return false;
                            }
                            let other_header = TraitImplHeader {
                                trait_name: matches[other_idx].0.decl.trait_name.clone(),
                                for_ty: matches[other_idx].0.for_ty.clone(),
                                type_params: matches[other_idx].0.decl.type_params.clone(),
                                public: matches[other_idx].0.decl.public,
                                span,
                            };
                            trait_impl_is_more_specific(&other_header, &candidate_header)
                        })
                    })
                    .collect::<Vec<_>>();
                if best_indices.len() != 1 {
                    return Err(RRException::new(
                        "RR.SemanticError",
                        RRCode::E1002,
                        Stage::Lower,
                        format!(
                            "ambiguous generic impl of trait '{}' for '{}'",
                            trait_name,
                            receiver_ty.key()
                        ),
                    )
                    .at(span));
                }
                let (info, subst) = matches.swap_remove(best_indices[0]);
                self.instantiate_generic_trait_impl(info, subst, receiver_ty.clone(), span)?;
                if self.trait_impls.contains_key(&key) {
                    self.ensure_supertrait_impls_for_type(trait_name, receiver_ty, span)
                } else {
                    Ok(false)
                }
            }
        }
    }
    pub(crate) fn ensure_supertrait_impls_for_type(
        &mut self,
        trait_name: &str,
        receiver_ty: &HirTypeRef,
        span: Span,
    ) -> RR<bool> {
        let supertraits = self
            .trait_defs
            .get(trait_name)
            .map(|info| info.decl.supertraits.clone())
            .unwrap_or_default();
        for supertrait in supertraits {
            if !self.ensure_trait_impl_for_type(&supertrait, receiver_ty, span)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(crate) fn trait_assoc_type_owners(&self, trait_name: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        let mut stack = vec![trait_name.to_string()];
        let mut seen = FxHashSet::default();
        while let Some(name) = stack.pop() {
            if !seen.insert(name.clone()) {
                continue;
            }
            let Some(info) = self.trait_defs.get(&name) else {
                continue;
            };
            for assoc_ty in &info.decl.assoc_types {
                out.push((name.clone(), assoc_ty.name.clone()));
            }
            for supertrait in &info.decl.supertraits {
                stack.push(supertrait.clone());
            }
        }
        out
    }
    pub(crate) fn associated_type_for_impl(
        &mut self,
        trait_name: &str,
        receiver_ty: &HirTypeRef,
        assoc_name: &str,
        span: Span,
    ) -> RR<Option<HirTypeRef>> {
        if !self.ensure_trait_impl_for_type(trait_name, receiver_ty, span)? {
            return Ok(None);
        }
        Ok(self
            .trait_impls
            .get(&(trait_name.to_string(), receiver_ty.key()))
            .and_then(|impl_info| impl_info.assoc_types.get(assoc_name))
            .cloned())
    }
    pub(crate) fn subst_with_assoc_type_projections(
        &mut self,
        bounds: &[ast::TraitBound],
        subst: &FxHashMap<String, HirTypeRef>,
        span: Span,
    ) -> RR<FxHashMap<String, HirTypeRef>> {
        let mut out = subst.clone();
        let requested_unqualified_projections = bounds
            .iter()
            .filter(|bound| {
                Self::type_projection_parts(&bound.type_name)
                    .is_some_and(|parts| parts.trait_name.is_none())
            })
            .map(|bound| bound.type_name.clone())
            .collect::<FxHashSet<_>>();
        let requested_qualified_bound_projections = bounds
            .iter()
            .filter(|bound| {
                Self::type_projection_parts(&bound.type_name)
                    .is_some_and(|parts| parts.trait_name.is_some())
            })
            .map(|bound| bound.type_name.clone())
            .collect::<FxHashSet<_>>();
        let mut ambiguous_unqualified_projections = FxHashSet::default();
        let mut ambiguous_qualified_bound_projections = FxHashSet::default();
        let mut changed = true;
        while changed {
            changed = false;
            for bound in bounds {
                let Some(base_ty) = out.get(&bound.type_name).cloned() else {
                    continue;
                };
                for trait_name in &bound.trait_names {
                    for (owner_trait, assoc_name) in self.trait_assoc_type_owners(trait_name) {
                        let Some(assoc_ty) = self.associated_type_for_impl(
                            &owner_trait,
                            &base_ty,
                            &assoc_name,
                            span,
                        )?
                        else {
                            continue;
                        };
                        let owner_projection_key = Self::qualified_type_projection_key(
                            &bound.type_name,
                            &owner_trait,
                            &assoc_name,
                        );
                        changed |= Self::insert_assoc_projection_subst(
                            &mut out,
                            owner_projection_key,
                            &assoc_ty,
                            span,
                        )?;
                        let bound_projection_key = Self::qualified_type_projection_key(
                            &bound.type_name,
                            trait_name,
                            &assoc_name,
                        );
                        changed |= Self::insert_alias_assoc_projection_subst(
                            &mut out,
                            bound_projection_key,
                            &assoc_ty,
                            &mut ambiguous_qualified_bound_projections,
                            &requested_qualified_bound_projections,
                            span,
                        )?;

                        let unqualified_projection_key =
                            format!("{}::{}", bound.type_name, assoc_name);
                        changed |= Self::insert_alias_assoc_projection_subst(
                            &mut out,
                            unqualified_projection_key,
                            &assoc_ty,
                            &mut ambiguous_unqualified_projections,
                            &requested_unqualified_projections,
                            span,
                        )?;
                    }
                }
            }
        }
        Ok(out)
    }
    pub(crate) fn instantiate_generic_trait_impl(
        &mut self,
        info: GenericTraitImplInfo,
        subst: FxHashMap<String, HirTypeRef>,
        concrete_ty: HirTypeRef,
        span: Span,
    ) -> RR<()> {
        let key = (info.decl.trait_name.clone(), concrete_ty.key());
        if self.trait_impls.contains_key(&key)
            || !self.generic_impl_instantiations.insert(key.clone())
        {
            return Ok(());
        }

        self.validate_trait_bounds_for_subst(&info.decl.where_bounds, &subst, span)?;
        let subst =
            self.subst_with_assoc_type_projections(&info.decl.where_bounds, &subst, span)?;
        let Some(trait_info) = self.trait_defs.get(&info.decl.trait_name).cloned() else {
            return Ok(());
        };

        let mut method_symbols = FxHashMap::default();
        for trait_method in &trait_info.decl.methods {
            let mangled = trait_names::trait_method_mangle(
                &info.decl.trait_name,
                &concrete_ty,
                &trait_method.name,
            );
            method_symbols.insert(trait_method.name.clone(), mangled);
        }
        let mut const_symbols = FxHashMap::default();
        for trait_const in &trait_info.decl.assoc_consts {
            let mangled = trait_names::trait_const_mangle(
                &info.decl.trait_name,
                &concrete_ty,
                &trait_const.name,
            );
            const_symbols.insert(trait_const.name.clone(), mangled);
        }
        let mut inst_subst = subst.clone();
        inst_subst.insert("Self".to_string(), concrete_ty.clone());
        let mut inst_assoc_types = FxHashMap::default();
        for assoc_ty in &info.decl.assoc_types {
            let assoc_value = Self::substitute_type_expr(assoc_ty.ty.clone(), &subst);
            let assoc_value_ref = Self::ast_type_ref(&assoc_value);
            inst_subst.insert(format!("Self::{}", assoc_ty.name), assoc_value_ref.clone());
            inst_assoc_types.insert(assoc_ty.name.clone(), assoc_value_ref);
        }
        self.trait_impls.insert(
            key,
            TraitImplInfo {
                trait_name: info.decl.trait_name.clone(),
                for_ty: concrete_ty.clone(),
                assoc_types: inst_assoc_types,
                method_symbols: method_symbols.clone(),
                const_symbols: const_symbols.clone(),
                public: info.decl.public && trait_info.decl.public,
            },
        );

        let impl_assoc_consts = info.decl.assoc_consts.clone();
        let mut methods_by_name = info
            .decl
            .methods
            .into_iter()
            .map(|method| (method.name.clone(), method))
            .collect::<FxHashMap<_, _>>();
        for trait_method in trait_info.decl.methods.clone() {
            let method = if let Some(method) = methods_by_name.remove(&trait_method.name) {
                method
            } else if let Some(default_body) = trait_method.default_body {
                ast::FnDecl {
                    name: trait_method.name.clone(),
                    type_params: Vec::new(),
                    params: trait_method.params,
                    ret_ty_hint: trait_method.ret_ty_hint,
                    where_bounds: trait_method.where_bounds,
                    body: default_body,
                    public: false,
                }
            } else {
                continue;
            };
            let Some(mangled) = method_symbols.get(&method.name).cloned() else {
                continue;
            };
            let inst_params = method
                .params
                .into_iter()
                .map(|param| Self::substitute_fn_param_type(param, &inst_subst))
                .collect();
            let inst_ret = method
                .ret_ty_hint
                .map(|ty| Self::substitute_type_expr(ty, &inst_subst));
            let inst_body = Self::substitute_block_type_hints(method.body, &inst_subst);
            let inst_fn = self.lower_fn(LowerFnParts {
                name: mangled,
                type_params: Vec::new(),
                params: inst_params,
                ret_ty_hint: inst_ret,
                where_bounds: Vec::new(),
                body: inst_body,
                span,
            })?;
            self.pending_fns.push(inst_fn);
        }
        let assoc_consts_by_name = impl_assoc_consts
            .iter()
            .map(|assoc_const| (assoc_const.name.clone(), assoc_const))
            .collect::<FxHashMap<_, _>>();
        for trait_const in &trait_info.decl.assoc_consts {
            let (ty_hint, value, item_span) =
                if let Some(impl_const) = assoc_consts_by_name.get(&trait_const.name) {
                    (
                        impl_const.ty_hint.clone(),
                        impl_const.value.clone(),
                        impl_const.span,
                    )
                } else if let Some(default) = trait_const.default.clone() {
                    (trait_const.ty_hint.clone(), default, trait_const.span)
                } else {
                    continue;
                };
            let Some(mangled) = const_symbols.get(&trait_const.name).cloned() else {
                continue;
            };
            let ret_ty_hint = Self::substitute_type_expr(ty_hint, &inst_subst);
            let value = Self::substitute_expr_type_hints(value, &inst_subst);
            let body = ast::Block {
                stmts: vec![ast::Stmt {
                    kind: ast::StmtKind::ExprStmt {
                        expr: value.clone(),
                    },
                    span: value.span,
                }],
                span: value.span,
            };
            let inst_fn = self.lower_fn(LowerFnParts {
                name: mangled,
                type_params: Vec::new(),
                params: Vec::new(),
                ret_ty_hint: Some(ret_ty_hint),
                where_bounds: Vec::new(),
                body,
                span: item_span,
            })?;
            self.pending_fns.push(inst_fn);
        }
        Ok(())
    }
    pub(crate) fn trait_impl_method_for_type(
        &mut self,
        trait_name: &str,
        method_name: &str,
        receiver_ty: &HirTypeRef,
        span: Span,
    ) -> RR<Option<SymbolId>> {
        if let Some(type_key) = self.current_generic_ref_key(receiver_ty)
            && self.generic_ref_has_trait_bound(&type_key, trait_name)
        {
            return Ok(None);
        }
        if self.type_ref_contains_current_type_param(receiver_ty) {
            return Ok(None);
        }
        let trait_has_method = self.trait_defs.get(trait_name).is_some_and(|trait_info| {
            trait_info
                .decl
                .methods
                .iter()
                .any(|method| method.name == method_name)
        });
        if !trait_has_method {
            return Ok(None);
        }

        if !self.ensure_trait_impl_for_type(trait_name, receiver_ty, span)? {
            return Ok(None);
        }
        let mangled = self
            .trait_impls
            .get(&(trait_name.to_string(), receiver_ty.key()))
            .and_then(|impl_info| impl_info.method_symbols.get(method_name))
            .cloned();
        Ok(mangled.map(|mangled| self.intern_symbol(&mangled)))
    }
}
