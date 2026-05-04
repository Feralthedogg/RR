use super::*;
impl Lowerer {
    pub(crate) fn flush_pending_fns(&mut self, items: &mut Vec<HirItem>) {
        if self.pending_fns.is_empty() {
            return;
        }
        for f in self.pending_fns.drain(..) {
            items.push(HirItem::Fn(f));
        }
    }
    pub fn lower_module(&mut self, prog: ast::Program, mod_id: ModuleId) -> RR<HirModule> {
        self.register_trait_decls(&prog.stmts)?;
        self.register_generic_fn_decls(&prog.stmts)?;
        self.register_impl_decls(&prog.stmts)?;

        let mut items = Vec::new();
        for stmt in prog.stmts {
            // Top-level function declarations stay as module items; all other
            // top-level statements are preserved as statement items.
            match stmt.kind {
                ast::StmtKind::FnDecl {
                    name,
                    type_params,
                    params,
                    ret_ty_hint,
                    where_bounds,
                    body,
                } => {
                    let fn_item = self.lower_fn(LowerFnParts {
                        name,
                        type_params,
                        params,
                        ret_ty_hint,
                        where_bounds,
                        body,
                        span: stmt.span,
                    })?;
                    items.push(HirItem::Fn(fn_item));
                    self.flush_pending_fns(&mut items);
                }
                ast::StmtKind::TraitDecl(decl) => {
                    let trait_item = self.lower_trait_decl(decl, stmt.span)?;
                    items.push(HirItem::Trait(trait_item));
                    self.flush_pending_fns(&mut items);
                }
                ast::StmtKind::ImplDecl(decl) => {
                    let (impl_item, method_fns) = self.lower_impl_decl(decl, stmt.span)?;
                    items.push(HirItem::Impl(impl_item));
                    for method_fn in method_fns {
                        items.push(HirItem::Fn(method_fn));
                    }
                    self.flush_pending_fns(&mut items);
                }
                ast::StmtKind::Import { source, path, spec } => {
                    match source {
                        ast::ImportSource::Module => {
                            if !matches!(spec, ast::ImportSpec::Glob) {
                                return Err(RRException::new(
                                    "RR.SemanticError",
                                    RRCode::E1002,
                                    Stage::Lower,
                                    "RR module import does not support named or namespace specifiers"
                                        .to_string(),
                                )
                                .at(stmt.span));
                            }
                            let import = HirImport {
                                module: path,
                                spec: HirImportSpec::Glob,
                                span: stmt.span,
                            };
                            items.push(HirItem::Import(import));
                        }
                        ast::ImportSource::RPackage => match spec {
                            ast::ImportSpec::Glob => {
                                let alias = path.clone();
                                if self.r_import_aliases.contains_key(&alias) {
                                    let prev_name = self
                                        .r_import_aliases
                                        .get(&alias)
                                        .and_then(|sym| self.symbols.get(sym))
                                        .cloned()
                                        .unwrap_or_else(|| "<unknown>".to_string());
                                    return Err(RRException::new(
                                        "RR.SemanticError",
                                        RRCode::E1002,
                                        Stage::Lower,
                                        format!(
                                            "R namespace alias '{}' conflicts with imported symbol '{}'; choose another alias",
                                            alias, prev_name
                                        ),
                                    )
                                    .at(stmt.span));
                                }
                                if let Some(prev_pkg) =
                                    self.r_namespace_aliases.get(&alias).cloned()
                                    && prev_pkg != path
                                {
                                    return Err(RRException::new(
                                        "RR.SemanticError",
                                        RRCode::E1002,
                                        Stage::Lower,
                                        format!(
                                            "R namespace alias '{}' is already bound to package '{}'; choose another alias",
                                            alias, prev_pkg
                                        ),
                                    )
                                    .at(stmt.span));
                                }
                                self.r_namespace_aliases.insert(alias, path);
                            }
                            ast::ImportSpec::Named(bindings) => {
                                for binding in bindings {
                                    let local =
                                        binding.local.unwrap_or_else(|| binding.imported.clone());
                                    let qualified = format!("{}::{}", path, binding.imported);
                                    let sym = self.intern_symbol(&qualified);
                                    if let Some(prev) = self.r_import_aliases.get(&local).copied()
                                        && prev != sym
                                    {
                                        let prev_name = self
                                            .symbols
                                            .get(&prev)
                                            .cloned()
                                            .unwrap_or_else(|| "<unknown>".to_string());
                                        return Err(RRException::new(
                                                "RR.SemanticError",
                                                RRCode::E1002,
                                                Stage::Lower,
                                                format!(
                                                    "R import local '{}' is already bound to '{}'; use 'as' to choose a different local name",
                                                    local, prev_name
                                                ),
                                            )
                                            .at(stmt.span));
                                    }
                                    if let Some(prev_pkg) = self.r_namespace_aliases.get(&local)
                                        && prev_pkg != &path
                                    {
                                        return Err(RRException::new(
                                                "RR.SemanticError",
                                                RRCode::E1002,
                                                Stage::Lower,
                                                format!(
                                                    "R import local '{}' conflicts with namespace alias for package '{}'; use 'as' to rename the imported symbol",
                                                    local, prev_pkg
                                                ),
                                            )
                                            .at(stmt.span));
                                    }
                                    self.r_import_aliases.insert(local, sym);
                                }
                            }
                            ast::ImportSpec::Namespace(alias) => {
                                if self.r_import_aliases.contains_key(&alias) {
                                    let prev_name = self
                                        .r_import_aliases
                                        .get(&alias)
                                        .and_then(|sym| self.symbols.get(sym))
                                        .cloned()
                                        .unwrap_or_else(|| "<unknown>".to_string());
                                    return Err(RRException::new(
                                            "RR.SemanticError",
                                            RRCode::E1002,
                                            Stage::Lower,
                                            format!(
                                                "R namespace alias '{}' conflicts with imported symbol '{}'; choose another alias",
                                                alias, prev_name
                                            ),
                                        )
                                        .at(stmt.span));
                                }
                                if let Some(prev_pkg) =
                                    self.r_namespace_aliases.get(&alias).cloned()
                                    && prev_pkg != path
                                {
                                    return Err(RRException::new(
                                            "RR.SemanticError",
                                            RRCode::E1002,
                                            Stage::Lower,
                                            format!(
                                                "R namespace alias '{}' is already bound to package '{}'; choose another alias",
                                                alias, prev_pkg
                                            ),
                                        )
                                        .at(stmt.span));
                                }
                                self.r_namespace_aliases.insert(alias, path);
                            }
                        },
                    }
                    self.flush_pending_fns(&mut items);
                }
                ast::StmtKind::Export(fndecl) => {
                    let mut fn_item = self.lower_fn(LowerFnParts {
                        name: fndecl.name,
                        type_params: fndecl.type_params,
                        params: fndecl.params,
                        ret_ty_hint: fndecl.ret_ty_hint,
                        where_bounds: fndecl.where_bounds,
                        body: fndecl.body,
                        span: stmt.span,
                    })?;
                    fn_item.public = true;
                    items.push(HirItem::Fn(fn_item));
                    self.flush_pending_fns(&mut items);
                }
                _ => {
                    let s = self.lower_stmt(stmt)?;
                    items.push(HirItem::Stmt(s));
                    self.flush_pending_fns(&mut items);
                }
            }
        }

        Ok(HirModule {
            id: mod_id,
            path: vec![],
            items,
        })
    }
}
