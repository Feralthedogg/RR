use super::*;
impl Lowerer {
    pub fn prune_private_module_metadata(&mut self, prog: &ast::Program) {
        let mut module_traits = FxHashSet::default();
        let mut public_traits = FxHashSet::default();
        let mut module_impl_traits = FxHashSet::default();
        let mut private_generic_fns = Vec::new();

        for stmt in &prog.stmts {
            match &stmt.kind {
                ast::StmtKind::TraitDecl(decl) => {
                    module_traits.insert(decl.name.clone());
                    if decl.public {
                        public_traits.insert(decl.name.clone());
                    }
                }
                ast::StmtKind::ImplDecl(decl) => {
                    module_impl_traits.insert(decl.trait_name.clone());
                }
                ast::StmtKind::FnDecl {
                    name, type_params, ..
                } if !type_params.is_empty() => private_generic_fns.push(name.clone()),
                _ => {}
            }
        }

        for name in private_generic_fns {
            self.generic_fns.remove(&name);
        }

        self.trait_defs
            .retain(|name, _| !module_traits.contains(name) || public_traits.contains(name));

        self.trait_impls.retain(|(trait_name, _), impl_info| {
            if module_traits.contains(trait_name) || module_impl_traits.contains(trait_name) {
                impl_info.public
                    && self
                        .trait_defs
                        .get(trait_name)
                        .is_some_and(|trait_info| trait_info.decl.public)
            } else {
                true
            }
        });
        self.generic_trait_impls.retain(|info| {
            if module_traits.contains(&info.decl.trait_name)
                || module_impl_traits.contains(&info.decl.trait_name)
            {
                info.decl.public
                    && self
                        .trait_defs
                        .get(&info.decl.trait_name)
                        .is_some_and(|trait_info| trait_info.decl.public)
            } else {
                true
            }
        });
        self.negative_trait_impls.retain(|header| {
            if module_traits.contains(&header.trait_name)
                || module_impl_traits.contains(&header.trait_name)
            {
                header.public
                    && self
                        .trait_defs
                        .get(&header.trait_name)
                        .is_some_and(|trait_info| trait_info.decl.public)
            } else {
                true
            }
        });
        let visible_impl_keys = self
            .trait_impls
            .keys()
            .cloned()
            .collect::<FxHashSet<(String, String)>>();
        self.generic_impl_instantiations
            .retain(|key| visible_impl_keys.contains(key));
    }
}
