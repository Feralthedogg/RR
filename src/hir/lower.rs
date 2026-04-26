use crate::error::{RR, RRCode, RRException, Stage};
use crate::hir::def::*;
use crate::syntax::ast;
use crate::typeck::trait_solver::{
    TraitImplHeader, TraitSolver, trait_impl_is_more_specific, trait_impl_patterns_overlap,
};
use crate::utils::{Span, did_you_mean};
use rustc_hash::{FxHashMap, FxHashSet};

mod call_dispatch;

#[derive(Clone)]
struct TraitDeclInfo {
    decl: ast::TraitDecl,
}

#[derive(Clone)]
struct TraitImplInfo {
    trait_name: String,
    for_ty: HirTypeRef,
    assoc_types: FxHashMap<String, HirTypeRef>,
    method_symbols: FxHashMap<String, String>,
    const_symbols: FxHashMap<String, String>,
    public: bool,
}

#[derive(Clone)]
struct GenericTraitImplInfo {
    decl: ast::ImplDecl,
    for_ty: HirTypeRef,
}

#[derive(Clone)]
struct GenericFnInfo {
    decl: ast::FnDecl,
}

enum TraitAssocConstResolution {
    NotAssocConst,
    GenericBound,
    Concrete(SymbolId),
}

enum TraitStaticMethodResolution {
    NotStaticMethod,
    GenericBound,
    Concrete(SymbolId),
}

#[derive(Clone, Copy)]
struct TypeProjectionParts<'a> {
    base: &'a str,
    trait_name: Option<&'a str>,
    assoc: &'a str,
}

pub struct Lowerer {
    // Symbol resolution state
    scopes: Vec<FxHashMap<String, LocalId>>,
    next_local_id: u32,
    next_sym_id: u32,

    // Context flags
    in_tidy: bool,

    // Mapping for current function's locals
    local_names: FxHashMap<LocalId, String>,
    local_emitted_names: FxHashSet<String>,

    // Global Symbol Table
    symbols: FxHashMap<SymbolId, String>,
    symbols_rev: FxHashMap<String, SymbolId>,
    // Collected lowering warnings (reported by caller)
    warnings: Vec<String>,
    // If true, assignment to undeclared names is an error.
    strict_let: bool,
    // If true, emit warnings for implicit declarations.
    warn_implicit_decl: bool,
    // Lambda-lifted synthetic functions.
    pending_fns: Vec<HirFn>,
    // Top-level aliases of function values, e.g. `let f = fn(...) { ... }`.
    // Used so function bodies can resolve `f(...)` directly to lifted symbols.
    global_fn_aliases: FxHashMap<String, SymbolId>,
    // Source-level type hints retained for static trait dispatch. These are
    // deliberately separate from MIR type terms because user-defined trait
    // receiver names may not have a concrete MIR type yet.
    local_trait_types: FxHashMap<LocalId, HirTypeRef>,
    trait_defs: FxHashMap<String, TraitDeclInfo>,
    trait_impls: FxHashMap<(String, String), TraitImplInfo>,
    generic_trait_impls: Vec<GenericTraitImplInfo>,
    negative_trait_impls: Vec<TraitImplHeader>,
    generic_impl_instantiations: FxHashSet<(String, String)>,
    generic_fns: FxHashMap<String, GenericFnInfo>,
    generic_instantiations: FxHashMap<(String, Vec<String>), SymbolId>,
    current_type_params: FxHashSet<String>,
    current_where_bounds: FxHashMap<String, FxHashSet<String>>,
    // Named imports from R packages, lowered to pkg::symbol references.
    r_import_aliases: FxHashMap<String, SymbolId>,
    // Namespace imports from R packages, lowered from ns.foo(...) to pkg::foo(...).
    r_namespace_aliases: FxHashMap<String, String>,
}

impl Default for Lowerer {
    fn default() -> Self {
        Self::new()
    }
}

impl Lowerer {
    pub fn new() -> Self {
        Self::with_policy(true, false)
    }

    pub fn with_policy(strict_let: bool, warn_implicit_decl: bool) -> Self {
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
            strict_let,
            warn_implicit_decl,
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

    fn enter_scope(&mut self) {
        self.scopes.push(FxHashMap::default());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_local(&mut self, name: &str) -> LocalId {
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

    fn lookup(&self, name: &str) -> Option<LocalId> {
        for scope in self.scopes.iter().rev() {
            if let Some(&id) = scope.get(name) {
                return Some(id);
            }
        }
        None
    }

    fn intern_symbol(&mut self, name: &str) -> SymbolId {
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

    fn alloc_fn_id(&mut self) -> FnId {
        let id = FnId(self.next_sym_id);
        self.next_sym_id += 1;
        id
    }

    fn alloc_lambda_name(&mut self) -> String {
        let n = self.next_sym_id;
        self.next_sym_id += 1;
        format!("__lambda_{}", n)
    }

    fn ast_type_ref(expr: &ast::TypeExpr) -> HirTypeRef {
        match expr {
            ast::TypeExpr::Named(name) => HirTypeRef::Named(name.clone()),
            ast::TypeExpr::Generic { base, args } => HirTypeRef::Generic {
                base: base.clone(),
                args: args.iter().map(Self::ast_type_ref).collect(),
            },
        }
    }

    fn dyn_trait_name(expr: &ast::TypeExpr) -> Option<&str> {
        let ast::TypeExpr::Named(name) = expr else {
            return None;
        };
        name.strip_prefix("dyn ")
    }

    fn type_ref_contains_type_param(ty: &HirTypeRef, type_params: &FxHashSet<String>) -> bool {
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

    fn lower_trait_bounds(&mut self, bounds: Vec<ast::TraitBound>) -> RR<Vec<HirTraitBound>> {
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

    fn where_bound_map(bounds: &[ast::TraitBound]) -> FxHashMap<String, FxHashSet<String>> {
        let mut out: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
        for bound in bounds {
            let entry = out.entry(bound.type_name.clone()).or_default();
            for trait_name in &bound.trait_names {
                entry.insert(trait_name.clone());
            }
        }
        out
    }

    fn type_projection_parts(name: &str) -> Option<TypeProjectionParts<'_>> {
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

    fn qualified_type_projection_key(base: &str, trait_name: &str, assoc: &str) -> String {
        format!("<{} as {}>::{}", base, trait_name, assoc)
    }

    fn insert_assoc_projection_subst(
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

    fn insert_alias_assoc_projection_subst(
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

    fn self_assoc_projection_name(name: &str) -> Option<&str> {
        if let Some(rest) = name.strip_prefix("Self::") {
            return Some(rest);
        }
        let parts = Self::type_projection_parts(name)?;
        (parts.base == "Self").then_some(parts.assoc)
    }

    fn current_generic_ref_key(&self, ty: &HirTypeRef) -> Option<String> {
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

    fn type_ref_contains_current_type_param(&self, ty: &HirTypeRef) -> bool {
        Self::type_ref_contains_type_param(ty, &self.current_type_params)
    }

    fn generic_ref_has_trait_bound(&self, type_key: &str, trait_name: &str) -> bool {
        self.current_where_bounds
            .get(type_key)
            .is_some_and(|traits| {
                traits
                    .iter()
                    .any(|bound_trait| self.trait_implies_trait(bound_trait, trait_name))
            })
    }

    fn public_metadata_stmts(&self, stmts: &[ast::Stmt]) -> Vec<ast::Stmt> {
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

    fn is_current_type_param_ref(&self, ty: &HirTypeRef) -> Option<String> {
        match ty {
            HirTypeRef::Named(name) if self.current_type_params.contains(name) => {
                Some(name.clone())
            }
            _ => None,
        }
    }

    fn type_param_has_trait_bound(&self, type_param: &str, trait_name: &str) -> bool {
        self.generic_ref_has_trait_bound(type_param, trait_name)
    }

    fn trait_implies_trait(&self, have: &str, want: &str) -> bool {
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

    fn type_param_method_bound_candidates(
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

    fn trait_supertraits_have_method(&self, decl: &ast::TraitDecl, method_name: &str) -> bool {
        decl.supertraits
            .iter()
            .any(|supertrait| self.trait_has_method_transitive(supertrait, method_name))
    }

    fn trait_has_method_transitive(&self, trait_name: &str, method_name: &str) -> bool {
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

    fn trait_method_mangle(trait_name: &str, for_ty: &HirTypeRef, method: &str) -> String {
        fn sanitize(input: &str) -> String {
            let mut out = String::new();
            for ch in input.chars() {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    out.push(ch);
                } else {
                    out.push('_');
                }
            }
            out
        }
        format!(
            "__rr_trait_{}_{}_{}",
            sanitize(trait_name),
            sanitize(&for_ty.key()),
            sanitize(method)
        )
    }

    fn trait_const_mangle(trait_name: &str, for_ty: &HirTypeRef, name: &str) -> String {
        fn sanitize(input: &str) -> String {
            let mut out = String::new();
            for ch in input.chars() {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    out.push(ch);
                } else {
                    out.push('_');
                }
            }
            out
        }
        format!(
            "__rr_trait_const_{}_{}_{}",
            sanitize(trait_name),
            sanitize(&for_ty.key()),
            sanitize(name)
        )
    }

    fn type_ref_matches_trait_sig(
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

    fn type_ref_matches_trait_ty(
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

    fn impl_assoc_type_satisfies_trait_decl(
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

    fn register_trait_decls(&mut self, stmts: &[ast::Stmt]) -> RR<()> {
        for stmt in stmts {
            if let ast::StmtKind::TraitDecl(decl) = &stmt.kind {
                if self.trait_defs.contains_key(&decl.name) {
                    return Err(RRException::new(
                        "RR.SemanticError",
                        RRCode::E1002,
                        Stage::Lower,
                        format!("duplicate trait declaration '{}'", decl.name),
                    )
                    .at(stmt.span));
                }
                let mut method_names = FxHashSet::default();
                let mut assoc_item_names = FxHashSet::default();
                for method in &decl.methods {
                    if !method_names.insert(method.name.clone()) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "duplicate method '{}' in trait '{}'",
                                method.name, decl.name
                            ),
                        )
                        .at(method.span));
                    }
                    if !assoc_item_names.insert(method.name.clone()) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "duplicate associated item '{}' in trait '{}'",
                                method.name, decl.name
                            ),
                        )
                        .at(method.span));
                    }
                }
                let mut assoc_type_names = FxHashSet::default();
                for assoc_ty in &decl.assoc_types {
                    if !assoc_type_names.insert(assoc_ty.name.clone()) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "duplicate associated type '{}' in trait '{}'",
                                assoc_ty.name, decl.name
                            ),
                        )
                        .at(assoc_ty.span));
                    }
                    if !assoc_item_names.insert(assoc_ty.name.clone()) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "duplicate associated item '{}' in trait '{}'",
                                assoc_ty.name, decl.name
                            ),
                        )
                        .at(assoc_ty.span));
                    }
                }
                let mut assoc_const_names = FxHashSet::default();
                for assoc_const in &decl.assoc_consts {
                    if !assoc_const_names.insert(assoc_const.name.clone()) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "duplicate associated const '{}' in trait '{}'",
                                assoc_const.name, decl.name
                            ),
                        )
                        .at(assoc_const.span));
                    }
                    if !assoc_item_names.insert(assoc_const.name.clone()) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "duplicate associated item '{}' in trait '{}'",
                                assoc_const.name, decl.name
                            ),
                        )
                        .at(assoc_const.span));
                    }
                }
                for supertrait in &decl.supertraits {
                    if supertrait == &decl.name {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!("trait '{}' cannot list itself as a supertrait", decl.name),
                        )
                        .at(stmt.span));
                    }
                    if !self.trait_defs.contains_key(supertrait) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "trait '{}' references unknown supertrait '{}'",
                                decl.name, supertrait
                            ),
                        )
                        .at(stmt.span));
                    }
                }
                self.trait_defs
                    .insert(decl.name.clone(), TraitDeclInfo { decl: decl.clone() });
            }
        }
        Ok(())
    }

    fn register_generic_fn_decls(&mut self, stmts: &[ast::Stmt]) -> RR<()> {
        for stmt in stmts {
            let fndecl = match &stmt.kind {
                ast::StmtKind::FnDecl {
                    name,
                    type_params,
                    params,
                    ret_ty_hint,
                    where_bounds,
                    body,
                } if !type_params.is_empty() => ast::FnDecl {
                    name: name.clone(),
                    type_params: type_params.clone(),
                    params: params.clone(),
                    ret_ty_hint: ret_ty_hint.clone(),
                    where_bounds: where_bounds.clone(),
                    body: body.clone(),
                    public: false,
                },
                ast::StmtKind::Export(fndecl) if !fndecl.type_params.is_empty() => fndecl.clone(),
                _ => continue,
            };
            if self.generic_fns.contains_key(&fndecl.name) {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!("duplicate generic function declaration '{}'", fndecl.name),
                )
                .at(stmt.span));
            }
            self.generic_fns
                .insert(fndecl.name.clone(), GenericFnInfo { decl: fndecl });
        }
        Ok(())
    }

    fn register_impl_decls(&mut self, stmts: &[ast::Stmt]) -> RR<()> {
        for stmt in stmts {
            if let ast::StmtKind::ImplDecl(decl) = &stmt.kind {
                let Some(trait_info) = self.trait_defs.get(&decl.trait_name).cloned() else {
                    return Err(RRException::new(
                        "RR.SemanticError",
                        RRCode::E1002,
                        Stage::Lower,
                        format!("impl references unknown trait '{}'", decl.trait_name),
                    )
                    .at(stmt.span));
                };
                let for_ty = Self::ast_type_ref(&decl.for_ty);
                let impl_key = (decl.trait_name.clone(), for_ty.key());
                let impl_type_params: FxHashSet<String> =
                    decl.type_params.iter().cloned().collect();
                let is_generic_impl = !decl.type_params.is_empty()
                    || Self::type_ref_contains_type_param(&for_ty, &impl_type_params);
                let is_public_impl = decl.public && trait_info.decl.public;
                let new_header = TraitImplHeader {
                    trait_name: decl.trait_name.clone(),
                    for_ty: for_ty.clone(),
                    type_params: decl.type_params.clone(),
                    public: is_public_impl,
                    span: stmt.span,
                };
                if decl.negative {
                    if !decl.methods.is_empty()
                        || !decl.assoc_types.is_empty()
                        || !decl.assoc_consts.is_empty()
                    {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "negative impl of trait '{}' for '{}' cannot define methods, associated types, or associated consts",
                                decl.trait_name,
                                for_ty.key()
                            ),
                        )
                        .at(stmt.span));
                    }
                    for existing in &self.negative_trait_impls {
                        if trait_impl_patterns_overlap(existing, &new_header) {
                            return Err(RRException::new(
                                "RR.SemanticError",
                                RRCode::E1002,
                                Stage::Lower,
                                format!(
                                    "overlapping negative impl of trait '{}' for '{}' conflicts with existing negative impl for '{}'",
                                    decl.trait_name,
                                    for_ty.key(),
                                    existing.for_ty.key()
                                ),
                            )
                            .at(stmt.span));
                        }
                    }
                    for ((trait_name, _), impl_info) in &self.trait_impls {
                        let positive = TraitImplHeader {
                            trait_name: trait_name.clone(),
                            for_ty: impl_info.for_ty.clone(),
                            type_params: Vec::new(),
                            public: impl_info.public,
                            span: stmt.span,
                        };
                        if trait_impl_patterns_overlap(&positive, &new_header)
                            && !trait_impl_is_more_specific(&new_header, &positive)
                        {
                            return Err(RRException::new(
                                "RR.SemanticError",
                                RRCode::E1002,
                                Stage::Lower,
                                format!(
                                    "negative impl of trait '{}' for '{}' conflicts with existing positive impl for '{}'",
                                    decl.trait_name,
                                    for_ty.key(),
                                    positive.for_ty.key()
                                ),
                            )
                            .at(stmt.span));
                        }
                    }
                    for info in &self.generic_trait_impls {
                        let positive = TraitImplHeader {
                            trait_name: info.decl.trait_name.clone(),
                            for_ty: info.for_ty.clone(),
                            type_params: info.decl.type_params.clone(),
                            public: info.decl.public,
                            span: stmt.span,
                        };
                        if trait_impl_patterns_overlap(&positive, &new_header)
                            && !trait_impl_is_more_specific(&new_header, &positive)
                        {
                            return Err(RRException::new(
                                "RR.SemanticError",
                                RRCode::E1002,
                                Stage::Lower,
                                format!(
                                    "negative impl of trait '{}' for '{}' conflicts with existing positive impl for '{}'",
                                    decl.trait_name,
                                    for_ty.key(),
                                    positive.for_ty.key()
                                ),
                            )
                            .at(stmt.span));
                        }
                    }
                    self.negative_trait_impls.push(new_header);
                    continue;
                }
                for negative in &self.negative_trait_impls {
                    if trait_impl_patterns_overlap(negative, &new_header)
                        && !trait_impl_is_more_specific(negative, &new_header)
                    {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "impl of trait '{}' for '{}' conflicts with negative impl for '{}'",
                                decl.trait_name,
                                for_ty.key(),
                                negative.for_ty.key()
                            ),
                        )
                        .at(stmt.span));
                    }
                }
                let mut solver = TraitSolver::new();
                for ((trait_name, _), impl_info) in &self.trait_impls {
                    solver.add_impl(TraitImplHeader {
                        trait_name: trait_name.clone(),
                        for_ty: impl_info.for_ty.clone(),
                        type_params: Vec::new(),
                        public: impl_info.public,
                        span: stmt.span,
                    })?;
                }
                for info in &self.generic_trait_impls {
                    solver.add_impl(TraitImplHeader {
                        trait_name: info.decl.trait_name.clone(),
                        for_ty: info.for_ty.clone(),
                        type_params: info.decl.type_params.clone(),
                        public: info.decl.public,
                        span: stmt.span,
                    })?;
                }
                solver.add_impl(new_header)?;
                if !is_generic_impl && self.trait_impls.contains_key(&impl_key) {
                    return Err(RRException::new(
                        "RR.SemanticError",
                        RRCode::E1002,
                        Stage::Lower,
                        format!(
                            "duplicate impl of trait '{}' for '{}'",
                            decl.trait_name,
                            for_ty.key()
                        ),
                    )
                    .at(stmt.span));
                }
                if is_generic_impl
                    && self.generic_trait_impls.iter().any(|info| {
                        info.decl.trait_name == decl.trait_name && info.for_ty.key() == for_ty.key()
                    })
                {
                    return Err(RRException::new(
                        "RR.SemanticError",
                        RRCode::E1002,
                        Stage::Lower,
                        format!(
                            "duplicate generic impl of trait '{}' for '{}'",
                            decl.trait_name,
                            for_ty.key()
                        ),
                    )
                    .at(stmt.span));
                }

                let mut methods_by_name = FxHashMap::default();
                for method in &decl.methods {
                    if methods_by_name
                        .insert(method.name.clone(), method)
                        .is_some()
                    {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "duplicate impl method '{}' for trait '{}'",
                                method.name, decl.trait_name
                            ),
                        )
                        .at(method.body.span));
                    }
                }

                let mut assoc_types_by_name = FxHashMap::default();
                for assoc_ty in &decl.assoc_types {
                    if assoc_types_by_name
                        .insert(assoc_ty.name.clone(), Self::ast_type_ref(&assoc_ty.ty))
                        .is_some()
                    {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "duplicate associated type '{}' in impl of trait '{}'",
                                assoc_ty.name, decl.trait_name
                            ),
                        )
                        .at(assoc_ty.span));
                    }
                }
                for assoc_ty in &trait_info.decl.assoc_types {
                    if !Self::impl_assoc_type_satisfies_trait_decl(&assoc_types_by_name, assoc_ty) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "impl of trait '{}' for '{}' is missing associated type '{}'",
                                decl.trait_name,
                                for_ty.key(),
                                assoc_ty.name
                            ),
                        )
                        .at(stmt.span));
                    }
                }

                let mut assoc_consts_by_name = FxHashMap::default();
                for assoc_const in &decl.assoc_consts {
                    if assoc_consts_by_name
                        .insert(assoc_const.name.clone(), assoc_const)
                        .is_some()
                    {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "duplicate associated const '{}' in impl of trait '{}'",
                                assoc_const.name, decl.trait_name
                            ),
                        )
                        .at(assoc_const.span));
                    }
                }
                for assoc_const in &trait_info.decl.assoc_consts {
                    match assoc_consts_by_name.get(&assoc_const.name) {
                        Some(impl_const) => {
                            let trait_ty = Some(Self::ast_type_ref(&assoc_const.ty_hint));
                            let impl_ty = Some(Self::ast_type_ref(&impl_const.ty_hint));
                            if !Self::type_ref_matches_trait_sig(
                                &impl_ty,
                                &trait_ty,
                                &for_ty,
                                &assoc_types_by_name,
                            ) {
                                return Err(RRException::new(
                                    "RR.SemanticError",
                                    RRCode::E1002,
                                    Stage::Lower,
                                    format!(
                                        "impl associated const '{}.{}' type does not match trait signature",
                                        decl.trait_name, assoc_const.name
                                    ),
                                )
                                .at(impl_const.span));
                            }
                        }
                        None if assoc_const.default.is_some() => {}
                        None => {
                            return Err(RRException::new(
                                "RR.SemanticError",
                                RRCode::E1002,
                                Stage::Lower,
                                format!(
                                    "impl of trait '{}' for '{}' is missing associated const '{}'",
                                    decl.trait_name,
                                    for_ty.key(),
                                    assoc_const.name
                                ),
                            )
                            .at(stmt.span));
                        }
                    }
                }
                for impl_const in &decl.assoc_consts {
                    if !trait_info
                        .decl
                        .assoc_consts
                        .iter()
                        .any(|assoc_const| assoc_const.name == impl_const.name)
                    {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "impl associated const '{}' is not declared by trait '{}'",
                                impl_const.name, decl.trait_name
                            ),
                        )
                        .at(impl_const.span));
                    }
                }

                let mut method_symbols = FxHashMap::default();
                for trait_method in &trait_info.decl.methods {
                    let impl_method = methods_by_name.get(&trait_method.name);
                    if impl_method.is_none() && trait_method.default_body.is_none() {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "impl of trait '{}' for '{}' is missing method '{}'",
                                decl.trait_name,
                                for_ty.key(),
                                trait_method.name
                            ),
                        )
                        .at(stmt.span));
                    };
                    if let Some(impl_method) = impl_method {
                        self.validate_impl_method_signature(
                            &decl.trait_name,
                            &for_ty,
                            &assoc_types_by_name,
                            trait_method,
                            impl_method,
                        )?;
                    }
                    let mangled =
                        Self::trait_method_mangle(&decl.trait_name, &for_ty, &trait_method.name);
                    method_symbols.insert(trait_method.name.clone(), mangled);
                }

                let mut const_symbols = FxHashMap::default();
                for trait_const in &trait_info.decl.assoc_consts {
                    let mangled =
                        Self::trait_const_mangle(&decl.trait_name, &for_ty, &trait_const.name);
                    const_symbols.insert(trait_const.name.clone(), mangled);
                }

                for impl_method in &decl.methods {
                    if !trait_info
                        .decl
                        .methods
                        .iter()
                        .any(|method| method.name == impl_method.name)
                    {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "impl method '{}' is not declared by trait '{}'",
                                impl_method.name, decl.trait_name
                            ),
                        )
                        .at(impl_method.body.span));
                    }
                }

                if is_generic_impl {
                    self.generic_trait_impls.push(GenericTraitImplInfo {
                        decl: decl.clone(),
                        for_ty,
                    });
                } else {
                    self.trait_impls.insert(
                        impl_key,
                        TraitImplInfo {
                            trait_name: decl.trait_name.clone(),
                            for_ty,
                            assoc_types: assoc_types_by_name.clone(),
                            method_symbols,
                            const_symbols,
                            public: is_public_impl,
                        },
                    );
                }
            }
        }
        Ok(())
    }

    fn validate_impl_method_signature(
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

    fn flush_pending_fns(&mut self, items: &mut Vec<HirItem>) {
        if self.pending_fns.is_empty() {
            return;
        }
        for f in self.pending_fns.drain(..) {
            items.push(HirItem::Fn(f));
        }
    }

    fn lower_trait_decl(&mut self, decl: ast::TraitDecl, span: Span) -> RR<HirTrait> {
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

    fn lower_impl_decl(&mut self, decl: ast::ImplDecl, span: Span) -> RR<(HirImpl, Vec<HirFn>)> {
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
            let impl_fn = self.lower_fn(
                mangled,
                method.type_params,
                method.params,
                method.ret_ty_hint,
                method.where_bounds,
                method.body,
                span,
            )?;
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
            let impl_fn = self.lower_fn(
                mangled,
                method.type_params,
                method.params,
                method.ret_ty_hint,
                method.where_bounds,
                method.body,
                span,
            )?;
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
                let const_fn = self.lower_fn(
                    mangled,
                    Vec::new(),
                    Vec::new(),
                    Some(ret_ty_hint.clone()),
                    Vec::new(),
                    body,
                    item_span,
                )?;
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

    fn apply_implicit_tail_return(mut body: HirBlock) -> HirBlock {
        if body
            .stmts
            .iter()
            .any(|s| matches!(s, HirStmt::Return { .. }))
        {
            return body;
        }
        if let Some(last) = body.stmts.pop() {
            match last {
                HirStmt::Expr { expr, span } => {
                    body.stmts.push(HirStmt::Return {
                        value: Some(expr),
                        span,
                    });
                }
                other => body.stmts.push(other),
            }
        }
        body
    }

    fn dotted_name_from_expr(expr: &ast::Expr) -> Option<String> {
        match &expr.kind {
            ast::ExprKind::Name(n) => Some(n.clone()),
            ast::ExprKind::Field { base, name } => {
                let mut s = Self::dotted_name_from_expr(base)?;
                s.push('.');
                s.push_str(name);
                Some(s)
            }
            _ => None,
        }
    }

    fn dotted_name_from_field(base: &ast::Expr, field: &str) -> Option<String> {
        let mut s = Self::dotted_name_from_expr(base)?;
        s.push('.');
        s.push_str(field);
        Some(s)
    }

    fn root_is_unbound_for_dotted(&self, dotted: &str) -> bool {
        let root = dotted.split('.').next().unwrap_or(dotted);
        self.lookup(root).is_none()
    }

    fn lower_dotted_ref(&mut self, dotted: &str, span: Span) -> HirExpr {
        if let Some(lid) = self.lookup(dotted) {
            HirExpr::Local(lid)
        } else if let Some(sym) = self.global_fn_aliases.get(dotted).copied() {
            HirExpr::Global(sym, span)
        } else if let Some(sym) = self.r_import_aliases.get(dotted).copied() {
            HirExpr::Global(sym, span)
        } else if let Some((root, rest)) = dotted.split_once('.')
            && let Some(pkg) = self.r_namespace_aliases.get(root)
        {
            HirExpr::Global(self.intern_symbol(&format!("{}::{}", pkg, rest)), span)
        } else {
            HirExpr::Global(self.intern_symbol(dotted), span)
        }
    }

    fn resolve_or_declare_local_for_assign(&mut self, name: &str, span: Span) -> RR<LocalId> {
        if let Some(lid) = self.lookup(name) {
            return Ok(lid);
        }
        if self.strict_let {
            let mut err = RRException::new(
                "RR.SemanticError",
                RRCode::E1001,
                Stage::Lower,
                format!("assignment to undeclared variable '{}'", name),
            )
            .at(span)
            .note("Declare it first with `let` before assignment.");
            if let Some(suggestion) = did_you_mean(
                name,
                self.scopes.iter().flat_map(|scope| scope.keys().cloned()),
            ) {
                err = err.help(suggestion);
            }
            return Err(err);
        }
        let lid = self.declare_local(name);
        if self.warn_implicit_decl {
            let where_msg = if span.start_line > 0 {
                format!("{}:{}", span.start_line, span.start_col)
            } else {
                "unknown".to_string()
            };
            self.warnings.push(format!(
                "implicit declaration via assignment: '{}' at {} (treated as `let {} = ...;`). Use an explicit lowering policy to forbid or allow this legacy behavior.",
                name, where_msg, name
            ));
        }
        Ok(lid)
    }

    fn collect_lambda_captures(
        &self,
        params: &[ast::FnParam],
        body: &ast::Block,
    ) -> Vec<(String, LocalId)> {
        fn in_scopes(scopes: &[FxHashSet<String>], name: &str) -> bool {
            scopes.iter().rev().any(|s| s.contains(name))
        }

        fn record_capture(
            lowerer: &Lowerer,
            scopes: &[FxHashSet<String>],
            seen: &mut FxHashSet<String>,
            captures: &mut Vec<(String, LocalId)>,
            name: &str,
        ) {
            if in_scopes(scopes, name) {
                return;
            }
            if let Some(lid) = lowerer.lookup(name)
                && seen.insert(name.to_string())
            {
                captures.push((name.to_string(), lid));
            }
        }

        fn collect_pat_binders(p: &ast::Pattern, out: &mut FxHashSet<String>) {
            match &p.kind {
                ast::PatternKind::Bind(n) => {
                    out.insert(n.clone());
                }
                ast::PatternKind::List { items, rest } => {
                    for it in items {
                        collect_pat_binders(it, out);
                    }
                    if let Some(r) = rest {
                        out.insert(r.clone());
                    }
                }
                ast::PatternKind::Record { fields } => {
                    for (_, fp) in fields {
                        collect_pat_binders(fp, out);
                    }
                }
                ast::PatternKind::Wild | ast::PatternKind::Lit(_) => {}
            }
        }

        fn visit_expr(
            lowerer: &Lowerer,
            scopes: &mut Vec<FxHashSet<String>>,
            seen: &mut FxHashSet<String>,
            captures: &mut Vec<(String, LocalId)>,
            expr: &ast::Expr,
        ) {
            match &expr.kind {
                ast::ExprKind::Name(n) => record_capture(lowerer, scopes, seen, captures, n),
                ast::ExprKind::Unary { rhs, .. } => {
                    visit_expr(lowerer, scopes, seen, captures, rhs)
                }
                ast::ExprKind::Formula { lhs, rhs } => {
                    if let Some(lhs) = lhs {
                        visit_expr(lowerer, scopes, seen, captures, lhs);
                    }
                    visit_expr(lowerer, scopes, seen, captures, rhs);
                }
                ast::ExprKind::Binary { lhs, rhs, .. } => {
                    visit_expr(lowerer, scopes, seen, captures, lhs);
                    visit_expr(lowerer, scopes, seen, captures, rhs);
                }
                ast::ExprKind::Range { a, b } => {
                    visit_expr(lowerer, scopes, seen, captures, a);
                    visit_expr(lowerer, scopes, seen, captures, b);
                }
                ast::ExprKind::Call {
                    callee,
                    type_args: _,
                    args,
                } => {
                    visit_expr(lowerer, scopes, seen, captures, callee);
                    for a in args {
                        visit_expr(lowerer, scopes, seen, captures, a);
                    }
                }
                ast::ExprKind::NamedArg { value, .. } => {
                    visit_expr(lowerer, scopes, seen, captures, value)
                }
                ast::ExprKind::Index { base, idx } => {
                    visit_expr(lowerer, scopes, seen, captures, base);
                    for i in idx {
                        visit_expr(lowerer, scopes, seen, captures, i);
                    }
                }
                ast::ExprKind::Field { base, .. } => {
                    visit_expr(lowerer, scopes, seen, captures, base)
                }
                ast::ExprKind::VectorLit(xs) => {
                    for x in xs {
                        visit_expr(lowerer, scopes, seen, captures, x);
                    }
                }
                ast::ExprKind::RecordLit(fields) => {
                    for (_, v) in fields {
                        visit_expr(lowerer, scopes, seen, captures, v);
                    }
                }
                ast::ExprKind::Pipe { lhs, rhs_call } => {
                    visit_expr(lowerer, scopes, seen, captures, lhs);
                    visit_expr(lowerer, scopes, seen, captures, rhs_call);
                }
                ast::ExprKind::Try { expr } => visit_expr(lowerer, scopes, seen, captures, expr),
                ast::ExprKind::Unquote(e) => visit_expr(lowerer, scopes, seen, captures, e),
                ast::ExprKind::Match { scrutinee, arms } => {
                    visit_expr(lowerer, scopes, seen, captures, scrutinee);
                    for arm in arms {
                        let mut arm_scope = FxHashSet::default();
                        collect_pat_binders(&arm.pat, &mut arm_scope);
                        scopes.push(arm_scope);
                        if let Some(g) = &arm.guard {
                            visit_expr(lowerer, scopes, seen, captures, g);
                        }
                        visit_expr(lowerer, scopes, seen, captures, &arm.body);
                        scopes.pop();
                    }
                }
                ast::ExprKind::Lambda {
                    params,
                    ret_ty_hint: _,
                    body,
                } => {
                    let mut lambda_scope = FxHashSet::default();
                    for p in params {
                        lambda_scope.insert(p.name.clone());
                    }
                    scopes.push(lambda_scope);
                    visit_block(lowerer, scopes, seen, captures, body);
                    scopes.pop();
                }
                ast::ExprKind::Column(_) | ast::ExprKind::ColRef(_) | ast::ExprKind::Lit(_) => {}
            }
        }

        fn visit_stmt(
            lowerer: &Lowerer,
            scopes: &mut Vec<FxHashSet<String>>,
            seen: &mut FxHashSet<String>,
            captures: &mut Vec<(String, LocalId)>,
            stmt: &ast::Stmt,
        ) {
            match &stmt.kind {
                ast::StmtKind::Let {
                    name,
                    ty_hint: _,
                    init,
                } => {
                    if let Some(e) = init {
                        visit_expr(lowerer, scopes, seen, captures, e);
                    }
                    if let Some(scope) = scopes.last_mut() {
                        scope.insert(name.clone());
                    }
                }
                ast::StmtKind::Assign { target, value } => {
                    visit_expr(lowerer, scopes, seen, captures, value);
                    match &target.kind {
                        ast::LValueKind::Name(n) => {
                            if !in_scopes(scopes, n)
                                && lowerer.lookup(n).is_none()
                                && let Some(scope) = scopes.last_mut()
                            {
                                scope.insert(n.clone());
                            }
                        }
                        ast::LValueKind::Index { base, idx } => {
                            visit_expr(lowerer, scopes, seen, captures, base);
                            for i in idx {
                                visit_expr(lowerer, scopes, seen, captures, i);
                            }
                        }
                        ast::LValueKind::Field { base, .. } => {
                            visit_expr(lowerer, scopes, seen, captures, base);
                        }
                    }
                }
                ast::StmtKind::If {
                    cond,
                    then_blk,
                    else_blk,
                } => {
                    visit_expr(lowerer, scopes, seen, captures, cond);
                    scopes.push(FxHashSet::default());
                    visit_block(lowerer, scopes, seen, captures, then_blk);
                    scopes.pop();
                    if let Some(eb) = else_blk {
                        scopes.push(FxHashSet::default());
                        visit_block(lowerer, scopes, seen, captures, eb);
                        scopes.pop();
                    }
                }
                ast::StmtKind::While { cond, body } => {
                    visit_expr(lowerer, scopes, seen, captures, cond);
                    scopes.push(FxHashSet::default());
                    visit_block(lowerer, scopes, seen, captures, body);
                    scopes.pop();
                }
                ast::StmtKind::For { var, iter, body } => {
                    visit_expr(lowerer, scopes, seen, captures, iter);
                    let mut loop_scope = FxHashSet::default();
                    loop_scope.insert(var.clone());
                    scopes.push(loop_scope);
                    visit_block(lowerer, scopes, seen, captures, body);
                    scopes.pop();
                }
                ast::StmtKind::Return { value } => {
                    if let Some(v) = value {
                        visit_expr(lowerer, scopes, seen, captures, v);
                    }
                }
                ast::StmtKind::ExprStmt { expr } | ast::StmtKind::Expr(expr) => {
                    visit_expr(lowerer, scopes, seen, captures, expr);
                }
                ast::StmtKind::FnDecl { .. }
                | ast::StmtKind::TraitDecl(_)
                | ast::StmtKind::ImplDecl(_)
                | ast::StmtKind::Import { .. }
                | ast::StmtKind::Export(_)
                | ast::StmtKind::Break
                | ast::StmtKind::Next => {}
            }
        }

        fn visit_block(
            lowerer: &Lowerer,
            scopes: &mut Vec<FxHashSet<String>>,
            seen: &mut FxHashSet<String>,
            captures: &mut Vec<(String, LocalId)>,
            block: &ast::Block,
        ) {
            for s in &block.stmts {
                visit_stmt(lowerer, scopes, seen, captures, s);
            }
        }

        let mut scopes: Vec<FxHashSet<String>> =
            vec![params.iter().map(|p| p.name.clone()).collect()];
        let mut seen = FxHashSet::default();
        let mut captures = Vec::new();
        visit_block(self, &mut scopes, &mut seen, &mut captures, body);
        captures
    }

    fn infer_param_type_hint(default: &ast::Expr) -> Option<Ty> {
        match &default.kind {
            ast::ExprKind::Lit(ast::Lit::Int(_)) => Some(Ty::Int),
            ast::ExprKind::Lit(ast::Lit::Float(_)) => Some(Ty::Double),
            ast::ExprKind::Lit(ast::Lit::Bool(_)) => Some(Ty::Logical),
            ast::ExprKind::Lit(ast::Lit::Str(_)) => Some(Ty::Char),
            ast::ExprKind::Lit(ast::Lit::Null) => Some(Ty::Null),
            _ => None,
        }
    }

    fn parse_type_hint_expr(expr: &ast::TypeExpr) -> Option<Ty> {
        match expr {
            ast::TypeExpr::Named(name) => match name.to_ascii_lowercase().as_str() {
                "any" => Some(Ty::Any),
                "null" => Some(Ty::Null),
                "bool" | "boolean" | "logical" => Some(Ty::Logical),
                "int" | "integer" | "i32" | "i64" | "isize" => Some(Ty::Int),
                "float" | "double" | "numeric" | "f32" | "f64" => Some(Ty::Double),
                "str" | "string" | "char" | "character" => Some(Ty::Char),
                _ => None,
            },
            ast::TypeExpr::Generic { base, args } => {
                let base = base.to_ascii_lowercase();
                if base == "vector" && args.len() == 1 {
                    return Some(Ty::Vector(Box::new(
                        Self::parse_type_hint_expr(&args[0]).unwrap_or(Ty::Any),
                    )));
                }
                if base == "matrix" && args.len() == 1 {
                    return Some(Ty::Matrix(Box::new(
                        Self::parse_type_hint_expr(&args[0]).unwrap_or(Ty::Any),
                    )));
                }
                if base == "option" && args.len() == 1 {
                    return Some(Ty::Option(Box::new(
                        Self::parse_type_hint_expr(&args[0]).unwrap_or(Ty::Any),
                    )));
                }
                if base == "list" && args.len() == 1 {
                    return Some(Ty::List(Box::new(
                        Self::parse_type_hint_expr(&args[0]).unwrap_or(Ty::Any),
                    )));
                }
                if base == "box" && args.len() == 1 {
                    return Self::parse_type_hint_expr(&args[0])
                        .map(|inner| Ty::Box(Box::new(inner)));
                }
                None
            }
        }
    }

    fn lower_lambda_expr(
        &mut self,
        params: Vec<ast::FnParam>,
        ret_ty_hint: Option<ast::TypeExpr>,
        body: ast::Block,
        span: Span,
    ) -> RR<HirExpr> {
        let captures = self.collect_lambda_captures(&params, &body);
        let lambda_name = self.alloc_lambda_name();
        let lambda_sym = self.intern_symbol(&lambda_name);
        let fn_id = self.alloc_fn_id();

        let saved_scopes = std::mem::replace(&mut self.scopes, vec![FxHashMap::default()]);
        let saved_local_names = std::mem::take(&mut self.local_names);
        let saved_local_emitted_names = std::mem::take(&mut self.local_emitted_names);
        let saved_local_trait_types = std::mem::take(&mut self.local_trait_types);
        let saved_type_params = std::mem::take(&mut self.current_type_params);
        let saved_where_bounds = std::mem::take(&mut self.current_where_bounds);
        let saved_next_local = self.next_local_id;
        self.next_local_id = 0;

        let mut hir_params: Vec<HirParam> = Vec::new();
        for (cap_name, _) in &captures {
            let _cap_local = self.declare_local(cap_name);
            let cap_sym = self.intern_symbol(cap_name);
            hir_params.push(HirParam {
                name: cap_sym,
                ty: None,
                default: None,
                span,
            });
        }
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
        for (p, psym) in params.into_iter().zip(param_syms.into_iter()) {
            let ty_hint = p
                .ty_hint
                .as_ref()
                .and_then(Self::parse_type_hint_expr)
                .or_else(|| p.default.as_ref().and_then(Self::infer_param_type_hint));
            let default = if let Some(d) = p.default {
                Some(self.lower_expr(d)?)
            } else {
                None
            };
            hir_params.push(HirParam {
                name: psym,
                ty: ty_hint,
                default,
                span: p.span,
            });
        }
        let hir_body = Self::apply_implicit_tail_return(self.lower_block(body)?);
        let lambda_local_names = std::mem::take(&mut self.local_names);

        self.scopes = saved_scopes;
        self.local_names = saved_local_names;
        self.local_emitted_names = saved_local_emitted_names;
        self.local_trait_types = saved_local_trait_types;
        self.current_type_params = saved_type_params;
        self.current_where_bounds = saved_where_bounds;
        self.next_local_id = saved_next_local;

        self.pending_fns.push(HirFn {
            id: fn_id,
            name: lambda_sym,
            type_params: Vec::new(),
            where_bounds: Vec::new(),
            params: hir_params,
            has_varargs: false,
            ret_ty: ret_ty_hint.as_ref().and_then(Self::parse_type_hint_expr),
            body: hir_body,
            attrs: HirFnAttrs {
                inline_hint: InlineHint::Default,
                tidy_safe: false,
            },
            span,
            local_names: lambda_local_names,
            public: false,
        });

        if captures.is_empty() {
            return Ok(HirExpr::Global(lambda_sym, span));
        }

        let make_sym = self.intern_symbol("rr_closure_make");
        let mut args = Vec::new();
        args.push(HirArg::Pos(HirExpr::Global(lambda_sym, span)));
        for (_, lid) in captures {
            args.push(HirArg::Pos(HirExpr::Local(lid)));
        }
        Ok(HirExpr::Call(HirCall {
            callee: Box::new(HirExpr::Global(make_sym, span)),
            args,
            span,
        }))
    }

    fn formula_term_text(expr: &ast::Expr) -> Option<String> {
        match &expr.kind {
            ast::ExprKind::Name(name) => Some(name.clone()),
            ast::ExprKind::Column(name) => Some(name.clone()),
            ast::ExprKind::Field { base, name } => {
                Self::formula_term_text(base).map(|prefix| format!("{prefix}.{name}"))
            }
            ast::ExprKind::Binary { op, lhs, rhs } => {
                let lhs = Self::formula_term_text(lhs)?;
                let rhs = Self::formula_term_text(rhs)?;
                let op_str = match op {
                    ast::BinOp::Add => "+",
                    ast::BinOp::Sub => "-",
                    ast::BinOp::Mul => "*",
                    ast::BinOp::Div => "/",
                    _ => return None,
                };
                Some(format!("{lhs} {op_str} {rhs}"))
            }
            ast::ExprKind::Lit(ast::Lit::Str(s)) => Some(s.clone()),
            _ => None,
        }
    }

    fn lower_formula_expr(
        &mut self,
        lhs: Option<ast::Expr>,
        rhs: ast::Expr,
        span: Span,
    ) -> RR<HirExpr> {
        let formula_text = if let Some(lhs) = lhs {
            let Some(lhs_text) = Self::formula_term_text(&lhs) else {
                return Err(Self::lower_formula_error(span));
            };
            let Some(rhs_text) = Self::formula_term_text(&rhs) else {
                return Err(Self::lower_formula_error(span));
            };
            format!("{lhs_text} ~ {rhs_text}")
        } else {
            let Some(rhs_text) = Self::formula_term_text(&rhs) else {
                return Err(Self::lower_formula_error(span));
            };
            format!("~{rhs_text}")
        };
        let callee = HirExpr::Global(self.intern_symbol("stats::as.formula"), span);
        Ok(HirExpr::Call(HirCall {
            callee: Box::new(callee),
            args: vec![HirArg::Pos(HirExpr::Lit(HirLit::Char(formula_text)))],
            span,
        }))
    }

    fn lower_formula_unary_expr(&mut self, rhs: ast::Expr, span: Span) -> RR<HirExpr> {
        self.lower_formula_expr(None, rhs, span)
    }

    fn lower_formula_binary_expr(
        &mut self,
        lhs: ast::Expr,
        rhs: ast::Expr,
        span: Span,
    ) -> RR<HirExpr> {
        self.lower_formula_expr(Some(lhs), rhs, span)
    }

    fn lower_formula_error(span: Span) -> RRException {
        RRException::new(
            "RR.TypeError",
            crate::error::RRCode::E1002,
            crate::error::Stage::Lower,
            "formula shorthand currently supports names, columns, dotted field paths, string literals, and simple infix formulas over those terms",
        )
        .at(span)
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
                    let fn_item = self.lower_fn(
                        name,
                        type_params,
                        params,
                        ret_ty_hint,
                        where_bounds,
                        body,
                        stmt.span,
                    )?;
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
                    let mut fn_item = self.lower_fn(
                        fndecl.name,
                        fndecl.type_params,
                        fndecl.params,
                        fndecl.ret_ty_hint,
                        fndecl.where_bounds,
                        fndecl.body,
                        stmt.span,
                    )?;
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

    fn lower_fn(
        &mut self,
        name: String,
        type_params: Vec<String>,
        params: Vec<ast::FnParam>,
        ret_ty_hint: Option<ast::TypeExpr>,
        where_bounds: Vec<ast::TraitBound>,
        body: ast::Block,
        span: Span,
    ) -> RR<HirFn> {
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
        for (p, psym) in params.into_iter().zip(param_syms.into_iter()) {
            let ty_hint = p
                .ty_hint
                .as_ref()
                .and_then(Self::parse_type_hint_expr)
                .or_else(|| p.default.as_ref().and_then(Self::infer_param_type_hint));
            let default = if let Some(d) = p.default {
                Some(self.lower_expr(d)?)
            } else {
                None
            };
            hir_params.push(HirParam {
                name: psym,
                ty: ty_hint,
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

    fn lower_block(&mut self, block: ast::Block) -> RR<HirBlock> {
        let mut stmts = Vec::new();
        for s in block.stmts {
            stmts.push(self.lower_stmt(s)?);
        }
        Ok(HirBlock {
            stmts,
            span: block.span,
        })
    }

    fn lower_stmt(&mut self, stmt: ast::Stmt) -> RR<HirStmt> {
        match stmt.kind {
            ast::StmtKind::Let {
                name,
                ty_hint,
                init,
            } => {
                let dyn_trait = ty_hint.as_ref().and_then(Self::dyn_trait_name);
                let init_trait_ty = init
                    .as_ref()
                    .and_then(|expr| self.trait_type_of_ast_expr(expr));
                let expected_ty = if dyn_trait.is_some() {
                    init_trait_ty.clone()
                } else {
                    ty_hint.as_ref().map(Self::ast_type_ref)
                };
                let val = if let Some(e) = init {
                    Some(self.lower_expr_with_expected(e, expected_ty.as_ref())?)
                } else {
                    None
                };
                let lid = self.declare_local(&name);
                if let Some(trait_name) = dyn_trait {
                    if let Some(concrete_ty) = init_trait_ty.clone() {
                        if !self.ensure_trait_impl_for_type(trait_name, &concrete_ty, stmt.span)? {
                            return Err(RRException::new(
                                "RR.SemanticError",
                                RRCode::E1002,
                                Stage::Lower,
                                format!(
                                    "dyn trait binding requires '{}' to implement '{}'",
                                    concrete_ty.key(),
                                    trait_name
                                ),
                            )
                            .at(stmt.span));
                        }
                        self.local_trait_types.insert(lid, concrete_ty);
                    } else if let Some(ty_hint) = &ty_hint {
                        self.local_trait_types
                            .insert(lid, Self::ast_type_ref(ty_hint));
                    }
                } else if let Some(ty_hint) = &ty_hint {
                    self.local_trait_types
                        .insert(lid, Self::ast_type_ref(ty_hint));
                } else if let Some(inferred_ty) = init_trait_ty {
                    self.local_trait_types.insert(lid, inferred_ty);
                }
                let sym = self.intern_symbol(&name);
                if self.scopes.len() == 1
                    && let Some(HirExpr::Global(global_sym, _)) = &val
                    && self
                        .symbols
                        .get(global_sym)
                        .map(|s| s.starts_with("__lambda_"))
                        .unwrap_or(false)
                {
                    self.global_fn_aliases.insert(name.clone(), *global_sym);
                }
                Ok(HirStmt::Let {
                    local: lid,
                    name: sym,
                    ty: ty_hint.as_ref().and_then(Self::parse_type_hint_expr),
                    init: val,
                    span: stmt.span,
                })
            }
            ast::StmtKind::Assign { target, value } => {
                let lhs = self.lower_lvalue(target)?;
                let rhs = self.lower_expr(value)?;
                if self.scopes.len() == 1
                    && let HirLValue::Local(_lid) = &lhs
                    && let HirExpr::Global(global_sym, _) = &rhs
                    && self
                        .symbols
                        .get(global_sym)
                        .map(|s| s.starts_with("__lambda_"))
                        .unwrap_or(false)
                    && let Some(name) = self.local_name_of_lvalue(&lhs)
                {
                    self.global_fn_aliases.insert(name, *global_sym);
                }
                Ok(HirStmt::Assign {
                    target: lhs,
                    value: rhs,
                    span: stmt.span,
                })
            }
            ast::StmtKind::If {
                cond,
                then_blk,
                else_blk,
            } => {
                let c = self.lower_expr(cond)?;
                self.enter_scope();
                let t = self.lower_block(then_blk)?;
                self.exit_scope();
                let e = if let Some(blk) = else_blk {
                    self.enter_scope();
                    let lowered = self.lower_block(blk)?;
                    self.exit_scope();
                    Some(lowered)
                } else {
                    None
                };
                Ok(HirStmt::If {
                    cond: c,
                    then_blk: t,
                    else_blk: e,
                    span: stmt.span,
                })
            }
            ast::StmtKind::While { cond, body } => {
                let c = self.lower_expr(cond)?;
                self.enter_scope();
                let b = self.lower_block(body)?;
                self.exit_scope();
                Ok(HirStmt::While {
                    cond: c,
                    body: b,
                    span: stmt.span,
                })
            }
            ast::StmtKind::For { var, iter, body } => {
                let iter_expr = self.lower_expr(iter)?;
                self.enter_scope();
                let lid = self.declare_local(&var);

                // Canonicalize known iterator forms for better downstream optimization.
                let iter_kind = match iter_expr {
                    HirExpr::Range { start, end } => HirForIter::Range {
                        var: lid,
                        start: *start,
                        end: *end,
                        inclusive: true,
                    },
                    HirExpr::Call(call) => {
                        let one_arg = call.args.len() == 1;
                        match (&*call.callee, one_arg) {
                            (HirExpr::Global(sym, _), true) => {
                                let name = self.symbols.get(sym).cloned().unwrap_or_default();
                                let arg_expr = match call.args[0].clone() {
                                    HirArg::Pos(e) => e,
                                    HirArg::Named { value, .. } => value,
                                };
                                if name == "seq_len" {
                                    HirForIter::SeqLen {
                                        var: lid,
                                        len: arg_expr,
                                    }
                                } else if name == "seq_along" {
                                    HirForIter::SeqAlong {
                                        var: lid,
                                        xs: arg_expr,
                                    }
                                } else {
                                    HirForIter::SeqAlong {
                                        var: lid,
                                        xs: HirExpr::Call(call),
                                    }
                                }
                            }
                            _ => HirForIter::SeqAlong {
                                var: lid,
                                xs: HirExpr::Call(call),
                            },
                        }
                    }
                    other => HirForIter::SeqAlong {
                        var: lid,
                        xs: other,
                    },
                };

                let body_hir = self.lower_block(body)?;
                self.exit_scope();
                Ok(HirStmt::For {
                    iter: iter_kind,
                    body: body_hir,
                    span: stmt.span,
                })
            }
            ast::StmtKind::Return { value } => {
                let v = if let Some(e) = value {
                    Some(self.lower_expr(e)?)
                } else {
                    None
                };
                Ok(HirStmt::Return {
                    value: v,
                    span: stmt.span,
                })
            }
            ast::StmtKind::Break => Ok(HirStmt::Break { span: stmt.span }),
            ast::StmtKind::Next => Ok(HirStmt::Next { span: stmt.span }),
            ast::StmtKind::ExprStmt { expr } => Ok(HirStmt::Expr {
                expr: self.lower_expr(expr)?,
                span: stmt.span,
            }),
            _ => Err(RRException::new(
                "Feature.NotImpl",
                RRCode::E3001,
                Stage::Lower,
                "Stmt kind not supported".to_string(),
            )),
        }
    }

    fn trait_method_callee(callee: &ast::Expr) -> Option<(String, String)> {
        let ast::ExprKind::Field { base, name } = &callee.kind else {
            return None;
        };
        let ast::ExprKind::Name(trait_name) = &base.kind else {
            return None;
        };
        Some((trait_name.clone(), name.clone()))
    }

    fn trait_receiver_expr(args: &[ast::Expr]) -> Option<&ast::Expr> {
        let first = args.first()?;
        match &first.kind {
            ast::ExprKind::NamedArg { name, value } if name == "self" => Some(value),
            _ => Some(first),
        }
    }

    fn trait_type_of_ast_expr(&self, expr: &ast::Expr) -> Option<HirTypeRef> {
        match &expr.kind {
            ast::ExprKind::Name(name) => {
                let local = self.lookup(name)?;
                self.local_trait_types.get(&local).cloned()
            }
            ast::ExprKind::Lit(ast::Lit::Int(_)) => Some(HirTypeRef::Named("int".to_string())),
            ast::ExprKind::Lit(ast::Lit::Float(_)) => Some(HirTypeRef::Named("float".to_string())),
            ast::ExprKind::Lit(ast::Lit::Bool(_)) => Some(HirTypeRef::Named("bool".to_string())),
            ast::ExprKind::Lit(ast::Lit::Str(_)) => Some(HirTypeRef::Named("str".to_string())),
            ast::ExprKind::Lit(ast::Lit::Null) => Some(HirTypeRef::Named("null".to_string())),
            ast::ExprKind::Binary { op, lhs, .. } => {
                let (trait_name, method_name) = Self::operator_trait_for_binop(*op)?;
                let lhs_ty = self.trait_type_of_ast_expr(lhs)?;
                self.trait_method_return_type_for_receiver(&lhs_ty, trait_name, method_name)
            }
            ast::ExprKind::Unary { op, rhs } => {
                let (trait_name, method_name) = Self::operator_trait_for_unop(*op)?;
                let rhs_ty = self.trait_type_of_ast_expr(rhs)?;
                self.trait_method_return_type_for_receiver(&rhs_ty, trait_name, method_name)
            }
            ast::ExprKind::Call {
                callee,
                type_args,
                args,
            } => self.trait_type_of_call_expr(callee, type_args, args),
            _ => None,
        }
    }

    fn trait_type_of_call_expr(
        &self,
        callee: &ast::Expr,
        type_args: &[ast::TypeExpr],
        args: &[ast::Expr],
    ) -> Option<HirTypeRef> {
        if let ast::ExprKind::Name(name) = &callee.kind {
            let info = self.generic_fns.get(name)?;
            let subst = self
                .infer_generic_subst(&info.decl, type_args, args, None, callee.span)
                .ok()?;
            let ret_ty = info.decl.ret_ty_hint.clone()?;
            let ret_ty = Self::substitute_type_expr(ret_ty, &subst);
            return Some(Self::ast_type_ref(&ret_ty));
        }

        if let Some((trait_name, method_name)) = Self::trait_method_callee(callee)
            && self.trait_defs.contains_key(&trait_name)
            && let Some(receiver) = Self::trait_receiver_expr(args)
        {
            let receiver_ty = self.trait_type_of_ast_expr(receiver)?;
            return self.trait_method_return_type_for_receiver(
                &receiver_ty,
                &trait_name,
                &method_name,
            );
        }

        if let ast::ExprKind::Field { base, name } = &callee.kind {
            let receiver_ty = self.trait_type_of_ast_expr(base)?;
            return self.trait_method_return_type_for_receiver(&receiver_ty, "", name);
        }

        None
    }

    fn trait_method_return_type_for_receiver(
        &self,
        receiver_ty: &HirTypeRef,
        explicit_trait_name: &str,
        method_name: &str,
    ) -> Option<HirTypeRef> {
        if !explicit_trait_name.is_empty() {
            return self.trait_method_return_type(explicit_trait_name, method_name, receiver_ty);
        }

        if let Some(type_key) = self.current_generic_ref_key(receiver_ty) {
            let candidates = self.type_param_method_bound_candidates(&type_key, method_name);
            return match candidates.as_slice() {
                [trait_name] => self.trait_method_return_type(trait_name, method_name, receiver_ty),
                _ => None,
            };
        }

        if self.type_ref_contains_current_type_param(receiver_ty) {
            return None;
        }

        let receiver_key = receiver_ty.key();
        let mut candidates = self
            .trait_impls
            .iter()
            .filter_map(|((trait_name, for_ty), impl_info)| {
                (for_ty == &receiver_key && impl_info.method_symbols.contains_key(method_name))
                    .then_some(trait_name.clone())
            })
            .collect::<Vec<_>>();
        candidates.sort();
        candidates.dedup();
        match candidates.as_slice() {
            [trait_name] => self.trait_method_return_type(trait_name, method_name, receiver_ty),
            _ => None,
        }
    }

    fn trait_method_return_type(
        &self,
        trait_name: &str,
        method_name: &str,
        receiver_ty: &HirTypeRef,
    ) -> Option<HirTypeRef> {
        let ret_ty = self.trait_method_return_type_expr(trait_name, method_name)?;
        Some(Self::substitute_self_type_ref(
            Self::ast_type_ref(&ret_ty),
            receiver_ty,
        ))
    }

    fn trait_method_return_type_expr(
        &self,
        trait_name: &str,
        method_name: &str,
    ) -> Option<ast::TypeExpr> {
        let mut stack = vec![trait_name.to_string()];
        let mut seen = FxHashSet::default();
        while let Some(name) = stack.pop() {
            if !seen.insert(name.clone()) {
                continue;
            }
            let Some(info) = self.trait_defs.get(&name) else {
                continue;
            };
            if let Some(method) = info
                .decl
                .methods
                .iter()
                .find(|method| method.name == method_name)
            {
                return method.ret_ty_hint.clone();
            }
            stack.extend(info.decl.supertraits.iter().cloned());
        }
        None
    }

    fn substitute_self_type_ref(ty: HirTypeRef, receiver_ty: &HirTypeRef) -> HirTypeRef {
        match ty {
            HirTypeRef::Named(name) if name == "Self" => receiver_ty.clone(),
            HirTypeRef::Named(name) => HirTypeRef::Named(name),
            HirTypeRef::Generic { base, args } => HirTypeRef::Generic {
                base,
                args: args
                    .into_iter()
                    .map(|arg| Self::substitute_self_type_ref(arg, receiver_ty))
                    .collect(),
            },
        }
    }

    fn lower_call_args(&mut self, args: Vec<ast::Expr>) -> RR<Vec<HirArg>> {
        let mut hargs = Vec::with_capacity(args.len());
        for arg in args {
            match arg.kind {
                ast::ExprKind::NamedArg { name, value } => {
                    let sym = self.intern_symbol(&name);
                    hargs.push(HirArg::Named {
                        name: sym,
                        value: self.lower_expr(*value)?,
                    });
                }
                _ => hargs.push(HirArg::Pos(self.lower_expr(arg)?)),
            }
        }
        Ok(hargs)
    }

    fn hir_binop(op: ast::BinOp) -> HirBinOp {
        match op {
            ast::BinOp::Add => HirBinOp::Add,
            ast::BinOp::Sub => HirBinOp::Sub,
            ast::BinOp::Mul => HirBinOp::Mul,
            ast::BinOp::Div => HirBinOp::Div,
            ast::BinOp::Mod => HirBinOp::Mod,
            ast::BinOp::MatMul => HirBinOp::MatMul,
            ast::BinOp::Eq => HirBinOp::Eq,
            ast::BinOp::Ne => HirBinOp::Ne,
            ast::BinOp::Lt => HirBinOp::Lt,
            ast::BinOp::Le => HirBinOp::Le,
            ast::BinOp::Gt => HirBinOp::Gt,
            ast::BinOp::Ge => HirBinOp::Ge,
            ast::BinOp::And => HirBinOp::And,
            ast::BinOp::Or => HirBinOp::Or,
        }
    }

    fn operator_trait_for_binop(op: ast::BinOp) -> Option<(&'static str, &'static str)> {
        match op {
            ast::BinOp::Add => Some(("Add", "add")),
            ast::BinOp::Sub => Some(("Sub", "sub")),
            ast::BinOp::Mul => Some(("Mul", "mul")),
            ast::BinOp::Div => Some(("Div", "div")),
            ast::BinOp::Mod => Some(("Mod", "mod")),
            ast::BinOp::MatMul => Some(("MatMul", "matmul")),
            ast::BinOp::Eq
            | ast::BinOp::Ne
            | ast::BinOp::Lt
            | ast::BinOp::Le
            | ast::BinOp::Gt
            | ast::BinOp::Ge
            | ast::BinOp::And
            | ast::BinOp::Or => None,
        }
    }

    fn operator_trait_for_unop(op: ast::UnaryOp) -> Option<(&'static str, &'static str)> {
        match op {
            ast::UnaryOp::Neg => Some(("Neg", "neg")),
            ast::UnaryOp::Not | ast::UnaryOp::Formula => None,
        }
    }

    fn infer_generic_type_from_hir_pattern(
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

    fn validate_trait_bounds_for_subst(
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

    fn ensure_trait_impl_for_type(
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

    fn ensure_supertrait_impls_for_type(
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

    fn trait_assoc_type_owners(&self, trait_name: &str) -> Vec<(String, String)> {
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

    fn associated_type_for_impl(
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

    fn subst_with_assoc_type_projections(
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

    fn instantiate_generic_trait_impl(
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
            let mangled =
                Self::trait_method_mangle(&info.decl.trait_name, &concrete_ty, &trait_method.name);
            method_symbols.insert(trait_method.name.clone(), mangled);
        }
        let mut const_symbols = FxHashMap::default();
        for trait_const in &trait_info.decl.assoc_consts {
            let mangled =
                Self::trait_const_mangle(&info.decl.trait_name, &concrete_ty, &trait_const.name);
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
            let inst_fn = self.lower_fn(
                mangled,
                Vec::new(),
                inst_params,
                inst_ret,
                Vec::new(),
                inst_body,
                span,
            )?;
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
            let inst_fn = self.lower_fn(
                mangled,
                Vec::new(),
                Vec::new(),
                Some(ret_ty_hint),
                Vec::new(),
                body,
                item_span,
            )?;
            self.pending_fns.push(inst_fn);
        }
        Ok(())
    }

    fn trait_impl_method_for_type(
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

    fn resolve_trait_assoc_const_call(
        &mut self,
        trait_name: &str,
        const_name: &str,
        type_args: &[ast::TypeExpr],
        args: &[ast::Expr],
        span: Span,
    ) -> RR<TraitAssocConstResolution> {
        let Some(trait_info) = self.trait_defs.get(trait_name) else {
            return Ok(TraitAssocConstResolution::NotAssocConst);
        };
        if !trait_info
            .decl
            .assoc_consts
            .iter()
            .any(|assoc_const| assoc_const.name == const_name)
        {
            return Ok(TraitAssocConstResolution::NotAssocConst);
        }
        if !args.is_empty() {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "associated const '{}.{}' does not accept call arguments",
                    trait_name, const_name
                ),
            )
            .at(span));
        }
        if type_args.len() != 1 {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "associated const '{}.{}' requires exactly one explicit receiver type argument",
                    trait_name, const_name
                ),
            )
            .at(span));
        }
        let receiver_ty = Self::ast_type_ref(&type_args[0]);
        if let Some(type_key) = self.current_generic_ref_key(&receiver_ty) {
            if self.generic_ref_has_trait_bound(&type_key, trait_name) {
                return Ok(TraitAssocConstResolution::GenericBound);
            }
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "generic associated const '{}.{}' requires bound `{}: {}`",
                    trait_name, const_name, type_key, trait_name
                ),
            )
            .at(span));
        }
        let receiver_key = receiver_ty.key();
        if !self.ensure_trait_impl_for_type(trait_name, &receiver_ty, span)? {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "no impl of trait '{}' for associated const receiver type '{}'",
                    trait_name, receiver_key
                ),
            )
            .at(span));
        }
        let Some(mangled) = self
            .trait_impls
            .get(&(trait_name.to_string(), receiver_key.clone()))
            .and_then(|impl_info| impl_info.const_symbols.get(const_name))
            .cloned()
        else {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "impl of trait '{}' for '{}' has no associated const '{}'",
                    trait_name, receiver_key, const_name
                ),
            )
            .at(span));
        };
        Ok(TraitAssocConstResolution::Concrete(
            self.intern_symbol(&mangled),
        ))
    }

    fn resolve_trait_static_method_call(
        &mut self,
        trait_name: &str,
        method_name: &str,
        type_args: &[ast::TypeExpr],
        args: &[ast::Expr],
        span: Span,
    ) -> RR<TraitStaticMethodResolution> {
        let Some(trait_info) = self.trait_defs.get(trait_name) else {
            return Ok(TraitStaticMethodResolution::NotStaticMethod);
        };
        let Some(method) = trait_info
            .decl
            .methods
            .iter()
            .find(|method| method.name == method_name)
        else {
            return Ok(TraitStaticMethodResolution::NotStaticMethod);
        };
        if type_args.len() != 1 {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "static trait method '{}.{}' requires exactly one explicit receiver type argument",
                    trait_name, method_name
                ),
            )
            .at(span));
        }
        if method
            .params
            .first()
            .is_some_and(|param| param.name == "self")
            && args.is_empty()
        {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "trait method '{}.{}' requires a receiver argument",
                    trait_name, method_name
                ),
            )
            .at(span));
        }

        let receiver_ty = Self::ast_type_ref(&type_args[0]);
        if let Some(type_key) = self.current_generic_ref_key(&receiver_ty) {
            if self.generic_ref_has_trait_bound(&type_key, trait_name) {
                return Ok(TraitStaticMethodResolution::GenericBound);
            }
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "generic static trait method '{}.{}' requires bound `{}: {}`",
                    trait_name, method_name, type_key, trait_name
                ),
            )
            .at(span));
        }

        let receiver_key = receiver_ty.key();
        if !self.ensure_trait_impl_for_type(trait_name, &receiver_ty, span)? {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "no impl of trait '{}' for static method receiver type '{}'",
                    trait_name, receiver_key
                ),
            )
            .at(span));
        }
        let Some(mangled) = self
            .trait_impls
            .get(&(trait_name.to_string(), receiver_key.clone()))
            .and_then(|impl_info| impl_info.method_symbols.get(method_name))
            .cloned()
        else {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "impl of trait '{}' for '{}' has no method '{}'",
                    trait_name, receiver_key, method_name
                ),
            )
            .at(span));
        };
        Ok(TraitStaticMethodResolution::Concrete(
            self.intern_symbol(&mangled),
        ))
    }

    fn type_ref_to_ast_type(ty: &HirTypeRef) -> ast::TypeExpr {
        match ty {
            HirTypeRef::Named(name) => ast::TypeExpr::Named(name.clone()),
            HirTypeRef::Generic { base, args } => ast::TypeExpr::Generic {
                base: base.clone(),
                args: args.iter().map(Self::type_ref_to_ast_type).collect(),
            },
        }
    }

    fn const_int_from_type_ref(ty: &HirTypeRef) -> Option<i64> {
        let HirTypeRef::Named(name) = ty else {
            return None;
        };
        name.strip_prefix('#')?.parse().ok()
    }

    fn substitute_type_expr(
        ty: ast::TypeExpr,
        subst: &FxHashMap<String, HirTypeRef>,
    ) -> ast::TypeExpr {
        match ty {
            ast::TypeExpr::Named(name) => subst
                .get(&name)
                .map(Self::type_ref_to_ast_type)
                .unwrap_or(ast::TypeExpr::Named(name)),
            ast::TypeExpr::Generic { base, args } => {
                let args = args
                    .into_iter()
                    .map(|arg| Self::substitute_type_expr(arg, subst))
                    .collect::<Vec<_>>();
                let key = format!(
                    "{}<{}>",
                    base,
                    args.iter()
                        .map(Self::type_expr_key_for_subst)
                        .collect::<Vec<_>>()
                        .join(",")
                );
                subst
                    .get(&key)
                    .map(Self::type_ref_to_ast_type)
                    .unwrap_or(ast::TypeExpr::Generic { base, args })
            }
        }
    }

    fn type_expr_key_for_subst(ty: &ast::TypeExpr) -> String {
        match ty {
            ast::TypeExpr::Named(name) => name.clone(),
            ast::TypeExpr::Generic { base, args } => format!(
                "{}<{}>",
                base,
                args.iter()
                    .map(Self::type_expr_key_for_subst)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }

    fn substitute_fn_param_type(
        mut param: ast::FnParam,
        subst: &FxHashMap<String, HirTypeRef>,
    ) -> ast::FnParam {
        param.ty_hint = param
            .ty_hint
            .map(|ty| Self::substitute_type_expr(ty, subst));
        if let Some(default) = param.default {
            param.default = Some(Self::substitute_expr_type_hints(default, subst));
        }
        param
    }

    fn substitute_stmt_type_hints(
        mut stmt: ast::Stmt,
        subst: &FxHashMap<String, HirTypeRef>,
    ) -> ast::Stmt {
        stmt.kind = match stmt.kind {
            ast::StmtKind::Let {
                name,
                ty_hint,
                init,
            } => ast::StmtKind::Let {
                name,
                ty_hint: ty_hint.map(|ty| Self::substitute_type_expr(ty, subst)),
                init: init.map(|expr| Self::substitute_expr_type_hints(expr, subst)),
            },
            ast::StmtKind::Assign { target, value } => ast::StmtKind::Assign {
                target,
                value: Self::substitute_expr_type_hints(value, subst),
            },
            ast::StmtKind::FnDecl {
                name,
                type_params,
                params,
                ret_ty_hint,
                where_bounds,
                body,
            } => ast::StmtKind::FnDecl {
                name,
                type_params,
                params: params
                    .into_iter()
                    .map(|param| Self::substitute_fn_param_type(param, subst))
                    .collect(),
                ret_ty_hint: ret_ty_hint.map(|ty| Self::substitute_type_expr(ty, subst)),
                where_bounds,
                body: Self::substitute_block_type_hints(body, subst),
            },
            ast::StmtKind::If {
                cond,
                then_blk,
                else_blk,
            } => ast::StmtKind::If {
                cond: Self::substitute_expr_type_hints(cond, subst),
                then_blk: Self::substitute_block_type_hints(then_blk, subst),
                else_blk: else_blk.map(|blk| Self::substitute_block_type_hints(blk, subst)),
            },
            ast::StmtKind::While { cond, body } => ast::StmtKind::While {
                cond: Self::substitute_expr_type_hints(cond, subst),
                body: Self::substitute_block_type_hints(body, subst),
            },
            ast::StmtKind::For { var, iter, body } => ast::StmtKind::For {
                var,
                iter: Self::substitute_expr_type_hints(iter, subst),
                body: Self::substitute_block_type_hints(body, subst),
            },
            ast::StmtKind::Return { value } => ast::StmtKind::Return {
                value: value.map(|expr| Self::substitute_expr_type_hints(expr, subst)),
            },
            ast::StmtKind::ExprStmt { expr } => ast::StmtKind::ExprStmt {
                expr: Self::substitute_expr_type_hints(expr, subst),
            },
            ast::StmtKind::Expr(expr) => {
                ast::StmtKind::Expr(Self::substitute_expr_type_hints(expr, subst))
            }
            other => other,
        };
        stmt
    }

    fn substitute_block_type_hints(
        mut block: ast::Block,
        subst: &FxHashMap<String, HirTypeRef>,
    ) -> ast::Block {
        block.stmts = block
            .stmts
            .into_iter()
            .map(|stmt| Self::substitute_stmt_type_hints(stmt, subst))
            .collect();
        block
    }

    fn substitute_expr_type_hints(
        mut expr: ast::Expr,
        subst: &FxHashMap<String, HirTypeRef>,
    ) -> ast::Expr {
        expr.kind = match expr.kind {
            ast::ExprKind::Name(name) => subst
                .get(&name)
                .and_then(Self::const_int_from_type_ref)
                .map(|value| ast::ExprKind::Lit(ast::Lit::Int(value)))
                .unwrap_or(ast::ExprKind::Name(name)),
            ast::ExprKind::Unary { op, rhs } => ast::ExprKind::Unary {
                op,
                rhs: Box::new(Self::substitute_expr_type_hints(*rhs, subst)),
            },
            ast::ExprKind::Formula { lhs, rhs } => ast::ExprKind::Formula {
                lhs: lhs.map(|expr| Box::new(Self::substitute_expr_type_hints(*expr, subst))),
                rhs: Box::new(Self::substitute_expr_type_hints(*rhs, subst)),
            },
            ast::ExprKind::Binary { op, lhs, rhs } => ast::ExprKind::Binary {
                op,
                lhs: Box::new(Self::substitute_expr_type_hints(*lhs, subst)),
                rhs: Box::new(Self::substitute_expr_type_hints(*rhs, subst)),
            },
            ast::ExprKind::Range { a, b } => ast::ExprKind::Range {
                a: Box::new(Self::substitute_expr_type_hints(*a, subst)),
                b: Box::new(Self::substitute_expr_type_hints(*b, subst)),
            },
            ast::ExprKind::Call {
                callee,
                type_args,
                args,
            } => ast::ExprKind::Call {
                callee: Box::new(Self::substitute_expr_type_hints(*callee, subst)),
                type_args: type_args
                    .into_iter()
                    .map(|ty| Self::substitute_type_expr(ty, subst))
                    .collect(),
                args: args
                    .into_iter()
                    .map(|arg| Self::substitute_expr_type_hints(arg, subst))
                    .collect(),
            },
            ast::ExprKind::NamedArg { name, value } => ast::ExprKind::NamedArg {
                name,
                value: Box::new(Self::substitute_expr_type_hints(*value, subst)),
            },
            ast::ExprKind::Index { base, idx } => ast::ExprKind::Index {
                base: Box::new(Self::substitute_expr_type_hints(*base, subst)),
                idx: idx
                    .into_iter()
                    .map(|idx| Self::substitute_expr_type_hints(idx, subst))
                    .collect(),
            },
            ast::ExprKind::Field { base, name } => ast::ExprKind::Field {
                base: Box::new(Self::substitute_expr_type_hints(*base, subst)),
                name,
            },
            ast::ExprKind::VectorLit(xs) => ast::ExprKind::VectorLit(
                xs.into_iter()
                    .map(|expr| Self::substitute_expr_type_hints(expr, subst))
                    .collect(),
            ),
            ast::ExprKind::RecordLit(fields) => ast::ExprKind::RecordLit(
                fields
                    .into_iter()
                    .map(|(name, value)| (name, Self::substitute_expr_type_hints(value, subst)))
                    .collect(),
            ),
            ast::ExprKind::Pipe { lhs, rhs_call } => ast::ExprKind::Pipe {
                lhs: Box::new(Self::substitute_expr_type_hints(*lhs, subst)),
                rhs_call: Box::new(Self::substitute_expr_type_hints(*rhs_call, subst)),
            },
            ast::ExprKind::Try { expr } => ast::ExprKind::Try {
                expr: Box::new(Self::substitute_expr_type_hints(*expr, subst)),
            },
            ast::ExprKind::Unquote(expr) => {
                ast::ExprKind::Unquote(Box::new(Self::substitute_expr_type_hints(*expr, subst)))
            }
            ast::ExprKind::Match { scrutinee, arms } => ast::ExprKind::Match {
                scrutinee: Box::new(Self::substitute_expr_type_hints(*scrutinee, subst)),
                arms: arms
                    .into_iter()
                    .map(|mut arm| {
                        if let Some(guard) = arm.guard {
                            arm.guard =
                                Some(Box::new(Self::substitute_expr_type_hints(*guard, subst)));
                        }
                        arm.body = Box::new(Self::substitute_expr_type_hints(*arm.body, subst));
                        arm
                    })
                    .collect(),
            },
            ast::ExprKind::Lambda {
                params,
                ret_ty_hint,
                body,
            } => ast::ExprKind::Lambda {
                params: params
                    .into_iter()
                    .map(|param| Self::substitute_fn_param_type(param, subst))
                    .collect(),
                ret_ty_hint: ret_ty_hint.map(|ty| Self::substitute_type_expr(ty, subst)),
                body: Self::substitute_block_type_hints(body, subst),
            },
            other => other,
        };
        expr
    }

    fn bind_generic_type_param(
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

    fn infer_generic_type_from_param(
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

    fn generic_call_arg_expr<'a>(
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

    fn infer_generic_subst(
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

    fn validate_generic_bounds_for_subst(
        &mut self,
        decl: &ast::FnDecl,
        subst: &FxHashMap<String, HirTypeRef>,
        span: Span,
    ) -> RR<()> {
        self.validate_trait_bounds_for_subst(&decl.where_bounds, subst, span)
    }

    fn generic_instance_name(name: &str, concrete_tys: &[HirTypeRef]) -> String {
        fn sanitize(input: &str) -> String {
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

    fn resolve_generic_call(
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
        let inst_fn = self.lower_fn(
            inst_name,
            Vec::new(),
            inst_params,
            inst_ret,
            Vec::new(),
            inst_body,
            span,
        )?;
        self.pending_fns.push(inst_fn);
        Ok(Some(inst_sym))
    }

    fn receiver_method_candidates(
        &mut self,
        receiver_ty: &HirTypeRef,
        method_name: &str,
        span: Span,
    ) -> RR<Vec<(String, String)>> {
        for trait_name in self
            .generic_trait_impls
            .iter()
            .filter(|info| {
                info.decl
                    .methods
                    .iter()
                    .any(|method| method.name == method_name)
                    && self
                        .trait_defs
                        .get(&info.decl.trait_name)
                        .is_some_and(|trait_info| {
                            trait_info
                                .decl
                                .methods
                                .iter()
                                .any(|method| method.name == method_name)
                        })
            })
            .map(|info| info.decl.trait_name.clone())
            .collect::<Vec<_>>()
        {
            self.ensure_trait_impl_for_type(&trait_name, receiver_ty, span)?;
        }
        let receiver_key = receiver_ty.key();
        let mut candidates = Vec::new();
        for ((trait_name, for_ty), impl_info) in &self.trait_impls {
            if for_ty != &receiver_key {
                continue;
            }
            if let Some(mangled) = impl_info.method_symbols.get(method_name) {
                candidates.push((trait_name.clone(), mangled.clone()));
            }
        }
        candidates.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(candidates)
    }

    fn resolve_receiver_method_call(
        &mut self,
        receiver: &ast::Expr,
        method_name: &str,
        span: Span,
    ) -> RR<Option<SymbolId>> {
        let Some(receiver_ty) = self.trait_type_of_ast_expr(receiver) else {
            return Ok(None);
        };
        if let Some(type_key) = self.current_generic_ref_key(&receiver_ty) {
            let candidates = self.type_param_method_bound_candidates(&type_key, method_name);
            return match candidates.as_slice() {
                [] => Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "generic receiver type '{}' uses method '{}' without a matching trait bound",
                        type_key, method_name
                    ),
                )
                .at(span)
                .note(format!(
                    "Add a bound such as `where {}: TraitWith{}` before using this method.",
                    type_key,
                    method_name
                        .chars()
                        .next()
                        .map(|ch| ch.to_uppercase().collect::<String>())
                        .unwrap_or_default()
                ))),
                [_] => Ok(None),
                _ => Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "ambiguous generic trait method '{}.{}'; bounds [{}] all provide it. Use explicit Trait.method(receiver, ...) syntax",
                        type_key,
                        method_name,
                        candidates.join(", ")
                    ),
                )
                .at(span)),
            };
        }
        if self.type_ref_contains_current_type_param(&receiver_ty) {
            return Ok(None);
        }
        let candidates = self.receiver_method_candidates(&receiver_ty, method_name, span)?;
        match candidates.as_slice() {
            [] => Ok(None),
            [(_, mangled)] => Ok(Some(self.intern_symbol(mangled))),
            _ => {
                let trait_names = candidates
                    .iter()
                    .map(|(trait_name, _)| trait_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "ambiguous trait method '{}.{}' for receiver type '{}'; candidates are [{}]. Use explicit Trait.method(receiver, ...) syntax",
                        receiver_ty.key(),
                        method_name,
                        receiver_ty.key(),
                        trait_names
                    ),
                )
                .at(span))
            }
        }
    }

    fn resolve_trait_call(
        &mut self,
        trait_name: &str,
        method_name: &str,
        args: &[ast::Expr],
        span: Span,
    ) -> RR<Option<SymbolId>> {
        let Some(trait_info) = self.trait_defs.get(trait_name) else {
            return Ok(None);
        };
        if !trait_info
            .decl
            .methods
            .iter()
            .any(|method| method.name == method_name)
        {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!("trait '{}' has no method '{}'", trait_name, method_name),
            )
            .at(span));
        }
        let Some(receiver) = Self::trait_receiver_expr(args) else {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "trait method '{}.{}' requires a receiver argument",
                    trait_name, method_name
                ),
            )
            .at(span));
        };
        let Some(receiver_ty) = self.trait_type_of_ast_expr(receiver) else {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "trait method '{}.{}' requires a receiver with an explicit static type hint",
                    trait_name, method_name
                ),
            )
            .at(receiver.span));
        };
        if let Some(type_key) = self.current_generic_ref_key(&receiver_ty) {
            if self.generic_ref_has_trait_bound(&type_key, trait_name) {
                return Ok(None);
            }
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "generic trait call '{}.{}' requires bound `{}: {}`",
                    trait_name, method_name, type_key, trait_name
                ),
            )
            .at(receiver.span));
        }
        let receiver_key = receiver_ty.key();
        if !self.ensure_trait_impl_for_type(trait_name, &receiver_ty, receiver.span)? {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "no impl of trait '{}' for receiver type '{}'",
                    trait_name, receiver_key
                ),
            )
            .at(receiver.span));
        }
        let Some(impl_info) = self
            .trait_impls
            .get(&(trait_name.to_string(), receiver_key.clone()))
        else {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "no impl of trait '{}' for receiver type '{}'",
                    trait_name, receiver_key
                ),
            )
            .at(receiver.span));
        };
        let Some(mangled) = impl_info.method_symbols.get(method_name).cloned() else {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "impl of trait '{}' for '{}' has no method '{}'",
                    trait_name, receiver_key, method_name
                ),
            )
            .at(span));
        };
        Ok(Some(self.intern_symbol(&mangled)))
    }

    fn lower_expr(&mut self, expr: ast::Expr) -> RR<HirExpr> {
        self.lower_expr_inner(expr, None)
    }

    fn lower_expr_with_expected(
        &mut self,
        expr: ast::Expr,
        expected_ret_ty: Option<&HirTypeRef>,
    ) -> RR<HirExpr> {
        self.lower_expr_inner(expr, expected_ret_ty)
    }

    fn lower_expr_inner(
        &mut self,
        expr: ast::Expr,
        expected_ret_ty: Option<&HirTypeRef>,
    ) -> RR<HirExpr> {
        match expr.kind {
            ast::ExprKind::Lit(l) => {
                let hl = match l {
                    ast::Lit::Int(i) => HirLit::Int(i),
                    ast::Lit::Float(f) => HirLit::Double(f),
                    ast::Lit::Str(s) => HirLit::Char(s),
                    ast::Lit::Bool(b) => HirLit::Bool(b),
                    ast::Lit::Na => HirLit::NA,
                    ast::Lit::Null => HirLit::Null,
                };
                Ok(HirExpr::Lit(hl))
            }
            ast::ExprKind::Name(n) => {
                if let Some(lid) = self.lookup(&n) {
                    Ok(HirExpr::Local(lid))
                } else if let Some(sym) = self.global_fn_aliases.get(&n).copied() {
                    Ok(HirExpr::Global(sym, expr.span))
                } else if let Some(sym) = self.r_import_aliases.get(&n).copied() {
                    Ok(HirExpr::Global(sym, expr.span))
                } else {
                    Ok(HirExpr::Global(self.intern_symbol(&n), expr.span))
                }
            }
            ast::ExprKind::Binary { op, lhs, rhs } => {
                if let Some((trait_name, method_name)) = Self::operator_trait_for_binop(op)
                    && let Some(lhs_ty) = self.trait_type_of_ast_expr(&lhs)
                    && let Some(type_key) = self.current_generic_ref_key(&lhs_ty)
                {
                    if !self.generic_ref_has_trait_bound(&type_key, trait_name) {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "generic operator '{}' requires bound `{}: {}`",
                                method_name, type_key, trait_name
                            ),
                        )
                        .at(expr.span));
                    }
                }
                if let Some((trait_name, method_name)) = Self::operator_trait_for_binop(op)
                    && let Some(lhs_ty) = self.trait_type_of_ast_expr(&lhs)
                    && let Some(method_sym) = self.trait_impl_method_for_type(
                        trait_name,
                        method_name,
                        &lhs_ty,
                        expr.span,
                    )?
                {
                    let hargs = vec![
                        HirArg::Pos(self.lower_expr(*lhs)?),
                        HirArg::Pos(self.lower_expr(*rhs)?),
                    ];
                    return Ok(HirExpr::Call(HirCall {
                        callee: Box::new(HirExpr::Global(method_sym, expr.span)),
                        args: hargs,
                        span: expr.span,
                    }));
                }
                let hop = Self::hir_binop(op);
                Ok(HirExpr::Binary {
                    op: hop,
                    lhs: Box::new(self.lower_expr(*lhs)?),
                    rhs: Box::new(self.lower_expr(*rhs)?),
                })
            }
            ast::ExprKind::Formula { lhs, rhs } => {
                if let Some(lhs) = lhs {
                    self.lower_formula_binary_expr(*lhs, *rhs, expr.span)
                } else {
                    self.lower_formula_unary_expr(*rhs, expr.span)
                }
            }
            ast::ExprKind::Unary { op, rhs } => {
                if matches!(op, ast::UnaryOp::Formula) {
                    return self.lower_formula_unary_expr(*rhs, expr.span);
                }
                if let Some((trait_name, method_name)) = Self::operator_trait_for_unop(op)
                    && let Some(rhs_ty) = self.trait_type_of_ast_expr(&rhs)
                    && let Some(type_key) = self.current_generic_ref_key(&rhs_ty)
                    && !self.generic_ref_has_trait_bound(&type_key, trait_name)
                {
                    return Err(RRException::new(
                        "RR.SemanticError",
                        RRCode::E1002,
                        Stage::Lower,
                        format!(
                            "generic operator '{}' requires bound `{}: {}`",
                            method_name, type_key, trait_name
                        ),
                    )
                    .at(expr.span));
                }
                if let Some((trait_name, method_name)) = Self::operator_trait_for_unop(op)
                    && let Some(rhs_ty) = self.trait_type_of_ast_expr(&rhs)
                    && let Some(method_sym) = self.trait_impl_method_for_type(
                        trait_name,
                        method_name,
                        &rhs_ty,
                        expr.span,
                    )?
                {
                    let hargs = vec![HirArg::Pos(self.lower_expr(*rhs)?)];
                    return Ok(HirExpr::Call(HirCall {
                        callee: Box::new(HirExpr::Global(method_sym, expr.span)),
                        args: hargs,
                        span: expr.span,
                    }));
                }
                let hop = match op {
                    ast::UnaryOp::Not => HirUnOp::Not,
                    ast::UnaryOp::Neg => HirUnOp::Neg,
                    ast::UnaryOp::Formula => unreachable!("formula unary lowered earlier"),
                };
                Ok(HirExpr::Unary {
                    op: hop,
                    expr: Box::new(self.lower_expr(*rhs)?),
                })
            }
            ast::ExprKind::Lambda {
                params,
                ret_ty_hint,
                body,
            } => self.lower_lambda_expr(params, ret_ty_hint, body, expr.span),
            ast::ExprKind::Call {
                callee,
                type_args,
                args,
            } => self.lower_call_expr(*callee, type_args, args, expected_ret_ty, expr.span),
            ast::ExprKind::Pipe { lhs, rhs_call } => {
                let lhs_h = self.lower_expr(*lhs)?;
                match rhs_call.kind {
                    ast::ExprKind::Call {
                        callee,
                        type_args: _,
                        args,
                    } => {
                        let c = self.lower_expr(*callee)?;
                        let mut hargs = Vec::with_capacity(args.len() + 1);
                        hargs.push(HirArg::Pos(lhs_h));
                        for a in args {
                            match a.kind {
                                ast::ExprKind::NamedArg { name, value } => {
                                    let sym = self.intern_symbol(&name);
                                    hargs.push(HirArg::Named {
                                        name: sym,
                                        value: self.lower_expr(*value)?,
                                    });
                                }
                                _ => hargs.push(HirArg::Pos(self.lower_expr(a)?)),
                            }
                        }
                        Ok(HirExpr::Call(HirCall {
                            callee: Box::new(c),
                            args: hargs,
                            span: expr.span,
                        }))
                    }
                    ast::ExprKind::Try { expr: inner } => match inner.kind {
                        ast::ExprKind::Call {
                            callee,
                            type_args: _,
                            args,
                        } => {
                            let c = self.lower_expr(*callee)?;
                            let mut hargs = Vec::with_capacity(args.len() + 1);
                            hargs.push(HirArg::Pos(lhs_h));
                            for a in args {
                                match a.kind {
                                    ast::ExprKind::NamedArg { name, value } => {
                                        let sym = self.intern_symbol(&name);
                                        hargs.push(HirArg::Named {
                                            name: sym,
                                            value: self.lower_expr(*value)?,
                                        });
                                    }
                                    _ => hargs.push(HirArg::Pos(self.lower_expr(a)?)),
                                }
                            }
                            let call = HirExpr::Call(HirCall {
                                callee: Box::new(c),
                                args: hargs,
                                span: expr.span,
                            });
                            Ok(HirExpr::Try(Box::new(call)))
                        }
                        _ => Err(RRException::new(
                            "RR.ParseError",
                            RRCode::E0001,
                            Stage::Lower,
                            "RHS of |> must be call or call?".to_string(),
                        )),
                    },
                    _ => Err(RRException::new(
                        "RR.ParseError",
                        RRCode::E0001,
                        Stage::Lower,
                        "RHS of |> must be call".to_string(),
                    )),
                }
            }
            ast::ExprKind::Field { base, name } => {
                if let Some(dotted) = Self::dotted_name_from_field(&base, &name)
                    .filter(|d| self.root_is_unbound_for_dotted(d))
                {
                    return Ok(self.lower_dotted_ref(&dotted, expr.span));
                }
                let b = self.lower_expr(*base)?;
                let sym = self.intern_symbol(&name);
                Ok(HirExpr::Field {
                    base: Box::new(b),
                    name: sym,
                })
            }
            // v6 features
            ast::ExprKind::Match { scrutinee, arms } => {
                let s = self.lower_expr(*scrutinee)?;
                let mut harms = Vec::new();
                for arm in arms {
                    self.enter_scope(); // Arm scope
                    let pat = self.lower_pattern(arm.pat)?;
                    let guard = if let Some(g) = arm.guard {
                        Some(self.lower_expr(*g)?)
                    } else {
                        None
                    };
                    let body = self.lower_expr(*arm.body)?;
                    self.exit_scope();

                    harms.push(HirMatchArm {
                        pat,
                        guard,
                        body,
                        span: arm.span,
                    });
                }
                Ok(HirExpr::Match {
                    scrut: Box::new(s),
                    arms: harms,
                })
            }
            ast::ExprKind::Try { expr: e } => Ok(HirExpr::Try(Box::new(self.lower_expr(*e)?))),
            ast::ExprKind::Column(n) => Ok(HirExpr::Column(n)),
            ast::ExprKind::ColRef(n) => Ok(HirExpr::Column(n)),
            ast::ExprKind::Unquote(e) => {
                let inner = self.lower_expr(*e)?;
                Ok(HirExpr::Unquote(Box::new(inner)))
            }
            ast::ExprKind::Index { base, idx } => {
                if let Some(base_ty) = self.trait_type_of_ast_expr(&base) {
                    if let Some(type_key) = self.current_generic_ref_key(&base_ty)
                        && !self.generic_ref_has_trait_bound(&type_key, "Index")
                    {
                        return Err(RRException::new(
                            "RR.SemanticError",
                            RRCode::E1002,
                            Stage::Lower,
                            format!(
                                "generic index operation requires bound `{}: Index`",
                                type_key
                            ),
                        )
                        .at(expr.span));
                    }
                    if let Some(method_sym) =
                        self.trait_impl_method_for_type("Index", "index", &base_ty, expr.span)?
                    {
                        let mut hargs = Vec::with_capacity(idx.len() + 1);
                        hargs.push(HirArg::Pos(self.lower_expr(*base)?));
                        for i in idx {
                            hargs.push(HirArg::Pos(self.lower_expr(i)?));
                        }
                        return Ok(HirExpr::Call(HirCall {
                            callee: Box::new(HirExpr::Global(method_sym, expr.span)),
                            args: hargs,
                            span: expr.span,
                        }));
                    }
                }
                let b = self.lower_expr(*base)?;
                let mut indices = Vec::new();
                for i in idx {
                    indices.push(self.lower_expr(i)?);
                }
                Ok(HirExpr::Index {
                    base: Box::new(b),
                    index: indices,
                })
            }
            ast::ExprKind::Range { a, b } => {
                let start = self.lower_expr(*a)?;
                let end = self.lower_expr(*b)?;
                Ok(HirExpr::Range {
                    start: Box::new(start),
                    end: Box::new(end),
                })
            }
            ast::ExprKind::VectorLit(elems) => {
                let mut helems = Vec::new();
                for e in elems {
                    helems.push(self.lower_expr(e)?);
                }
                Ok(HirExpr::VectorLit(helems))
            }
            ast::ExprKind::RecordLit(fields) => {
                let mut hfields = Vec::new();
                for (k, v) in fields {
                    let sym = self.intern_symbol(&k);
                    hfields.push((sym, self.lower_expr(v)?));
                }
                Ok(HirExpr::ListLit(hfields))
            }
            _ => Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!("unsupported expression in HIR lowering: {:?}", expr.kind),
            )
            .at(expr.span)
            .push_frame("hir::lower::lower_expr/1", Some(expr.span))),
        }
    }

    fn lower_pattern(&mut self, pat: ast::Pattern) -> RR<HirPat> {
        match pat.kind {
            ast::PatternKind::Wild => Ok(HirPat::Wild),
            ast::PatternKind::Lit(l) => {
                let hl = match l {
                    ast::Lit::Int(i) => HirLit::Int(i),
                    ast::Lit::Float(f) => HirLit::Double(f),
                    ast::Lit::Str(s) => HirLit::Char(s),
                    ast::Lit::Bool(b) => HirLit::Bool(b),
                    ast::Lit::Na => HirLit::NA,
                    ast::Lit::Null => HirLit::Null,
                };
                Ok(HirPat::Lit(hl))
            }
            ast::PatternKind::Bind(n) => {
                let lid = self.declare_local(&n);
                let sym = self.intern_symbol(&n);
                Ok(HirPat::Bind {
                    name: sym,
                    local: lid,
                })
            }
            ast::PatternKind::List { items, rest } => {
                let mut hitems = Vec::new();
                for i in items {
                    hitems.push(self.lower_pattern(i)?);
                }

                let hrest = if let Some(n) = rest {
                    let lid = self.declare_local(&n);
                    let sym = self.intern_symbol(&n);
                    Some((sym, lid))
                } else {
                    None
                };
                Ok(HirPat::List {
                    items: hitems,
                    rest: hrest,
                })
            }
            ast::PatternKind::Record { fields } => {
                let mut hfields = Vec::new();
                for (name, p) in fields {
                    let sym = self.intern_symbol(&name);
                    let hp = self.lower_pattern(p)?;
                    hfields.push((sym, hp));
                }
                Ok(HirPat::Record { fields: hfields })
            }
        }
    }

    fn lower_lvalue(&mut self, lval: ast::LValue) -> RR<HirLValue> {
        let lv_span = lval.span;
        match lval.kind {
            ast::LValueKind::Name(n) => {
                let lid = self.resolve_or_declare_local_for_assign(&n, lv_span)?;
                Ok(HirLValue::Local(lid))
            }
            ast::LValueKind::Index { base, idx } => {
                let b = self.lower_expr(base)?;
                let mut indices = Vec::new();
                for i in idx {
                    indices.push(self.lower_expr(i)?);
                }
                Ok(HirLValue::Index {
                    base: b,
                    index: indices,
                })
            }
            ast::LValueKind::Field { base, name } => {
                if let Some(dotted) = Self::dotted_name_from_field(&base, &name)
                    .filter(|d| self.root_is_unbound_for_dotted(d))
                {
                    let lid = self.resolve_or_declare_local_for_assign(&dotted, lv_span)?;
                    return Ok(HirLValue::Local(lid));
                }
                let b = self.lower_expr(base)?;
                let sym = self.intern_symbol(&name);
                Ok(HirLValue::Field { base: b, name: sym })
            }
        }
    }

    fn local_name_of_lvalue(&self, lval: &HirLValue) -> Option<String> {
        match lval {
            HirLValue::Local(id) => self.local_names.get(id).cloned(),
            _ => None,
        }
    }
}
