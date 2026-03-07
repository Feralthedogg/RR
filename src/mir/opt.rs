use crate::mir::*;
use crate::syntax::ast::BinOp;
use crate::typeck::{LenSym, PrimTy, TypeState, TypeTerm};
use crate::{error::InternalCompilerError, error::Stage};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::time::Instant;

pub mod bce;
pub mod de_ssa;
pub mod fresh_alloc;
pub mod gvn;
pub mod inline;
pub mod intrinsics;
pub mod licm;
pub mod loop_analysis;
pub mod loop_opt;
pub mod parallel_copy;
pub mod sccp;
pub mod simplify;
pub mod tco;
pub mod type_specialize;
pub mod v_opt;

pub struct TachyonEngine;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ClampBound {
    ConstOne,
    ConstSix,
    Var(String),
}

#[derive(Debug, Clone)]
struct CubeIndexReturnVars {
    face_var: String,
    x_var: String,
    y_var: String,
    size_var: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TachyonProgressTier {
    Always,
    Heavy,
    DeSsa,
}

impl TachyonProgressTier {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Heavy => "heavy",
            Self::DeSsa => "de-ssa",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TachyonProgress {
    pub tier: TachyonProgressTier,
    pub completed: usize,
    pub total: usize,
    pub function: String,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TachyonPulseStats {
    pub vectorized: usize,
    pub reduced: usize,
    pub vector_loops_seen: usize,
    pub vector_skipped: usize,
    pub vector_skip_no_iv: usize,
    pub vector_skip_non_canonical_bound: usize,
    pub vector_skip_unsupported_cfg_shape: usize,
    pub vector_skip_indirect_index_access: usize,
    pub vector_skip_store_effects: usize,
    pub vector_skip_no_supported_pattern: usize,
    pub simplified_loops: usize,
    pub tco_hits: usize,
    pub sccp_hits: usize,
    pub intrinsics_hits: usize,
    pub gvn_hits: usize,
    pub licm_hits: usize,
    pub fresh_alloc_hits: usize,
    pub bce_hits: usize,
    pub simplify_hits: usize,
    pub dce_hits: usize,
    pub inline_rounds: usize,
    pub inline_cleanup_hits: usize,
    pub de_ssa_hits: usize,
    pub always_tier_functions: usize,
    pub optimized_functions: usize,
    pub skipped_functions: usize,
    pub full_opt_ir_limit: usize,
    pub full_opt_fn_limit: usize,
    pub total_program_ir: usize,
    pub max_function_ir: usize,
    pub selective_budget_mode: bool,
}

impl TachyonPulseStats {
    fn accumulate(&mut self, other: Self) {
        self.vectorized += other.vectorized;
        self.reduced += other.reduced;
        self.vector_loops_seen += other.vector_loops_seen;
        self.vector_skipped += other.vector_skipped;
        self.vector_skip_no_iv += other.vector_skip_no_iv;
        self.vector_skip_non_canonical_bound += other.vector_skip_non_canonical_bound;
        self.vector_skip_unsupported_cfg_shape += other.vector_skip_unsupported_cfg_shape;
        self.vector_skip_indirect_index_access += other.vector_skip_indirect_index_access;
        self.vector_skip_store_effects += other.vector_skip_store_effects;
        self.vector_skip_no_supported_pattern += other.vector_skip_no_supported_pattern;
        self.simplified_loops += other.simplified_loops;
        self.tco_hits += other.tco_hits;
        self.sccp_hits += other.sccp_hits;
        self.intrinsics_hits += other.intrinsics_hits;
        self.gvn_hits += other.gvn_hits;
        self.licm_hits += other.licm_hits;
        self.fresh_alloc_hits += other.fresh_alloc_hits;
        self.bce_hits += other.bce_hits;
        self.simplify_hits += other.simplify_hits;
        self.dce_hits += other.dce_hits;
        self.inline_rounds += other.inline_rounds;
        self.inline_cleanup_hits += other.inline_cleanup_hits;
        self.de_ssa_hits += other.de_ssa_hits;
        self.always_tier_functions += other.always_tier_functions;
        self.optimized_functions += other.optimized_functions;
        self.skipped_functions += other.skipped_functions;
    }
}

#[derive(Debug, Clone)]
struct FunctionBudgetProfile {
    name: String,
    ir_size: usize,
    score: usize,
    weighted_score: usize,
    density: usize,
    hot_weight: usize,
    within_fn_limit: bool,
}

#[derive(Debug, Clone)]
struct ProgramOptPlan {
    program_limit: usize,
    fn_limit: usize,
    total_ir: usize,
    max_fn_ir: usize,
    selective_mode: bool,
    selected_functions: FxHashSet<String>,
}

// Backward compatibility alias for older call sites.
pub type MirOptimizer = TachyonEngine;

impl Default for TachyonEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TachyonEngine {
    pub fn new() -> Self {
        Self
    }

    fn verify_or_panic(fn_ir: &FnIR, stage: &str) {
        if let Err(e) = crate::mir::verify::verify_ir(fn_ir) {
            InternalCompilerError::new(
                Stage::Opt,
                format!(
                    "MIR verification failed at {} for function '{}': {}",
                    stage, fn_ir.name, e
                ),
            )
            .into_exception()
            .display(None, None);
        }
    }

    fn verify_or_reject(fn_ir: &mut FnIR, stage: &str) -> bool {
        match crate::mir::verify::verify_ir(fn_ir) {
            Ok(()) => true,
            Err(e) => {
                fn_ir.unsupported_dynamic = true;
                let reason = format!("invalid MIR at {}: {}", stage, e);
                if !fn_ir.fallback_reasons.iter().any(|r| r == &reason) {
                    fn_ir.fallback_reasons.push(reason);
                }
                false
            }
        }
    }

    fn env_bool(key: &str, default_v: bool) -> bool {
        match env::var(key) {
            Ok(v) => matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            ),
            Err(_) => default_v,
        }
    }

    fn env_usize(key: &str, default_v: usize) -> usize {
        env::var(key)
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(default_v)
    }

    fn verify_each_pass() -> bool {
        Self::env_bool("RR_VERIFY_EACH_PASS", false)
    }

    fn maybe_verify(fn_ir: &FnIR, stage: &str) {
        if Self::verify_each_pass() {
            Self::verify_or_panic(fn_ir, stage);
        }
    }

    fn max_opt_iterations() -> usize {
        Self::env_usize("RR_OPT_MAX_ITERS", 24)
    }

    fn max_inline_rounds() -> usize {
        Self::env_usize("RR_INLINE_MAX_ROUNDS", 3)
    }

    fn max_full_opt_ir() -> usize {
        Self::env_usize("RR_MAX_FULL_OPT_IR", 2500)
    }

    fn max_full_opt_fn_ir() -> usize {
        Self::env_usize("RR_MAX_FULL_OPT_FN_IR", 900)
    }

    fn adaptive_ir_budget_enabled() -> bool {
        Self::env_bool("RR_ADAPTIVE_IR_BUDGET", false)
    }

    fn selective_budget_enabled() -> bool {
        Self::env_bool("RR_SELECTIVE_OPT_BUDGET", true) || Self::adaptive_ir_budget_enabled()
    }

    fn heavy_pass_fn_ir() -> usize {
        Self::env_usize("RR_HEAVY_PASS_FN_IR", 650)
    }

    fn always_bce_fn_ir() -> usize {
        let default_limit = Self::heavy_pass_fn_ir().max(64);
        Self::env_usize("RR_ALWAYS_BCE_FN_IR", default_limit)
    }

    fn max_fn_opt_ms() -> u128 {
        Self::env_usize("RR_MAX_FN_OPT_MS", 250) as u128
    }

    fn always_tier_max_iters() -> usize {
        Self::env_usize("RR_ALWAYS_TIER_ITERS", 2).clamp(1, 6)
    }

    fn licm_enabled() -> bool {
        Self::env_bool("RR_ENABLE_LICM", false)
    }

    fn gvn_enabled() -> bool {
        Self::env_bool("RR_ENABLE_GVN", false)
    }

    fn profile_use_path() -> Option<String> {
        env::var("RR_PROFILE_USE").ok().and_then(|v| {
            let p = v.trim();
            if p.is_empty() {
                None
            } else {
                Some(p.to_string())
            }
        })
    }

    fn wrap_trace_enabled() -> bool {
        Self::env_bool("RR_WRAP_TRACE", false)
    }

    fn debug_wrap_candidates(all_fns: &FxHashMap<String, FnIR>) {
        if !Self::wrap_trace_enabled() {
            return;
        }
        let names = Self::sorted_fn_names(all_fns);
        for name in names {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            if fn_ir.params.len() != 4 {
                continue;
            }
            let mut if_terms = 0usize;
            let mut store_count = 0usize;
            let mut eval_count = 0usize;
            let mut phi_count = 0usize;
            let mut call_names: FxHashSet<String> = FxHashSet::default();
            for bb in &fn_ir.blocks {
                if matches!(bb.term, Terminator::If { .. }) {
                    if_terms += 1;
                }
                for ins in &bb.instrs {
                    match ins {
                        Instr::Eval { .. } => eval_count += 1,
                        Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. } => store_count += 1,
                        Instr::Assign { .. } => {}
                    }
                }
            }
            for v in &fn_ir.values {
                match &v.kind {
                    ValueKind::Phi { .. } => phi_count += 1,
                    ValueKind::Call { callee, .. } => {
                        call_names.insert(callee.clone());
                    }
                    _ => {}
                }
            }
            eprintln!(
                "   [wrap-cand] {} params=4 blocks={} if={} stores={} eval={} phi={} calls={:?}",
                fn_ir.name,
                fn_ir.blocks.len(),
                if_terms,
                store_count,
                eval_count,
                phi_count,
                call_names
            );
        }
    }

    fn is_floor_like_single_positional_call(
        callee: &str,
        args: &[ValueId],
        names: &[Option<String>],
    ) -> bool {
        if !matches!(callee, "floor" | "ceiling" | "trunc") {
            return false;
        }
        if args.len() != 1 || names.len() > 1 {
            return false;
        }
        names
            .first()
            .and_then(std::option::Option::as_ref)
            .is_none()
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
            let mut found = false;
            let mut slot: Option<usize> = None;
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
            if !found {
                return None;
            }
            slot
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

    fn int_vector_ty_for_param_slot(slot: usize) -> TypeState {
        TypeState::vector(PrimTy::Int, false).with_len(Some(LenSym((slot as u32) + 1)))
    }

    fn collect_floor_index_param_slots(fn_ir: &FnIR) -> FxHashSet<usize> {
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
                if let Some(slot) = Self::param_slot_for_value(fn_ir, args[0]) {
                    slots.insert(slot);
                }
                continue;
            }
            if !Self::is_floor_like_single_positional_call(callee, args, names) {
                continue;
            }
            let Some(inner) = args.first().copied() else {
                continue;
            };
            match &fn_ir.values[inner].kind {
                ValueKind::Index1D { base, .. } => {
                    if let Some(slot) = Self::param_slot_for_value(fn_ir, *base) {
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
                    if let Some(slot) = Self::param_slot_for_value(fn_ir, inner_args[0]) {
                        slots.insert(slot);
                    }
                }
                _ => {}
            }
        }
        slots
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

    fn collect_floor_index_base_vars(fn_ir: &FnIR) -> FxHashSet<String> {
        let mut vars = FxHashSet::default();
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
                if let Some(var) = Self::value_base_var_name(fn_ir, args[0]) {
                    vars.insert(var);
                }
                continue;
            }
            if !Self::is_floor_like_single_positional_call(callee, args, names) {
                continue;
            }
            let Some(inner) = args.first().copied() else {
                continue;
            };
            match &fn_ir.values[inner].kind {
                ValueKind::Index1D { base, .. } => {
                    if let Some(var) = Self::value_base_var_name(fn_ir, *base) {
                        vars.insert(var);
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
                    if let Some(var) = Self::value_base_var_name(fn_ir, inner_args[0]) {
                        vars.insert(var);
                    }
                }
                _ => {}
            }
        }
        vars
    }

    fn value_is_proven_int_index_vector(
        fn_ir: &FnIR,
        vid: ValueId,
        proven_param_slots: &FxHashSet<usize>,
    ) -> bool {
        let val = &fn_ir.values[vid];
        if val.value_ty.is_numeric_vector() && val.value_ty.prim == PrimTy::Int {
            return true;
        }
        if matches!(&val.value_term, TypeTerm::Vector(inner) if **inner == TypeTerm::Int) {
            return true;
        }
        if let Some(slot) = Self::param_slot_for_value(fn_ir, vid) {
            return proven_param_slots.contains(&slot);
        }
        false
    }

    fn collect_proven_floor_index_param_slots(
        all_fns: &FxHashMap<String, FnIR>,
    ) -> FxHashMap<String, FxHashSet<usize>> {
        let mut floor_slots_by_fn: FxHashMap<String, FxHashSet<usize>> = FxHashMap::default();
        let ordered_names = Self::sorted_fn_names(all_fns);
        for name in &ordered_names {
            let Some(fn_ir) = all_fns.get(name) else {
                continue;
            };
            let slots = Self::collect_floor_index_param_slots(fn_ir);
            if !slots.is_empty() {
                floor_slots_by_fn.insert(name.clone(), slots);
            }
        }
        if floor_slots_by_fn.is_empty() {
            return FxHashMap::default();
        }

        let mut proven_by_fn: FxHashMap<String, FxHashSet<usize>> = FxHashMap::default();
        let mut changed = true;
        let mut guard = 0usize;
        while changed && guard < 16 {
            guard += 1;
            changed = false;
            for callee_name in &ordered_names {
                let Some(floor_slots) = floor_slots_by_fn.get(callee_name) else {
                    continue;
                };
                let mut sorted_slots: Vec<usize> = floor_slots.iter().copied().collect();
                sorted_slots.sort_unstable();
                for slot in sorted_slots {
                    if proven_by_fn
                        .get(callee_name)
                        .is_some_and(|slots| slots.contains(&slot))
                    {
                        continue;
                    }
                    let mut saw_call = false;
                    let mut all_calls_proven = true;
                    for caller_name in &ordered_names {
                        let Some(caller_ir) = all_fns.get(caller_name) else {
                            continue;
                        };
                        let empty_slots = FxHashSet::default();
                        let caller_slots = proven_by_fn.get(caller_name).unwrap_or(&empty_slots);
                        for val in &caller_ir.values {
                            let ValueKind::Call { callee, args, .. } = &val.kind else {
                                continue;
                            };
                            if callee != callee_name {
                                continue;
                            }
                            saw_call = true;
                            let Some(arg) = args.get(slot).copied() else {
                                all_calls_proven = false;
                                break;
                            };
                            if !Self::value_is_proven_int_index_vector(caller_ir, arg, caller_slots)
                            {
                                all_calls_proven = false;
                                break;
                            }
                        }
                        if !all_calls_proven {
                            break;
                        }
                    }
                    if saw_call && all_calls_proven {
                        let inserted = proven_by_fn
                            .entry(callee_name.clone())
                            .or_default()
                            .insert(slot);
                        if inserted {
                            changed = true;
                        }
                    }
                }
            }
        }

        proven_by_fn
    }

    fn has_var_index_vector_canonicalization(fn_ir: &FnIR, var_name: &str) -> bool {
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var_name {
                    continue;
                }
                let ValueKind::Call { callee, args, .. } = &fn_ir.values[*src].kind else {
                    continue;
                };
                if callee != "rr_index_vec_floor" || args.is_empty() {
                    continue;
                }
                if let Some(base_name) = Self::value_base_var_name(fn_ir, args[0])
                    && base_name == var_name
                {
                    return true;
                }
            }
        }
        false
    }

    fn mark_floor_index_param_metadata(fn_ir: &mut FnIR, slots: &FxHashSet<usize>) -> bool {
        let mut changed = false;
        for vid in 0..fn_ir.values.len() {
            let kind = fn_ir.values[vid].kind.clone();
            match kind {
                ValueKind::Param { index } if slots.contains(&index) => {
                    let int_vec = Self::int_vector_ty_for_param_slot(index);
                    if fn_ir.values[vid].value_ty != int_vec {
                        fn_ir.values[vid].value_ty = int_vec;
                        changed = true;
                    }
                    if !fn_ir.values[vid]
                        .facts
                        .has(Facts::IS_VECTOR | Facts::INT_SCALAR)
                    {
                        fn_ir.values[vid]
                            .facts
                            .add(Facts::IS_VECTOR | Facts::INT_SCALAR);
                        changed = true;
                    }
                }
                ValueKind::Load { var } => {
                    let Some(slot) = fn_ir.params.iter().position(|p| p == &var) else {
                        continue;
                    };
                    if !slots.contains(&slot) {
                        continue;
                    }
                    let int_vec = Self::int_vector_ty_for_param_slot(slot);
                    if fn_ir.values[vid].value_ty != int_vec {
                        fn_ir.values[vid].value_ty = int_vec;
                        changed = true;
                    }
                    if !fn_ir.values[vid]
                        .facts
                        .has(Facts::IS_VECTOR | Facts::INT_SCALAR)
                    {
                        fn_ir.values[vid]
                            .facts
                            .add(Facts::IS_VECTOR | Facts::INT_SCALAR);
                        changed = true;
                    }
                }
                ValueKind::Index1D { base, .. } => {
                    let Some(slot) = Self::param_slot_for_value(fn_ir, base) else {
                        continue;
                    };
                    if !slots.contains(&slot) {
                        continue;
                    }
                    let int_scalar = TypeState::scalar(PrimTy::Int, false);
                    if fn_ir.values[vid].value_ty != int_scalar {
                        fn_ir.values[vid].value_ty = int_scalar;
                        changed = true;
                    }
                    if !fn_ir.values[vid].facts.has(Facts::INT_SCALAR) {
                        fn_ir.values[vid].facts.add(Facts::INT_SCALAR);
                        changed = true;
                    }
                }
                ValueKind::Call {
                    callee,
                    args,
                    names,
                } if Self::is_floor_like_single_positional_call(&callee, &args, &names) => {
                    let Some(inner) = args.first().copied() else {
                        continue;
                    };
                    let slot = match &fn_ir.values[inner].kind {
                        ValueKind::Index1D { base, .. } => Self::param_slot_for_value(fn_ir, *base),
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
                            Self::param_slot_for_value(fn_ir, inner_args[0])
                        }
                        _ => None,
                    };
                    let Some(slot) = slot else {
                        continue;
                    };
                    if !slots.contains(&slot) {
                        continue;
                    }
                    let int_scalar = TypeState::scalar(PrimTy::Int, false);
                    if fn_ir.values[vid].value_ty != int_scalar {
                        fn_ir.values[vid].value_ty = int_scalar;
                        changed = true;
                    }
                    if !fn_ir.values[vid].facts.has(Facts::INT_SCALAR) {
                        fn_ir.values[vid].facts.add(Facts::INT_SCALAR);
                        changed = true;
                    }
                }
                ValueKind::Call { callee, args, .. } if callee == "rr_index1_read_idx" => {
                    if args.is_empty() {
                        continue;
                    }
                    let Some(slot) = Self::param_slot_for_value(fn_ir, args[0]) else {
                        continue;
                    };
                    if !slots.contains(&slot) {
                        continue;
                    }
                    let int_scalar = TypeState::scalar(PrimTy::Int, false);
                    if fn_ir.values[vid].value_ty != int_scalar {
                        fn_ir.values[vid].value_ty = int_scalar;
                        changed = true;
                    }
                    if !fn_ir.values[vid].facts.has(Facts::INT_SCALAR) {
                        fn_ir.values[vid].facts.add(Facts::INT_SCALAR);
                        changed = true;
                    }
                }
                _ => {}
            }
        }
        changed
    }

    fn canonicalize_floor_index_params(
        fn_ir: &mut FnIR,
        proven_param_slots: Option<&FxHashSet<usize>>,
    ) -> bool {
        let slots = Self::collect_floor_index_param_slots(fn_ir);
        let base_vars = Self::collect_floor_index_base_vars(fn_ir);
        if slots.is_empty() && base_vars.is_empty() {
            return false;
        }

        let target_bb = if fn_ir.entry < fn_ir.blocks.len() {
            fn_ir.entry
        } else if fn_ir.body_head < fn_ir.blocks.len() {
            fn_ir.body_head
        } else {
            return false;
        };

        let mut sorted_vars: Vec<String> = base_vars.into_iter().collect();
        sorted_vars.sort();

        let mut prefix: Vec<Instr> = Vec::new();
        let mut changed = false;
        for var_name in sorted_vars {
            let skip_canonicalize = fn_ir
                .params
                .iter()
                .position(|p| p == &var_name)
                .is_some_and(|slot| proven_param_slots.is_some_and(|slots| slots.contains(&slot)));
            if skip_canonicalize {
                continue;
            }
            if Self::has_var_index_vector_canonicalization(fn_ir, &var_name) {
                continue;
            }
            let load = fn_ir.add_value(
                ValueKind::Load {
                    var: var_name.clone(),
                },
                crate::utils::Span::dummy(),
                Facts::empty(),
                Some(var_name.clone()),
            );
            let mut facts = Facts::empty();
            facts.add(Facts::IS_VECTOR | Facts::INT_SCALAR | Facts::ONE_BASED);
            let floor_vec = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_index_vec_floor".to_string(),
                    args: vec![load],
                    names: vec![None],
                },
                crate::utils::Span::dummy(),
                facts,
                None,
            );
            fn_ir.values[load].value_ty = TypeState::vector(PrimTy::Int, false);
            fn_ir.values[floor_vec].value_ty = TypeState::vector(PrimTy::Int, false);
            prefix.push(Instr::Assign {
                dst: var_name,
                src: floor_vec,
                span: crate::utils::Span::dummy(),
            });
            changed = true;
        }

        if !prefix.is_empty() {
            let bb = &mut fn_ir.blocks[target_bb];
            let mut merged = prefix;
            merged.extend(std::mem::take(&mut bb.instrs));
            bb.instrs = merged;
        }

        changed | Self::mark_floor_index_param_metadata(fn_ir, &slots)
    }

    fn load_hot_profile_counts() -> FxHashMap<String, usize> {
        let mut counts = FxHashMap::default();
        let Some(path) = Self::profile_use_path() else {
            return counts;
        };
        let Ok(content) = fs::read_to_string(path) else {
            return counts;
        };

        for raw in content.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (name, count_str) = if let Some((k, v)) = line.split_once('=') {
                (k.trim(), v.trim())
            } else if let Some((k, v)) = line.split_once(':') {
                (k.trim(), v.trim())
            } else {
                let mut parts = line.split_whitespace();
                let Some(k) = parts.next() else { continue };
                let Some(v) = parts.next() else { continue };
                (k, v)
            };
            if name.is_empty() {
                continue;
            }
            let Ok(parsed) = count_str.parse::<usize>() else {
                continue;
            };
            let entry = counts.entry(name.to_string()).or_insert(0);
            *entry = (*entry).saturating_add(parsed);
        }
        counts
    }

    fn fn_static_hotness(fn_ir: &FnIR) -> usize {
        let mut loops = 0usize;
        let mut branches = 0usize;
        let mut calls = 0usize;
        let mut stores = 0usize;
        for (bid, bb) in fn_ir.blocks.iter().enumerate() {
            match bb.term {
                Terminator::If {
                    then_bb, else_bb, ..
                } => {
                    branches += 1;
                    if then_bb <= bid {
                        loops += 1;
                    }
                    if else_bb <= bid {
                        loops += 1;
                    }
                }
                Terminator::Goto(t) => {
                    if t <= bid {
                        loops += 1;
                    }
                }
                _ => {}
            }
            for ins in &bb.instrs {
                if matches!(ins, Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. }) {
                    stores += 1;
                }
            }
        }
        for v in &fn_ir.values {
            if matches!(v.kind, ValueKind::Call { .. } | ValueKind::Intrinsic { .. }) {
                calls += 1;
            }
        }
        loops
            .saturating_mul(20)
            .saturating_add(branches.saturating_mul(8))
            .saturating_add(calls.saturating_mul(6))
            .saturating_add(stores.saturating_mul(4))
    }

    fn fn_ir_fingerprint(fn_ir: &FnIR) -> u64 {
        fn hash_instr(h: &mut DefaultHasher, instr: &Instr) {
            match instr {
                Instr::Assign { dst, src, .. } => {
                    1u8.hash(h);
                    dst.hash(h);
                    src.hash(h);
                }
                Instr::Eval { val, .. } => {
                    2u8.hash(h);
                    val.hash(h);
                }
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_safe,
                    is_na_safe,
                    is_vector,
                    ..
                } => {
                    3u8.hash(h);
                    base.hash(h);
                    idx.hash(h);
                    val.hash(h);
                    is_safe.hash(h);
                    is_na_safe.hash(h);
                    is_vector.hash(h);
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    4u8.hash(h);
                    base.hash(h);
                    r.hash(h);
                    c.hash(h);
                    val.hash(h);
                }
            }
        }

        let mut h = DefaultHasher::new();
        fn_ir.name.hash(&mut h);
        fn_ir.params.hash(&mut h);
        fn_ir.entry.hash(&mut h);
        fn_ir.body_head.hash(&mut h);
        fn_ir.blocks.len().hash(&mut h);
        fn_ir.values.len().hash(&mut h);
        for v in &fn_ir.values {
            v.kind.hash(&mut h);
            v.origin_var.hash(&mut h);
            v.phi_block.hash(&mut h);
        }
        for b in &fn_ir.blocks {
            b.term.hash(&mut h);
            b.instrs.len().hash(&mut h);
            for ins in &b.instrs {
                hash_instr(&mut h, ins);
            }
        }
        h.finish()
    }

    fn fn_ir_size(fn_ir: &FnIR) -> usize {
        let instrs: usize = fn_ir.blocks.iter().map(|b| b.instrs.len()).sum();
        fn_ir.values.len() + instrs
    }

    fn fn_opt_score(fn_ir: &FnIR) -> usize {
        let mut score = 0usize;
        for v in &fn_ir.values {
            score += match &v.kind {
                ValueKind::Binary { .. } => 3,
                ValueKind::Unary { .. } => 2,
                ValueKind::Call { .. } => 5,
                ValueKind::Intrinsic { .. } => 8,
                ValueKind::Index1D { .. } | ValueKind::Index2D { .. } => 4,
                ValueKind::Phi { .. } => 2,
                ValueKind::Len { .. } | ValueKind::Range { .. } => 2,
                _ => 1,
            };
        }
        for b in &fn_ir.blocks {
            if matches!(b.term, Terminator::If { .. }) {
                score += 8;
            }
            for ins in &b.instrs {
                score += match ins {
                    Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. } => 6,
                    Instr::Eval { .. } => 2,
                    Instr::Assign { .. } => 1,
                };
            }
        }
        // Mild size-bias so tiny helper functions don't always dominate ranking.
        score.saturating_add(Self::fn_ir_size(fn_ir) / 12)
    }

    fn adaptive_full_opt_limits(
        all_fns: &FxHashMap<String, FnIR>,
        total_ir: usize,
        max_fn_ir: usize,
    ) -> (usize, usize) {
        let base_prog = Self::max_full_opt_ir();
        let base_fn = Self::max_full_opt_fn_ir();
        if !Self::adaptive_ir_budget_enabled() {
            return (base_prog, base_fn);
        }

        let fn_count = all_fns.len().max(1);
        let avg_ir = total_ir / fn_count;
        let mut branch_terms = 0usize;
        let mut call_like = 0usize;
        let mut mem_like = 0usize;
        let mut arith_like = 0usize;

        let ordered_names = Self::sorted_fn_names(all_fns);
        for name in &ordered_names {
            let Some(fn_ir) = all_fns.get(name) else {
                continue;
            };
            for blk in &fn_ir.blocks {
                if matches!(blk.term, Terminator::If { .. }) {
                    branch_terms += 1;
                }
                for ins in &blk.instrs {
                    if matches!(ins, Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. }) {
                        mem_like += 1;
                    }
                }
            }
            for v in &fn_ir.values {
                match &v.kind {
                    ValueKind::Binary { .. } | ValueKind::Unary { .. } => arith_like += 1,
                    ValueKind::Call { .. } | ValueKind::Intrinsic { .. } => call_like += 1,
                    ValueKind::Index1D { .. } | ValueKind::Index2D { .. } => mem_like += 1,
                    _ => {}
                }
            }
        }

        let hot_ops = branch_terms
            .saturating_add(call_like)
            .saturating_add(mem_like)
            .saturating_add(arith_like);
        let hot_density_permille = if total_ir == 0 {
            0
        } else {
            hot_ops.saturating_mul(1000) / total_ir
        };
        let fn_bonus = fn_count.saturating_mul(32).min(1800);
        let avg_bonus = avg_ir.saturating_mul(2).min(3200);
        let density_bonus = hot_density_permille.saturating_mul(3).min(1400);
        let max_skew_bonus = max_fn_ir.saturating_sub(avg_ir).min(1200);

        let program_upper = base_prog.max(12_000);
        let fn_upper = base_fn.max(1_600);

        let program_limit = base_prog
            .saturating_add(fn_bonus)
            .saturating_add(avg_bonus)
            .saturating_add(density_bonus)
            .saturating_add(max_skew_bonus / 4)
            .clamp(base_prog, program_upper);

        let fn_limit = base_fn
            .saturating_add(avg_ir.saturating_mul(2).min(500))
            .saturating_add(hot_density_permille.min(300))
            .clamp(base_fn, fn_upper);

        (program_limit, fn_limit)
    }

    fn fn_hot_weight(
        name: &str,
        fn_ir: &FnIR,
        profile_counts: &FxHashMap<String, usize>,
        max_profile_count: usize,
    ) -> usize {
        let static_hot = Self::fn_static_hotness(fn_ir).min(800);
        let static_weight = 1024usize.saturating_add(static_hot.saturating_mul(3));
        let profile_weight = match profile_counts.get(name).copied() {
            Some(count) if max_profile_count > 0 => {
                1024usize.saturating_add(count.saturating_mul(3072) / max_profile_count)
            }
            _ => 1024usize,
        };
        static_weight
            .saturating_mul(profile_weight)
            .saturating_div(1024)
    }

    fn build_opt_plan_with_profile(
        all_fns: &FxHashMap<String, FnIR>,
        profile_counts: &FxHashMap<String, usize>,
    ) -> ProgramOptPlan {
        let total_ir: usize = all_fns.values().map(Self::fn_ir_size).sum();
        let max_fn_ir: usize = all_fns.values().map(Self::fn_ir_size).max().unwrap_or(0);
        let (program_limit, fn_limit) =
            Self::adaptive_full_opt_limits(all_fns, total_ir, max_fn_ir);

        let mut selected = FxHashSet::default();
        let needs_budget = total_ir > program_limit || max_fn_ir > fn_limit;
        let ordered_names = Self::sorted_fn_names(all_fns);
        if !needs_budget {
            for name in &ordered_names {
                let Some(fn_ir) = all_fns.get(name) else {
                    continue;
                };
                if !fn_ir.unsupported_dynamic {
                    selected.insert(name.clone());
                }
            }
            return ProgramOptPlan {
                program_limit,
                fn_limit,
                total_ir,
                max_fn_ir,
                selective_mode: false,
                selected_functions: selected,
            };
        }

        let mut profiles = Vec::new();
        let soft_fn_limit = fn_limit.min(Self::heavy_pass_fn_ir().max(64));
        let max_profile_count = profile_counts.values().copied().max().unwrap_or(0);
        for name in &ordered_names {
            let Some(fn_ir) = all_fns.get(name) else {
                continue;
            };
            if fn_ir.unsupported_dynamic {
                continue;
            }
            let ir_size = Self::fn_ir_size(fn_ir);
            let score = Self::fn_opt_score(fn_ir);
            let hot_weight = Self::fn_hot_weight(name, fn_ir, profile_counts, max_profile_count);
            let weighted_score = score.saturating_mul(hot_weight).saturating_div(1024);
            let density = weighted_score.saturating_mul(1024) / ir_size.max(1);
            profiles.push(FunctionBudgetProfile {
                name: name.clone(),
                ir_size,
                score,
                weighted_score,
                density,
                hot_weight,
                within_fn_limit: ir_size <= soft_fn_limit,
            });
        }

        profiles.sort_by(|a, b| {
            b.within_fn_limit
                .cmp(&a.within_fn_limit)
                .then_with(|| b.density.cmp(&a.density))
                .then_with(|| b.hot_weight.cmp(&a.hot_weight))
                .then_with(|| b.weighted_score.cmp(&a.weighted_score))
                .then_with(|| b.score.cmp(&a.score))
                .then_with(|| a.ir_size.cmp(&b.ir_size))
                .then_with(|| a.name.cmp(&b.name))
        });

        let mut used_budget = 0usize;
        for p in &profiles {
            if !p.within_fn_limit {
                continue;
            }
            if used_budget.saturating_add(p.ir_size) > program_limit {
                continue;
            }
            used_budget = used_budget.saturating_add(p.ir_size);
            selected.insert(p.name.clone());
        }

        if selected.is_empty()
            && let Some(fallback) = profiles
                .iter()
                .filter(|p| p.ir_size <= soft_fn_limit.saturating_mul(2))
                .min_by_key(|p| p.ir_size)
                .or_else(|| profiles.iter().min_by_key(|p| p.ir_size))
        {
            selected.insert(fallback.name.clone());
        }

        ProgramOptPlan {
            program_limit,
            fn_limit,
            total_ir,
            max_fn_ir,
            selective_mode: true,
            selected_functions: selected,
        }
    }

    fn build_opt_plan(all_fns: &FxHashMap<String, FnIR>) -> ProgramOptPlan {
        let profile_counts = Self::load_hot_profile_counts();
        Self::build_opt_plan_with_profile(all_fns, &profile_counts)
    }

    fn sorted_fn_names(all_fns: &FxHashMap<String, FnIR>) -> Vec<String> {
        let mut names: Vec<String> = all_fns.keys().cloned().collect();
        names.sort();
        names
    }

    // Required lowering-to-codegen stabilization passes.
    // This must run even in O0, because codegen cannot emit Phi.
    pub fn stabilize_for_codegen(&self, all_fns: &mut FxHashMap<String, FnIR>) {
        let ordered_names = Self::sorted_fn_names(all_fns);
        for name in &ordered_names {
            let Some(fn_ir) = all_fns.get_mut(name) else {
                continue;
            };
            if !Self::verify_or_reject(fn_ir, "PrepareForCodegen/Start") {
                continue;
            }
            let _ = de_ssa::run(fn_ir);
            // Keep this lightweight but convergent to avoid dead temp noise after De-SSA.
            // Hybrid fallback functions skip cleanup to preserve dynamic semantics.
            if !fn_ir.unsupported_dynamic {
                let mut changed = true;
                let mut guard = 0;
                while changed && guard < 8 {
                    guard += 1;
                    changed = false;
                    changed |= self.simplify_cfg(fn_ir);
                    changed |= self.dce(fn_ir);
                }
            }
            let _ = Self::verify_or_reject(fn_ir, "PrepareForCodegen/End");
        }
    }

    fn run_always_tier_with_stats(
        &self,
        fn_ir: &mut FnIR,
        proven_param_slots: Option<&FxHashSet<usize>>,
    ) -> TachyonPulseStats {
        let mut stats = TachyonPulseStats::default();
        if fn_ir.unsupported_dynamic {
            return stats;
        }
        if !Self::verify_or_reject(fn_ir, "AlwaysTier/Start") {
            return stats;
        }
        let canonicalized_index_params =
            Self::canonicalize_floor_index_params(fn_ir, proven_param_slots);
        if canonicalized_index_params {
            Self::maybe_verify(fn_ir, "After AlwaysTier/ParamIndexCanonicalize");
        }

        stats.always_tier_functions = 1;
        let mut changed = true;
        let mut iter = 0usize;
        let max_iters = Self::always_tier_max_iters();
        let mut seen = FxHashSet::default();
        seen.insert(Self::fn_ir_fingerprint(fn_ir));
        let fn_ir_size = Self::fn_ir_size(fn_ir);
        let run_light_sccp = fn_ir_size <= Self::heavy_pass_fn_ir().saturating_mul(2);
        let loop_opt = loop_opt::MirLoopOptimizer::new();

        while changed && iter < max_iters {
            iter += 1;
            changed = false;
            let before_hash = Self::fn_ir_fingerprint(fn_ir);

            let sc_changed = self.simplify_cfg(fn_ir);
            if sc_changed {
                stats.simplify_hits += 1;
                changed = true;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/SimplifyCFG");

            if run_light_sccp {
                let sccp_changed = sccp::MirSCCP::new().optimize(fn_ir);
                if sccp_changed {
                    stats.sccp_hits += 1;
                    changed = true;
                }
                Self::maybe_verify(fn_ir, "After AlwaysTier/SCCP");

                let intr_changed = intrinsics::optimize(fn_ir);
                if intr_changed {
                    stats.intrinsics_hits += 1;
                    changed = true;
                }
                Self::maybe_verify(fn_ir, "After AlwaysTier/Intrinsics");
            }

            let type_spec_changed = type_specialize::optimize(fn_ir);
            if type_spec_changed {
                changed = true;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/TypeSpecialize");

            let loop_changed_count = loop_opt.optimize_with_count(fn_ir);
            if loop_changed_count > 0 {
                stats.simplified_loops += loop_changed_count;
                changed = true;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/LoopOpt");

            let dce_changed = self.dce(fn_ir);
            if dce_changed {
                stats.dce_hits += 1;
                changed = true;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/DCE");

            let after_hash = Self::fn_ir_fingerprint(fn_ir);
            if after_hash == before_hash {
                break;
            }
            if !seen.insert(after_hash) {
                break;
            }
        }

        // Apply one bounded BCE sweep after convergence so skipped heavy-tier functions
        // can still get guard elimination opportunities without large compile-time spikes.
        if fn_ir_size <= Self::always_bce_fn_ir() {
            let bce_changed = bce::optimize(fn_ir);
            if bce_changed {
                stats.bce_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/BCE");
        }

        let _ = Self::verify_or_reject(fn_ir, "AlwaysTier/End");
        stats
    }

    pub fn run_program(&self, all_fns: &mut FxHashMap<String, FnIR>) {
        let _ = self.run_program_with_stats(all_fns);
    }

    pub fn run_program_with_stats(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
    ) -> TachyonPulseStats {
        self.run_program_with_stats_inner(all_fns, None)
    }

    pub fn run_program_with_stats_progress(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        on_progress: &mut dyn FnMut(TachyonProgress),
    ) -> TachyonPulseStats {
        self.run_program_with_stats_inner(all_fns, Some(on_progress))
    }

    fn emit_progress(
        progress: &mut Option<&mut dyn FnMut(TachyonProgress)>,
        tier: TachyonProgressTier,
        completed: usize,
        total: usize,
        function: &str,
    ) {
        if let Some(cb) = progress.as_deref_mut() {
            cb(TachyonProgress {
                tier,
                completed,
                total,
                function: function.to_string(),
            });
        }
    }

    fn run_program_with_stats_inner(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        mut progress: Option<&mut dyn FnMut(TachyonProgress)>,
    ) -> TachyonPulseStats {
        /*
        // 1. Clean
        simplify::SimplifyCFG::new().optimize(fn_ir);

        loop {
             let mut changed = false;

             // 2. Sccp
             // changed |= sccp::MirSccp::new().optimize(fn_ir);

             // 3. LICM
             // changed |= licm::MirLicm::new().optimize(fn_ir);

             // 4. Clean again
             changed |= simplify::SimplifyCFG::new().optimize(fn_ir);

             if !changed { break; }
        }

        // TCO
        tco::optimize(fn_ir);

        // Final polish (DCE/cleanup)
        simplify::SimplifyCFG::new().optimize(fn_ir);
        */

        let mut stats = TachyonPulseStats::default();
        let plan = Self::build_opt_plan(all_fns);
        let selective_enabled = Self::selective_budget_enabled();
        let run_heavy_tier = !plan.selective_mode || selective_enabled;
        let run_full_inline_tier = run_heavy_tier;
        stats.total_program_ir = plan.total_ir;
        stats.max_function_ir = plan.max_fn_ir;
        stats.full_opt_ir_limit = plan.program_limit;
        stats.full_opt_fn_limit = plan.fn_limit;
        stats.selective_budget_mode = plan.selective_mode && selective_enabled;
        let ordered_names = Self::sorted_fn_names(all_fns);
        let ordered_total = ordered_names.len();
        let proven_floor_param_slots = Self::collect_proven_floor_index_param_slots(all_fns);

        // Tier A (always): lightweight canonicalization for every safe function.
        for (idx, name) in ordered_names.iter().enumerate() {
            let Some(fn_ir) = all_fns.get_mut(name) else {
                continue;
            };
            let s = self.run_always_tier_with_stats(fn_ir, proven_floor_param_slots.get(name));
            stats.accumulate(s);
            Self::emit_progress(
                &mut progress,
                TachyonProgressTier::Always,
                idx + 1,
                ordered_total,
                name,
            );
        }
        Self::debug_wrap_candidates(all_fns);

        let wrap_index_helpers = if run_heavy_tier {
            Self::collect_wrap_index_helpers(all_fns)
        } else {
            FxHashSet::default()
        };
        if !wrap_index_helpers.is_empty() {
            let rewrites = Self::rewrite_wrap_index_helper_calls(all_fns, &wrap_index_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                eprintln!(
                    "   [wrap] rewrote {} call site(s) using helper(s): {:?}",
                    rewrites, wrap_index_helpers
                );
            }
        }

        let cube_index_helpers = if run_heavy_tier {
            Self::collect_cube_index_helpers(all_fns)
        } else {
            FxHashSet::default()
        };
        if !cube_index_helpers.is_empty() {
            let rewrites = Self::rewrite_cube_index_helper_calls(all_fns, &cube_index_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                eprintln!(
                    "   [cube] rewrote {} call site(s) using helper(s): {:?}",
                    rewrites, cube_index_helpers
                );
            }
        }

        let heavy_targets_exist =
            run_heavy_tier && (!plan.selective_mode || !plan.selected_functions.is_empty());
        let callmap_user_whitelist = if heavy_targets_exist {
            Self::collect_callmap_user_whitelist(all_fns)
        } else {
            FxHashSet::default()
        };

        // Tier B (selective-heavy): optimize full pass pipeline only for selected functions.
        for (idx, name) in ordered_names.iter().enumerate() {
            let Some(fn_ir) = all_fns.get_mut(name) else {
                continue;
            };
            if fn_ir.unsupported_dynamic {
                stats.skipped_functions += 1;
                let _ = Self::verify_or_reject(fn_ir, "SkipOpt/UnsupportedDynamic");
                Self::emit_progress(
                    &mut progress,
                    TachyonProgressTier::Heavy,
                    idx + 1,
                    ordered_total,
                    name,
                );
                continue;
            }
            let selected = !plan.selective_mode || plan.selected_functions.contains(name);
            if !run_heavy_tier || !selected {
                stats.skipped_functions += 1;
                let reason = if !run_heavy_tier {
                    "SkipOpt/HeavyTierDisabled"
                } else {
                    "SkipOpt/Budget"
                };
                let _ = Self::verify_or_reject(fn_ir, reason);
                Self::emit_progress(
                    &mut progress,
                    TachyonProgressTier::Heavy,
                    idx + 1,
                    ordered_total,
                    name,
                );
                continue;
            }
            stats.optimized_functions += 1;
            let s = self.run_function_with_stats_with_proven(
                fn_ir,
                &callmap_user_whitelist,
                proven_floor_param_slots.get(name),
            );
            stats.accumulate(s);
            Self::emit_progress(
                &mut progress,
                TachyonProgressTier::Heavy,
                idx + 1,
                ordered_total,
                name,
            );
        }

        // Tier C (full-program): bounded inter-procedural inlining.
        if run_full_inline_tier {
            let mut changed = true;
            let mut iter = 0;
            let inliner = inline::MirInliner::new();
            let hot_filter = if plan.selective_mode {
                Some(&plan.selected_functions)
            } else {
                None
            };
            while changed && iter < Self::max_inline_rounds() {
                changed = false;
                iter += 1;
                // Inlining needs access to the whole map
                let local_changed = inliner.optimize_with_hot_filter(all_fns, hot_filter);
                let ordered_names = Self::sorted_fn_names(all_fns);
                for name in &ordered_names {
                    let Some(fn_ir) = all_fns.get(name) else {
                        continue;
                    };
                    Self::maybe_verify(fn_ir, "After Inlining");
                }
                if local_changed {
                    stats.inline_rounds += 1;
                    changed = true;
                    // Re-optimize each function if inlining happened
                    for name in &ordered_names {
                        let Some(fn_ir) = all_fns.get_mut(name) else {
                            continue;
                        };
                        if fn_ir.unsupported_dynamic {
                            Self::maybe_verify(
                                fn_ir,
                                "After Inline Cleanup (Skipped: UnsupportedDynamic)",
                            );
                            continue;
                        }
                        // Run lightweight cleanup after inlining.
                        let inline_sc_changed = self.simplify_cfg(fn_ir);
                        let inline_dce_changed = self.dce(fn_ir);
                        if inline_sc_changed || inline_dce_changed {
                            stats.inline_cleanup_hits += 1;
                        }
                        if inline_sc_changed {
                            stats.simplify_hits += 1;
                        }
                        if inline_dce_changed {
                            stats.dce_hits += 1;
                        }
                        Self::maybe_verify(fn_ir, "After Inline Cleanup");
                    }
                }
            }
        }

        // 3. De-SSA (Phi elimination via parallel copy) before codegen.
        let ordered_names = Self::sorted_fn_names(all_fns);
        let de_ssa_total = ordered_names.len();
        for (idx, name) in ordered_names.iter().enumerate() {
            let Some(fn_ir) = all_fns.get_mut(name) else {
                continue;
            };
            let de_ssa_changed = de_ssa::run(fn_ir);
            if de_ssa_changed {
                stats.de_ssa_hits += 1;
            }
            // Cleanup after De-SSA to drop dead temps and unreachable blocks.
            if !fn_ir.unsupported_dynamic {
                let sc_changed = self.simplify_cfg(fn_ir);
                let dce_changed = self.dce(fn_ir);
                if sc_changed {
                    stats.simplify_hits += 1;
                }
                if dce_changed {
                    stats.dce_hits += 1;
                }
            }
            let _ = Self::verify_or_reject(fn_ir, "After De-SSA");
            Self::emit_progress(
                &mut progress,
                TachyonProgressTier::DeSsa,
                idx + 1,
                de_ssa_total,
                name,
            );
        }
        stats
    }

    pub fn run_function(&self, fn_ir: &mut FnIR) {
        let empty = FxHashSet::default();
        let _ = self.run_function_with_stats(fn_ir, &empty);
    }

    pub fn run_function_with_stats(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
    ) -> TachyonPulseStats {
        self.run_function_with_proven_index_slots(fn_ir, callmap_user_whitelist, None)
    }

    fn run_function_with_stats_with_proven(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        proven_param_slots: Option<&FxHashSet<usize>>,
    ) -> TachyonPulseStats {
        self.run_function_with_proven_index_slots(fn_ir, callmap_user_whitelist, proven_param_slots)
    }

    fn run_function_with_proven_index_slots(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        proven_param_slots: Option<&FxHashSet<usize>>,
    ) -> TachyonPulseStats {
        let mut stats = TachyonPulseStats::default();
        let mut changed = true;
        let loop_opt = loop_opt::MirLoopOptimizer::new();
        let mut iterations = 0;
        let mut seen_hashes = FxHashSet::default();
        let start_time = Instant::now();
        let fn_ir_size = Self::fn_ir_size(fn_ir);
        let max_iters = if fn_ir_size > 2200 {
            4
        } else if fn_ir_size > 1400 {
            8
        } else if fn_ir_size > 900 {
            12
        } else {
            Self::max_opt_iterations()
        };
        let heavy_pass_budgeted = fn_ir_size > Self::heavy_pass_fn_ir();

        // Initial Verify
        if !Self::verify_or_reject(fn_ir, "Start") {
            return stats;
        }
        let canonicalized_index_params =
            Self::canonicalize_floor_index_params(fn_ir, proven_param_slots);
        if canonicalized_index_params {
            Self::maybe_verify(fn_ir, "After ParamIndexCanonicalize");
        }
        seen_hashes.insert(Self::fn_ir_fingerprint(fn_ir));

        while changed && iterations < max_iters {
            if start_time.elapsed().as_millis() > Self::max_fn_opt_ms() {
                break;
            }
            changed = false;
            iterations += 1;
            let before_hash = Self::fn_ir_fingerprint(fn_ir);

            // 1. Structural Transformations
            let mut pass_changed = false;
            let run_heavy_structural = !(heavy_pass_budgeted && iterations > 1);

            if run_heavy_structural {
                let type_spec_changed = type_specialize::optimize(fn_ir);
                Self::maybe_verify(fn_ir, "After TypeSpecialize");
                pass_changed |= type_spec_changed;

                // Vectorization
                let v_stats =
                    v_opt::optimize_with_stats_with_whitelist(fn_ir, callmap_user_whitelist);
                stats.vectorized += v_stats.vectorized;
                stats.reduced += v_stats.reduced;
                stats.vector_loops_seen += v_stats.loops_seen;
                stats.vector_skipped += v_stats.skipped;
                stats.vector_skip_no_iv += v_stats.skip_no_iv;
                stats.vector_skip_non_canonical_bound += v_stats.skip_non_canonical_bound;
                stats.vector_skip_unsupported_cfg_shape += v_stats.skip_unsupported_cfg_shape;
                stats.vector_skip_indirect_index_access += v_stats.skip_indirect_index_access;
                stats.vector_skip_store_effects += v_stats.skip_store_effects;
                stats.vector_skip_no_supported_pattern += v_stats.skip_no_supported_pattern;
                let v_changed = v_stats.changed();
                Self::maybe_verify(fn_ir, "After Vectorization");
                pass_changed |= v_changed;

                let type_spec_post_vec = type_specialize::optimize(fn_ir);
                Self::maybe_verify(fn_ir, "After TypeSpecialize(PostVec)");
                pass_changed |= type_spec_post_vec;

                // TCO
                let tco_changed = tco::optimize(fn_ir);
                if tco_changed {
                    stats.tco_hits += 1;
                }
                Self::maybe_verify(fn_ir, "After TCO");
                pass_changed |= tco_changed;
            }

            if pass_changed {
                changed = true;
                // Intensive cleanup after structural changes
                let sc_changed = self.simplify_cfg(fn_ir);
                if sc_changed {
                    stats.simplify_hits += 1;
                }
                Self::maybe_verify(fn_ir, "After Structural SimplifyCFG");
                let dce_changed = self.dce(fn_ir);
                if dce_changed {
                    stats.dce_hits += 1;
                }
                Self::maybe_verify(fn_ir, "After Structural DCE");
                changed |= sc_changed || dce_changed;
            }

            // 2. Standard optimization passes
            let sc_changed = self.simplify_cfg(fn_ir);
            if sc_changed {
                stats.simplify_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After SimplifyCFG");
            changed |= sc_changed;

            let sccp_changed = sccp::MirSCCP::new().optimize(fn_ir);
            if sccp_changed {
                stats.sccp_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After SCCP");
            changed |= sccp_changed;

            let intr_changed = intrinsics::optimize(fn_ir);
            if intr_changed {
                stats.intrinsics_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After Intrinsics");
            changed |= intr_changed;

            let gvn_changed = if Self::gvn_enabled() {
                let c = gvn::optimize(fn_ir);
                if c {
                    stats.gvn_hits += 1;
                }
                c
            } else {
                false
            };
            Self::maybe_verify(fn_ir, "After GVN");
            changed |= gvn_changed;

            let simplify_changed = simplify::optimize(fn_ir);
            if simplify_changed {
                stats.simplify_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After Simplify");
            changed |= simplify_changed;

            let dce_changed = self.dce(fn_ir);
            if dce_changed {
                stats.dce_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After DCE");
            changed |= dce_changed;

            if !(heavy_pass_budgeted && iterations > 1) {
                let loop_changed_count = loop_opt.optimize_with_count(fn_ir);
                stats.simplified_loops += loop_changed_count;
                let loop_changed = loop_changed_count > 0;
                Self::maybe_verify(fn_ir, "After LoopOpt");
                changed |= loop_changed;

                let licm_changed = if Self::licm_enabled() {
                    let c = licm::MirLicm::new().optimize(fn_ir);
                    if c {
                        stats.licm_hits += 1;
                    }
                    c
                } else {
                    false
                };
                Self::maybe_verify(fn_ir, "After LICM");
                changed |= licm_changed;

                let fresh_changed = fresh_alloc::optimize(fn_ir);
                if fresh_changed {
                    stats.fresh_alloc_hits += 1;
                }
                Self::maybe_verify(fn_ir, "After FreshAlloc");
                changed |= fresh_changed;

                let bce_changed = bce::optimize(fn_ir);
                if bce_changed {
                    stats.bce_hits += 1;
                }
                Self::maybe_verify(fn_ir, "After BCE");
                changed |= bce_changed;
            }
            // check_elimination remains disabled.

            let after_hash = Self::fn_ir_fingerprint(fn_ir);
            if after_hash == before_hash {
                break;
            }
            if !seen_hashes.insert(after_hash) {
                // Degenerate oscillation guard.
                break;
            }
            changed |= after_hash != before_hash;
        }

        // Final polishing pass
        let mut polishing = true;
        let mut polish_guard = 0usize;
        let mut polish_seen: FxHashSet<u64> = FxHashSet::default();
        while polishing && polish_guard < 16 {
            if start_time.elapsed().as_millis() > Self::max_fn_opt_ms() {
                break;
            }
            polish_guard += 1;
            let before_polish = Self::fn_ir_fingerprint(fn_ir);
            polishing = self.simplify_cfg(fn_ir);
            if polishing {
                stats.simplify_hits += 1;
            }
            let dce_changed = self.dce(fn_ir);
            if dce_changed {
                stats.dce_hits += 1;
            }
            polishing |= dce_changed;
            let after_polish = Self::fn_ir_fingerprint(fn_ir);
            if after_polish == before_polish || !polish_seen.insert(after_polish) {
                break;
            }
        }
        let _ = Self::verify_or_reject(fn_ir, "End");
        stats
    }

    // Backward-compat wrappers.
    pub fn prepare_for_codegen(&self, all_fns: &mut FxHashMap<String, FnIR>) {
        self.stabilize_for_codegen(all_fns);
    }

    pub fn optimize_all(&self, all_fns: &mut FxHashMap<String, FnIR>) {
        self.run_program(all_fns);
    }

    pub fn optimize_function(&self, fn_ir: &mut FnIR) {
        self.run_function(fn_ir);
    }

    fn collect_callmap_user_whitelist(all_fns: &FxHashMap<String, FnIR>) -> FxHashSet<String> {
        let mut whitelist: FxHashSet<String> = FxHashSet::default();
        let mut changed = true;
        while changed {
            changed = false;
            let ordered_names = Self::sorted_fn_names(all_fns);
            for name in &ordered_names {
                let Some(fn_ir) = all_fns.get(name) else {
                    continue;
                };
                if whitelist.contains(name) {
                    continue;
                }
                if Self::is_callmap_vector_safe_user_fn(name, fn_ir, &whitelist) {
                    whitelist.insert(name.clone());
                    changed = true;
                }
            }
        }
        whitelist
    }

    fn collect_wrap_index_helpers(all_fns: &FxHashMap<String, FnIR>) -> FxHashSet<String> {
        let mut helpers = FxHashSet::default();
        let ordered = Self::sorted_fn_names(all_fns);
        for name in ordered {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            if Self::is_wrap_index_helper_fn(fn_ir) {
                helpers.insert(name);
            }
        }
        helpers
    }

    fn rewrite_wrap_index_helper_calls(
        all_fns: &mut FxHashMap<String, FnIR>,
        helpers: &FxHashSet<String>,
    ) -> usize {
        let mut rewrites = 0usize;
        for fn_ir in all_fns.values_mut() {
            for v in &mut fn_ir.values {
                let ValueKind::Call {
                    callee,
                    args,
                    names,
                } = &mut v.kind
                else {
                    continue;
                };
                if !helpers.contains(callee.as_str()) || args.len() != 4 {
                    continue;
                }
                *callee = "rr_wrap_index_vec_i".to_string();
                *names = vec![None, None, None, None];
                rewrites += 1;
            }
        }
        rewrites
    }

    fn is_wrap_index_helper_fn(fn_ir: &FnIR) -> bool {
        macro_rules! fail {
            ($msg:expr) => {{
                if Self::wrap_trace_enabled() {
                    eprintln!("   [wrap-detect] {}: {}", fn_ir.name, $msg);
                }
                return false;
            }};
        }
        if fn_ir.unsupported_dynamic || fn_ir.params.len() != 4 {
            return false;
        }

        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                match ins {
                    Instr::Assign { .. } => {}
                    Instr::Eval { .. }
                    | Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. } => fail!("contains eval/store"),
                }
            }
        }

        let mut rules: Vec<(String, bool, usize)> = Vec::new();
        for bb in &fn_ir.blocks {
            let Terminator::If {
                cond,
                then_bb,
                else_bb,
            } = bb.term
            else {
                continue;
            };
            let Some((var, is_lt, bound_param)) =
                Self::parse_wrap_if_rule(fn_ir, cond, then_bb, else_bb)
            else {
                fail!("if rule parse failed");
            };
            rules.push((var, is_lt, bound_param));
        }

        if rules.len() != 4 {
            fail!("if rule count != 4");
        }

        let mut by_var: FxHashMap<String, Vec<(bool, usize)>> = FxHashMap::default();
        for (var, is_lt, bound) in rules {
            by_var.entry(var).or_default().push((is_lt, bound));
        }
        if by_var.len() != 2 {
            fail!("rule vars != 2");
        }

        let mut x_var: Option<String> = None;
        let mut y_var: Option<String> = None;
        for (var, rs) in &by_var {
            if rs.len() != 2 {
                fail!("rules per var != 2");
            }
            let mut saw_lt = None;
            let mut saw_gt = None;
            for (is_lt, bound) in rs {
                if *is_lt {
                    saw_lt = Some(*bound);
                } else {
                    saw_gt = Some(*bound);
                }
            }
            let Some(lt_bound) = saw_lt else {
                fail!("missing lt bound");
            };
            let Some(gt_bound) = saw_gt else {
                fail!("missing gt bound");
            };
            if lt_bound != gt_bound {
                fail!("lt/gt bound mismatch");
            }
            match lt_bound {
                2 => x_var = Some(var.clone()),
                3 => y_var = Some(var.clone()),
                _ => fail!("bound param not 2/3"),
            }
        }

        let Some(x_var) = x_var else {
            fail!("missing x var");
        };
        let Some(y_var) = y_var else {
            fail!("missing y var");
        };

        if !Self::assignments_match_wrap_sources(fn_ir, &x_var, 0, 2)
            || !Self::assignments_match_wrap_sources(fn_ir, &y_var, 1, 3)
        {
            fail!("assignment source mismatch");
        }

        if !Self::return_matches_wrap_expr(fn_ir, &x_var, &y_var) {
            fail!("return expression mismatch");
        }
        if Self::wrap_trace_enabled() {
            eprintln!("   [wrap-detect] {}: matched", fn_ir.name);
        }
        true
    }

    fn parse_wrap_if_rule(
        fn_ir: &FnIR,
        cond: ValueId,
        then_bb: BlockId,
        else_bb: BlockId,
    ) -> Option<(String, bool, usize)> {
        let then_assign = Self::single_assign_block(fn_ir, then_bb);
        if Self::wrap_trace_enabled() && then_assign.is_none() {
            eprintln!(
                "   [wrap-rule] {}: then_bb {} is not single-assign",
                fn_ir.name, then_bb
            );
        }
        let (then_assign_var, then_assign_src) = then_assign?;
        let else_assign = Self::single_assign_block(fn_ir, else_bb);
        if else_assign.is_some() {
            if Self::wrap_trace_enabled() {
                eprintln!(
                    "   [wrap-rule] {}: else_bb {} has assign",
                    fn_ir.name, else_bb
                );
            }
            return None;
        }

        let (cond_var, op_is_lt, cond_bound_param, cond_bound_is_one) =
            match Self::parse_wrap_cond(fn_ir, cond) {
                Some(v) => v,
                None => {
                    if Self::wrap_trace_enabled() {
                        eprintln!(
                            "   [wrap-rule] {}: cond {} parse failed kind={:?}",
                            fn_ir.name, cond, fn_ir.values[cond].kind
                        );
                    }
                    return None;
                }
            };
        if then_assign_var != cond_var {
            if Self::wrap_trace_enabled() {
                eprintln!(
                    "   [wrap-rule] {}: then dst {} != cond var {}",
                    fn_ir.name, then_assign_var, cond_var
                );
            }
            return None;
        }

        let assign_src_param = Self::value_param_index(fn_ir, then_assign_src);
        let assign_src_is_one = Self::value_is_const_one(fn_ir, then_assign_src);
        if op_is_lt && cond_bound_is_one {
            let Some(p) = assign_src_param else {
                if Self::wrap_trace_enabled() {
                    eprintln!(
                        "   [wrap-rule] {}: lt rule src is not param (src={} kind={:?} origin={:?})",
                        fn_ir.name,
                        then_assign_src,
                        fn_ir.values[then_assign_src].kind,
                        fn_ir.values[then_assign_src].origin_var
                    );
                }
                return None;
            };
            if p >= 2 {
                return Some((cond_var, true, p));
            }
            if Self::wrap_trace_enabled() {
                eprintln!(
                    "   [wrap-rule] {}: lt rule bound param {} < 2",
                    fn_ir.name, p
                );
            }
            return None;
        }
        if !op_is_lt && assign_src_is_one {
            let Some(p) = cond_bound_param else {
                if Self::wrap_trace_enabled() {
                    eprintln!(
                        "   [wrap-rule] {}: gt rule bound is not param (cond={})",
                        fn_ir.name, cond
                    );
                }
                return None;
            };
            if p >= 2 {
                return Some((cond_var, false, p));
            }
            if Self::wrap_trace_enabled() {
                eprintln!(
                    "   [wrap-rule] {}: gt rule bound param {} < 2",
                    fn_ir.name, p
                );
            }
            return None;
        }
        if Self::wrap_trace_enabled() {
            eprintln!(
                "   [wrap-rule] {}: no matching lt/gt rewrite case (op_is_lt={}, bound_is_one={}, src_param={:?}, src_one={}, cond_param={:?})",
                fn_ir.name,
                op_is_lt,
                cond_bound_is_one,
                assign_src_param,
                assign_src_is_one,
                cond_bound_param
            );
        }
        None
    }

    fn single_assign_block(fn_ir: &FnIR, bid: BlockId) -> Option<(String, ValueId)> {
        let mut out: Option<(String, ValueId)> = None;
        for ins in &fn_ir.blocks[bid].instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                return None;
            };
            if out.is_some() {
                return None;
            }
            out = Some((dst.clone(), *src));
        }
        out
    }

    fn parse_wrap_cond(fn_ir: &FnIR, cond: ValueId) -> Option<(String, bool, Option<usize>, bool)> {
        let cond = Self::resolve_load_alias_value(fn_ir, cond);
        let ValueKind::Binary { op, lhs, rhs } = &fn_ir.values[cond].kind else {
            return None;
        };
        let lhs_var = Self::value_non_param_var_name(fn_ir, *lhs);
        let rhs_var = Self::value_non_param_var_name(fn_ir, *rhs);
        let lhs_is_one = Self::value_is_const_one(fn_ir, *lhs);
        let rhs_is_one = Self::value_is_const_one(fn_ir, *rhs);
        let lhs_param = Self::value_param_index(fn_ir, *lhs);
        let rhs_param = Self::value_param_index(fn_ir, *rhs);

        let out = match op {
            BinOp::Lt | BinOp::Gt => {
                if let Some(var) = lhs_var {
                    let is_lt = matches!(op, BinOp::Lt);
                    Some((var, is_lt, rhs_param, rhs_is_one))
                } else if let Some(var) = rhs_var {
                    let is_lt = matches!(op, BinOp::Gt);
                    Some((var, is_lt, lhs_param, lhs_is_one))
                } else {
                    None
                }
            }
            _ => None,
        };
        if out.is_none() && Self::wrap_trace_enabled() {
            eprintln!(
                "   [wrap-cond] {} cond={} op={:?} lhs={} kind={:?} origin={:?} rhs={} kind={:?} origin={:?}",
                fn_ir.name,
                cond,
                op,
                lhs,
                fn_ir.values[*lhs].kind,
                fn_ir.values[*lhs].origin_var,
                rhs,
                fn_ir.values[*rhs].kind,
                fn_ir.values[*rhs].origin_var
            );
        }
        out
    }

    fn assignments_match_wrap_sources(
        fn_ir: &FnIR,
        var: &str,
        seed_param: usize,
        bound_param: usize,
    ) -> bool {
        let mut saw_seed = false;
        let mut saw_bound = false;
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                if let Some(p) = Self::value_param_index(fn_ir, *src) {
                    if p == seed_param {
                        saw_seed = true;
                        continue;
                    }
                    if p == bound_param {
                        saw_bound = true;
                        continue;
                    }
                    return false;
                }
                if Self::value_is_const_one(fn_ir, *src) {
                    continue;
                }
                return false;
            }
        }
        saw_seed && saw_bound
    }

    fn return_matches_wrap_expr(fn_ir: &FnIR, x_var: &str, y_var: &str) -> bool {
        let mut return_vals = Vec::new();
        for bb in &fn_ir.blocks {
            if let Terminator::Return(Some(v)) = bb.term {
                return_vals.push(v);
            }
        }
        if return_vals.len() != 1 {
            return false;
        }
        let ret = Self::resolve_load_alias_value(fn_ir, return_vals[0]);
        Self::is_wrap_return_expr(fn_ir, ret, x_var, y_var)
    }

    fn is_wrap_return_expr(fn_ir: &FnIR, ret: ValueId, x_var: &str, y_var: &str) -> bool {
        let ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } = &fn_ir.values[ret].kind
        else {
            return false;
        };
        Self::is_wrap_return_form(fn_ir, *lhs, *rhs, x_var, y_var)
            || Self::is_wrap_return_form(fn_ir, *rhs, *lhs, x_var, y_var)
    }

    fn is_wrap_return_form(
        fn_ir: &FnIR,
        mul_side: ValueId,
        x_side: ValueId,
        x_var: &str,
        y_var: &str,
    ) -> bool {
        if Self::value_var_name(fn_ir, x_side).as_deref() != Some(x_var) {
            return false;
        }
        let ValueKind::Binary {
            op: BinOp::Mul,
            lhs,
            rhs,
        } = &fn_ir.values[Self::resolve_load_alias_value(fn_ir, mul_side)].kind
        else {
            return false;
        };

        let lhs_is_y = Self::is_y_minus_one(fn_ir, *lhs, y_var);
        let rhs_is_y = Self::is_y_minus_one(fn_ir, *rhs, y_var);
        let lhs_is_w = Self::value_param_index(fn_ir, *lhs) == Some(2);
        let rhs_is_w = Self::value_param_index(fn_ir, *rhs) == Some(2);

        (lhs_is_y && rhs_is_w) || (rhs_is_y && lhs_is_w)
    }

    fn is_y_minus_one(fn_ir: &FnIR, vid: ValueId, y_var: &str) -> bool {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        let ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } = &fn_ir.values[v].kind
        else {
            return false;
        };
        Self::value_var_name(fn_ir, *lhs).as_deref() == Some(y_var)
            && Self::value_is_const_one(fn_ir, *rhs)
    }

    fn value_var_name(fn_ir: &FnIR, vid: ValueId) -> Option<String> {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        if let ValueKind::Load { var } = &fn_ir.values[v].kind {
            return Some(var.clone());
        }
        fn_ir.values[v].origin_var.clone()
    }

    fn value_non_param_var_name(fn_ir: &FnIR, vid: ValueId) -> Option<String> {
        let var = Self::value_var_name(fn_ir, vid)?;
        if Self::param_index_for_var(fn_ir, &var).is_some() {
            None
        } else {
            Some(var)
        }
    }

    fn param_index_for_var(fn_ir: &FnIR, var: &str) -> Option<usize> {
        if let Some(idx) = fn_ir.params.iter().position(|p| p == var) {
            return Some(idx);
        }
        if let Some(stripped) = var.strip_prefix(".arg_") {
            return fn_ir.params.iter().position(|p| p == stripped);
        }
        if let Some(stripped) = var.strip_prefix("arg_") {
            return fn_ir.params.iter().position(|p| p == stripped);
        }
        None
    }

    fn value_param_index(fn_ir: &FnIR, vid: ValueId) -> Option<usize> {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        match fn_ir.values[v].kind {
            ValueKind::Param { index } => Some(index),
            _ => {
                let var = fn_ir.values[v].origin_var.as_deref()?;
                Self::param_index_for_var(fn_ir, var)
            }
        }
    }

    fn value_is_const_one(fn_ir: &FnIR, vid: ValueId) -> bool {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        match fn_ir.values[v].kind {
            ValueKind::Const(Lit::Int(n)) => n == 1,
            ValueKind::Const(Lit::Float(f)) => (f - 1.0).abs() < f64::EPSILON,
            _ => false,
        }
    }

    fn value_is_const_six(fn_ir: &FnIR, vid: ValueId) -> bool {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        match fn_ir.values[v].kind {
            ValueKind::Const(Lit::Int(n)) => n == 6,
            ValueKind::Const(Lit::Float(f)) => (f - 6.0).abs() < f64::EPSILON,
            _ => false,
        }
    }

    fn collect_cube_index_helpers(all_fns: &FxHashMap<String, FnIR>) -> FxHashSet<String> {
        let round_helpers = Self::collect_round_helpers(all_fns);
        let mut helpers = FxHashSet::default();
        let ordered = Self::sorted_fn_names(all_fns);
        for name in ordered {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            if Self::is_cube_index_helper_fn(fn_ir, &round_helpers) {
                helpers.insert(name);
            }
        }
        helpers
    }

    fn collect_round_helpers(all_fns: &FxHashMap<String, FnIR>) -> FxHashSet<String> {
        let mut helpers = FxHashSet::default();
        for name in Self::sorted_fn_names(all_fns) {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            if Self::is_rr_round_helper_fn(fn_ir) {
                helpers.insert(name);
            }
        }
        helpers
    }

    fn rewrite_cube_index_helper_calls(
        all_fns: &mut FxHashMap<String, FnIR>,
        helpers: &FxHashSet<String>,
    ) -> usize {
        let mut rewrites = 0usize;
        for fn_ir in all_fns.values_mut() {
            for v in &mut fn_ir.values {
                let ValueKind::Call {
                    callee,
                    args,
                    names,
                } = &mut v.kind
                else {
                    continue;
                };
                if !helpers.contains(callee.as_str()) || args.len() != 4 {
                    continue;
                }
                *callee = "rr_idx_cube_vec_i".to_string();
                *names = vec![None, None, None, None];
                rewrites += 1;
            }
        }
        rewrites
    }

    fn is_cube_index_helper_fn(fn_ir: &FnIR, round_helpers: &FxHashSet<String>) -> bool {
        macro_rules! fail {
            ($msg:expr) => {{
                if Self::wrap_trace_enabled() {
                    eprintln!("   [cube-detect] {}: {}", fn_ir.name, $msg);
                }
                return false;
            }};
        }

        if fn_ir.unsupported_dynamic || fn_ir.params.len() != 4 {
            return false;
        }
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                match ins {
                    Instr::Assign { .. } => {}
                    Instr::Eval { .. }
                    | Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. } => fail!("contains eval/store"),
                }
            }
        }

        let Some(vars) = Self::cube_index_return_vars(fn_ir) else {
            fail!("return expression mismatch");
        };
        if !Self::assignments_match_cube_seed_source(fn_ir, round_helpers, &vars.face_var, 0)
            || !Self::assignments_match_cube_seed_source(fn_ir, round_helpers, &vars.x_var, 1)
            || !Self::assignments_match_cube_seed_source(fn_ir, round_helpers, &vars.y_var, 2)
            || !Self::assignments_match_cube_seed_source(fn_ir, round_helpers, &vars.size_var, 3)
        {
            fail!("seed assignment mismatch");
        }

        let mut rules: Vec<(String, bool, ClampBound)> = Vec::new();
        for bb in &fn_ir.blocks {
            let Terminator::If {
                cond,
                then_bb,
                else_bb,
            } = bb.term
            else {
                continue;
            };
            let Some(rule) = Self::parse_cube_if_rule(fn_ir, cond, then_bb, else_bb) else {
                fail!("if rule parse failed");
            };
            rules.push(rule);
        }
        if rules.len() != 6 {
            fail!("if rule count != 6");
        }

        let expected = [
            (
                vars.face_var.clone(),
                vec![(true, ClampBound::ConstOne), (false, ClampBound::ConstSix)],
            ),
            (
                vars.x_var.clone(),
                vec![
                    (true, ClampBound::ConstOne),
                    (false, ClampBound::Var(vars.size_var.clone())),
                ],
            ),
            (
                vars.y_var.clone(),
                vec![
                    (true, ClampBound::ConstOne),
                    (false, ClampBound::Var(vars.size_var.clone())),
                ],
            ),
        ];
        for (var, wanted) in expected {
            let seen: Vec<(bool, ClampBound)> = rules
                .iter()
                .filter(|(rule_var, _, _)| rule_var == &var)
                .map(|(_, is_lt, bound)| (*is_lt, bound.clone()))
                .collect();
            if seen.len() != wanted.len() {
                fail!("rule multiplicity mismatch");
            }
            for need in wanted {
                if !seen.contains(&need) {
                    fail!("missing clamp rule");
                }
            }
        }

        if Self::wrap_trace_enabled() {
            eprintln!("   [cube-detect] {}: matched", fn_ir.name);
        }
        true
    }

    fn cube_index_return_vars(fn_ir: &FnIR) -> Option<CubeIndexReturnVars> {
        let mut returns = Vec::new();
        for bb in &fn_ir.blocks {
            if let Terminator::Return(Some(v)) = bb.term {
                returns.push(v);
            }
        }
        if returns.len() != 1 {
            return None;
        }
        let ret = Self::resolve_load_alias_value(fn_ir, returns[0]);
        let mut terms = Vec::new();
        Self::flatten_assoc_binop(fn_ir, ret, BinOp::Add, &mut terms);
        if terms.len() != 3 {
            return None;
        }

        let mut face_var: Option<String> = None;
        let mut x_var: Option<String> = None;
        let mut y_var: Option<String> = None;
        let mut size_var: Option<String> = None;

        for term in terms {
            if let Some(var) = Self::value_var_name(fn_ir, term) {
                if y_var.is_some() {
                    return None;
                }
                y_var = Some(var);
                continue;
            }

            let mut factors = Vec::new();
            Self::flatten_assoc_binop(fn_ir, term, BinOp::Mul, &mut factors);
            let sub_vars: Vec<String> = factors
                .iter()
                .filter_map(|f| Self::parse_var_minus_one(fn_ir, *f))
                .collect();
            let plain_vars: Vec<String> = factors
                .iter()
                .filter_map(|f| Self::value_var_name(fn_ir, *f))
                .collect();

            match (sub_vars.as_slice(), plain_vars.as_slice()) {
                ([sub], [size]) => {
                    if x_var.is_some() {
                        return None;
                    }
                    x_var = Some(sub.clone());
                    match &size_var {
                        None => size_var = Some(size.clone()),
                        Some(prev) if prev == size => {}
                        Some(_) => return None,
                    }
                }
                ([sub], [size_a, size_b]) if size_a == size_b => {
                    if face_var.is_some() {
                        return None;
                    }
                    face_var = Some(sub.clone());
                    match &size_var {
                        None => size_var = Some(size_a.clone()),
                        Some(prev) if prev == size_a => {}
                        Some(_) => return None,
                    }
                }
                _ => return None,
            }
        }

        Some(CubeIndexReturnVars {
            face_var: face_var?,
            x_var: x_var?,
            y_var: y_var?,
            size_var: size_var?,
        })
    }

    fn flatten_assoc_binop(fn_ir: &FnIR, vid: ValueId, op: BinOp, out: &mut Vec<ValueId>) {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        match &fn_ir.values[v].kind {
            ValueKind::Binary { op: bop, lhs, rhs } if *bop == op => {
                Self::flatten_assoc_binop(fn_ir, *lhs, op, out);
                Self::flatten_assoc_binop(fn_ir, *rhs, op, out);
            }
            _ => out.push(v),
        }
    }

    fn parse_var_minus_one(fn_ir: &FnIR, vid: ValueId) -> Option<String> {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        let ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } = &fn_ir.values[v].kind
        else {
            return None;
        };
        if !Self::value_is_const_one(fn_ir, *rhs) {
            return None;
        }
        Self::value_var_name(fn_ir, *lhs)
    }

    fn block_assignments(fn_ir: &FnIR, bid: BlockId) -> Option<Vec<(String, ValueId)>> {
        let mut out = Vec::new();
        for ins in &fn_ir.blocks[bid].instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                return None;
            };
            out.push((dst.clone(), *src));
        }
        Some(out)
    }

    fn parse_cube_bound(fn_ir: &FnIR, vid: ValueId) -> Option<ClampBound> {
        if Self::value_is_const_one(fn_ir, vid) {
            return Some(ClampBound::ConstOne);
        }
        if Self::value_is_const_six(fn_ir, vid) {
            return Some(ClampBound::ConstSix);
        }
        Self::value_var_name(fn_ir, vid).map(ClampBound::Var)
    }

    fn parse_cube_cond(fn_ir: &FnIR, cond: ValueId) -> Option<(String, bool, ClampBound)> {
        let cond = Self::resolve_load_alias_value(fn_ir, cond);
        let ValueKind::Binary { op, lhs, rhs } = &fn_ir.values[cond].kind else {
            return None;
        };
        match op {
            BinOp::Lt | BinOp::Gt => {
                if let Some(var) = Self::value_non_param_var_name(fn_ir, *lhs) {
                    Some((
                        var,
                        matches!(op, BinOp::Lt),
                        Self::parse_cube_bound(fn_ir, *rhs)?,
                    ))
                } else if let Some(var) = Self::value_non_param_var_name(fn_ir, *rhs) {
                    Some((
                        var,
                        matches!(op, BinOp::Gt),
                        Self::parse_cube_bound(fn_ir, *lhs)?,
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn cube_bound_matches_value(fn_ir: &FnIR, bound: &ClampBound, vid: ValueId) -> bool {
        match bound {
            ClampBound::ConstOne => Self::value_is_const_one(fn_ir, vid),
            ClampBound::ConstSix => Self::value_is_const_six(fn_ir, vid),
            ClampBound::Var(var) => {
                Self::value_var_name(fn_ir, vid).as_deref() == Some(var.as_str())
            }
        }
    }

    fn is_benign_cube_aux_assignment(
        fn_ir: &FnIR,
        dst: &str,
        src: ValueId,
        cond_var: &str,
        bound: &ClampBound,
    ) -> bool {
        match bound {
            ClampBound::Var(bound_var) if dst == bound_var => {
                let src_var = Self::value_var_name(fn_ir, src);
                src_var.as_deref() == Some(cond_var)
                    || src_var.as_deref() == Some(bound_var.as_str())
            }
            _ => false,
        }
    }

    fn parse_cube_if_rule(
        fn_ir: &FnIR,
        cond: ValueId,
        then_bb: BlockId,
        else_bb: BlockId,
    ) -> Option<(String, bool, ClampBound)> {
        let then_assigns = Self::block_assignments(fn_ir, then_bb)?;
        if then_assigns.is_empty() {
            return None;
        }
        let else_assigns = Self::block_assignments(fn_ir, else_bb)?;
        if !else_assigns.is_empty() {
            return None;
        }
        let (cond_var, is_lt, bound) = Self::parse_cube_cond(fn_ir, cond)?;
        let mut saw_primary = false;
        for (dst, src) in then_assigns {
            if dst == cond_var && Self::cube_bound_matches_value(fn_ir, &bound, src) {
                saw_primary = true;
                continue;
            }
            if !Self::is_benign_cube_aux_assignment(fn_ir, &dst, src, &cond_var, &bound) {
                return None;
            }
        }
        if !saw_primary {
            return None;
        }
        Some((cond_var, is_lt, bound))
    }

    fn assignments_match_cube_seed_source(
        fn_ir: &FnIR,
        round_helpers: &FxHashSet<String>,
        var: &str,
        param_idx: usize,
    ) -> bool {
        let mut saw_seed = false;
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                if Self::value_param_index(fn_ir, *src) == Some(param_idx)
                    || Self::is_round_call_of_param(fn_ir, round_helpers, *src, param_idx)
                {
                    saw_seed = true;
                }
            }
        }
        saw_seed
    }

    fn is_round_call_of_param(
        fn_ir: &FnIR,
        round_helpers: &FxHashSet<String>,
        vid: ValueId,
        param_idx: usize,
    ) -> bool {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        let ValueKind::Call { callee, args, .. } = &fn_ir.values[v].kind else {
            return false;
        };
        args.len() == 1
            && Self::value_param_index(fn_ir, args[0]) == Some(param_idx)
            && (callee == "round" || round_helpers.contains(callee))
    }

    fn is_rr_round_helper_fn(fn_ir: &FnIR) -> bool {
        macro_rules! fail {
            ($msg:expr) => {{
                if Self::wrap_trace_enabled() {
                    eprintln!("   [round-detect] {}: {}", fn_ir.name, $msg);
                }
                return false;
            }};
        }
        if fn_ir.unsupported_dynamic || fn_ir.params.len() != 1 {
            fail!("unsupported dynamic or arity != 1");
        }
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                match ins {
                    Instr::Assign { .. } => {}
                    Instr::Eval { .. }
                    | Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. } => fail!("contains eval/store"),
                }
            }
        }

        let mut saw_mod_seed = false;
        let mut rem_var: Option<String> = None;
        let mut branch_term: Option<(BlockId, BlockId, ValueId)> = None;
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                let v = Self::resolve_load_alias_value(fn_ir, *src);
                let ValueKind::Binary {
                    op: BinOp::Mod,
                    lhs,
                    rhs,
                } = &fn_ir.values[v].kind
                else {
                    continue;
                };
                if Self::value_param_index(fn_ir, *lhs) == Some(0)
                    && Self::value_is_const_one(fn_ir, *rhs)
                {
                    saw_mod_seed = true;
                    rem_var = Some(dst.clone());
                }
            }
            if let Terminator::If {
                cond,
                then_bb,
                else_bb,
            } = bb.term
            {
                branch_term = Some((then_bb, else_bb, cond));
            }
        }
        let Some(rem_var) = rem_var else {
            fail!("missing rem seed");
        };
        if !saw_mod_seed {
            fail!("mod seed not seen");
        }
        let Some((then_bb, else_bb, cond)) = branch_term else {
            fail!("missing branch");
        };
        let cond = Self::resolve_load_alias_value(fn_ir, cond);
        let ValueKind::Binary {
            op: BinOp::Ge,
            lhs,
            rhs,
        } = &fn_ir.values[cond].kind
        else {
            fail!("cond is not >= binary");
        };
        if Self::value_var_name(fn_ir, *lhs).as_deref() != Some(rem_var.as_str())
            || !Self::value_is_const_half(fn_ir, *rhs)
        {
            fail!("cond does not compare rem >= 0.5");
        }

        if !Self::block_returns_param_minus_rem_plus_one(fn_ir, then_bb, 0, &rem_var) {
            fail!("then block is not (x - r) + 1");
        }
        if !Self::block_returns_param_minus_rem(fn_ir, else_bb, 0, &rem_var) {
            fail!("else block is not x - r");
        }
        true
    }

    fn block_returns_param_minus_rem_plus_one(
        fn_ir: &FnIR,
        bid: BlockId,
        param_idx: usize,
        rem_var: &str,
    ) -> bool {
        let Some(val) = Self::block_single_return_value(fn_ir, bid) else {
            return false;
        };
        let v = Self::resolve_load_alias_value(fn_ir, val);
        let ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } = &fn_ir.values[v].kind
        else {
            return false;
        };
        (Self::block_returns_param_minus_rem_expr(fn_ir, *lhs, param_idx, rem_var)
            && Self::value_is_const_one(fn_ir, *rhs))
            || (Self::block_returns_param_minus_rem_expr(fn_ir, *rhs, param_idx, rem_var)
                && Self::value_is_const_one(fn_ir, *lhs))
    }

    fn block_returns_param_minus_rem(
        fn_ir: &FnIR,
        bid: BlockId,
        param_idx: usize,
        rem_var: &str,
    ) -> bool {
        let Some(val) = Self::block_single_return_value(fn_ir, bid) else {
            return false;
        };
        Self::block_returns_param_minus_rem_expr(fn_ir, val, param_idx, rem_var)
    }

    fn block_returns_param_minus_rem_expr(
        fn_ir: &FnIR,
        vid: ValueId,
        param_idx: usize,
        rem_var: &str,
    ) -> bool {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        let ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } = &fn_ir.values[v].kind
        else {
            return false;
        };
        Self::value_param_index(fn_ir, *lhs) == Some(param_idx)
            && Self::value_var_name(fn_ir, *rhs).as_deref() == Some(rem_var)
    }

    fn block_single_return_value(fn_ir: &FnIR, bid: BlockId) -> Option<ValueId> {
        match fn_ir.blocks[bid].term {
            Terminator::Return(Some(v)) => Some(v),
            _ => None,
        }
    }

    fn value_is_const_half(fn_ir: &FnIR, vid: ValueId) -> bool {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        match fn_ir.values[v].kind {
            ValueKind::Const(Lit::Float(f)) => (f - 0.5).abs() < f64::EPSILON,
            _ => false,
        }
    }

    fn is_callmap_vector_safe_user_fn(
        name: &str,
        fn_ir: &FnIR,
        user_whitelist: &FxHashSet<String>,
    ) -> bool {
        if fn_ir.unsupported_dynamic {
            return false;
        }
        if name.starts_with("Sym_top_") {
            return false;
        }

        // Restrict to expression-like functions: no stores, no explicit eval, no branching.
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                match ins {
                    Instr::Assign { .. } => {}
                    Instr::Eval { .. } => return false,
                    Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. } => return false,
                }
            }
            match bb.term {
                Terminator::Goto(_) | Terminator::Return(_) | Terminator::Unreachable => {}
                Terminator::If { .. } => return false,
            }
        }

        // All returns must be value-returns and vector-safe expression trees.
        let mut saw_return = false;
        for bb in &fn_ir.blocks {
            if let Terminator::Return(ret) = bb.term {
                let Some(ret_vid) = ret else { return false };
                saw_return = true;
                if !Self::is_vector_safe_user_expr(
                    fn_ir,
                    ret_vid,
                    user_whitelist,
                    &mut FxHashSet::default(),
                ) {
                    return false;
                }
            }
        }
        saw_return
    }

    fn is_vector_safe_user_expr(
        fn_ir: &FnIR,
        vid: ValueId,
        user_whitelist: &FxHashSet<String>,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        let vid = Self::resolve_load_alias_value(fn_ir, vid);
        if !seen.insert(vid) {
            return true;
        }
        match &fn_ir.values[vid].kind {
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => true,
            ValueKind::Unary { rhs, .. } => {
                Self::is_vector_safe_user_expr(fn_ir, *rhs, user_whitelist, seen)
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                Self::is_vector_safe_user_expr(fn_ir, *lhs, user_whitelist, seen)
                    && Self::is_vector_safe_user_expr(fn_ir, *rhs, user_whitelist, seen)
            }
            ValueKind::Call { callee, args, .. } => {
                (v_opt::is_builtin_vector_safe_call(callee, args.len())
                    || user_whitelist.contains(callee))
                    && args
                        .iter()
                        .all(|a| Self::is_vector_safe_user_expr(fn_ir, *a, user_whitelist, seen))
            }
            ValueKind::Intrinsic { args, .. } => args
                .iter()
                .all(|a| Self::is_vector_safe_user_expr(fn_ir, *a, user_whitelist, seen)),
            ValueKind::Phi { args } => args
                .iter()
                .all(|(a, _)| Self::is_vector_safe_user_expr(fn_ir, *a, user_whitelist, seen)),
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                Self::is_vector_safe_user_expr(fn_ir, *base, user_whitelist, seen)
            }
            ValueKind::Range { start, end } => {
                Self::is_vector_safe_user_expr(fn_ir, *start, user_whitelist, seen)
                    && Self::is_vector_safe_user_expr(fn_ir, *end, user_whitelist, seen)
            }
            ValueKind::Index1D { .. } | ValueKind::Index2D { .. } => false,
        }
    }

    fn resolve_load_alias_value(fn_ir: &FnIR, vid: ValueId) -> ValueId {
        fn unique_assign_source(fn_ir: &FnIR, var: &str) -> Option<ValueId> {
            let mut src: Option<ValueId> = None;
            for bb in &fn_ir.blocks {
                for ins in &bb.instrs {
                    let Instr::Assign { dst, src: s, .. } = ins else {
                        continue;
                    };
                    if dst != var {
                        continue;
                    }
                    match src {
                        None => src = Some(*s),
                        Some(prev) if prev == *s => {}
                        Some(_) => return None,
                    }
                }
            }
            src
        }

        let mut cur = vid;
        let mut seen = FxHashSet::default();
        while seen.insert(cur) {
            if let ValueKind::Load { var } = &fn_ir.values[cur].kind
                && let Some(src) = unique_assign_source(fn_ir, var)
            {
                cur = src;
                continue;
            }
            break;
        }
        cur
    }

    fn simplify_cfg(&self, fn_ir: &mut FnIR) -> bool {
        let mut changed = false;

        // 1. Identify reachable blocks
        let mut reachable = FxHashSet::default();
        let mut queue = vec![fn_ir.entry];
        reachable.insert(fn_ir.entry);

        let mut head = 0;
        while head < queue.len() {
            let bid = queue[head];
            head += 1;

            if let Some(blk) = fn_ir.blocks.get(bid) {
                match &blk.term {
                    Terminator::Goto(target) => {
                        if reachable.insert(*target) {
                            queue.push(*target);
                        }
                    }
                    Terminator::If {
                        then_bb, else_bb, ..
                    } => {
                        if reachable.insert(*then_bb) {
                            queue.push(*then_bb);
                        }
                        if reachable.insert(*else_bb) {
                            queue.push(*else_bb);
                        }
                    }
                    _ => {}
                }
            }
        }

        // 2. Clear out unreachable blocks
        for bid in 0..fn_ir.blocks.len() {
            if !reachable.contains(&bid) {
                let blk = &mut fn_ir.blocks[bid];
                if !blk.instrs.is_empty() || !matches!(blk.term, Terminator::Unreachable) {
                    blk.instrs.clear();
                    blk.term = Terminator::Unreachable;
                    changed = true;
                }
            }
        }

        changed
    }

    fn dce(&self, fn_ir: &mut FnIR) -> bool {
        let mut changed = false;

        // 1. Mark used values
        let mut used = FxHashSet::default();

        // Final values used in terminators
        for blk in &fn_ir.blocks {
            match &blk.term {
                Terminator::If { cond, .. } => {
                    used.insert(*cond);
                }
                Terminator::Return(Some(id)) => {
                    used.insert(*id);
                }
                _ => {}
            }
        }

        // Instructions with side effects are roots
        for blk in &fn_ir.blocks {
            for instr in &blk.instrs {
                if self.has_side_effect_instr(instr, &fn_ir.values) {
                    match instr {
                        Instr::Assign { src, .. } => {
                            used.insert(*src);
                        }
                        Instr::Eval { val, .. } => {
                            used.insert(*val);
                        }
                        Instr::StoreIndex1D { base, idx, val, .. } => {
                            used.insert(*base);
                            used.insert(*idx);
                            used.insert(*val);
                        }
                        Instr::StoreIndex2D {
                            base, r, c, val, ..
                        } => {
                            used.insert(*base);
                            used.insert(*r);
                            used.insert(*c);
                            used.insert(*val);
                        }
                    }
                }
            }
        }

        // 2. Propagate usage (transitive closure)
        let mut worklist: Vec<ValueId> = used.iter().cloned().collect();
        while let Some(vid) = worklist.pop() {
            let val = &fn_ir.values[vid];
            match &val.kind {
                ValueKind::Binary { lhs, rhs, .. } => {
                    if used.insert(*lhs) {
                        worklist.push(*lhs);
                    }
                    if used.insert(*rhs) {
                        worklist.push(*rhs);
                    }
                }
                ValueKind::Unary { rhs, .. } => {
                    if used.insert(*rhs) {
                        worklist.push(*rhs);
                    }
                }
                ValueKind::Call { args, .. } => {
                    for a in args {
                        if used.insert(*a) {
                            worklist.push(*a);
                        }
                    }
                }
                ValueKind::Phi { args } => {
                    for (a, _) in args {
                        if used.insert(*a) {
                            worklist.push(*a);
                        }
                    }
                }
                ValueKind::Index1D { base, idx, .. } => {
                    if used.insert(*base) {
                        worklist.push(*base);
                    }
                    if used.insert(*idx) {
                        worklist.push(*idx);
                    }
                }
                ValueKind::Index2D { base, r, c } => {
                    if used.insert(*base) {
                        worklist.push(*base);
                    }
                    if used.insert(*r) {
                        worklist.push(*r);
                    }
                    if used.insert(*c) {
                        worklist.push(*c);
                    }
                }
                ValueKind::Len { base } => {
                    if used.insert(*base) {
                        worklist.push(*base);
                    }
                }
                ValueKind::Indices { base } => {
                    if used.insert(*base) {
                        worklist.push(*base);
                    }
                }
                ValueKind::Range { start, end } => {
                    if used.insert(*start) {
                        worklist.push(*start);
                    }
                    if used.insert(*end) {
                        worklist.push(*end);
                    }
                }
                _ => {}
            }
        }

        // 3. Remove dead instructions
        for blk in &mut fn_ir.blocks {
            let old_len = blk.instrs.len();
            let values = &fn_ir.values; // Grab values before retain closure
            blk.instrs.retain(|instr| {
                if self.has_side_effect_instr(instr, values) {
                    return true;
                }

                match instr {
                    Instr::Assign { src, .. } => used.contains(src),
                    Instr::Eval { val, .. } => used.contains(val),
                    _ => true,
                }
            });
            if blk.instrs.len() != old_len {
                changed = true;
            }
        }

        changed
    }

    fn has_side_effect_instr(&self, instr: &Instr, values: &[Value]) -> bool {
        match instr {
            Instr::StoreIndex1D { .. } => true,
            Instr::StoreIndex2D { .. } => true,
            Instr::Assign { .. } => {
                // Assignments are kept conservative unless proven dead.
                true
            }
            Instr::Eval { val, .. } => self.has_side_effect_val(*val, values),
        }
    }

    fn has_side_effect_val(&self, val_id: ValueId, values: &[Value]) -> bool {
        let val = &values[val_id];
        match &val.kind {
            ValueKind::Call { callee, .. } => {
                // Whitelist known pure functions
                let pure = [
                    "length",
                    "c",
                    "seq_along",
                    "list",
                    "sum",
                    "mean",
                    "min",
                    "max",
                    "rr_field_get",
                    "rr_named_list",
                ];
                if pure.contains(&callee.as_str()) {
                    return false;
                }
                true // Assume unknown calls have side effects
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                self.has_side_effect_val(*lhs, values) || self.has_side_effect_val(*rhs, values)
            }
            ValueKind::Unary { rhs, .. } => self.has_side_effect_val(*rhs, values),
            _ => false,
        }
    }

    fn check_elimination(&self, fn_ir: &mut FnIR) -> bool {
        let mut changed = false;

        // 1. Run Dataflow Analysis to get Interval Facts
        let facts = crate::mir::flow::DataflowSolver::analyze_function(fn_ir);

        // 2. Scan for Indexing operations
        // We need to iterate over values and instructions.

        // OPTIMIZATION: Index1D (Value)
        for val_idx in 0..fn_ir.values.len() {
            let mut is_proven_safe = false;
            {
                let val = &fn_ir.values[val_idx];
                if let ValueKind::Index1D {
                    base, idx, is_safe, ..
                } = &val.kind
                    && !*is_safe
                    && self.is_safe_access(fn_ir, *base, *idx, &facts)
                {
                    is_proven_safe = true;
                }
            }
            if is_proven_safe
                && let ValueKind::Index1D {
                    ref mut is_safe, ..
                } = fn_ir.values[val_idx].kind
            {
                *is_safe = true;
                changed = true;
            }
        }

        // OPTIMIZATION: StoreIndex1D (Instruction)
        for blk_idx in 0..fn_ir.blocks.len() {
            for instr_idx in 0..fn_ir.blocks[blk_idx].instrs.len() {
                let mut is_proven_safe = false;
                {
                    let instr = &fn_ir.blocks[blk_idx].instrs[instr_idx];
                    if let Instr::StoreIndex1D {
                        base, idx, is_safe, ..
                    } = instr
                        && !*is_safe
                        && self.is_safe_access(fn_ir, *base, *idx, &facts)
                    {
                        is_proven_safe = true;
                    }
                }
                if is_proven_safe
                    && let Instr::StoreIndex1D {
                        ref mut is_safe, ..
                    } = fn_ir.blocks[blk_idx].instrs[instr_idx]
                {
                    *is_safe = true;
                    changed = true;
                }
            }
        }

        changed
    }

    fn is_safe_access(
        &self,
        fn_ir: &FnIR,
        base_id: ValueId,
        idx_id: ValueId,
        facts: &FxHashMap<ValueId, crate::mir::flow::Facts>,
    ) -> bool {
        let f = facts.get(&idx_id).cloned().unwrap_or(Facts::empty());

        // Basic check: If it's ONE_BASED and fits in length.
        // Proving "fits in length" is hard without symbolic intervals.
        // Heuristic: If idx_id is from `Phi` of a loop whose limit is `len(base)`.

        // Case A: Index comes from `indices(base)`
        // `ValueKind::Indices { base: b }` where b == base_id?
        // Or if idx_id is a Phi whose inputs come from indices(base).

        // Case B: induction-variable pattern.
        if f.has(Facts::ONE_BASED) && self.is_derived_from_len(fn_ir, idx_id, base_id, facts) {
            return true;
        }

        false
    }

    fn is_derived_from_len(
        &self,
        fn_ir: &FnIR,
        val_id: ValueId,
        base_id: ValueId,
        _facts: &FxHashMap<ValueId, crate::mir::flow::Facts>,
    ) -> bool {
        let val = &fn_ir.values[val_id];
        match &val.kind {
            ValueKind::Indices { base } => *base == base_id,
            ValueKind::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
            } => {
                if let ValueKind::Const(Lit::Int(1)) = &fn_ir.values[*rhs].kind {
                    return self.is_loop_induction(fn_ir, *lhs, base_id);
                }
                false
            }
            ValueKind::Phi { args } => args
                .iter()
                .any(|(id, _)| self.is_derived_from_len(fn_ir, *id, base_id, _facts)),
            _ => false,
        }
    }

    fn is_loop_induction(&self, fn_ir: &FnIR, val_id: ValueId, _base_id: ValueId) -> bool {
        let val = &fn_ir.values[val_id];
        if let ValueKind::Phi { args } = &val.kind {
            for (arg_id, _) in args {
                let arg_val = &fn_ir.values[*arg_id];
                if let ValueKind::Const(Lit::Int(0)) = &arg_val.kind {
                    // Heuristic: a phi starting at zero is treated as induction.
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::Span;

    fn dummy_fn(name: &str, approx_size: usize) -> FnIR {
        let mut fn_ir = FnIR::new(name.to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let mut ret = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        // fn_ir_size = values + instrs; keep instrs=0 and control value count directly.
        let target_values = approx_size.max(1);
        while fn_ir.values.len() < target_values {
            ret = fn_ir.add_value(
                ValueKind::Const(Lit::Int(fn_ir.values.len() as i64)),
                Span::default(),
                Facts::empty(),
                None,
            );
        }
        fn_ir.blocks[entry].term = Terminator::Return(Some(ret));
        fn_ir
    }

    fn fn_with_unreachable_block(name: &str) -> FnIR {
        let mut fn_ir = FnIR::new(name.to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;
        let dead = fn_ir.add_block();
        let ret = fn_ir.add_value(
            ValueKind::Const(Lit::Int(7)),
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(ret));
        fn_ir.blocks[dead].term = Terminator::Return(Some(ret));
        fn_ir
    }

    fn program_snapshot(all_fns: &FxHashMap<String, FnIR>) -> Vec<(String, String)> {
        let mut names: Vec<String> = all_fns.keys().cloned().collect();
        names.sort();
        names
            .into_iter()
            .filter_map(|name| {
                all_fns
                    .get(&name)
                    .map(|fn_ir| (name, format!("{:?}", fn_ir)))
            })
            .collect()
    }

    #[test]
    fn opt_plan_selects_all_under_budget() {
        let mut all = FxHashMap::default();
        all.insert("a".to_string(), dummy_fn("a", 120));
        all.insert("b".to_string(), dummy_fn("b", 180));

        let plan = TachyonEngine::build_opt_plan(&all);
        assert!(!plan.selective_mode);
        assert_eq!(plan.selected_functions.len(), all.len());
    }

    #[test]
    fn opt_plan_selects_subset_over_budget() {
        let mut all = FxHashMap::default();
        for i in 0..5 {
            let name = format!("f{}", i);
            all.insert(name.clone(), dummy_fn(&name, 700));
        }

        let plan = TachyonEngine::build_opt_plan(&all);
        assert!(plan.selective_mode);
        assert!(!plan.selected_functions.is_empty());
        assert!(plan.selected_functions.len() < all.len());
    }

    #[test]
    fn opt_plan_prefers_profile_hot_function_under_budget() {
        let mut all = FxHashMap::default();
        all.insert("a".to_string(), dummy_fn("a", 620));
        all.insert("b".to_string(), dummy_fn("b", 620));
        all.insert("c".to_string(), dummy_fn("c", 620));
        all.insert("d".to_string(), dummy_fn("d", 620));
        all.insert("hot".to_string(), dummy_fn("hot", 620));

        let mut profile = FxHashMap::default();
        profile.insert("hot".to_string(), 1000usize);
        let plan = TachyonEngine::build_opt_plan_with_profile(&all, &profile);
        assert!(plan.selected_functions.contains("hot"));
    }

    #[test]
    fn always_tier_runs_light_cleanup() {
        let mut f = fn_with_unreachable_block("cleanup");
        let stats = TachyonEngine::new().run_always_tier_with_stats(&mut f, None);
        assert_eq!(stats.always_tier_functions, 1);
        assert!(crate::mir::verify::verify_ir(&f).is_ok());
    }

    #[test]
    fn run_program_is_stable_across_insertion_order() {
        let mut all_a = FxHashMap::default();
        all_a.insert("alpha".to_string(), dummy_fn("alpha", 450));
        all_a.insert("beta".to_string(), dummy_fn("beta", 460));
        all_a.insert("gamma".to_string(), dummy_fn("gamma", 470));

        let mut all_b = FxHashMap::default();
        all_b.insert("gamma".to_string(), dummy_fn("gamma", 470));
        all_b.insert("beta".to_string(), dummy_fn("beta", 460));
        all_b.insert("alpha".to_string(), dummy_fn("alpha", 450));

        let engine = TachyonEngine::new();
        let _ = engine.run_program_with_stats(&mut all_a);
        let _ = engine.run_program_with_stats(&mut all_b);

        assert_eq!(program_snapshot(&all_a), program_snapshot(&all_b));
    }

    #[test]
    fn run_program_emits_progress_events_in_deterministic_order() {
        let mut all = FxHashMap::default();
        all.insert("gamma".to_string(), dummy_fn("gamma", 470));
        all.insert("beta".to_string(), dummy_fn("beta", 460));
        all.insert("alpha".to_string(), dummy_fn("alpha", 450));

        let engine = TachyonEngine::new();
        let mut events = Vec::new();
        {
            let mut cb = |event: TachyonProgress| events.push(event);
            let _ = engine.run_program_with_stats_progress(&mut all, &mut cb);
        }

        for tier in [
            TachyonProgressTier::Always,
            TachyonProgressTier::Heavy,
            TachyonProgressTier::DeSsa,
        ] {
            let tier_events: Vec<&TachyonProgress> =
                events.iter().filter(|e| e.tier == tier).collect();
            assert_eq!(tier_events.len(), 3);
            assert_eq!(
                tier_events
                    .iter()
                    .map(|e| e.function.as_str())
                    .collect::<Vec<_>>(),
                vec!["alpha", "beta", "gamma"]
            );
            assert!(
                tier_events
                    .windows(2)
                    .all(|w| w[0].completed < w[1].completed)
            );
            let last = tier_events.last().expect("non-empty tier events");
            assert_eq!(last.completed, last.total);
        }
    }
}
