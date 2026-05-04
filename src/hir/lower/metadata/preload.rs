use super::*;
impl Lowerer {
    pub fn preload_module_metadata(&mut self, prog: &ast::Program) -> RR<()> {
        self.register_trait_decls(&prog.stmts)?;
        self.register_generic_fn_decls(&prog.stmts)?;
        self.register_impl_decls(&prog.stmts)
    }
    pub fn preload_public_module_metadata(&mut self, prog: &ast::Program) -> RR<()> {
        let stmts = self.public_metadata_stmts(&prog.stmts);
        self.register_trait_decls(&stmts)?;
        self.register_generic_fn_decls(&stmts)?;
        self.register_impl_decls(&stmts)
    }
    pub(crate) fn public_metadata_stmts(&self, stmts: &[ast::Stmt]) -> Vec<ast::Stmt> {
        let public_traits = stmts
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                ast::StmtKind::TraitDecl(decl) if decl.public => Some(decl.name.clone()),
                _ => None,
            })
            .collect::<FxHashSet<_>>();

        stmts
            .iter()
            .filter(|stmt| match &stmt.kind {
                ast::StmtKind::TraitDecl(decl) => decl.public,
                ast::StmtKind::ImplDecl(decl) => {
                    decl.public
                        && (public_traits.contains(&decl.trait_name)
                            || self.trait_defs.contains_key(&decl.trait_name))
                }
                ast::StmtKind::Export(fndecl) => !fndecl.type_params.is_empty(),
                _ => false,
            })
            .cloned()
            .collect()
    }
}
