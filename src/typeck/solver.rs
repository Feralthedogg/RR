use crate::error::{RR, RRCode, RRException, Stage};
use crate::hir::def::Ty;
use crate::mir::{FnIR, Terminator, ValueId, ValueKind};
use rustc_hash::FxHashMap;
use std::collections::VecDeque;

use super::builtin_sigs::{infer_builtin, infer_builtin_term};
use super::constraints::{ConstraintSet, TypeConstraint};
use super::lattice::{NaTy, PrimTy, ShapeTy, TypeState};
use super::term::{TypeTerm, from_hir_ty as term_from_hir_ty, from_lit as lit_term};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeMode {
    Strict,
    Gradual,
}

impl TypeMode {
    pub fn from_str(v: &str) -> Option<Self> {
        match v.trim().to_ascii_lowercase().as_str() {
            "strict" => Some(Self::Strict),
            "gradual" => Some(Self::Gradual),
            _ => None,
        }
    }

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
    pub fn from_str(v: &str) -> Option<Self> {
        match v.trim().to_ascii_lowercase().as_str() {
            "off" => Some(Self::Off),
            "optional" => Some(Self::Optional),
            "required" => Some(Self::Required),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Optional => "optional",
            Self::Required => "required",
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
        Ty::List(_)
        | Ty::Box(_)
        | Ty::DataFrame(_)
        | Ty::Union(_)
        | Ty::Option(_)
        | Ty::Result(_, _) => TypeState::vector(PrimTy::Any, false),
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
    if args.iter().any(|a| a.shape == ShapeTy::Vector) {
        ShapeTy::Vector
    } else {
        ShapeTy::Scalar
    }
}

pub fn analyze_program(all_fns: &mut FxHashMap<String, FnIR>, cfg: TypeConfig) -> RR<()> {
    let mut fn_ret: FxHashMap<String, TypeState> = FxHashMap::default();
    let mut fn_ret_term: FxHashMap<String, TypeTerm> = FxHashMap::default();
    for (name, fn_ir) in all_fns.iter() {
        fn_ret.insert(
            name.clone(),
            fn_ir.ret_ty_hint.unwrap_or(TypeState::unknown()),
        );
        fn_ret_term.insert(
            name.clone(),
            fn_ir.ret_term_hint.clone().unwrap_or(TypeTerm::Any),
        );
    }

    let mut changed = true;
    let mut guard = 0usize;
    while changed && guard < 16 {
        guard += 1;
        changed = false;
        let names: Vec<String> = all_fns.keys().cloned().collect();
        for name in names {
            let fn_ir = all_fns.get_mut(&name).expect("fn exists");
            let ret = analyze_function(fn_ir, &fn_ret)?;
            let ret_term = analyze_function_terms(fn_ir, &fn_ret_term);
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
    }

    for (name, fn_ir) in all_fns.iter() {
        if cfg.mode != TypeMode::Strict {
            continue;
        }

        if let Some(h) = &fn_ir.ret_term_hint {
            let inferred_term = &fn_ir.inferred_ret_term;
            if !h.is_any() && !inferred_term.is_any() && !h.compatible_with(inferred_term) {
                return Err(RRException::new(
                    "RR.TypeError",
                    RRCode::E1010,
                    Stage::Mir,
                    format!(
                        "type hint conflict in function '{}': return hint {:?} vs inferred {:?}",
                        name, h, inferred_term
                    ),
                )
                .note("Use a compatible return type or remove conflicting annotation."));
            }
        } else if let Some(h) = fn_ir.ret_ty_hint {
            // Backward-compatible primitive-only clash check when structural hint is absent.
            let inferred = fn_ir.inferred_ret_ty;
            if h != TypeState::unknown() && inferred != TypeState::unknown() {
                let clash = h.prim != PrimTy::Any
                    && inferred.prim != PrimTy::Any
                    && h.prim != inferred.prim;
                if clash {
                    return Err(
                        RRException::new(
                            "RR.TypeError",
                            RRCode::E1010,
                            Stage::Mir,
                            format!(
                                "type hint conflict in function '{}': return hint {:?} vs inferred {:?}",
                                name, h, inferred
                            ),
                        )
                        .note("Use a compatible return type or remove conflicting annotation."),
                    );
                }
            }
        }
    }

    if cfg.mode == TypeMode::Strict {
        validate_strict(all_fns)?;
    }

    Ok(())
}

fn validate_strict(all_fns: &FxHashMap<String, FnIR>) -> RR<()> {
    for (fname, fn_ir) in all_fns {
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
                    return Err(RRException::new(
                        "RR.TypeError",
                        RRCode::E1012,
                        Stage::Mir,
                        format!(
                            "strict mode unresolved condition type in function '{}' (value #{})",
                            fname, cond
                        ),
                    )
                    .note("Add a logical type hint or simplify condition expression."));
                }
            }
            for ins in &bb.instrs {
                match ins {
                    crate::mir::Instr::StoreIndex1D { idx, .. } => {
                        let ity = fn_ir.values[*idx].value_ty;
                        if has_explicit_hints && ity.is_unknown() {
                            return Err(
                                RRException::new(
                                    "RR.TypeError",
                                    RRCode::E1012,
                                    Stage::Mir,
                                    format!(
                                        "strict mode unresolved index type in function '{}' (value #{})",
                                        fname, idx
                                    ),
                                )
                                .note("Add an integer index hint or explicit cast before indexing."),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        for v in &fn_ir.values {
            if let ValueKind::Call { callee, args, .. } = &v.kind {
                if let Some(callee_fn) = all_fns.get(callee) {
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
                            return Err(RRException::new(
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
                            ));
                        }

                        let expected = callee_fn.param_ty_hints[i];
                        let got = fn_ir.values[args[i]].value_ty;
                        if !is_arg_compatible(expected, got) {
                            return Err(RRException::new(
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
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn analyze_function(fn_ir: &mut FnIR, fn_ret: &FxHashMap<String, TypeState>) -> RR<TypeState> {
    let mut changed = true;
    let mut guard = 0usize;

    while changed && guard < 32 {
        guard += 1;
        changed = false;
        for vid in 0..fn_ir.values.len() {
            let old = fn_ir.values[vid].value_ty;
            let new = infer_value_type(fn_ir, vid, fn_ret);
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
    if ret_ty == TypeState::unknown() {
        if let Some(h) = fn_ir.ret_ty_hint {
            ret_ty = h;
        }
    }

    Ok(ret_ty)
}

fn analyze_function_terms(fn_ir: &mut FnIR, fn_ret: &FxHashMap<String, TypeTerm>) -> TypeTerm {
    let mut changed = true;
    let mut guard = 0usize;

    while changed && guard < 32 {
        guard += 1;
        changed = false;
        for vid in 0..fn_ir.values.len() {
            let old = fn_ir.values[vid].value_term.clone();
            let new = infer_value_term(fn_ir, vid, fn_ret);
            let joined = old.join(&new);
            if joined != old {
                fn_ir.values[vid].value_term = joined;
                changed = true;
            }
        }
    }

    // Projection constraints sharpen nested container terms (e.g. List<Box<T>> indexing).
    let mut cs = ConstraintSet::default();
    let vars: Vec<_> = (0..fn_ir.values.len()).map(|_| cs.fresh_var()).collect();
    for (vid, v) in fn_ir.values.iter().enumerate() {
        cs.add(TypeConstraint::Bind(vars[vid], v.value_term.clone()));
        match &v.kind {
            ValueKind::Phi { args } => {
                for (arg, _) in args {
                    cs.add(TypeConstraint::Eq(vars[vid], vars[*arg]));
                }
            }
            ValueKind::Index1D { base, .. } => {
                cs.add(TypeConstraint::ElementOf {
                    container: vars[*base],
                    element: vars[vid],
                });
            }
            ValueKind::Call { callee, args, .. } if callee == "unbox" && !args.is_empty() => {
                cs.add(TypeConstraint::Unbox {
                    boxed: vars[args[0]],
                    value: vars[vid],
                });
            }
            _ => {}
        }
    }
    cs.solve();
    for (vid, slot) in fn_ir.values.iter_mut().enumerate() {
        let resolved = cs.resolve(vars[vid]);
        slot.value_term = slot.value_term.join(&resolved);
    }

    let mut ret_term = TypeTerm::Any;
    let reachable = compute_reachable(fn_ir);
    for (bid, bb) in fn_ir.blocks.iter().enumerate() {
        if !reachable.get(bid).copied().unwrap_or(false) {
            continue;
        }
        if let Terminator::Return(Some(v)) = bb.term {
            ret_term = ret_term.join(&fn_ir.values[v].value_term);
        }
    }

    if ret_term.is_any() {
        if let Some(h) = &fn_ir.ret_term_hint {
            ret_term = h.clone();
        }
    }

    ret_term
}

fn infer_value_term(fn_ir: &FnIR, vid: ValueId, fn_ret: &FxHashMap<String, TypeTerm>) -> TypeTerm {
    let val = &fn_ir.values[vid];
    match &val.kind {
        ValueKind::Const(l) => lit_term(l),
        ValueKind::Param { index } => fn_ir
            .param_term_hints
            .get(*index)
            .cloned()
            .unwrap_or(TypeTerm::Any),
        ValueKind::Len { .. } => TypeTerm::Int,
        ValueKind::Indices { .. } | ValueKind::Range { .. } => {
            TypeTerm::Vector(Box::new(TypeTerm::Int))
        }
        ValueKind::Unary { rhs, .. } => {
            let r = fn_ir.values[*rhs].value_term.clone();
            match r {
                TypeTerm::Int | TypeTerm::Double => r,
                TypeTerm::Vector(inner) => TypeTerm::Vector(inner),
                _ => TypeTerm::Any,
            }
        }
        ValueKind::Binary { op, lhs, rhs } => {
            use crate::syntax::ast::BinOp;
            let l = fn_ir.values[*lhs].value_term.clone();
            let r = fn_ir.values[*rhs].value_term.clone();
            match op {
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    TypeTerm::Logical
                }
                BinOp::And | BinOp::Or => TypeTerm::Logical,
                _ => match (l, r) {
                    (TypeTerm::Double, TypeTerm::Int)
                    | (TypeTerm::Int, TypeTerm::Double)
                    | (TypeTerm::Double, TypeTerm::Double) => TypeTerm::Double,
                    (TypeTerm::Int, TypeTerm::Int) => TypeTerm::Int,
                    (TypeTerm::Vector(a), TypeTerm::Vector(b)) => {
                        TypeTerm::Vector(Box::new(a.join(&b)))
                    }
                    (TypeTerm::Vector(a), b) | (b, TypeTerm::Vector(a)) => {
                        TypeTerm::Vector(Box::new(a.join(&b)))
                    }
                    (TypeTerm::Matrix(a), TypeTerm::Matrix(b)) => {
                        TypeTerm::Matrix(Box::new(a.join(&b)))
                    }
                    _ => TypeTerm::Any,
                },
            }
        }
        ValueKind::Phi { args } => {
            let mut out = TypeTerm::Any;
            for (a, _) in args {
                out = out.join(&fn_ir.values[*a].value_term);
            }
            out
        }
        ValueKind::Call { callee, args, .. } => {
            let arg_terms: Vec<TypeTerm> = args
                .iter()
                .map(|a| fn_ir.values[*a].value_term.clone())
                .collect();
            if let Some(t) = infer_builtin_term(callee, &arg_terms) {
                return t;
            }
            if callee.starts_with("Sym_") {
                return fn_ret.get(callee).cloned().unwrap_or(TypeTerm::Any);
            }
            TypeTerm::Any
        }
        ValueKind::Index1D { base, .. } | ValueKind::Index2D { base, .. } => {
            fn_ir.values[*base].value_term.index_element()
        }
        ValueKind::Load { .. } => TypeTerm::Any,
        ValueKind::Intrinsic { op, args } => {
            use crate::mir::IntrinsicOp;
            match op {
                IntrinsicOp::VecSumF64 | IntrinsicOp::VecMeanF64 => TypeTerm::Double,
                _ => {
                    if args.is_empty() {
                        TypeTerm::Any
                    } else {
                        TypeTerm::Vector(Box::new(TypeTerm::Double))
                    }
                }
            }
        }
    }
}

fn infer_value_type(
    fn_ir: &FnIR,
    vid: ValueId,
    fn_ret: &FxHashMap<String, TypeState>,
) -> TypeState {
    let val = &fn_ir.values[vid];
    match &val.kind {
        ValueKind::Const(l) => lit_type(l),
        ValueKind::Param { index } => fn_ir
            .param_ty_hints
            .get(*index)
            .copied()
            .unwrap_or(TypeState::unknown()),
        ValueKind::Len { .. } => TypeState::scalar(PrimTy::Int, true),
        ValueKind::Indices { .. } => TypeState::vector(PrimTy::Int, true),
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
                        na: NaTy::Maybe,
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
                _ => {
                    let prim = match (l.prim, r.prim) {
                        (PrimTy::Int, PrimTy::Int) => PrimTy::Int,
                        (PrimTy::Int, PrimTy::Double)
                        | (PrimTy::Double, PrimTy::Int)
                        | (PrimTy::Double, PrimTy::Double) => PrimTy::Double,
                        _ => PrimTy::Any,
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
            let arg_tys: Vec<TypeState> = args.iter().map(|a| fn_ir.values[*a].value_ty).collect();
            if let Some(b) = infer_builtin(callee, &arg_tys) {
                return b;
            }
            if callee.starts_with("Sym_") {
                return fn_ret.get(callee).copied().unwrap_or(TypeState::unknown());
            }
            TypeState::unknown()
        }
        ValueKind::Index1D { base, .. } => {
            let b = fn_ir.values[*base].value_ty;
            TypeState {
                prim: b.prim,
                shape: ShapeTy::Scalar,
                na: NaTy::Maybe,
                len_sym: None,
            }
        }
        ValueKind::Index2D { base, .. } => {
            let b = fn_ir.values[*base].value_ty;
            TypeState {
                prim: b.prim,
                shape: ShapeTy::Scalar,
                na: NaTy::Maybe,
                len_sym: None,
            }
        }
        ValueKind::Load { .. } => TypeState::unknown(),
        ValueKind::Intrinsic { op, args } => {
            use crate::mir::IntrinsicOp;
            match op {
                IntrinsicOp::VecSumF64 | IntrinsicOp::VecMeanF64 => {
                    TypeState::scalar(PrimTy::Double, false)
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
