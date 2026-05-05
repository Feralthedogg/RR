use super::*;
impl Lowerer {
    pub(crate) fn lower_fn(&mut self, parts: LowerFnParts) -> RR<HirFn> {
        let LowerFnParts {
            name,
            type_params,
            params,
            ret_ty_hint,
            where_bounds,
            body,
            span,
        } = parts;
        let fn_id = self.alloc_fn_id();
        let sym_name = self.intern_symbol(&name);

        // Isolate function-local scope from module-level bindings.
        // Names not declared in the function should lower as globals.
        let saved_scopes = std::mem::replace(&mut self.scopes, vec![FxHashMap::default()]);
        let saved_local_names = std::mem::take(&mut self.local_names);
        let saved_local_emitted_names = std::mem::take(&mut self.local_emitted_names);
        let saved_local_trait_types = std::mem::take(&mut self.local_trait_types);
        let saved_type_params = std::mem::take(&mut self.current_type_params);
        let saved_where_bounds = std::mem::take(&mut self.current_where_bounds);
        let saved_next_local = self.next_local_id;
        self.next_local_id = 0;
        self.current_type_params = type_params.iter().cloned().collect();
        self.current_where_bounds = Self::where_bound_map(&where_bounds);

        self.enter_scope();
        let mut hir_params: Vec<HirParam> = Vec::new();
        let mut param_syms = Vec::with_capacity(params.len());
        for p in &params {
            let pid = self.declare_local(&p.name);
            if let Some(ty_hint) = &p.ty_hint {
                self.local_trait_types
                    .insert(pid, Self::ast_type_ref(ty_hint));
            }
            let psym = self.intern_symbol(&p.name);
            param_syms.push(psym);
        }
        for (p, psym) in params.into_iter().zip(param_syms) {
            let explicit_ty_hint = p.ty_hint.as_ref().and_then(Self::parse_type_hint_expr);
            let default_ty_hint = explicit_ty_hint
                .is_none()
                .then(|| p.default.as_ref().and_then(Self::infer_param_type_hint))
                .flatten();
            let ty_inferred = default_ty_hint.is_some();
            let ty_hint = explicit_ty_hint.or(default_ty_hint);
            let default = if let Some(d) = p.default {
                Some(self.lower_expr(d)?)
            } else {
                None
            };
            hir_params.push(HirParam {
                name: psym,
                ty: ty_hint,
                ty_inferred,
                default,
                span: p.span,
            });
        }

        let hir_body = Self::apply_implicit_tail_return(self.lower_block(body)?);
        self.exit_scope();

        let local_names = self.local_names.drain().collect();

        self.scopes = saved_scopes;
        self.local_names = saved_local_names;
        self.local_emitted_names = saved_local_emitted_names;
        self.local_trait_types = saved_local_trait_types;
        self.current_type_params = saved_type_params;
        self.current_where_bounds = saved_where_bounds;
        self.next_local_id = saved_next_local;

        // These variables are not defined in the original code,
        // but the instruction implies they should be used as variables.
        // To maintain syntactic correctness, we'll define them with their original literal values.
        let has_varargs = false;
        let ret_ty = ret_ty_hint.as_ref().and_then(Self::parse_type_hint_expr);

        Ok(HirFn {
            id: fn_id,
            name: sym_name,
            type_params,
            where_bounds: self.lower_trait_bounds(where_bounds)?,
            params: hir_params,
            has_varargs,
            ret_ty,
            ret_ty_inferred: false,
            body: hir_body,
            attrs: HirFnAttrs {
                inline_hint: InlineHint::Default,
                tidy_safe: false,
            },
            span,
            local_names,
            public: false, // Default private
        })
    }
}
