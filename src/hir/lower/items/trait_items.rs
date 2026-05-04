use super::*;
impl Lowerer {
    pub(crate) fn validate_impl_method_signature(
        &self,
        trait_name: &str,
        for_ty: &HirTypeRef,
        assoc_types: &FxHashMap<String, HirTypeRef>,
        trait_method: &ast::TraitMethodSig,
        impl_method: &ast::FnDecl,
    ) -> RR<()> {
        if trait_method.params.len() != impl_method.params.len() {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "impl method '{}.{}' has {} parameter(s), expected {}",
                    trait_name,
                    trait_method.name,
                    impl_method.params.len(),
                    trait_method.params.len()
                ),
            )
            .at(impl_method.body.span));
        }
        for (trait_param, impl_param) in trait_method.params.iter().zip(&impl_method.params) {
            let trait_ty = trait_param.ty_hint.as_ref().map(Self::ast_type_ref);
            let impl_ty = impl_param.ty_hint.as_ref().map(Self::ast_type_ref);
            if !Self::type_ref_matches_trait_sig(&impl_ty, &trait_ty, for_ty, assoc_types) {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "impl method '{}.{}' parameter '{}' type does not match trait signature",
                        trait_name, trait_method.name, impl_param.name
                    ),
                )
                .at(impl_param.span));
            }
        }
        let trait_ret = trait_method.ret_ty_hint.as_ref().map(Self::ast_type_ref);
        let impl_ret = impl_method.ret_ty_hint.as_ref().map(Self::ast_type_ref);
        if !Self::type_ref_matches_trait_sig(&impl_ret, &trait_ret, for_ty, assoc_types) {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "impl method '{}.{}' return type does not match trait signature",
                    trait_name, trait_method.name
                ),
            )
            .at(impl_method.body.span));
        }
        Ok(())
    }
    pub(crate) fn lower_trait_decl(&mut self, decl: ast::TraitDecl, span: Span) -> RR<HirTrait> {
        let name = self.intern_symbol(&decl.name);
        let mut methods = Vec::with_capacity(decl.methods.len());
        for method in decl.methods {
            let method_name = self.intern_symbol(&method.name);
            let mut params = Vec::with_capacity(method.params.len());
            for param in method.params {
                params.push(HirTraitParamSig {
                    name: self.intern_symbol(&param.name),
                    ty: param.ty_hint.as_ref().map(Self::ast_type_ref),
                    span: param.span,
                });
            }
            methods.push(HirTraitMethodSig {
                name: method_name,
                params,
                ret_ty: method.ret_ty_hint.as_ref().map(Self::ast_type_ref),
                where_bounds: self.lower_trait_bounds(method.where_bounds)?,
                span: method.span,
            });
        }
        Ok(HirTrait {
            name,
            type_params: decl.type_params,
            supertraits: decl
                .supertraits
                .into_iter()
                .map(|name| self.intern_symbol(&name))
                .collect(),
            where_bounds: self.lower_trait_bounds(decl.where_bounds)?,
            assoc_types: decl
                .assoc_types
                .into_iter()
                .map(|assoc_ty| HirTraitAssocType {
                    name: self.intern_symbol(&assoc_ty.name),
                    span: assoc_ty.span,
                })
                .collect(),
            assoc_consts: decl
                .assoc_consts
                .into_iter()
                .map(|assoc_const| HirTraitAssocConst {
                    name: self.intern_symbol(&assoc_const.name),
                    ty: Self::ast_type_ref(&assoc_const.ty_hint),
                    span: assoc_const.span,
                })
                .collect(),
            methods,
            span,
            public: decl.public,
        })
    }
    pub(crate) fn lower_impl_decl(
        &mut self,
        decl: ast::ImplDecl,
        span: Span,
    ) -> RR<(HirImpl, Vec<HirFn>)> {
        let for_ty = Self::ast_type_ref(&decl.for_ty);
        if decl.negative {
            return Ok((
                HirImpl {
                    trait_name: self.intern_symbol(&decl.trait_name),
                    type_params: decl.type_params,
                    negative: true,
                    for_ty,
                    where_bounds: self.lower_trait_bounds(decl.where_bounds)?,
                    assoc_types: Vec::new(),
                    assoc_consts: Vec::new(),
                    methods: Vec::new(),
                    span,
                    public: decl.public,
                },
                Vec::new(),
            ));
        }
        let impl_type_params: FxHashSet<String> = decl.type_params.iter().cloned().collect();
        if !decl.type_params.is_empty()
            || Self::type_ref_contains_type_param(&for_ty, &impl_type_params)
        {
            return Ok((
                HirImpl {
                    trait_name: self.intern_symbol(&decl.trait_name),
                    type_params: decl.type_params,
                    negative: false,
                    for_ty,
                    where_bounds: self.lower_trait_bounds(decl.where_bounds)?,
                    assoc_types: decl
                        .assoc_types
                        .into_iter()
                        .map(|assoc_ty| HirImplAssocType {
                            name: self.intern_symbol(&assoc_ty.name),
                            ty: Self::ast_type_ref(&assoc_ty.ty),
                            span: assoc_ty.span,
                        })
                        .collect(),
                    assoc_consts: Vec::new(),
                    methods: Vec::new(),
                    span,
                    public: decl.public,
                },
                Vec::new(),
            ));
        }
        let impl_info = self
            .trait_impls
            .get(&(decl.trait_name.clone(), for_ty.key()))
            .cloned()
            .ok_or_else(|| {
                RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "missing registered impl for trait '{}' and type '{}'",
                        decl.trait_name,
                        for_ty.key()
                    ),
                )
                .at(span)
            })?;

        let mut hir_methods = Vec::new();
        let mut lowered_fns = Vec::new();
        let trait_info = self.trait_defs.get(&decl.trait_name).cloned();
        let mut impl_type_subst = FxHashMap::default();
        impl_type_subst.insert("Self".to_string(), impl_info.for_ty.clone());
        for assoc_ty in &decl.assoc_types {
            impl_type_subst.insert(
                format!("Self::{}", assoc_ty.name),
                Self::ast_type_ref(&assoc_ty.ty),
            );
        }
        let mut methods_by_name = decl
            .methods
            .into_iter()
            .map(|method| (method.name.clone(), method))
            .collect::<FxHashMap<_, _>>();
        for trait_method in trait_info
            .as_ref()
            .map(|info| info.decl.methods.as_slice())
            .unwrap_or(&[])
        {
            let method = if let Some(method) = methods_by_name.remove(&trait_method.name) {
                method
            } else if let Some(default_body) = trait_method.default_body.clone() {
                ast::FnDecl {
                    name: trait_method.name.clone(),
                    type_params: Vec::new(),
                    params: trait_method
                        .params
                        .clone()
                        .into_iter()
                        .map(|param| Self::substitute_fn_param_type(param, &impl_type_subst))
                        .collect(),
                    ret_ty_hint: trait_method
                        .ret_ty_hint
                        .clone()
                        .map(|ty| Self::substitute_type_expr(ty, &impl_type_subst)),
                    where_bounds: trait_method.where_bounds.clone(),
                    body: Self::substitute_block_type_hints(default_body, &impl_type_subst),
                    public: false,
                }
            } else {
                continue;
            };
            let Some(mangled) = impl_info.method_symbols.get(&method.name).cloned() else {
                continue;
            };
            let trait_method = self.intern_symbol(&method.name);
            let impl_fn = self.lower_fn(LowerFnParts {
                name: mangled,
                type_params: method.type_params,
                params: method.params,
                ret_ty_hint: method.ret_ty_hint,
                where_bounds: method.where_bounds,
                body: method.body,
                span,
            })?;
            let impl_fn_sym = impl_fn.name;
            hir_methods.push(HirImplMethod {
                trait_method,
                impl_fn: impl_fn_sym,
                span,
            });
            lowered_fns.push(impl_fn);
        }
        for method in methods_by_name.into_values() {
            let Some(mangled) = impl_info.method_symbols.get(&method.name).cloned() else {
                continue;
            };
            let trait_method = self.intern_symbol(&method.name);
            let impl_fn = self.lower_fn(LowerFnParts {
                name: mangled,
                type_params: method.type_params,
                params: method.params,
                ret_ty_hint: method.ret_ty_hint,
                where_bounds: method.where_bounds,
                body: method.body,
                span,
            })?;
            let impl_fn_sym = impl_fn.name;
            hir_methods.push(HirImplMethod {
                trait_method,
                impl_fn: impl_fn_sym,
                span,
            });
            lowered_fns.push(impl_fn);
        }

        let assoc_consts_by_name = decl
            .assoc_consts
            .iter()
            .map(|assoc_const| (assoc_const.name.clone(), assoc_const))
            .collect::<FxHashMap<_, _>>();
        let mut hir_assoc_consts = Vec::new();
        if let Some(trait_info) = trait_info.as_ref() {
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
                let Some(mangled) = impl_info.const_symbols.get(&trait_const.name).cloned() else {
                    continue;
                };
                let ret_ty_hint = Self::substitute_type_expr(ty_hint, &impl_type_subst);
                let value = Self::substitute_expr_type_hints(value, &impl_type_subst);
                let body = ast::Block {
                    stmts: vec![ast::Stmt {
                        kind: ast::StmtKind::ExprStmt {
                            expr: value.clone(),
                        },
                        span: value.span,
                    }],
                    span: value.span,
                };
                let const_fn = self.lower_fn(LowerFnParts {
                    name: mangled,
                    type_params: Vec::new(),
                    params: Vec::new(),
                    ret_ty_hint: Some(ret_ty_hint.clone()),
                    where_bounds: Vec::new(),
                    body,
                    span: item_span,
                })?;
                let value_fn = const_fn.name;
                hir_assoc_consts.push(HirImplAssocConst {
                    name: self.intern_symbol(&trait_const.name),
                    ty: Self::ast_type_ref(&ret_ty_hint),
                    value_fn,
                    span: item_span,
                });
                lowered_fns.push(const_fn);
            }
        }

        Ok((
            HirImpl {
                trait_name: self.intern_symbol(&impl_info.trait_name),
                type_params: decl.type_params,
                negative: false,
                for_ty: impl_info.for_ty,
                where_bounds: self.lower_trait_bounds(decl.where_bounds)?,
                assoc_types: decl
                    .assoc_types
                    .into_iter()
                    .map(|assoc_ty| HirImplAssocType {
                        name: self.intern_symbol(&assoc_ty.name),
                        ty: Self::ast_type_ref(&assoc_ty.ty),
                        span: assoc_ty.span,
                    })
                    .collect(),
                assoc_consts: hir_assoc_consts,
                methods: hir_methods,
                span,
                public: decl.public,
            },
            lowered_fns,
        ))
    }
}
