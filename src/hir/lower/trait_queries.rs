use super::*;
impl Lowerer {
    pub(crate) fn is_current_type_param_ref(&self, ty: &HirTypeRef) -> Option<String> {
        match ty {
            HirTypeRef::Named(name) if self.current_type_params.contains(name) => {
                Some(name.clone())
            }
            _ => None,
        }
    }
    pub(crate) fn type_param_has_trait_bound(&self, type_param: &str, trait_name: &str) -> bool {
        self.generic_ref_has_trait_bound(type_param, trait_name)
    }
    pub(crate) fn trait_implies_trait(&self, have: &str, want: &str) -> bool {
        if have == want {
            return true;
        }
        let mut stack = vec![have.to_string()];
        let mut seen = FxHashSet::default();
        while let Some(name) = stack.pop() {
            if !seen.insert(name.clone()) {
                continue;
            }
            let Some(info) = self.trait_defs.get(&name) else {
                continue;
            };
            for supertrait in &info.decl.supertraits {
                if supertrait == want {
                    return true;
                }
                stack.push(supertrait.clone());
            }
        }
        false
    }
    pub(crate) fn type_param_method_bound_candidates(
        &self,
        type_param: &str,
        method_name: &str,
    ) -> Vec<String> {
        let mut candidates = self
            .current_where_bounds
            .get(type_param)
            .into_iter()
            .flat_map(|traits| traits.iter())
            .filter(|trait_name| {
                self.trait_defs
                    .get(trait_name.as_str())
                    .is_some_and(|trait_info| {
                        trait_info
                            .decl
                            .methods
                            .iter()
                            .any(|method| method.name == method_name)
                            || self.trait_supertraits_have_method(&trait_info.decl, method_name)
                    })
            })
            .cloned()
            .collect::<Vec<_>>();
        candidates.sort();
        candidates
    }
    pub(crate) fn trait_supertraits_have_method(
        &self,
        decl: &ast::TraitDecl,
        method_name: &str,
    ) -> bool {
        decl.supertraits
            .iter()
            .any(|supertrait| self.trait_has_method_transitive(supertrait, method_name))
    }
    pub(crate) fn trait_has_method_transitive(&self, trait_name: &str, method_name: &str) -> bool {
        let mut stack = vec![trait_name.to_string()];
        let mut seen = FxHashSet::default();
        while let Some(name) = stack.pop() {
            if !seen.insert(name.clone()) {
                continue;
            }
            let Some(info) = self.trait_defs.get(&name) else {
                continue;
            };
            if info
                .decl
                .methods
                .iter()
                .any(|method| method.name == method_name)
            {
                return true;
            }
            stack.extend(info.decl.supertraits.iter().cloned());
        }
        false
    }
    pub(crate) fn any_trait_has_method(&self, method_name: &str) -> bool {
        self.trait_defs
            .keys()
            .any(|trait_name| self.trait_has_method_transitive(trait_name, method_name))
    }
}
