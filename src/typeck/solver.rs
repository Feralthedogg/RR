use crate::diagnostic::{DiagnosticBuilder, finish_diagnostics};
use crate::error::{InternalCompilerError, RR, RRCode, RRException, Stage};
use crate::hir::def::Ty;
use crate::mir::{FnIR, Instr, Terminator, ValueId, ValueKind};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

use super::builtin_sigs::{infer_builtin, infer_builtin_term};
use super::constraints::{ConstraintSet, TypeConstraint};
use super::lattice::{LenSym, NaTy, PrimTy, ShapeTy, TypeState};
use super::term::{
    TypeTerm, from_hir_ty as term_from_hir_ty,
    from_hir_ty_with_symbols as term_from_hir_ty_with_symbols, from_lit as lit_term,
};

#[path = "solver/index_demands.rs"]
mod index_demands;
#[path = "solver/terms.rs"]
mod terms;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeMode {
    Strict,
    Gradual,
}

impl TypeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Gradual => "gradual",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBackend {
    Off,
    Optional,
    Required,
}

impl NativeBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Optional => "optional",
            Self::Required => "required",
        }
    }
}

impl std::str::FromStr for TypeMode {
    type Err = ();

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        match v.trim().to_ascii_lowercase().as_str() {
            "strict" => Ok(Self::Strict),
            "gradual" => Ok(Self::Gradual),
            _ => Err(()),
        }
    }
}

impl std::str::FromStr for NativeBackend {
    type Err = ();

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        match v.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "optional" => Ok(Self::Optional),
            "required" => Ok(Self::Required),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TypeConfig {
    pub mode: TypeMode,
    pub native_backend: NativeBackend,
}

impl Default for TypeConfig {
    fn default() -> Self {
        Self {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        }
    }
}

fn from_hir_ty(ty: &Ty) -> TypeState {
    match ty {
        Ty::Any => TypeState::unknown(),
        Ty::Null => TypeState::null(),
        Ty::Logical => TypeState::scalar(PrimTy::Logical, true),
        Ty::Int => TypeState::scalar(PrimTy::Int, true),
        Ty::Double => TypeState::scalar(PrimTy::Double, true),
        Ty::Char => TypeState::scalar(PrimTy::Char, true),
        Ty::Vector(inner) => TypeState::vector(from_hir_ty(inner).prim, true),
        Ty::Matrix(inner) => TypeState::matrix(from_hir_ty(inner).prim, true),
        Ty::List(_) => TypeState::vector(PrimTy::Any, false),
        Ty::Box(inner) => from_hir_ty(inner),
        Ty::DataFrame(_) => TypeState::matrix(PrimTy::Any, false),
        Ty::Union(xs) => xs
            .iter()
            .map(from_hir_ty)
            .fold(TypeState::unknown(), TypeState::join),
        Ty::Option(inner) => {
            let mut inner_ty = from_hir_ty(inner);
            inner_ty.na = NaTy::Maybe;
            inner_ty
        }
        Ty::Result(ok, err) => from_hir_ty(ok).join(from_hir_ty(err)),
        Ty::Never => TypeState::unknown(),
    }
}

fn from_hir_ty_term(ty: &Ty) -> TypeTerm {
    term_from_hir_ty(ty)
}

fn lit_type(lit: &crate::syntax::ast::Lit) -> TypeState {
    match lit {
        crate::syntax::ast::Lit::Int(_) => TypeState::scalar(PrimTy::Int, true),
        crate::syntax::ast::Lit::Float(_) => TypeState::scalar(PrimTy::Double, true),
        crate::syntax::ast::Lit::Bool(_) => TypeState::scalar(PrimTy::Logical, true),
        crate::syntax::ast::Lit::Str(_) => TypeState::scalar(PrimTy::Char, true),
        crate::syntax::ast::Lit::Null => TypeState::null(),
        crate::syntax::ast::Lit::Na => TypeState::scalar(PrimTy::Any, false),
    }
}

fn normalize_call_numeric_shape(args: &[TypeState]) -> ShapeTy {
    let has_matrix = args.iter().any(|a| a.shape == ShapeTy::Matrix);
    let has_vector = args.iter().any(|a| a.shape == ShapeTy::Vector);
    match (has_matrix, has_vector) {
        (true, true) => ShapeTy::Unknown,
        (true, false) => ShapeTy::Matrix,
        (false, true) => ShapeTy::Vector,
        (false, false) => ShapeTy::Scalar,
    }
}

fn type_state_from_term(term: &TypeTerm) -> TypeState {
    match term {
        TypeTerm::Any | TypeTerm::Never => TypeState::unknown(),
        TypeTerm::Null => TypeState::null(),
        TypeTerm::Logical => TypeState::scalar(PrimTy::Logical, false),
        TypeTerm::Int => TypeState::scalar(PrimTy::Int, false),
        TypeTerm::Double => TypeState::scalar(PrimTy::Double, false),
        TypeTerm::Char => TypeState::scalar(PrimTy::Char, false),
        TypeTerm::Vector(inner) => TypeState::vector(type_state_from_term(inner).prim, false),
        TypeTerm::Matrix(inner) => TypeState::matrix(type_state_from_term(inner).prim, false),
        TypeTerm::MatrixDim(inner, _, _) => {
            TypeState::matrix(type_state_from_term(inner).prim, false)
        }
        TypeTerm::DataFrame(cols) => {
            let prim = cols
                .iter()
                .map(type_state_from_term)
                .fold(TypeState::unknown(), TypeState::join)
                .prim;
            TypeState::matrix(prim, false)
        }
        TypeTerm::DataFrameNamed(cols) => {
            let prim = cols
                .iter()
                .map(|(_, term)| type_state_from_term(term))
                .fold(TypeState::unknown(), TypeState::join)
                .prim;
            TypeState::matrix(prim, false)
        }
        TypeTerm::List(_) => TypeState::vector(PrimTy::Any, false),
        TypeTerm::Boxed(inner) => type_state_from_term(inner),
        TypeTerm::Option(inner) => {
            let mut inner_ty = type_state_from_term(inner);
            inner_ty.na = NaTy::Maybe;
            inner_ty
        }
        TypeTerm::Union(xs) => xs
            .iter()
            .map(type_state_from_term)
            .fold(TypeState::unknown(), TypeState::join),
    }
}

fn refine_type_with_term(ty: TypeState, term: &TypeTerm) -> TypeState {
    let term_ty = type_state_from_term(term);
    let mut out = ty.join(term_ty);
    if out.len_sym.is_none() {
        out.len_sym = ty.len_sym.or(term_ty.len_sym);
    }
    out
}

fn promoted_numeric_prim(lhs: PrimTy, rhs: PrimTy) -> PrimTy {
    match (lhs, rhs) {
        (PrimTy::Int, PrimTy::Int) => PrimTy::Int,
        (PrimTy::Int, PrimTy::Double)
        | (PrimTy::Double, PrimTy::Int)
        | (PrimTy::Double, PrimTy::Double) => PrimTy::Double,
        (PrimTy::Any, other) | (other, PrimTy::Any) => other,
        _ => PrimTy::Any,
    }
}

pub fn analyze_program(all_fns: &mut FxHashMap<String, FnIR>, cfg: TypeConfig) -> RR<()> {
    let mut fn_ret: FxHashMap<String, TypeState> = FxHashMap::default();
    let mut fn_ret_term: FxHashMap<String, TypeTerm> = FxHashMap::default();
    let mut init_names: Vec<String> = all_fns.keys().cloned().collect();
    init_names.sort();
    for name in init_names {
        let Some(fn_ir) = all_fns.get(&name) else {
            continue;
        };
        fn_ret.insert(
            name.clone(),
            fn_ir.ret_ty_hint.unwrap_or(TypeState::unknown()),
        );
        fn_ret_term.insert(name, fn_ir.ret_term_hint.clone().unwrap_or(TypeTerm::Any));
    }

    let mut changed = true;
    let mut guard = 0usize;
    let mut scalar_ret_demands: FxHashSet<String> = FxHashSet::default();
    let mut vector_ret_demands: FxHashSet<String> = FxHashSet::default();
    while changed && guard < 16 {
        guard += 1;
        changed = false;
        let _ = apply_index_return_demands(
            all_fns,
            &mut fn_ret,
            &mut fn_ret_term,
            &scalar_ret_demands,
            &vector_ret_demands,
        );

        let mut names: Vec<String> = all_fns.keys().cloned().collect();
        names.sort();
        for name in names {
            let enforce_vector_ret = vector_ret_demands.contains(&name)
                && can_apply_index_return_override(
                    all_fns,
                    &name,
                    ShapeTy::Vector,
                    &TypeTerm::Vector(Box::new(TypeTerm::Int)),
                );
            let enforce_scalar_ret = scalar_ret_demands.contains(&name)
                && can_apply_index_return_override(all_fns, &name, ShapeTy::Scalar, &TypeTerm::Int);
            let Some(fn_ir) = all_fns.get_mut(&name) else {
                return Err(InternalCompilerError::new(
                    Stage::Mir,
                    format!("type solver missing function '{}'", name),
                )
                .into_exception());
            };
            let mut ret = analyze_function(fn_ir, &fn_ret)?;
            let mut ret_term = analyze_function_terms(fn_ir, &fn_ret_term);
            if enforce_vector_ret {
                ret = coerce_index_vector_return(ret);
                ret_term = TypeTerm::Vector(Box::new(TypeTerm::Int));
            } else if enforce_scalar_ret {
                ret = coerce_index_scalar_return(ret);
                ret_term = TypeTerm::Int;
            }

            let prev = fn_ret.get(&name).copied().unwrap_or(TypeState::unknown());
            let prev_term = fn_ret_term.get(&name).cloned().unwrap_or(TypeTerm::Any);
            if ret != prev {
                fn_ret.insert(name.clone(), ret);
                changed = true;
            }
            if ret_term != prev_term {
                fn_ret_term.insert(name.clone(), ret_term.clone());
                changed = true;
            }
            fn_ir.inferred_ret_ty = ret;
            fn_ir.inferred_ret_term = ret_term;
        }

        let index_param_slots = collect_index_vector_param_slots_by_function(all_fns);
        let next_scalar_ret_demands = collect_scalar_index_return_demands(all_fns);
        let next_vector_ret_demands =
            collect_vector_index_return_demands(all_fns, &index_param_slots);
        if next_scalar_ret_demands != scalar_ret_demands
            || next_vector_ret_demands != vector_ret_demands
        {
            scalar_ret_demands = next_scalar_ret_demands;
            vector_ret_demands = next_vector_ret_demands;
            changed = true;
        }
        if apply_index_return_demands(
            all_fns,
            &mut fn_ret,
            &mut fn_ret_term,
            &scalar_ret_demands,
            &vector_ret_demands,
        ) {
            changed = true;
        }
    }

    let mut type_errors = Vec::new();
    let mut names: Vec<String> = all_fns.keys().cloned().collect();
    names.sort();
    for name in names {
        let Some(fn_ir) = all_fns.get(&name) else {
            continue;
        };
        if cfg.mode != TypeMode::Strict {
            continue;
        }

        if let Some(h) = &fn_ir.ret_term_hint {
            let inferred_term = &fn_ir.inferred_ret_term;
            if !h.is_any() && !inferred_term.is_any() && !h.compatible_with(inferred_term) {
                type_errors.push(
                    DiagnosticBuilder::new(
                        "RR.TypeError",
                        RRCode::E1010,
                        Stage::Mir,
                        format!(
                            "type hint conflict in function '{}': return hint {:?} vs inferred {:?}",
                            name, h, inferred_term
                        ),
                    )
                    .at(fn_ir.ret_hint_span.unwrap_or(fn_ir.span))
                    .constraint(
                        fn_ir.ret_hint_span.unwrap_or(fn_ir.span),
                        format!("declared return type is constrained to {:?}", h),
                    )
                    .origin(
                        first_return_origin_span(fn_ir).unwrap_or(fn_ir.span),
                        format!("inferred return flow produces {:?}", inferred_term),
                    )
                    .use_site(
                        fn_ir.span,
                        "function body must satisfy the declared return contract",
                    )
                    .note("Strict mode compares return hints against the inferred function result.")
                    .fix(format!(
                        "change the return annotation to {:?}, or return a value compatible with {:?}",
                        inferred_term, h
                    ))
                    .build(),
                );
            }
        } else if let Some(h) = fn_ir.ret_ty_hint {
            // Backward-compatible primitive-only clash check when structural hint is absent.
            let inferred = fn_ir.inferred_ret_ty;
            if h != TypeState::unknown() && inferred != TypeState::unknown() {
                let clash = h.prim != PrimTy::Any
                    && inferred.prim != PrimTy::Any
                    && h.prim != inferred.prim;
                if clash {
                    type_errors.push(
                        DiagnosticBuilder::new(
                            "RR.TypeError",
                            RRCode::E1010,
                            Stage::Mir,
                            format!(
                                "type hint conflict in function '{}': return hint {:?} vs inferred {:?}",
                                name, h, inferred
                            ),
                        )
                        .at(fn_ir.ret_hint_span.unwrap_or(fn_ir.span))
                        .constraint(
                            fn_ir.ret_hint_span.unwrap_or(fn_ir.span),
                            format!("declared return type is constrained to {:?}", h),
                        )
                        .origin(
                            first_return_origin_span(fn_ir).unwrap_or(fn_ir.span),
                            format!("inferred return flow produces {:?}", inferred),
                        )
                        .use_site(
                            fn_ir.span,
                            "function body must satisfy the declared return contract",
                        )
                        .fix(format!(
                            "change the return annotation to {:?}, or return a {:?} value",
                            inferred, h
                        ))
                        .build(),
                    );
                }
            }
        }
    }

    if cfg.mode == TypeMode::Strict {
        type_errors.extend(validate_strict(all_fns));
    }
    finish_diagnostics(
        "RR.TypeError",
        RRCode::E1002,
        Stage::Mir,
        format!("type checking failed: {} error(s)", type_errors.len()),
        type_errors,
    )
}

fn validate_strict(all_fns: &FxHashMap<String, FnIR>) -> Vec<RRException> {
    let mut names: Vec<String> = all_fns.keys().cloned().collect();
    names.sort();
    let mut errors = Vec::new();
    for fname in names {
        let Some(fn_ir) = all_fns.get(&fname) else {
            continue;
        };
        let reachable = compute_reachable(fn_ir);
        let has_explicit_hints = fn_ir.ret_ty_hint.is_some()
            || fn_ir.ret_term_hint.is_some()
            || fn_ir.param_ty_hints.iter().any(|t| !t.is_unknown())
            || fn_ir.param_term_hints.iter().any(|t| !t.is_any());
        for (bid, bb) in fn_ir.blocks.iter().enumerate() {
            if !reachable.get(bid).copied().unwrap_or(false) {
                continue;
            }
            if let Terminator::If { cond, .. } = bb.term {
                let cty = fn_ir.values[cond].value_ty;
                if has_explicit_hints && cty.is_unknown() {
                    errors.push(
                        DiagnosticBuilder::new(
                            "RR.TypeError",
                            RRCode::E1012,
                            Stage::Mir,
                            format!(
                                "strict mode unresolved condition type in function '{}' (value #{})",
                                fname, cond
                            ),
                        )
                        .at(fn_ir.values[cond].span)
                        .origin(
                            fn_ir.values[cond].span,
                            "condition value originates here and has unresolved type facts",
                        )
                        .constraint(
                            fn_ir.span,
                            "strict mode requires branch conditions to be logical scalars",
                        )
                        .use_site(fn_ir.values[cond].span, "used here as an if/while condition")
                        .fix("add an explicit logical type hint or cast before the condition")
                        .build(),
                    );
                }
            }
            for ins in &bb.instrs {
                if let crate::mir::Instr::StoreIndex1D { idx, .. } = ins {
                    let ity = fn_ir.values[*idx].value_ty;
                    if has_explicit_hints && ity.is_unknown() {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.TypeError",
                                RRCode::E1012,
                                Stage::Mir,
                                format!(
                                    "strict mode unresolved index type in function '{}' (value #{})",
                                    fname, idx
                                ),
                            )
                            .at(fn_ir.values[*idx].span)
                            .origin(
                                fn_ir.values[*idx].span,
                                "index value originates here and has unresolved type facts",
                            )
                            .constraint(
                                fn_ir.span,
                                "strict mode requires assignment indices to be integer scalars",
                            )
                            .use_site(fn_ir.values[*idx].span, "used here as an index")
                            .fix("add an explicit integer type hint or cast before indexing")
                            .build(),
                        );
                    }
                }
                if let crate::mir::Instr::StoreIndex2D { base, r, c, .. } = ins
                    && has_explicit_hints
                {
                    let base_ty = fn_ir.values[*base].value_ty;
                    if base_ty.shape != ShapeTy::Unknown && base_ty.shape != ShapeTy::Matrix {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.TypeError",
                                RRCode::E1002,
                                Stage::Mir,
                                format!(
                                    "strict mode 2D assignment requires matrix-typed base in function '{}' (got {:?})",
                                    fname, base_ty
                                ),
                            )
                            .at(fn_ir.values[*base].span)
                            .constraint(
                                fn_ir.values[*base].span,
                                "2D assignment requires a matrix-typed base",
                            )
                            .use_site(fn_ir.values[*base].span, "used here as a 2D assignment base")
                            .fix("change the base type hint to matrix<T>, or use 1D indexing")
                            .build(),
                        );
                    }
                    for idx in [r, c] {
                        let ity = fn_ir.values[*idx].value_ty;
                        if ity.is_unknown() {
                            errors.push(
                                DiagnosticBuilder::new(
                                    "RR.TypeError",
                                    RRCode::E1012,
                                    Stage::Mir,
                                    format!(
                                        "strict mode unresolved 2D index type in function '{}' (value #{})",
                                        fname, idx
                                    ),
                                )
                                .at(fn_ir.values[*idx].span)
                                .constraint(
                                    fn_ir.span,
                                    "strict mode requires matrix indices to be integer scalars",
                                )
                                .use_site(fn_ir.values[*idx].span, "used here as a matrix index")
                                .fix("add an explicit integer type hint or cast before indexing")
                                .build(),
                            );
                        }
                    }
                }
            }
        }

        for v in &fn_ir.values {
            if has_explicit_hints && let ValueKind::Index2D { base, r, c } = &v.kind {
                let base_ty = fn_ir.values[*base].value_ty;
                if base_ty.shape != ShapeTy::Unknown && base_ty.shape != ShapeTy::Matrix {
                    errors.push(
                        DiagnosticBuilder::new(
                            "RR.TypeError",
                            RRCode::E1002,
                            Stage::Mir,
                            format!(
                                "strict mode 2D indexing requires matrix-typed base in function '{}' (got {:?})",
                                fname, base_ty
                            ),
                        )
                        .at(v.span)
                        .constraint(v.span, "2D indexing requires a matrix-typed base")
                        .use_site(v.span, "used here as a 2D indexing expression")
                        .fix("change the base type hint to matrix<T>, or use 1D indexing")
                        .build(),
                    );
                }
                for idx in [r, c] {
                    let ity = fn_ir.values[*idx].value_ty;
                    if ity.is_unknown() {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.TypeError",
                                RRCode::E1012,
                                Stage::Mir,
                                format!(
                                    "strict mode unresolved 2D index type in function '{}' (value #{})",
                                    fname, idx
                                ),
                            )
                            .at(fn_ir.values[*idx].span)
                            .constraint(
                                fn_ir.span,
                                "strict mode requires matrix indices to be integer scalars",
                            )
                            .use_site(fn_ir.values[*idx].span, "used here as a matrix index")
                            .fix("add an explicit integer type hint or cast before indexing")
                            .build(),
                        );
                    }
                }
            }
            if has_explicit_hints
                && let ValueKind::Call { callee, args, .. } = &v.kind
                && matches!(callee.as_str(), "rr_field_get" | "rr_field_set")
                && !args.is_empty()
            {
                let base_term = &fn_ir.values[args[0]].value_term;
                let field_name = args.get(1).and_then(|arg| match &fn_ir.values[*arg].kind {
                    ValueKind::Const(crate::syntax::ast::Lit::Str(name)) => Some(name.as_str()),
                    _ => None,
                });
                if let Some(field_name) = field_name
                    && base_term.has_exact_named_fields()
                {
                    let expected_field = base_term.exact_field_value(field_name);
                    if expected_field.is_none() {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.TypeError",
                                RRCode::E1002,
                                Stage::Mir,
                                format!(
                                    "strict mode field '{}' is not present in the visible dataframe schema for function '{}'",
                                    field_name, fname
                                ),
                            )
                            .at(v.span)
                            .constraint(
                                v.span,
                                format!("field '{}' must exist in the dataframe schema", field_name),
                            )
                            .use_site(v.span, "used here as a named dataframe field access")
                            .fix("change the field name or widen/remove the dataframe schema hint")
                            .build(),
                        );
                    } else if callee == "rr_field_set" && args.len() >= 3 {
                        let expected_field = expected_field.unwrap_or(TypeTerm::Any);
                        let got_term = fn_ir.values[args[2]].value_term.clone();
                        if !expected_field.is_any()
                            && !got_term.is_any()
                            && !expected_field.compatible_with(&got_term)
                        {
                            errors.push(
                                DiagnosticBuilder::new(
                                    "RR.TypeError",
                                    RRCode::E1011,
                                    Stage::Mir,
                                    format!(
                                        "dataframe field '{}' expects {:?}, got {:?} in function '{}'",
                                        field_name, expected_field, got_term, fname
                                    ),
                                )
                                .at(v.span)
                                .origin(
                                    fn_ir.values[args[2]].span,
                                    format!("assigned field value is inferred as {:?}", got_term),
                                )
                                .constraint(
                                    v.span,
                                    format!("field '{}' is constrained to {:?}", field_name, expected_field),
                                )
                                .use_site(v.span, "used here as a dataframe field assignment")
                                .fix("cast the assigned value or widen the dataframe schema hint")
                                .build(),
                            );
                        }
                    }
                }
            }
            if let ValueKind::Call { callee, args, .. } = &v.kind
                && let Some(callee_fn) = all_fns.get(callee)
            {
                let argc = args.len().min(callee_fn.param_ty_hints.len());
                for i in 0..argc {
                    let expected_term = callee_fn
                        .param_term_hints
                        .get(i)
                        .cloned()
                        .unwrap_or(TypeTerm::Any);
                    let got_term = fn_ir.values[args[i]].value_term.clone();
                    if !expected_term.is_any()
                        && !got_term.is_any()
                        && !expected_term.compatible_with(&got_term)
                    {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.TypeError",
                                RRCode::E1011,
                                Stage::Mir,
                                format!(
                                    "call signature type mismatch in '{}': arg {} expects {:?}, got {:?}",
                                    callee,
                                    i + 1,
                                    expected_term,
                                    got_term
                                ),
                            )
                            .at(v.span)
                            .origin(
                                fn_ir.values[args[i]].span,
                                format!("argument {} originates here with inferred type {:?}", i + 1, got_term),
                            )
                            .constraint(
                                callee_fn
                                    .param_hint_spans
                                    .get(i)
                                    .and_then(|s| *s)
                                    .or_else(|| callee_fn.param_spans.get(i).copied())
                                    .unwrap_or(callee_fn.span),
                                format!("callee parameter {} requires {:?}", i + 1, expected_term),
                            )
                            .use_site(v.span, "call site uses the argument here")
                            .fix(format!(
                                "cast argument {} or change the callee parameter annotation to a compatible type",
                                i + 1
                            ))
                            .build(),
                        );
                    }

                    let expected = callee_fn.param_ty_hints[i];
                    let got = fn_ir.values[args[i]].value_ty;
                    if !is_arg_compatible(expected, got) {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.TypeError",
                                RRCode::E1011,
                                Stage::Mir,
                                format!(
                                    "call signature type mismatch in '{}': arg {} expects {:?}, got {:?}",
                                    callee,
                                    i + 1,
                                    expected,
                                    got
                                ),
                            )
                            .at(v.span)
                            .origin(
                                fn_ir.values[args[i]].span,
                                format!("argument {} originates here with inferred type {:?}", i + 1, got),
                            )
                            .constraint(
                                callee_fn
                                    .param_hint_spans
                                    .get(i)
                                    .and_then(|s| *s)
                                    .or_else(|| callee_fn.param_spans.get(i).copied())
                                    .unwrap_or(callee_fn.span),
                                format!("callee parameter {} requires {:?}", i + 1, expected),
                            )
                            .use_site(v.span, "call site uses the argument here")
                            .fix(format!(
                                "cast argument {} or change the callee parameter annotation to a compatible type",
                                i + 1
                            ))
                            .build(),
                        );
                    }
                }
            }
        }
    }
    errors
}

fn first_return_origin_span(fn_ir: &FnIR) -> Option<crate::utils::Span> {
    let reachable = compute_reachable(fn_ir);
    for (bid, bb) in fn_ir.blocks.iter().enumerate() {
        if !reachable.get(bid).copied().unwrap_or(false) {
            continue;
        }
        if let Terminator::Return(Some(val)) = bb.term {
            return Some(fn_ir.values[val].span);
        }
    }
    None
}

fn analyze_function(fn_ir: &mut FnIR, fn_ret: &FxHashMap<String, TypeState>) -> RR<TypeState> {
    seed_param_len_symbols(fn_ir);
    let mut changed = true;
    let mut guard = 0usize;

    while changed && guard < 32 {
        guard += 1;
        changed = false;
        let var_tys = collect_var_types(fn_ir);
        for vid in 0..fn_ir.values.len() {
            let old = fn_ir.values[vid].value_ty;
            let new = infer_value_type(fn_ir, vid, fn_ret, &var_tys);
            let joined = old.join(new);
            if joined != old {
                fn_ir.values[vid].value_ty = joined;
                changed = true;
            }
        }
    }

    let mut ret_ty = TypeState::unknown();
    let reachable = compute_reachable(fn_ir);
    for (bid, bb) in fn_ir.blocks.iter().enumerate() {
        if !reachable.get(bid).copied().unwrap_or(false) {
            continue;
        }
        if let Terminator::Return(Some(v)) = bb.term {
            ret_ty = ret_ty.join(fn_ir.values[v].value_ty);
        }
    }

    // Use return hint only when no return value was observed in reachable blocks.
    if ret_ty == TypeState::unknown()
        && let Some(h) = fn_ir.ret_ty_hint
    {
        ret_ty = h;
    }

    Ok(ret_ty)
}

fn seed_param_len_symbols(fn_ir: &mut FnIR) {
    for (idx, hint) in fn_ir.param_ty_hints.iter_mut().enumerate() {
        if hint.len_sym.is_none() && matches!(hint.shape, ShapeTy::Vector | ShapeTy::Matrix) {
            *hint = hint.with_len(Some(LenSym((idx as u32).saturating_add(1))));
        }
    }
}

fn collect_var_types(fn_ir: &FnIR) -> FxHashMap<String, TypeState> {
    let mut out: FxHashMap<String, TypeState> = FxHashMap::default();
    for bb in &fn_ir.blocks {
        for ins in &bb.instrs {
            match ins {
                Instr::Assign { dst, src, .. } => {
                    let src_ty = fn_ir.values[*src].value_ty;
                    out.entry(dst.clone())
                        .and_modify(|acc| *acc = acc.join(src_ty))
                        .or_insert(src_ty);
                }
                Instr::StoreIndex1D { base, val, .. } => {
                    let Some(base_var) = value_base_var_name(fn_ir, *base) else {
                        continue;
                    };
                    let elem_ty = fn_ir.values[*val].value_ty;
                    let mut container_ty =
                        TypeState::vector(elem_ty.prim, elem_ty.na == NaTy::Never);
                    let len_sym = out
                        .get(&base_var)
                        .and_then(|ty| ty.len_sym)
                        .or(fn_ir.values[*base].value_ty.len_sym);
                    container_ty = container_ty.with_len(len_sym);
                    out.entry(base_var)
                        .and_modify(|acc| *acc = acc.join(container_ty))
                        .or_insert(container_ty);
                }
                Instr::StoreIndex2D { base, val, .. } => {
                    let Some(base_var) = value_base_var_name(fn_ir, *base) else {
                        continue;
                    };
                    let elem_ty = fn_ir.values[*val].value_ty;
                    let mut container_ty =
                        TypeState::matrix(elem_ty.prim, elem_ty.na == NaTy::Never);
                    let len_sym = out
                        .get(&base_var)
                        .and_then(|ty| ty.len_sym)
                        .or(fn_ir.values[*base].value_ty.len_sym);
                    container_ty = container_ty.with_len(len_sym);
                    out.entry(base_var)
                        .and_modify(|acc| *acc = acc.join(container_ty))
                        .or_insert(container_ty);
                }
                Instr::StoreIndex3D { base, val, .. } => {
                    let Some(base_var) = value_base_var_name(fn_ir, *base) else {
                        continue;
                    };
                    let elem_ty = fn_ir.values[*val].value_ty;
                    let mut container_ty =
                        TypeState::matrix(elem_ty.prim, elem_ty.na == NaTy::Never);
                    let len_sym = out
                        .get(&base_var)
                        .and_then(|ty| ty.len_sym)
                        .or(fn_ir.values[*base].value_ty.len_sym);
                    container_ty = container_ty.with_len(len_sym);
                    out.entry(base_var)
                        .and_modify(|acc| *acc = acc.join(container_ty))
                        .or_insert(container_ty);
                }
                Instr::Eval { .. } => {}
            }
        }
    }
    out
}

fn value_base_var_name(fn_ir: &FnIR, vid: ValueId) -> Option<String> {
    fn rec(fn_ir: &FnIR, vid: ValueId, seen: &mut FxHashSet<ValueId>) -> Option<String> {
        if !seen.insert(vid) {
            return None;
        }
        match &fn_ir.values.get(vid)?.kind {
            ValueKind::Load { var } => Some(var.clone()),
            ValueKind::Param { index } => fn_ir.params.get(*index).cloned(),
            ValueKind::Phi { args } => {
                let mut out: Option<String> = None;
                let mut saw = false;
                for (a, _) in args {
                    if *a == vid {
                        continue;
                    }
                    let name = rec(fn_ir, *a, seen)?;
                    saw = true;
                    match &out {
                        None => out = Some(name),
                        Some(prev) if prev == &name => {}
                        Some(_) => return None,
                    }
                }
                if saw { out } else { None }
            }
            _ => None,
        }
    }
    rec(fn_ir, vid, &mut FxHashSet::default())
}

fn is_floor_like_single_positional_call(
    callee: &str,
    args: &[ValueId],
    names: &[Option<String>],
) -> bool {
    matches!(callee, "floor" | "ceiling" | "trunc" | "round")
        && args.len() == 1
        && names.first().map(|name| name.is_none()).unwrap_or(true)
}

fn param_slot_for_value(fn_ir: &FnIR, vid: ValueId) -> Option<usize> {
    fn resolve_var_alias_slot(
        fn_ir: &FnIR,
        var: &str,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<usize> {
        if !seen_vars.insert(var.to_string()) {
            return None;
        }
        let mut slot: Option<usize> = None;
        let mut found = false;
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                found = true;
                let src_slot = resolve_value_slot(fn_ir, *src, seen_vals, seen_vars)?;
                match slot {
                    None => slot = Some(src_slot),
                    Some(prev) if prev == src_slot => {}
                    Some(_) => return None,
                }
            }
        }
        if found { slot } else { None }
    }

    fn resolve_value_slot(
        fn_ir: &FnIR,
        vid: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<usize> {
        if !seen_vals.insert(vid) {
            return None;
        }
        match &fn_ir.values.get(vid)?.kind {
            ValueKind::Param { index } => Some(*index),
            ValueKind::Load { var } => fn_ir
                .params
                .iter()
                .position(|p| p == var)
                .or_else(|| resolve_var_alias_slot(fn_ir, var, seen_vals, seen_vars)),
            ValueKind::Phi { args } => {
                let mut out: Option<usize> = None;
                let mut saw = false;
                for (a, _) in args {
                    if *a == vid {
                        continue;
                    }
                    let slot = resolve_value_slot(fn_ir, *a, seen_vals, seen_vars)?;
                    saw = true;
                    match out {
                        None => out = Some(slot),
                        Some(prev) if prev == slot => {}
                        Some(_) => return None,
                    }
                }
                if saw { out } else { None }
            }
            _ => None,
        }
    }

    resolve_value_slot(
        fn_ir,
        vid,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}

fn collect_index_vector_param_slots(fn_ir: &FnIR) -> FxHashSet<usize> {
    let mut slots = FxHashSet::default();
    for v in &fn_ir.values {
        let ValueKind::Call {
            callee,
            args,
            names,
        } = &v.kind
        else {
            continue;
        };
        if callee == "rr_index1_read_idx" && !args.is_empty() {
            if let Some(slot) = param_slot_for_value(fn_ir, args[0]) {
                slots.insert(slot);
            }
            continue;
        }
        if !is_floor_like_single_positional_call(callee, args, names) {
            continue;
        }
        let Some(inner) = args.first().copied() else {
            continue;
        };
        match &fn_ir.values[inner].kind {
            ValueKind::Index1D { base, .. } => {
                if let Some(slot) = param_slot_for_value(fn_ir, *base) {
                    slots.insert(slot);
                }
            }
            ValueKind::Call {
                callee: inner_callee,
                args: inner_args,
                names: inner_names,
            } if matches!(
                inner_callee.as_str(),
                "rr_index1_read" | "rr_index1_read_strict" | "rr_index1_read_floor"
            ) && (inner_args.len() == 2 || inner_args.len() == 3)
                && inner_names.iter().take(2).all(std::option::Option::is_none) =>
            {
                if let Some(slot) = param_slot_for_value(fn_ir, inner_args[0]) {
                    slots.insert(slot);
                }
            }
            _ => {}
        }
    }
    slots
}

fn collect_index_vector_param_slots_by_function(
    all_fns: &FxHashMap<String, FnIR>,
) -> FxHashMap<String, FxHashSet<usize>> {
    index_demands::collect_index_vector_param_slots_by_function(all_fns)
}

fn collect_scalar_index_return_demands(all_fns: &FxHashMap<String, FnIR>) -> FxHashSet<String> {
    index_demands::collect_scalar_index_return_demands(all_fns)
}

fn collect_vector_index_return_demands(
    all_fns: &FxHashMap<String, FnIR>,
    index_param_slots: &FxHashMap<String, FxHashSet<usize>>,
) -> FxHashSet<String> {
    index_demands::collect_vector_index_return_demands(all_fns, index_param_slots)
}

fn can_apply_index_return_override(
    all_fns: &FxHashMap<String, FnIR>,
    fname: &str,
    demanded_shape: ShapeTy,
    demanded_term: &TypeTerm,
) -> bool {
    index_demands::can_apply_index_return_override(all_fns, fname, demanded_shape, demanded_term)
}

fn coerce_index_scalar_return(ty: TypeState) -> TypeState {
    index_demands::coerce_index_scalar_return(ty)
}

fn coerce_index_vector_return(ty: TypeState) -> TypeState {
    index_demands::coerce_index_vector_return(ty)
}

fn apply_index_return_demands(
    all_fns: &FxHashMap<String, FnIR>,
    fn_ret: &mut FxHashMap<String, TypeState>,
    fn_ret_term: &mut FxHashMap<String, TypeTerm>,
    scalar_demands: &FxHashSet<String>,
    vector_demands: &FxHashSet<String>,
) -> bool {
    index_demands::apply_index_return_demands(
        all_fns,
        fn_ret,
        fn_ret_term,
        scalar_demands,
        vector_demands,
    )
}

fn analyze_function_terms(fn_ir: &mut FnIR, fn_ret: &FxHashMap<String, TypeTerm>) -> TypeTerm {
    terms::analyze_function_terms(fn_ir, fn_ret)
}

fn infer_value_term(fn_ir: &FnIR, vid: ValueId, fn_ret: &FxHashMap<String, TypeTerm>) -> TypeTerm {
    terms::infer_value_term(fn_ir, vid, fn_ret)
}

fn infer_value_type(
    fn_ir: &FnIR,
    vid: ValueId,
    fn_ret: &FxHashMap<String, TypeState>,
    var_tys: &FxHashMap<String, TypeState>,
) -> TypeState {
    let val = &fn_ir.values[vid];
    match &val.kind {
        ValueKind::Const(l) => lit_type(l),
        ValueKind::Param { index } => fn_ir
            .param_ty_hints
            .get(*index)
            .copied()
            .map(|ty| refine_type_with_term(ty, &fn_ir.param_term_hints[*index]))
            .unwrap_or(TypeState::unknown()),
        ValueKind::Len { .. } => TypeState::scalar(PrimTy::Int, true),
        ValueKind::Indices { base } => {
            let base_ty = fn_ir.values[*base].value_ty;
            TypeState::vector(PrimTy::Int, true).with_len(base_ty.len_sym)
        }
        ValueKind::Range { .. } => TypeState::vector(PrimTy::Int, true),
        ValueKind::Unary { rhs, .. } => {
            let r = fn_ir.values[*rhs].value_ty;
            TypeState {
                prim: if matches!(r.prim, PrimTy::Int | PrimTy::Double) {
                    r.prim
                } else {
                    PrimTy::Any
                },
                shape: r.shape,
                na: r.na,
                len_sym: r.len_sym,
            }
        }
        ValueKind::Binary { op, lhs, rhs } => {
            let l = fn_ir.values[*lhs].value_ty;
            let r = fn_ir.values[*rhs].value_ty;
            use crate::syntax::ast::BinOp;
            match op {
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    TypeState {
                        prim: PrimTy::Logical,
                        shape: normalize_call_numeric_shape(&[l, r]),
                        na: if l.na == NaTy::Never && r.na == NaTy::Never {
                            NaTy::Never
                        } else {
                            NaTy::Maybe
                        },
                        len_sym: if l.len_sym.is_some() && l.len_sym == r.len_sym {
                            l.len_sym
                        } else {
                            None
                        },
                    }
                }
                BinOp::And | BinOp::Or => TypeState {
                    prim: PrimTy::Logical,
                    shape: normalize_call_numeric_shape(&[l, r]),
                    na: NaTy::Maybe,
                    len_sym: if l.len_sym.is_some() && l.len_sym == r.len_sym {
                        l.len_sym
                    } else {
                        None
                    },
                },
                BinOp::MatMul => TypeState {
                    prim: match promoted_numeric_prim(l.prim, r.prim) {
                        PrimTy::Int | PrimTy::Double => PrimTy::Double,
                        other => other,
                    },
                    shape: if matches!(l.shape, ShapeTy::Vector | ShapeTy::Matrix)
                        && matches!(r.shape, ShapeTy::Vector | ShapeTy::Matrix)
                    {
                        ShapeTy::Matrix
                    } else {
                        ShapeTy::Unknown
                    },
                    na: if l.na == NaTy::Never && r.na == NaTy::Never {
                        NaTy::Never
                    } else {
                        NaTy::Maybe
                    },
                    len_sym: None,
                },
                _ => {
                    let prim = match op {
                        BinOp::Div => match (l.prim, r.prim) {
                            (PrimTy::Int, PrimTy::Int)
                            | (PrimTy::Int, PrimTy::Double)
                            | (PrimTy::Double, PrimTy::Int)
                            | (PrimTy::Double, PrimTy::Double) => PrimTy::Double,
                            (PrimTy::Any, other) | (other, PrimTy::Any) => other,
                            _ => PrimTy::Any,
                        },
                        BinOp::Mod => promoted_numeric_prim(l.prim, r.prim),
                        _ => promoted_numeric_prim(l.prim, r.prim),
                    };
                    TypeState {
                        prim,
                        shape: normalize_call_numeric_shape(&[l, r]),
                        na: if l.na == NaTy::Never && r.na == NaTy::Never {
                            NaTy::Never
                        } else {
                            NaTy::Maybe
                        },
                        len_sym: if l.len_sym.is_some() && l.len_sym == r.len_sym {
                            l.len_sym
                        } else {
                            None
                        },
                    }
                }
            }
        }
        ValueKind::Phi { args } => {
            let mut out = TypeState::unknown();
            for (a, _) in args {
                out = out.join(fn_ir.values[*a].value_ty);
            }
            out
        }
        ValueKind::Call { callee, args, .. } => {
            if callee == "seq_along" && args.len() == 1 {
                let base_ty = fn_ir.values[args[0]].value_ty;
                return TypeState::vector(PrimTy::Int, true).with_len(base_ty.len_sym);
            }
            if callee == "seq_len" && args.len() == 1 {
                let len_sym = match &fn_ir.values[args[0]].kind {
                    ValueKind::Len { base } => fn_ir.values[*base].value_ty.len_sym,
                    _ => None,
                };
                return TypeState::vector(PrimTy::Int, true).with_len(len_sym);
            }
            if callee == "rr_field_get" && !args.is_empty() {
                let field_name = args.get(1).and_then(|arg| match &fn_ir.values[*arg].kind {
                    ValueKind::Const(crate::syntax::ast::Lit::Str(name)) => Some(name.as_str()),
                    _ => None,
                });
                return type_state_from_term(
                    &fn_ir.values[args[0]]
                        .value_term
                        .field_value_named(field_name),
                );
            }
            if callee == "rr_field_exists" {
                return TypeState::scalar(PrimTy::Logical, true);
            }
            if callee == "rr_field_set" && !args.is_empty() {
                let field_name = args.get(1).and_then(|arg| match &fn_ir.values[*arg].kind {
                    ValueKind::Const(crate::syntax::ast::Lit::Str(name)) => Some(name.as_str()),
                    _ => None,
                });
                if let (Some(name), Some(value)) = (field_name, args.get(2)) {
                    let updated = fn_ir.values[args[0]]
                        .value_term
                        .updated_field_value_named(name, &fn_ir.values[*value].value_term);
                    return refine_type_with_term(fn_ir.values[args[0]].value_ty, &updated);
                }
                return fn_ir.values[args[0]].value_ty;
            }
            let arg_tys: Vec<TypeState> = args.iter().map(|a| fn_ir.values[*a].value_ty).collect();
            let arg_terms: Vec<TypeTerm> = args
                .iter()
                .map(|a| fn_ir.values[*a].value_term.clone())
                .collect();
            if let Some(b) = infer_builtin(callee, &arg_tys) {
                let with_builtin_term = if let Some(term) = infer_builtin_term(callee, &arg_terms) {
                    refine_type_with_term(b, &term)
                } else {
                    b
                };
                return refine_type_with_term(with_builtin_term, &fn_ir.values[vid].value_term);
            }
            if callee.starts_with("Sym_") {
                return fn_ret.get(callee).copied().unwrap_or(TypeState::unknown());
            }
            TypeState::unknown()
        }
        ValueKind::Index1D { base, .. } => {
            let b = fn_ir.values[*base].value_ty;
            refine_type_with_term(
                TypeState {
                    prim: b.prim,
                    shape: ShapeTy::Scalar,
                    na: NaTy::Maybe,
                    len_sym: None,
                },
                &fn_ir.values[*base].value_term.index_element(),
            )
        }
        ValueKind::Index2D { base, .. } => {
            let b = fn_ir.values[*base].value_ty;
            refine_type_with_term(
                TypeState {
                    prim: b.prim,
                    shape: ShapeTy::Scalar,
                    na: NaTy::Maybe,
                    len_sym: None,
                },
                &fn_ir.values[*base].value_term.index_element(),
            )
        }
        ValueKind::Index3D { base, .. } => {
            let b = fn_ir.values[*base].value_ty;
            refine_type_with_term(
                TypeState {
                    prim: b.prim,
                    shape: ShapeTy::Scalar,
                    na: NaTy::Maybe,
                    len_sym: None,
                },
                &fn_ir.values[*base].value_term.index_element(),
            )
        }
        ValueKind::Load { var } => {
            let ty = var_tys.get(var).copied().unwrap_or(TypeState::unknown());
            refine_type_with_term(
                ty,
                fn_ir
                    .values
                    .get(vid)
                    .map(|v| &v.value_term)
                    .unwrap_or(&TypeTerm::Any),
            )
        }
        ValueKind::RSymbol { .. } => TypeState::unknown(),
        ValueKind::Intrinsic { op, args } => {
            use crate::mir::IntrinsicOp;
            match op {
                IntrinsicOp::VecSumF64 | IntrinsicOp::VecMeanF64 => {
                    TypeState::scalar(PrimTy::Double, false)
                }
                IntrinsicOp::VecAbsF64 => {
                    let prim = args
                        .first()
                        .map(|arg| fn_ir.values[*arg].value_ty.prim)
                        .unwrap_or(PrimTy::Double);
                    TypeState::vector(
                        if matches!(prim, PrimTy::Int | PrimTy::Double) {
                            prim
                        } else {
                            PrimTy::Double
                        },
                        false,
                    )
                }
                IntrinsicOp::VecPmaxF64 | IntrinsicOp::VecPminF64 => {
                    let prim = args
                        .iter()
                        .map(|arg| fn_ir.values[*arg].value_ty.prim)
                        .fold(PrimTy::Any, promoted_numeric_prim);
                    TypeState::vector(
                        if matches!(prim, PrimTy::Int | PrimTy::Double) {
                            prim
                        } else {
                            PrimTy::Double
                        },
                        false,
                    )
                }
                _ => {
                    let mut out = TypeState::vector(PrimTy::Double, false);
                    if args.is_empty() {
                        out.shape = ShapeTy::Unknown;
                    }
                    out
                }
            }
        }
    }
}

pub fn hir_ty_to_type_state(ty: &Ty) -> TypeState {
    from_hir_ty(ty)
}

pub fn hir_ty_to_type_term(ty: &Ty) -> TypeTerm {
    from_hir_ty_term(ty)
}

pub fn hir_ty_to_type_term_with_symbols(
    ty: &Ty,
    symbols: &FxHashMap<crate::hir::def::SymbolId, String>,
) -> TypeTerm {
    term_from_hir_ty_with_symbols(ty, symbols)
}

fn compute_reachable(fn_ir: &FnIR) -> Vec<bool> {
    let mut reachable = vec![false; fn_ir.blocks.len()];
    if fn_ir.entry >= fn_ir.blocks.len() {
        return reachable;
    }
    let mut work = VecDeque::new();
    reachable[fn_ir.entry] = true;
    work.push_back(fn_ir.entry);

    while let Some(bb) = work.pop_front() {
        match fn_ir.blocks[bb].term {
            Terminator::Goto(t) => {
                if t < fn_ir.blocks.len() && !reachable[t] {
                    reachable[t] = true;
                    work.push_back(t);
                }
            }
            Terminator::If {
                then_bb, else_bb, ..
            } => {
                if then_bb < fn_ir.blocks.len() && !reachable[then_bb] {
                    reachable[then_bb] = true;
                    work.push_back(then_bb);
                }
                if else_bb < fn_ir.blocks.len() && !reachable[else_bb] {
                    reachable[else_bb] = true;
                    work.push_back(else_bb);
                }
            }
            Terminator::Return(_) | Terminator::Unreachable => {}
        }
    }

    reachable
}

fn is_arg_compatible(expected: TypeState, got: TypeState) -> bool {
    if expected.prim == PrimTy::Any || got.prim == PrimTy::Any {
        return true;
    }
    if expected.prim == got.prim {
        return true;
    }
    // Numeric widening accepted in strict call checking.
    matches!((expected.prim, got.prim), (PrimTy::Double, PrimTy::Int))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::Facts;
    use crate::syntax::ast::Lit;

    fn init_entry(fn_ir: &mut FnIR) {
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;
    }

    #[test]
    fn analyze_program_propagates_scalar_index_return_demand() {
        let mut producer = FnIR::new("Sym_1".to_string(), vec!["x".to_string()]);
        init_entry(&mut producer);
        let prod_param = producer.add_value(
            ValueKind::Param { index: 0 },
            crate::utils::Span::dummy(),
            Facts::empty(),
            Some("x".to_string()),
        );
        producer.blocks[producer.entry].term = Terminator::Return(Some(prod_param));

        let mut consumer = FnIR::new(
            "Sym_2".to_string(),
            vec!["arr".to_string(), "seed".to_string()],
        );
        init_entry(&mut consumer);
        let arr = consumer.add_value(
            ValueKind::Param { index: 0 },
            crate::utils::Span::dummy(),
            Facts::empty(),
            Some("arr".to_string()),
        );
        let seed = consumer.add_value(
            ValueKind::Param { index: 1 },
            crate::utils::Span::dummy(),
            Facts::empty(),
            Some("seed".to_string()),
        );
        let call_idx = consumer.add_value(
            ValueKind::Call {
                callee: "Sym_1".to_string(),
                args: vec![seed],
                names: vec![None],
            },
            crate::utils::Span::dummy(),
            Facts::empty(),
            None,
        );
        let read = consumer.add_value(
            ValueKind::Index1D {
                base: arr,
                idx: call_idx,
                is_safe: false,
                is_na_safe: false,
            },
            crate::utils::Span::dummy(),
            Facts::empty(),
            None,
        );
        consumer.blocks[consumer.entry].term = Terminator::Return(Some(read));

        let mut all_fns: FxHashMap<String, FnIR> = FxHashMap::default();
        all_fns.insert("Sym_1".to_string(), producer);
        all_fns.insert("Sym_2".to_string(), consumer);

        analyze_program(
            &mut all_fns,
            TypeConfig {
                mode: TypeMode::Gradual,
                native_backend: NativeBackend::Off,
            },
        )
        .expect("type analysis should succeed");

        let consumer_after = all_fns.get("Sym_2").expect("missing Sym_2");
        let call_ty = consumer_after.values[call_idx].value_ty;
        assert_eq!(call_ty.shape, ShapeTy::Scalar);
        assert_eq!(call_ty.prim, PrimTy::Int);
    }

    #[test]
    fn analyze_program_propagates_vector_index_return_demand() {
        let mut producer = FnIR::new("Sym_10".to_string(), vec!["x".to_string()]);
        init_entry(&mut producer);
        let prod_param = producer.add_value(
            ValueKind::Param { index: 0 },
            crate::utils::Span::dummy(),
            Facts::empty(),
            Some("x".to_string()),
        );
        producer.blocks[producer.entry].term = Terminator::Return(Some(prod_param));

        let mut kernel = FnIR::new(
            "Sym_20".to_string(),
            vec!["arr".to_string(), "idx_vec".to_string()],
        );
        init_entry(&mut kernel);
        let arr = kernel.add_value(
            ValueKind::Param { index: 0 },
            crate::utils::Span::dummy(),
            Facts::empty(),
            Some("arr".to_string()),
        );
        let idx_vec = kernel.add_value(
            ValueKind::Param { index: 1 },
            crate::utils::Span::dummy(),
            Facts::empty(),
            Some("idx_vec".to_string()),
        );
        let one = kernel.add_value(
            ValueKind::Const(Lit::Int(1)),
            crate::utils::Span::dummy(),
            Facts::empty(),
            None,
        );
        let idx_read = kernel.add_value(
            ValueKind::Index1D {
                base: idx_vec,
                idx: one,
                is_safe: false,
                is_na_safe: false,
            },
            crate::utils::Span::dummy(),
            Facts::empty(),
            None,
        );
        let floored = kernel.add_value(
            ValueKind::Call {
                callee: "floor".to_string(),
                args: vec![idx_read],
                names: vec![None],
            },
            crate::utils::Span::dummy(),
            Facts::empty(),
            None,
        );
        let gather = kernel.add_value(
            ValueKind::Index1D {
                base: arr,
                idx: floored,
                is_safe: false,
                is_na_safe: false,
            },
            crate::utils::Span::dummy(),
            Facts::empty(),
            None,
        );
        kernel.blocks[kernel.entry].term = Terminator::Return(Some(gather));

        let mut wrapper = FnIR::new(
            "Sym_30".to_string(),
            vec!["arr".to_string(), "seed".to_string()],
        );
        init_entry(&mut wrapper);
        let wrapper_arr = wrapper.add_value(
            ValueKind::Param { index: 0 },
            crate::utils::Span::dummy(),
            Facts::empty(),
            Some("arr".to_string()),
        );
        let wrapper_seed = wrapper.add_value(
            ValueKind::Param { index: 1 },
            crate::utils::Span::dummy(),
            Facts::empty(),
            Some("seed".to_string()),
        );
        let call_idx_vec = wrapper.add_value(
            ValueKind::Call {
                callee: "Sym_10".to_string(),
                args: vec![wrapper_seed],
                names: vec![None],
            },
            crate::utils::Span::dummy(),
            Facts::empty(),
            None,
        );
        let call_kernel = wrapper.add_value(
            ValueKind::Call {
                callee: "Sym_20".to_string(),
                args: vec![wrapper_arr, call_idx_vec],
                names: vec![None, None],
            },
            crate::utils::Span::dummy(),
            Facts::empty(),
            None,
        );
        wrapper.blocks[wrapper.entry].term = Terminator::Return(Some(call_kernel));

        let mut all_fns: FxHashMap<String, FnIR> = FxHashMap::default();
        all_fns.insert("Sym_10".to_string(), producer);
        all_fns.insert("Sym_20".to_string(), kernel);
        all_fns.insert("Sym_30".to_string(), wrapper);

        let index_slots = collect_index_vector_param_slots_by_function(&all_fns);
        assert!(
            index_slots
                .get("Sym_20")
                .is_some_and(|slots| slots.contains(&1)),
            "expected Sym_20 arg #2 to be detected as index-vector parameter"
        );
        let vec_demands = collect_vector_index_return_demands(&all_fns, &index_slots);
        assert!(
            vec_demands.contains("Sym_10"),
            "expected Sym_10 return to be demanded as index-vector producer"
        );

        analyze_program(
            &mut all_fns,
            TypeConfig {
                mode: TypeMode::Gradual,
                native_backend: NativeBackend::Off,
            },
        )
        .expect("type analysis should succeed");

        let wrapper_after = all_fns.get("Sym_30").expect("missing Sym_30");
        let call_ty = wrapper_after.values[call_idx_vec].value_ty;
        assert_eq!(call_ty.shape, ShapeTy::Vector);
        assert_eq!(call_ty.prim, PrimTy::Int);
    }
}
