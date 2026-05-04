use super::*;
impl Lowerer {
    pub fn new() -> Self {
        Self::with_policy(LoweringPolicy::default())
    }
    pub fn with_policy(policy: LoweringPolicy) -> Self {
        Self {
            scopes: vec![FxHashMap::default()], // Global scope?
            next_local_id: 0,
            next_sym_id: 0,
            in_tidy: false,
            local_names: FxHashMap::default(),
            local_emitted_names: FxHashSet::default(),
            symbols: FxHashMap::default(),
            symbols_rev: FxHashMap::default(),
            warnings: Vec::new(),
            strict_let: policy.strict_let,
            warn_implicit_decl: policy.warn_implicit_decl,
            pending_fns: Vec::new(),
            global_fn_aliases: FxHashMap::default(),
            local_trait_types: FxHashMap::default(),
            trait_defs: FxHashMap::default(),
            trait_impls: FxHashMap::default(),
            generic_trait_impls: Vec::new(),
            negative_trait_impls: Vec::new(),
            generic_impl_instantiations: FxHashSet::default(),
            generic_fns: FxHashMap::default(),
            generic_instantiations: FxHashMap::default(),
            current_type_params: FxHashSet::default(),
            current_where_bounds: FxHashMap::default(),
            r_import_aliases: FxHashMap::default(),
            r_namespace_aliases: FxHashMap::default(),
        }
    }
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }
    pub fn into_symbols(self) -> FxHashMap<SymbolId, String> {
        self.symbols
    }
    pub fn symbols_snapshot(&self) -> Vec<(SymbolId, String)> {
        let mut symbols: Vec<(SymbolId, String)> = self
            .symbols
            .iter()
            .map(|(id, name)| (*id, name.clone()))
            .collect();
        symbols.sort_by_key(|(id, _)| id.0);
        symbols
    }
    pub fn try_preload_symbols(&mut self, symbols: &[(SymbolId, String)]) -> bool {
        for (id, name) in symbols {
            if let Some(existing) = self.symbols.get(id)
                && existing != name
            {
                return false;
            }
            if let Some(existing_id) = self.symbols_rev.get(name)
                && existing_id != id
            {
                return false;
            }
        }
        for (id, name) in symbols {
            self.symbols.insert(*id, name.clone());
            self.symbols_rev.insert(name.clone(), *id);
            self.next_sym_id = self.next_sym_id.max(id.0.saturating_add(1));
        }
        true
    }
    pub(crate) fn enter_scope(&mut self) {
        self.scopes.push(FxHashMap::default());
    }
    pub(crate) fn exit_scope(&mut self) {
        self.scopes.pop();
    }
    pub(crate) fn declare_local(&mut self, name: &str) -> LocalId {
        let id = LocalId(self.next_local_id);
        self.next_local_id += 1;
        let mut emitted_name = name.to_string();
        if self.local_emitted_names.contains(&emitted_name) {
            let mut suffix = id.0;
            loop {
                let candidate = format!("{}_{}", name, suffix);
                if !self.local_emitted_names.contains(&candidate) {
                    emitted_name = candidate;
                    break;
                }
                suffix += 1;
            }
        }
        self.local_emitted_names.insert(emitted_name.clone());
        self.local_names.insert(id, emitted_name);
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), id);
        }
        id
    }
    pub(crate) fn lookup(&self, name: &str) -> Option<LocalId> {
        for scope in self.scopes.iter().rev() {
            if let Some(&id) = scope.get(name) {
                return Some(id);
            }
        }
        None
    }
    pub(crate) fn intern_symbol(&mut self, name: &str) -> SymbolId {
        if let Some(&id) = self.symbols_rev.get(name) {
            return id;
        }
        let id = SymbolId(self.next_sym_id);
        self.next_sym_id += 1;
        let owned = name.to_string();
        self.symbols.insert(id, owned.clone());
        self.symbols_rev.insert(owned, id);
        id
    }
    pub(crate) fn alloc_fn_id(&mut self) -> FnId {
        let id = FnId(self.next_sym_id);
        self.next_sym_id += 1;
        id
    }
    pub(crate) fn alloc_lambda_name(&mut self) -> String {
        let n = self.next_sym_id;
        self.next_sym_id += 1;
        format!("__lambda_{}", n)
    }
}
