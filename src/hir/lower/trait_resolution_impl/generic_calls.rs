use super::*;
impl Lowerer {
    pub(crate) fn bind_generic_type_param(
        subst: &mut FxHashMap<String, HirTypeRef>,
        type_param: &str,
        actual: HirTypeRef,
        span: Span,
    ) -> RR<()> {
        if let Some(prev) = subst.get(type_param) {
            if prev != &actual {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "generic type parameter '{}' inferred as both '{}' and '{}'",
                        type_param,
                        prev.key(),
                        actual.key()
                    ),
                )
                .at(span));
            }
        } else {
            subst.insert(type_param.to_string(), actual);
        }
        Ok(())
    }
    pub(crate) fn infer_generic_type_from_param(
        type_params: &FxHashSet<String>,
        formal: &ast::TypeExpr,
        actual: &HirTypeRef,
        subst: &mut FxHashMap<String, HirTypeRef>,
        span: Span,
    ) -> RR<()> {
        match formal {
            ast::TypeExpr::Named(name) if type_params.contains(name) => {
                Self::bind_generic_type_param(subst, name, actual.clone(), span)
            }
            ast::TypeExpr::Named(_) => Ok(()),
            ast::TypeExpr::Generic { base, args } => {
                let HirTypeRef::Generic {
                    base: actual_base,
                    args: actual_args,
                } = actual
                else {
                    return Ok(());
                };
                if base != actual_base || args.len() != actual_args.len() {
                    return Ok(());
                }
                for (formal_arg, actual_arg) in args.iter().zip(actual_args) {
                    Self::infer_generic_type_from_param(
                        type_params,
                        formal_arg,
                        actual_arg,
                        subst,
                        span,
                    )?;
                }
                Ok(())
            }
        }
    }
    pub(crate) fn generic_call_arg_expr<'a>(
        params: &[ast::FnParam],
        args: &'a [ast::Expr],
        param_idx: usize,
    ) -> Option<&'a ast::Expr> {
        let param_name = params.get(param_idx)?.name.as_str();
        let mut positional_idx = 0usize;
        for arg in args {
            match &arg.kind {
                ast::ExprKind::NamedArg { name, value } if name == param_name => {
                    return Some(value);
                }
                ast::ExprKind::NamedArg { .. } => {}
                _ if positional_idx == param_idx => return Some(arg),
                _ => positional_idx += 1,
            }
        }
        None
    }
    pub(crate) fn infer_generic_subst(
        &self,
        decl: &ast::FnDecl,
        explicit_type_args: &[ast::TypeExpr],
        args: &[ast::Expr],
        expected_ret_ty: Option<&HirTypeRef>,
        span: Span,
    ) -> RR<FxHashMap<String, HirTypeRef>> {
        let type_params: FxHashSet<String> = decl.type_params.iter().cloned().collect();
        let mut subst = FxHashMap::default();
        if !explicit_type_args.is_empty() {
            if explicit_type_args.len() != decl.type_params.len() {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "generic function '{}' expects {} explicit type argument(s), got {}",
                        decl.name,
                        decl.type_params.len(),
                        explicit_type_args.len()
                    ),
                )
                .at(span));
            }
            for (type_param, explicit_ty) in decl.type_params.iter().zip(explicit_type_args) {
                Self::bind_generic_type_param(
                    &mut subst,
                    type_param,
                    Self::ast_type_ref(explicit_ty),
                    span,
                )?;
            }
        }
        for (param_idx, param) in decl.params.iter().enumerate() {
            let Some(formal_ty) = &param.ty_hint else {
                continue;
            };
            let Some(arg_expr) = Self::generic_call_arg_expr(&decl.params, args, param_idx) else {
                continue;
            };
            let Some(actual_ty) = self.trait_type_of_ast_expr(arg_expr) else {
                continue;
            };
            Self::infer_generic_type_from_param(
                &type_params,
                formal_ty,
                &actual_ty,
                &mut subst,
                arg_expr.span,
            )?;
        }
        if let (Some(ret_ty_hint), Some(expected_ret_ty)) =
            (decl.ret_ty_hint.as_ref(), expected_ret_ty)
        {
            Self::infer_generic_type_from_param(
                &type_params,
                ret_ty_hint,
                expected_ret_ty,
                &mut subst,
                span,
            )?;
        }
        for type_param in &decl.type_params {
            if !subst.contains_key(type_param) {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "cannot infer generic type parameter '{}' for call to '{}'; add an explicit argument type hint at the call site",
                        type_param, decl.name
                    ),
                )
                .at(span));
            }
        }
        Ok(subst)
    }
    pub(crate) fn validate_generic_bounds_for_subst(
        &mut self,
        decl: &ast::FnDecl,
        subst: &FxHashMap<String, HirTypeRef>,
        span: Span,
    ) -> RR<()> {
        self.validate_trait_bounds_for_subst(&decl.where_bounds, subst, span)
    }
    pub(crate) fn generic_instance_name(name: &str, concrete_tys: &[HirTypeRef]) -> String {
        pub(crate) fn sanitize(input: &str) -> String {
            input
                .chars()
                .map(|ch| {
                    if ch.is_ascii_alphanumeric() || ch == '_' {
                        ch
                    } else {
                        '_'
                    }
                })
                .collect()
        }
        let suffix = concrete_tys
            .iter()
            .map(|ty| sanitize(&ty.key()))
            .collect::<Vec<_>>()
            .join("_");
        format!("__rr_mono_{}_{}", sanitize(name), suffix)
    }
    pub(crate) fn resolve_generic_call(
        &mut self,
        callee_name: &str,
        explicit_type_args: &[ast::TypeExpr],
        args: &[ast::Expr],
        expected_ret_ty: Option<&HirTypeRef>,
        span: Span,
    ) -> RR<Option<SymbolId>> {
        let Some(info) = self.generic_fns.get(callee_name).cloned() else {
            return Ok(None);
        };
        let subst =
            self.infer_generic_subst(&info.decl, explicit_type_args, args, expected_ret_ty, span)?;
        self.validate_generic_bounds_for_subst(&info.decl, &subst, span)?;
        let subst =
            self.subst_with_assoc_type_projections(&info.decl.where_bounds, &subst, span)?;
        let concrete_tys = info
            .decl
            .type_params
            .iter()
            .filter_map(|type_param| subst.get(type_param).cloned())
            .collect::<Vec<_>>();
        let key = (
            info.decl.name.clone(),
            concrete_tys.iter().map(HirTypeRef::key).collect::<Vec<_>>(),
        );
        if let Some(sym) = self.generic_instantiations.get(&key).copied() {
            return Ok(Some(sym));
        }

        let inst_name = Self::generic_instance_name(&info.decl.name, &concrete_tys);
        let inst_sym = self.intern_symbol(&inst_name);
        self.generic_instantiations.insert(key, inst_sym);
        let inst_params = info
            .decl
            .params
            .into_iter()
            .map(|param| Self::substitute_fn_param_type(param, &subst))
            .collect();
        let inst_ret = info
            .decl
            .ret_ty_hint
            .map(|ty| Self::substitute_type_expr(ty, &subst));
        let inst_body = Self::substitute_block_type_hints(info.decl.body, &subst);
        let inst_fn = self.lower_fn(LowerFnParts {
            name: inst_name,
            type_params: Vec::new(),
            params: inst_params,
            ret_ty_hint: inst_ret,
            where_bounds: Vec::new(),
            body: inst_body,
            span,
        })?;
        self.pending_fns.push(inst_fn);
        Ok(Some(inst_sym))
    }
}
