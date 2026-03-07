use crate::error::{InternalCompilerError, RR, RRCode, RRException, Stage};
use crate::hir::def::Ty;
use crate::mir::{FnIR, Instr, Terminator, ValueId, ValueKind};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

use super::builtin_sigs::{infer_builtin, infer_builtin_term};
use super::constraints::{ConstraintSet, TypeConstraint};
use super::lattice::{LenSym, NaTy, PrimTy, ShapeTy, TypeState};
use super::term::{TypeTerm, from_hir_ty as term_from_hir_ty, from_lit as lit_term};

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
    let mut names: Vec<String> = all_fns.keys().cloned().collect();
    names.sort();
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
                if let crate::mir::Instr::StoreIndex1D { idx, .. } = ins {
                    let ity = fn_ir.values[*idx].value_ty;
                    if has_explicit_hints && ity.is_unknown() {
                        return Err(RRException::new(
                            "RR.TypeError",
                            RRCode::E1012,
                            Stage::Mir,
                            format!(
                                "strict mode unresolved index type in function '{}' (value #{})",
                                fname, idx
                            ),
                        )
                        .note("Add an integer index hint or explicit cast before indexing."));
                    }
                }
            }
        }

        for v in &fn_ir.values {
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
    Ok(())
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
    let mut out: FxHashMap<String, FxHashSet<usize>> = FxHashMap::default();
    let mut names: Vec<String> = all_fns.keys().cloned().collect();
    names.sort();
    for name in names {
        let Some(fn_ir) = all_fns.get(&name) else {
            continue;
        };
        let slots = collect_index_vector_param_slots(fn_ir);
        if !slots.is_empty() {
            out.insert(name, slots);
        }
    }
    out
}

fn symbol_callee_for_value(fn_ir: &FnIR, vid: ValueId) -> Option<String> {
    fn resolve_var(
        fn_ir: &FnIR,
        var: &str,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<String> {
        if !seen_vars.insert(var.to_string()) {
            return None;
        }
        let mut out: Option<String> = None;
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
                let sym = resolve_value(fn_ir, *src, seen_vals, seen_vars)?;
                match &out {
                    None => out = Some(sym),
                    Some(prev) if prev == &sym => {}
                    Some(_) => return None,
                }
            }
        }
        if found { out } else { None }
    }

    fn resolve_value(
        fn_ir: &FnIR,
        vid: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<String> {
        if !seen_vals.insert(vid) {
            return None;
        }
        match &fn_ir.values.get(vid)?.kind {
            ValueKind::Call { callee, .. } if callee.starts_with("Sym_") => Some(callee.clone()),
            ValueKind::Load { var } => resolve_var(fn_ir, var, seen_vals, seen_vars),
            ValueKind::Phi { args } => {
                let mut out: Option<String> = None;
                let mut saw = false;
                for (a, _) in args {
                    if *a == vid {
                        continue;
                    }
                    let sym = resolve_value(fn_ir, *a, seen_vals, seen_vars)?;
                    saw = true;
                    match &out {
                        None => out = Some(sym),
                        Some(prev) if prev == &sym => {}
                        Some(_) => return None,
                    }
                }
                if saw { out } else { None }
            }
            _ => None,
        }
    }

    resolve_value(
        fn_ir,
        vid,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}

fn collect_scalar_index_return_demands(all_fns: &FxHashMap<String, FnIR>) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    let mut names: Vec<String> = all_fns.keys().cloned().collect();
    names.sort();
    for name in names {
        let Some(fn_ir) = all_fns.get(&name) else {
            continue;
        };
        let mut scalar_indices = FxHashSet::default();
        for v in &fn_ir.values {
            match &v.kind {
                ValueKind::Index1D { idx, .. } => {
                    scalar_indices.insert(*idx);
                }
                ValueKind::Call { callee, args, .. } if callee == "rr_index1_read_idx" => {
                    if args.len() >= 2 {
                        scalar_indices.insert(args[1]);
                    }
                }
                _ => {}
            }
        }
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                match ins {
                    Instr::StoreIndex1D { idx, .. } => {
                        scalar_indices.insert(*idx);
                    }
                    Instr::StoreIndex2D { r, c, .. } => {
                        scalar_indices.insert(*r);
                        scalar_indices.insert(*c);
                    }
                    _ => {}
                }
            }
        }
        for idx in scalar_indices {
            if let Some(sym) = symbol_callee_for_value(fn_ir, idx) {
                out.insert(sym);
            }
        }
    }
    out
}

fn collect_vector_index_return_demands(
    all_fns: &FxHashMap<String, FnIR>,
    index_param_slots: &FxHashMap<String, FxHashSet<usize>>,
) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    let mut names: Vec<String> = all_fns.keys().cloned().collect();
    names.sort();
    for name in names {
        let Some(fn_ir) = all_fns.get(&name) else {
            continue;
        };
        for v in &fn_ir.values {
            let ValueKind::Call { callee, args, .. } = &v.kind else {
                continue;
            };
            let Some(slots) = index_param_slots.get(callee) else {
                continue;
            };
            let mut ordered_slots: Vec<usize> = slots.iter().copied().collect();
            ordered_slots.sort_unstable();
            for slot in ordered_slots {
                let Some(arg) = args.get(slot).copied() else {
                    continue;
                };
                if let Some(sym) = symbol_callee_for_value(fn_ir, arg) {
                    out.insert(sym);
                }
            }
        }
    }
    out
}

fn can_apply_index_return_override(
    all_fns: &FxHashMap<String, FnIR>,
    fname: &str,
    demanded_shape: ShapeTy,
    demanded_term: &TypeTerm,
) -> bool {
    let Some(fn_ir) = all_fns.get(fname) else {
        return false;
    };
    if let Some(hint) = fn_ir.ret_ty_hint
        && hint != TypeState::unknown()
    {
        if hint.shape != ShapeTy::Unknown && hint.shape != demanded_shape {
            return false;
        }
        if hint.prim != PrimTy::Any && hint.prim != PrimTy::Int {
            return false;
        }
    }
    if let Some(term_hint) = &fn_ir.ret_term_hint
        && !term_hint.is_any()
        && !term_hint.compatible_with(demanded_term)
    {
        return false;
    }
    true
}

fn coerce_index_scalar_return(ty: TypeState) -> TypeState {
    let mut out = if ty == TypeState::unknown() {
        TypeState::scalar(PrimTy::Int, false)
    } else {
        ty
    };
    out.prim = PrimTy::Int;
    out.shape = ShapeTy::Scalar;
    out.len_sym = None;
    out
}

fn coerce_index_vector_return(ty: TypeState) -> TypeState {
    let mut out = if ty == TypeState::unknown() {
        TypeState::vector(PrimTy::Int, false)
    } else {
        ty
    };
    out.prim = PrimTy::Int;
    out.shape = ShapeTy::Vector;
    out
}

fn apply_index_return_demands(
    all_fns: &FxHashMap<String, FnIR>,
    fn_ret: &mut FxHashMap<String, TypeState>,
    fn_ret_term: &mut FxHashMap<String, TypeTerm>,
    scalar_demands: &FxHashSet<String>,
    vector_demands: &FxHashSet<String>,
) -> bool {
    let mut changed = false;

    let mut scalar_names: Vec<String> = scalar_demands.iter().cloned().collect();
    scalar_names.sort();
    for name in scalar_names {
        if !can_apply_index_return_override(all_fns, &name, ShapeTy::Scalar, &TypeTerm::Int) {
            continue;
        }
        if let Some(slot) = fn_ret.get_mut(&name) {
            let next = coerce_index_scalar_return(*slot);
            if *slot != next {
                *slot = next;
                changed = true;
            }
        }
        if let Some(slot) = fn_ret_term.get_mut(&name)
            && *slot != TypeTerm::Int
        {
            *slot = TypeTerm::Int;
            changed = true;
        }
    }

    let vec_term = TypeTerm::Vector(Box::new(TypeTerm::Int));
    let mut vector_names: Vec<String> = vector_demands.iter().cloned().collect();
    vector_names.sort();
    for name in vector_names {
        if !can_apply_index_return_override(all_fns, &name, ShapeTy::Vector, &vec_term) {
            continue;
        }
        if let Some(slot) = fn_ret.get_mut(&name) {
            let next = coerce_index_vector_return(*slot);
            if *slot != next {
                *slot = next;
                changed = true;
            }
        }
        if let Some(slot) = fn_ret_term.get_mut(&name)
            && *slot != vec_term
        {
            *slot = vec_term.clone();
            changed = true;
        }
    }

    changed
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

    if ret_term.is_any()
        && let Some(h) = &fn_ir.ret_term_hint
    {
        ret_term = h.clone();
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
    var_tys: &FxHashMap<String, TypeState>,
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
        ValueKind::Load { var } => var_tys.get(var).copied().unwrap_or(TypeState::unknown()),
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
