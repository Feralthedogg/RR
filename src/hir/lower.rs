use crate::error::{DiagnosticLabelKind, InternalCompilerError, RR, RRCode, RRException, Stage};
use crate::hir::def::*;
use crate::syntax::ast;
use crate::typeck::trait_solver::{
    TraitImplHeader, TraitSolver, trait_impl_is_more_specific, trait_impl_patterns_overlap,
};
use crate::utils::{Span, did_you_mean};
use rustc_hash::{FxHashMap, FxHashSet};

mod call_dispatch;
mod trait_names;
mod trait_resolution_impl;

#[derive(Clone)]
pub(crate) struct TraitDeclInfo {
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
pub(crate) struct GenericTraitImplInfo {
    decl: ast::ImplDecl,
    for_ty: HirTypeRef,
}

#[derive(Clone)]
struct GenericFnInfo {
    decl: ast::FnDecl,
}

pub(crate) struct LowerFnParts {
    name: String,
    type_params: Vec<String>,
    params: Vec<ast::FnParam>,
    ret_ty_hint: Option<ast::TypeExpr>,
    where_bounds: Vec<ast::TraitBound>,
    body: ast::Block,
    span: Span,
}

pub(crate) enum TraitAssocConstResolution {
    NotAssocConst,
    GenericBound,
    Concrete(SymbolId),
}

pub(crate) enum TraitStaticMethodResolution {
    NotStaticMethod,
    GenericBound,
    Concrete(SymbolId),
}

#[derive(Clone, Copy)]
pub(crate) struct TypeProjectionParts<'a> {
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

#[derive(Clone, Copy, Debug)]
pub struct LoweringPolicy {
    pub strict_let: bool,
    pub warn_implicit_decl: bool,
}

impl Default for LoweringPolicy {
    fn default() -> Self {
        Self {
            strict_let: true,
            warn_implicit_decl: false,
        }
    }
}

impl Default for Lowerer {
    fn default() -> Self {
        Self::new()
    }
}

#[path = "lower/context.rs"]
mod context;
#[path = "lower/exprs.rs"]
mod exprs;
#[path = "lower/items.rs"]
mod items;
#[path = "lower/metadata.rs"]
mod metadata;
#[path = "lower/stmts.rs"]
mod stmts;
#[path = "lower/trait_queries.rs"]
mod trait_queries;
#[path = "lower/type_refs.rs"]
mod type_refs;
