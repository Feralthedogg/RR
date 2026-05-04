use super::*;
impl Lowerer {
    pub(crate) fn ast_type_ref(expr: &ast::TypeExpr) -> HirTypeRef {
        match expr {
            ast::TypeExpr::Named(name) => HirTypeRef::Named(name.clone()),
            ast::TypeExpr::Generic { base, args } => HirTypeRef::Generic {
                base: base.clone(),
                args: args.iter().map(Self::ast_type_ref).collect(),
            },
        }
    }
    pub(crate) fn dyn_trait_name(expr: &ast::TypeExpr) -> Option<&str> {
        let ast::TypeExpr::Named(name) = expr else {
            return None;
        };
        name.strip_prefix("dyn ")
    }
    pub(crate) fn type_ref_contains_type_param(
        ty: &HirTypeRef,
        type_params: &FxHashSet<String>,
    ) -> bool {
        match ty {
            HirTypeRef::Named(name) => {
                type_params.contains(name)
                    || Self::type_projection_parts(name)
                        .is_some_and(|parts| type_params.contains(parts.base))
            }
            HirTypeRef::Generic { base, args } => {
                Self::type_projection_parts(base)
                    .is_some_and(|parts| type_params.contains(parts.base))
                    || args
                        .iter()
                        .any(|arg| Self::type_ref_contains_type_param(arg, type_params))
            }
        }
    }
    pub(crate) fn lower_trait_bounds(
        &mut self,
        bounds: Vec<ast::TraitBound>,
    ) -> RR<Vec<HirTraitBound>> {
        let mut lowered = Vec::with_capacity(bounds.len());
        for bound in bounds {
            let mut trait_names = Vec::with_capacity(bound.trait_names.len());
            for trait_name in bound.trait_names {
                if !self.trait_defs.contains_key(&trait_name) {
                    return Err(RRException::new(
                        "RR.SemanticError",
                        RRCode::E1002,
                        Stage::Lower,
                        format!("unknown trait '{}' in where clause", trait_name),
                    ));
                }
                trait_names.push(self.intern_symbol(&trait_name));
            }
            lowered.push(HirTraitBound {
                type_name: bound.type_name,
                trait_names,
            });
        }
        Ok(lowered)
    }
    pub(crate) fn where_bound_map(
        bounds: &[ast::TraitBound],
    ) -> FxHashMap<String, FxHashSet<String>> {
        let mut out: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
        for bound in bounds {
            let entry = out.entry(bound.type_name.clone()).or_default();
            for trait_name in &bound.trait_names {
                entry.insert(trait_name.clone());
            }
        }
        out
    }
    pub(crate) fn type_projection_parts(name: &str) -> Option<TypeProjectionParts<'_>> {
        if let Some(rest) = name.strip_prefix('<') {
            let (base, rest) = rest.split_once(" as ")?;
            let (trait_name, assoc) = rest.split_once(">::")?;
            if base.is_empty() || trait_name.is_empty() || assoc.is_empty() {
                return None;
            }
            return Some(TypeProjectionParts {
                base,
                trait_name: Some(trait_name),
                assoc,
            });
        }
        let (base, assoc) = name.split_once("::")?;
        if base.is_empty() || assoc.is_empty() {
            return None;
        }
        Some(TypeProjectionParts {
            base,
            trait_name: None,
            assoc,
        })
    }
    pub(crate) fn qualified_type_projection_key(
        base: &str,
        trait_name: &str,
        assoc: &str,
    ) -> String {
        format!("<{} as {}>::{}", base, trait_name, assoc)
    }
    pub(crate) fn insert_assoc_projection_subst(
        out: &mut FxHashMap<String, HirTypeRef>,
        projection_key: String,
        assoc_ty: &HirTypeRef,
        span: Span,
    ) -> RR<bool> {
        if let Some(prev) = out.get(&projection_key) {
            if prev != assoc_ty {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "associated type projection '{}' is ambiguous between '{}' and '{}'",
                        projection_key,
                        prev.key(),
                        assoc_ty.key()
                    ),
                )
                .at(span));
            }
            return Ok(false);
        }
        out.insert(projection_key, assoc_ty.clone());
        Ok(true)
    }
    pub(crate) fn insert_alias_assoc_projection_subst(
        out: &mut FxHashMap<String, HirTypeRef>,
        projection_key: String,
        assoc_ty: &HirTypeRef,
        ambiguous_projections: &mut FxHashSet<String>,
        requested_projections: &FxHashSet<String>,
        span: Span,
    ) -> RR<bool> {
        if ambiguous_projections.contains(&projection_key) {
            return Ok(false);
        }
        if let Some(prev) = out.get(&projection_key).cloned() {
            if &prev != assoc_ty {
                out.remove(&projection_key);
                ambiguous_projections.insert(projection_key.clone());
                if requested_projections.contains(&projection_key) {
                    return Err(RRException::new(
                        "RR.SemanticError",
                        RRCode::E1002,
                        Stage::Lower,
                        format!(
                            "associated type projection '{}' is ambiguous between '{}' and '{}'",
                            projection_key,
                            prev.key(),
                            assoc_ty.key()
                        ),
                    )
                    .at(span));
                }
            }
            return Ok(false);
        }
        out.insert(projection_key, assoc_ty.clone());
        Ok(true)
    }
    pub(crate) fn self_assoc_projection_name(name: &str) -> Option<&str> {
        if let Some(rest) = name.strip_prefix("Self::") {
            return Some(rest);
        }
        let parts = Self::type_projection_parts(name)?;
        (parts.base == "Self").then_some(parts.assoc)
    }
    pub(crate) fn current_generic_ref_key(&self, ty: &HirTypeRef) -> Option<String> {
        match ty {
            HirTypeRef::Named(name) if self.current_type_params.contains(name) => {
                Some(name.clone())
            }
            HirTypeRef::Named(name) => Self::type_projection_parts(name)
                .filter(|parts| self.current_type_params.contains(parts.base))
                .map(|_| name.clone()),
            HirTypeRef::Generic { base, .. } => Self::type_projection_parts(base)
                .filter(|parts| self.current_type_params.contains(parts.base))
                .map(|_| ty.key()),
        }
    }
    pub(crate) fn type_ref_contains_current_type_param(&self, ty: &HirTypeRef) -> bool {
        Self::type_ref_contains_type_param(ty, &self.current_type_params)
    }
    pub(crate) fn generic_ref_has_trait_bound(&self, type_key: &str, trait_name: &str) -> bool {
        self.current_where_bounds
            .get(type_key)
            .is_some_and(|traits| {
                traits
                    .iter()
                    .any(|bound_trait| self.trait_implies_trait(bound_trait, trait_name))
            })
    }
    pub(crate) fn type_ref_matches_trait_sig(
        impl_ty: &Option<HirTypeRef>,
        trait_ty: &Option<HirTypeRef>,
        for_ty: &HirTypeRef,
        assoc_types: &FxHashMap<String, HirTypeRef>,
    ) -> bool {
        let Some(trait_ty) = trait_ty else {
            return true;
        };
        let Some(impl_ty) = impl_ty else {
            return false;
        };
        Self::type_ref_matches_trait_ty(impl_ty, trait_ty, for_ty, assoc_types)
    }
    pub(crate) fn type_ref_matches_trait_ty(
        impl_ty: &HirTypeRef,
        trait_ty: &HirTypeRef,
        for_ty: &HirTypeRef,
        assoc_types: &FxHashMap<String, HirTypeRef>,
    ) -> bool {
        match trait_ty {
            HirTypeRef::Named(name) if name == "Self" => impl_ty == for_ty,
            HirTypeRef::Named(name) => {
                if let Some(assoc_name) = Self::self_assoc_projection_name(name) {
                    return assoc_types
                        .get(assoc_name)
                        .is_some_and(|assoc_ty| impl_ty == assoc_ty);
                }
                matches!(impl_ty, HirTypeRef::Named(impl_name) if impl_name == name)
            }
            HirTypeRef::Generic { base, args } => {
                if let Some(assoc_name) = Self::self_assoc_projection_name(base) {
                    let key = format!(
                        "{}<{}>",
                        assoc_name,
                        args.iter()
                            .map(HirTypeRef::key)
                            .collect::<Vec<_>>()
                            .join(",")
                    );
                    return assoc_types
                        .get(&key)
                        .is_some_and(|assoc_ty| impl_ty == assoc_ty);
                }
                let HirTypeRef::Generic {
                    base: impl_base,
                    args: impl_args,
                } = impl_ty
                else {
                    return false;
                };
                base == impl_base
                    && args.len() == impl_args.len()
                    && impl_args.iter().zip(args).all(|(impl_arg, trait_arg)| {
                        Self::type_ref_matches_trait_ty(impl_arg, trait_arg, for_ty, assoc_types)
                    })
            }
        }
    }
    pub(crate) fn impl_assoc_type_satisfies_trait_decl(
        impl_assoc_types: &FxHashMap<String, HirTypeRef>,
        trait_assoc_ty: &ast::TraitAssocType,
    ) -> bool {
        if trait_assoc_ty.type_params.is_empty() {
            return impl_assoc_types.contains_key(&trait_assoc_ty.name);
        }
        let prefix = format!("{}<", trait_assoc_ty.name);
        impl_assoc_types
            .keys()
            .any(|name| name.starts_with(&prefix))
    }
}
