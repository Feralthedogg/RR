use crate::compiler::scheduler::{
    CompilerParallelConfig, CompilerParallelStage, CompilerScheduler,
};
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

#[path = "opt/callmap.rs"]
mod callmap;
#[path = "opt/cfg_cleanup.rs"]
mod cfg_cleanup;
#[path = "opt/config.rs"]
mod config;
#[path = "opt/copy_cleanup.rs"]
mod copy_cleanup;
#[path = "opt/helpers.rs"]
mod helpers;
#[path = "opt/phase_order.rs"]
mod phase_order;
#[path = "opt/plan.rs"]
mod plan;
#[path = "opt/safety.rs"]
mod safety;
#[path = "opt/types.rs"]
mod types;
#[path = "opt/value_utils.rs"]
mod value_utils;

pub mod bce;
pub mod de_ssa;
pub mod fresh_alias;
pub mod fresh_alloc;
pub mod gvn;
pub mod inline;
pub mod intrinsics;
pub mod licm;
pub mod loop_analysis;
pub mod loop_opt;
pub mod parallel_copy;
#[path = "opt/poly/mod.rs"]
pub mod poly;
pub mod sccp;
pub mod simplify;
pub mod tco;
pub mod type_specialize;
#[path = "opt/v_opt/mod.rs"]
pub mod v_opt;

use self::types::{FunctionBudgetProfile, FunctionPhasePlan, PhaseScheduleId, ProgramOptPlan};
pub use self::types::{
    TachyonPassTiming, TachyonPassTimings, TachyonProgress, TachyonProgressTier, TachyonPulseStats,
};

#[derive(Debug, Default, Clone)]
pub struct TachyonRunProfile {
    pub pulse_stats: TachyonPulseStats,
    pub pass_timings: TachyonPassTimings,
    pub active_pass_groups: Vec<String>,
    pub plan_summary: Vec<String>,
}

pub struct TachyonEngine {
    phase_ordering_default_mode: types::PhaseOrderingMode,
    compile_mode: crate::compiler::CompileMode,
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
        Self {
            phase_ordering_default_mode: types::PhaseOrderingMode::Off,
            compile_mode: crate::compiler::CompileMode::Standard,
        }
    }

    pub(crate) fn with_phase_ordering_default_mode(
        phase_ordering_default_mode: types::PhaseOrderingMode,
    ) -> Self {
        Self {
            phase_ordering_default_mode,
            compile_mode: crate::compiler::CompileMode::Standard,
        }
    }

    pub(crate) fn with_phase_ordering_default_mode_and_compile_mode(
        phase_ordering_default_mode: types::PhaseOrderingMode,
        compile_mode: crate::compiler::CompileMode,
    ) -> Self {
        Self {
            phase_ordering_default_mode,
            compile_mode,
        }
    }

    fn fast_dev_enabled(&self) -> bool {
        matches!(self.compile_mode, crate::compiler::CompileMode::FastDev)
    }

    fn configured_max_opt_iterations(&self) -> usize {
        if self.fast_dev_enabled() {
            8
        } else {
            Self::max_opt_iterations()
        }
    }

    fn configured_max_inline_rounds(&self) -> usize {
        if self.fast_dev_enabled() {
            1
        } else {
            Self::max_inline_rounds()
        }
    }

    fn configured_heavy_pass_fn_ir(&self) -> usize {
        if self.fast_dev_enabled() {
            384
        } else {
            Self::heavy_pass_fn_ir()
        }
    }

    fn configured_always_bce_fn_ir(&self) -> usize {
        self.configured_heavy_pass_fn_ir().max(64)
    }

    fn configured_max_fn_opt_ms(&self) -> u128 {
        if self.fast_dev_enabled() {
            80
        } else {
            Self::max_fn_opt_ms()
        }
    }

    fn configured_always_tier_max_iters(&self) -> usize {
        if self.fast_dev_enabled() {
            1
        } else {
            Self::always_tier_max_iters()
        }
    }

    fn structural_optimizations_enabled(&self) -> bool {
        !self.fast_dev_enabled()
    }

    fn inline_tier_enabled(&self) -> bool {
        true
    }

    fn adjust_pass_groups_for_mode(&self, groups: &[types::PassGroup]) -> Vec<types::PassGroup> {
        groups
            .iter()
            .copied()
            .filter(|group| match group {
                types::PassGroup::Required | types::PassGroup::DevCheap => true,
                types::PassGroup::ReleaseExpensive | types::PassGroup::Experimental => {
                    !self.fast_dev_enabled()
                }
            })
            .collect()
    }

    fn active_pass_group_labels(&self) -> Vec<String> {
        let base = [
            types::PassGroup::Required,
            types::PassGroup::DevCheap,
            types::PassGroup::ReleaseExpensive,
            types::PassGroup::Experimental,
        ];
        self.adjust_pass_groups_for_mode(&base)
            .into_iter()
            .map(|group| group.label().to_string())
            .collect()
    }

    fn plan_summary_lines(
        &self,
        ordered_names: &[String],
        plans: &FxHashMap<String, types::FunctionPhasePlan>,
    ) -> Vec<String> {
        let mut out = Vec::new();
        for name in ordered_names {
            // Proof correspondence:
            // `PhasePlanSummarySoundness` refines this ordered-summary
            // consumption boundary on top of `PhasePlanLookupSoundness`.
            // The reduced model keeps the same traversal shape:
            // ordered function ids, lookup hit/miss, and summary entries that
            // expose schedule/profile/pass-group payload from the looked-up
            // plan.
            let Some(plan) = plans.get(name) else {
                continue;
            };
            let groups = plan
                .pass_groups
                .iter()
                .map(|group| group.label())
                .collect::<Vec<_>>()
                .join(",");
            out.push(format!(
                "{} schedule={} profile={} groups={}",
                name,
                plan.schedule.label(),
                plan.profile.label(),
                groups
            ));
        }
        out
    }

    fn verify_or_panic(fn_ir: &FnIR, stage: &str) {
        if let Err(e) = crate::mir::verify::verify_ir(fn_ir) {
            Self::dump_verify_failure(fn_ir, stage, &e.to_string());
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
        Self::clear_stale_phi_owner_metadata(fn_ir);
        match crate::mir::verify::verify_ir(fn_ir) {
            Ok(()) => true,
            Err(e) => {
                Self::dump_verify_failure(fn_ir, stage, &e.to_string());
                let reason = format!("invalid MIR at {}: {}", stage, e);
                fn_ir.mark_unsupported_dynamic(reason);
                false
            }
        }
    }

    fn clear_stale_phi_owner_metadata(fn_ir: &mut FnIR) {
        for value in &mut fn_ir.values {
            if !matches!(value.kind, ValueKind::Phi { .. }) {
                value.phi_block = None;
            }
        }
    }

    fn debug_stage_dump(fn_ir: &FnIR, stage: &str) {
        let Some(names) = std::env::var_os("RR_DEBUG_STAGE_FN") else {
            return;
        };
        let names = names.to_string_lossy().into_owned();
        let wanted: std::collections::HashSet<&str> = names
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if !wanted.contains(fn_ir.name.as_str()) {
            return;
        }
        if let Some(filter) = std::env::var_os("RR_DEBUG_STAGE_MATCH") {
            let filter = filter.to_string_lossy();
            if !stage.contains(filter.as_ref()) {
                return;
            }
        }
        eprintln!(
            "=== RR_DEBUG_STAGE {} :: {} ===\n{:#?}",
            fn_ir.name, stage, fn_ir
        );
    }

    fn is_floor_like_single_positional_call(
        callee: &str,
        args: &[ValueId],
        names: &[Option<String>],
        floor_helpers: &FxHashSet<String>,
    ) -> bool {
        if !matches!(callee, "floor" | "ceiling" | "trunc") && !floor_helpers.contains(callee) {
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

    fn collect_floor_index_param_slots(
        fn_ir: &FnIR,
        floor_helpers: &FxHashSet<String>,
    ) -> FxHashSet<usize> {
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
            if !Self::is_floor_like_single_positional_call(callee, args, names, floor_helpers) {
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

    fn collect_floor_index_base_vars(
        fn_ir: &FnIR,
        floor_helpers: &FxHashSet<String>,
    ) -> FxHashSet<String> {
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
            if !Self::is_floor_like_single_positional_call(callee, args, names, floor_helpers) {
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
        floor_helpers: &FxHashSet<String>,
    ) -> FxHashMap<String, FxHashSet<usize>> {
        let mut floor_slots_by_fn: FxHashMap<String, FxHashSet<usize>> = FxHashMap::default();
        let ordered_names = Self::sorted_fn_names(all_fns);
        for name in &ordered_names {
            let Some(fn_ir) = all_fns.get(name) else {
                continue;
            };
            let slots = Self::collect_floor_index_param_slots(fn_ir, floor_helpers);
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

    fn mark_floor_index_param_metadata(
        fn_ir: &mut FnIR,
        slots: &FxHashSet<usize>,
        floor_helpers: &FxHashSet<String>,
    ) -> bool {
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
                } if Self::is_floor_like_single_positional_call(
                    &callee,
                    &args,
                    &names,
                    floor_helpers,
                ) =>
                {
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
                ValueKind::Call {
                    callee,
                    args,
                    names,
                } if matches!(
                    callee.as_str(),
                    "rr_index1_read_vec" | "rr_index1_read_vec_floor" | "rr_gather"
                ) && (args.len() == 2 || args.len() == 3)
                    && names.iter().take(2).all(std::option::Option::is_none) =>
                {
                    let Some(slot) = Self::param_slot_for_value(fn_ir, args[0]) else {
                        continue;
                    };
                    if !slots.contains(&slot) {
                        continue;
                    }
                    let int_vec = TypeState::vector(PrimTy::Int, false);
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
                _ => {}
            }
        }
        changed
    }

    fn canonicalize_floor_index_params(
        fn_ir: &mut FnIR,
        proven_param_slots: Option<&FxHashSet<usize>>,
        floor_helpers: &FxHashSet<String>,
    ) -> bool {
        let slots = Self::collect_floor_index_param_slots(fn_ir, floor_helpers);
        let base_vars = Self::collect_floor_index_base_vars(fn_ir, floor_helpers);
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

        changed | Self::mark_floor_index_param_metadata(fn_ir, &slots, floor_helpers)
    }

    // Required lowering-to-codegen stabilization passes.
    // This must run even in O0, because codegen cannot emit Phi.
    fn stabilize_for_codegen_inner(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        verify_start: bool,
    ) {
        let floor_helpers = Self::collect_floor_helpers(all_fns);
        if !floor_helpers.is_empty() {
            let _ = Self::rewrite_floor_helper_calls(all_fns, &floor_helpers);
        }
        let abs_helpers = Self::collect_trivial_abs_helpers(all_fns);
        if !abs_helpers.is_empty() {
            let _ = Self::rewrite_trivial_abs_helper_calls(all_fns, &abs_helpers);
        }
        let unit_index_helpers = Self::collect_unit_index_helpers(all_fns);
        if !unit_index_helpers.is_empty() {
            let _ = Self::rewrite_unit_index_helper_calls(all_fns, &unit_index_helpers);
        }
        let minmax_helpers = Self::collect_trivial_minmax_helpers(all_fns);
        if !minmax_helpers.is_empty() {
            let _ = Self::rewrite_trivial_minmax_helper_calls(all_fns, &minmax_helpers);
        }
        let clamp_helpers = Self::collect_trivial_clamp_helpers(all_fns);
        if !clamp_helpers.is_empty() {
            let _ = Self::rewrite_trivial_clamp_helper_calls(all_fns, &clamp_helpers);
        }
        let wrap_index_helpers = Self::collect_wrap_index_helpers(all_fns);
        if !wrap_index_helpers.is_empty() {
            let _ = Self::rewrite_wrap_index_helper_calls(all_fns, &wrap_index_helpers);
        }
        let periodic_index_helpers = Self::collect_periodic_index_helpers(all_fns);
        if !periodic_index_helpers.is_empty() {
            let _ = Self::rewrite_periodic_index_helper_calls(all_fns, &periodic_index_helpers);
        }
        let cube_index_helpers = Self::collect_cube_index_helpers(all_fns);
        if !cube_index_helpers.is_empty() {
            let _ = Self::rewrite_cube_index_helper_calls(all_fns, &cube_index_helpers);
        }
        let ordered_names = Self::sorted_fn_names(all_fns);
        for name in &ordered_names {
            let Some(fn_ir) = all_fns.get_mut(name) else {
                continue;
            };
            if verify_start && !Self::verify_or_reject(fn_ir, "PrepareForCodegen/Start") {
                continue;
            }
            let _ = de_ssa::run(fn_ir);
            // Keep this lightweight but convergent to avoid dead temp noise after De-SSA.
            // Conservative interop functions skip cleanup to preserve package/runtime semantics.
            if !fn_ir.requires_conservative_optimization() {
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

    pub fn stabilize_for_codegen(&self, all_fns: &mut FxHashMap<String, FnIR>) {
        self.stabilize_for_codegen_inner(all_fns, true);
    }

    pub fn stabilize_for_codegen_relaxed_start(&self, all_fns: &mut FxHashMap<String, FnIR>) {
        self.stabilize_for_codegen_inner(all_fns, false);
    }

    fn run_always_tier_with_stats(
        &self,
        fn_ir: &mut FnIR,
        proven_param_slots: Option<&FxHashSet<usize>>,
        floor_helpers: &FxHashSet<String>,
    ) -> TachyonPulseStats {
        self.run_always_tier_with_profile(fn_ir, proven_param_slots, floor_helpers)
            .pulse_stats
    }

    fn run_always_tier_with_profile(
        &self,
        fn_ir: &mut FnIR,
        proven_param_slots: Option<&FxHashSet<usize>>,
        floor_helpers: &FxHashSet<String>,
    ) -> TachyonRunProfile {
        let mut profile = TachyonRunProfile::default();
        let stats = &mut profile.pulse_stats;
        let pass_timings = &mut profile.pass_timings;
        if fn_ir.requires_conservative_optimization() {
            return profile;
        }
        if !Self::verify_or_reject(fn_ir, "AlwaysTier/Start") {
            return profile;
        }
        let canonicalized_index_params =
            Self::canonicalize_floor_index_params(fn_ir, proven_param_slots, floor_helpers);
        if canonicalized_index_params {
            Self::maybe_verify(fn_ir, "After AlwaysTier/ParamIndexCanonicalize");
        }

        stats.always_tier_functions = 1;
        let mut changed = true;
        let mut iter = 0usize;
        let max_iters = self.configured_always_tier_max_iters();
        let mut seen = FxHashSet::default();
        seen.insert(Self::fn_ir_fingerprint(fn_ir));
        let fn_ir_size = Self::fn_ir_size(fn_ir);
        let run_light_sccp = fn_ir_size <= self.configured_heavy_pass_fn_ir().saturating_mul(2);
        let loop_opt = loop_opt::MirLoopOptimizer::new();

        while changed && iter < max_iters {
            iter += 1;
            changed = false;
            let before_hash = Self::fn_ir_fingerprint(fn_ir);

            // Proof correspondence:
            // `DataflowOptSoundness` approximates the local expression/value
            // rewrite slice in this always-tier loop (`sccp`, `gvn`, `dce`,
            // plus canonicalization-style simplifications).
            // `CfgOptSoundness` approximates `simplify_cfg` / reduced entry
            // retarget / dead-block cleanup style rewrites.
            // `LoopOptSoundness` approximates the loop-focused slice
            // (`tco`, `loop_opt`, bounded `bce`) under a reduced MIR model.
            let sc_changed =
                Self::timed_bool_pass(pass_timings, "simplify_cfg", || self.simplify_cfg(fn_ir));
            if sc_changed {
                stats.simplify_hits += 1;
                changed = true;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/SimplifyCFG");

            if run_light_sccp {
                let sccp_changed = Self::timed_bool_pass(pass_timings, "sccp", || {
                    sccp::MirSCCP::new().optimize(fn_ir)
                });
                if sccp_changed {
                    stats.sccp_hits += 1;
                    changed = true;
                }
                Self::maybe_verify(fn_ir, "After AlwaysTier/SCCP");

                let intr_changed = Self::timed_bool_pass(pass_timings, "intrinsics", || {
                    intrinsics::optimize(fn_ir)
                });
                if intr_changed {
                    stats.intrinsics_hits += 1;
                    changed = true;
                }
                Self::maybe_verify(fn_ir, "After AlwaysTier/Intrinsics");
            }

            let type_spec_changed = Self::timed_bool_pass(pass_timings, "type_specialize", || {
                type_specialize::optimize(fn_ir)
            });
            if type_spec_changed {
                changed = true;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/TypeSpecialize");

            let tco_changed = Self::timed_bool_pass(pass_timings, "tco", || tco::optimize(fn_ir));
            if tco_changed {
                stats.tco_hits += 1;
                changed = true;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/TCO");

            let loop_changed_count = Self::timed_count_pass(pass_timings, "loop_opt", || {
                loop_opt.optimize_with_count(fn_ir)
            });
            if loop_changed_count > 0 {
                stats.simplified_loops += loop_changed_count;
                changed = true;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/LoopOpt");

            let dce_changed = Self::timed_bool_pass(pass_timings, "dce", || self.dce(fn_ir));
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
        if fn_ir_size <= self.configured_always_bce_fn_ir() {
            let bce_changed = Self::timed_bool_pass(pass_timings, "bce", || bce::optimize(fn_ir));
            if bce_changed {
                stats.bce_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/BCE");
        }

        let _ = Self::verify_or_reject(fn_ir, "AlwaysTier/End");
        profile
    }

    pub fn run_program(&self, all_fns: &mut FxHashMap<String, FnIR>) {
        let _ = self.run_program_with_stats(all_fns);
    }

    pub fn run_program_with_stats(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
    ) -> TachyonPulseStats {
        let scheduler = CompilerScheduler::new(CompilerParallelConfig::default());
        self.run_program_with_scheduler(all_fns, &scheduler)
    }

    pub fn run_program_with_stats_progress(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        on_progress: &mut dyn FnMut(TachyonProgress),
    ) -> TachyonPulseStats {
        let scheduler = CompilerScheduler::new(CompilerParallelConfig::default());
        self.run_program_with_progress_and_scheduler(all_fns, &scheduler, on_progress)
    }

    pub fn run_program_with_stats_and_compiler_parallel(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        compiler_parallel_cfg: CompilerParallelConfig,
    ) -> TachyonPulseStats {
        let scheduler = CompilerScheduler::new(compiler_parallel_cfg);
        self.run_program_with_scheduler(all_fns, &scheduler)
    }

    pub fn run_program_with_stats_progress_and_compiler_parallel(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        compiler_parallel_cfg: CompilerParallelConfig,
        on_progress: &mut dyn FnMut(TachyonProgress),
    ) -> TachyonPulseStats {
        let scheduler = CompilerScheduler::new(compiler_parallel_cfg);
        self.run_program_with_progress_and_scheduler(all_fns, &scheduler, on_progress)
    }

    pub(crate) fn run_program_with_scheduler(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        scheduler: &CompilerScheduler,
    ) -> TachyonPulseStats {
        self.run_program_with_profile_and_scheduler(all_fns, scheduler)
            .pulse_stats
    }

    pub(crate) fn run_program_with_progress_and_scheduler(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        scheduler: &CompilerScheduler,
        on_progress: &mut dyn FnMut(TachyonProgress),
    ) -> TachyonPulseStats {
        self.run_program_with_profile_and_progress_scheduler(all_fns, scheduler, on_progress)
            .pulse_stats
    }

    pub(crate) fn run_program_with_profile_and_scheduler(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        scheduler: &CompilerScheduler,
    ) -> TachyonRunProfile {
        // Proof correspondence:
        // `ProgramApiWrapperSoundness.run_program_with_profile_and_scheduler_*`
        // fixes the reduced shell theorem family for this public optimizer
        // entrypoint. The reduced model treats this as orchestration around
        // the already-composed `run_program_with_profile_inner` boundary.
        self.run_program_with_profile_inner(all_fns, scheduler, None)
    }

    pub(crate) fn run_program_with_profile_and_progress_scheduler(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        scheduler: &CompilerScheduler,
        on_progress: &mut dyn FnMut(TachyonProgress),
    ) -> TachyonRunProfile {
        self.run_program_with_profile_inner(all_fns, scheduler, Some(on_progress))
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

    fn timed_bool_pass<F>(pass_timings: &mut TachyonPassTimings, pass: &'static str, f: F) -> bool
    where
        F: FnOnce() -> bool,
    {
        let started = Instant::now();
        let changed = f();
        pass_timings.record(pass, started.elapsed().as_nanos(), changed);
        changed
    }

    fn timed_count_pass<F>(pass_timings: &mut TachyonPassTimings, pass: &'static str, f: F) -> usize
    where
        F: FnOnce() -> usize,
    {
        let started = Instant::now();
        let changed_count = f();
        pass_timings.record(pass, started.elapsed().as_nanos(), changed_count > 0);
        changed_count
    }

    fn take_functions_in_order(
        all_fns: &mut FxHashMap<String, FnIR>,
        ordered_names: &[String],
    ) -> Vec<(String, FnIR)> {
        let mut jobs = Vec::with_capacity(ordered_names.len());
        for name in ordered_names {
            if let Some(fn_ir) = all_fns.remove(name) {
                jobs.push((name.clone(), fn_ir));
            }
        }
        jobs
    }

    fn restore_functions(all_fns: &mut FxHashMap<String, FnIR>, jobs: Vec<(String, FnIR)>) {
        for (name, fn_ir) in jobs {
            all_fns.insert(name, fn_ir);
        }
    }

    fn fn_is_self_recursive(fn_ir: &FnIR) -> bool {
        fn_ir.values.iter().any(|value| {
            matches!(
                &value.kind,
                ValueKind::Call { callee, .. } if callee == &fn_ir.name
            )
        })
    }

    fn run_program_with_profile_inner(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        scheduler: &CompilerScheduler,
        mut progress: Option<&mut dyn FnMut(TachyonProgress)>,
    ) -> TachyonRunProfile {
        // Proof correspondence:
        // `ProgramRunProfileInnerSoundness` fixes the reduced wrapper theorem
        // family for this whole function. The reduced model composes:
        // always-tier execution, heavy-tier plan flow, per-function heavy-tier
        // execution, plan-summary emission, and the post-tier cleanup/de-ssa
        // tail into one `run_program_with_profile_inner`-shaped boundary.
        let mut profile = TachyonRunProfile::default();
        let stats = &mut profile.pulse_stats;
        let pass_timings = &mut profile.pass_timings;
        profile.active_pass_groups = self.active_pass_group_labels();
        let plan = Self::build_opt_plan(all_fns);
        let selective_enabled = Self::selective_budget_enabled();
        let run_heavy_tier = !plan.selective_mode || selective_enabled;
        let run_full_inline_tier = run_heavy_tier
            && self.inline_tier_enabled()
            && !plan.selective_mode
            && plan.total_ir <= Self::max_full_opt_ir();
        stats.total_program_ir = plan.total_ir;
        stats.max_function_ir = plan.max_fn_ir;
        stats.full_opt_ir_limit = plan.program_limit;
        stats.full_opt_fn_limit = plan.fn_limit;
        stats.selective_budget_mode = plan.selective_mode && selective_enabled;
        let ordered_names = Self::sorted_fn_names(all_fns);
        let ordered_total = ordered_names.len();
        let floor_helpers = Self::collect_floor_helpers(all_fns);
        let proven_floor_param_slots =
            Self::collect_proven_floor_index_param_slots(all_fns, &floor_helpers);
        if !floor_helpers.is_empty() {
            let rewrites = Self::rewrite_floor_helper_calls(all_fns, &floor_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                let helper_names = Self::sorted_names(&floor_helpers).join(", ");
                eprintln!(
                    "   [floor] rewrote {} call site(s) using helper(s): {}",
                    rewrites, helper_names
                );
            }
        }
        let abs_helpers = Self::collect_trivial_abs_helpers(all_fns);
        if !abs_helpers.is_empty() {
            let rewrites = Self::rewrite_trivial_abs_helper_calls(all_fns, &abs_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                let helper_names = Self::sorted_names(&abs_helpers).join(", ");
                eprintln!(
                    "   [abs] rewrote {} call site(s) using helper(s): {}",
                    rewrites, helper_names
                );
            }
        }
        let unit_index_helpers = Self::collect_unit_index_helpers(all_fns);
        if !unit_index_helpers.is_empty() {
            let rewrites = Self::rewrite_unit_index_helper_calls(all_fns, &unit_index_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                let helper_names = Self::sorted_names(&unit_index_helpers).join(", ");
                eprintln!(
                    "   [unit-index] rewrote {} call site(s) using helper(s): {}",
                    rewrites, helper_names
                );
            }
        }
        let minmax_helpers = Self::collect_trivial_minmax_helpers(all_fns);
        if !minmax_helpers.is_empty() {
            let rewrites = Self::rewrite_trivial_minmax_helper_calls(all_fns, &minmax_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                let helper_names: FxHashSet<String> = minmax_helpers.keys().cloned().collect();
                let helper_names = Self::sorted_names(&helper_names).join(", ");
                eprintln!(
                    "   [minmax] rewrote {} call site(s) using helper(s): {}",
                    rewrites, helper_names
                );
            }
        }
        let clamp_helpers = Self::collect_trivial_clamp_helpers(all_fns);
        if !clamp_helpers.is_empty() {
            let rewrites = Self::rewrite_trivial_clamp_helper_calls(all_fns, &clamp_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                let helper_names = Self::sorted_names(&clamp_helpers).join(", ");
                eprintln!(
                    "   [clamp] rewrote {} call site(s) using helper(s): {}",
                    rewrites, helper_names
                );
            }
        }

        // Tier A (always): lightweight canonicalization for every safe function.
        let tier_a_total_ir: usize = ordered_names
            .iter()
            .filter_map(|name| all_fns.get(name))
            .map(Self::fn_ir_size)
            .sum();
        let tier_a_jobs = Self::take_functions_in_order(all_fns, &ordered_names);
        let tier_a_results = scheduler.map_stage(
            CompilerParallelStage::TachyonAlways,
            tier_a_jobs,
            tier_a_total_ir,
            |(name, mut fn_ir)| {
                let local_profile = self.run_always_tier_with_profile(
                    &mut fn_ir,
                    proven_floor_param_slots.get(&name),
                    &floor_helpers,
                );
                (name, fn_ir, local_profile)
            },
        );
        let mut restored_tier_a = Vec::with_capacity(tier_a_results.len());
        for (idx, (name, fn_ir, local_profile)) in tier_a_results.into_iter().enumerate() {
            stats.accumulate(local_profile.pulse_stats);
            pass_timings.accumulate(local_profile.pass_timings);
            Self::emit_progress(
                &mut progress,
                TachyonProgressTier::Always,
                idx + 1,
                ordered_total,
                &name,
            );
            restored_tier_a.push((name, fn_ir));
        }
        Self::restore_functions(all_fns, restored_tier_a);
        Self::debug_wrap_candidates(all_fns);

        let heavy_phase_plans = if run_heavy_tier {
            let selected_functions = if plan.selective_mode {
                Some(&plan.selected_functions)
            } else {
                None
            };
            // Proof correspondence:
            // `ProgramPhasePipelineSoundness` fixes the reduced program-level
            // composition boundary here:
            // `ProgramOptPlan -> selected_functions ->
            // collect_function_phase_plans -> plan_summary`.
            // The reduced model keeps the same heavy-tier disabled/empty case,
            // selected-function gating, lookup reuse, and summary emission
            // boundaries over the collected plan set.
            self.collect_function_phase_plans(all_fns, &ordered_names, selected_functions)
        } else {
            FxHashMap::default()
        };
        profile.plan_summary = self.plan_summary_lines(&ordered_names, &heavy_phase_plans);

        let wrap_index_helpers = if run_heavy_tier {
            Self::collect_wrap_index_helpers(all_fns)
        } else {
            FxHashSet::default()
        };
        if !wrap_index_helpers.is_empty() {
            let rewrites = Self::rewrite_wrap_index_helper_calls(all_fns, &wrap_index_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                let helper_names = Self::sorted_names(&wrap_index_helpers).join(", ");
                eprintln!(
                    "   [wrap] rewrote {} call site(s) using helper(s): {}",
                    rewrites, helper_names
                );
            }
        }
        let periodic_index_helpers = if run_heavy_tier {
            Self::collect_periodic_index_helpers(all_fns)
        } else {
            FxHashMap::default()
        };
        if !periodic_index_helpers.is_empty() {
            let rewrites =
                Self::rewrite_periodic_index_helper_calls(all_fns, &periodic_index_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                let helper_names: FxHashSet<String> =
                    periodic_index_helpers.keys().cloned().collect();
                let helper_names = Self::sorted_names(&helper_names).join(", ");
                eprintln!(
                    "   [wrap1d] rewrote {} call site(s) using helper(s): {}",
                    rewrites, helper_names
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
                let helper_names = Self::sorted_names(&cube_index_helpers).join(", ");
                eprintln!(
                    "   [cube] rewrote {} call site(s) using helper(s): {}",
                    rewrites, helper_names
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
        let tier_b_total_ir: usize = ordered_names
            .iter()
            .filter_map(|name| all_fns.get(name))
            .map(Self::fn_ir_size)
            .sum();
        let tier_b_jobs = Self::take_functions_in_order(all_fns, &ordered_names);
        let tier_b_results = scheduler.map_stage(
            CompilerParallelStage::TachyonHeavy,
            tier_b_jobs,
            tier_b_total_ir,
            |(name, mut fn_ir)| {
                // Proof correspondence:
                // `ProgramTierExecutionSoundness` fixes the reduced per-
                // function execution boundary for this closure. The reduced
                // model keeps the same branch split:
                // conservative skip, self-recursive skip, heavy-tier-disabled
                // skip, budget skip, collected-plan hit, and legacy-plan
                // fallback.
                let mut local_profile = TachyonRunProfile::default();
                if fn_ir.requires_conservative_optimization() {
                    local_profile.pulse_stats.skipped_functions += 1;
                    let _ = Self::verify_or_reject(&mut fn_ir, "SkipOpt/ConservativeInterop");
                    return (name, fn_ir, local_profile);
                }
                if Self::fn_is_self_recursive(&fn_ir) {
                    local_profile.pulse_stats.skipped_functions += 1;
                    let _ = Self::verify_or_reject(&mut fn_ir, "SkipOpt/SelfRecursive");
                    return (name, fn_ir, local_profile);
                }
                let selected = !plan.selective_mode || plan.selected_functions.contains(&name);
                if !run_heavy_tier || !selected {
                    local_profile.pulse_stats.skipped_functions += 1;
                    let reason = if !run_heavy_tier {
                        "SkipOpt/HeavyTierDisabled"
                    } else {
                        "SkipOpt/Budget"
                    };
                    let _ = Self::verify_or_reject(&mut fn_ir, reason);
                    return (name, fn_ir, local_profile);
                }
                local_profile.pulse_stats.optimized_functions += 1;
                let phase_plan = heavy_phase_plans
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| self.build_legacy_function_phase_plan(&name));
                let phase_profile = self.run_function_with_phase_plan_with_proven_profile(
                    &mut fn_ir,
                    &callmap_user_whitelist,
                    proven_floor_param_slots.get(&name),
                    &phase_plan,
                );
                local_profile
                    .pulse_stats
                    .accumulate(phase_profile.pulse_stats);
                local_profile
                    .pass_timings
                    .accumulate(phase_profile.pass_timings);
                (name, fn_ir, local_profile)
            },
        );
        let mut restored_tier_b = Vec::with_capacity(tier_b_results.len());
        for (idx, (name, fn_ir, local_profile)) in tier_b_results.into_iter().enumerate() {
            stats.accumulate(local_profile.pulse_stats);
            pass_timings.accumulate(local_profile.pass_timings);
            Self::emit_progress(
                &mut progress,
                TachyonProgressTier::Heavy,
                idx + 1,
                ordered_total,
                &name,
            );
            restored_tier_b.push((name, fn_ir));
        }
        Self::restore_functions(all_fns, restored_tier_b);

        // Tier C (full-program): bounded inter-procedural inlining.
        // Proof correspondence:
        // `ProgramPostTierStagesSoundness.inline_cleanup_stage_*` fixes the
        // reduced stage boundary for the inline cleanup slice below
        // (`simplify_cfg -> dce` after an inlining round).
        if run_full_inline_tier {
            let mut changed = true;
            let mut iter = 0;
            let inliner = if self.fast_dev_enabled() {
                inline::MirInliner::new_fast_dev()
            } else {
                inline::MirInliner::new()
            };
            let hot_filter = if plan.selective_mode {
                Some(&plan.selected_functions)
            } else {
                None
            };
            while changed && iter < self.configured_max_inline_rounds() {
                changed = false;
                iter += 1;
                // Inlining needs access to the whole map
                let local_changed = Self::timed_bool_pass(pass_timings, "inline", || {
                    inliner.optimize_with_hot_filter(all_fns, hot_filter)
                });
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
                    let cleanup_total_ir: usize = ordered_names
                        .iter()
                        .filter_map(|name| all_fns.get(name))
                        .map(Self::fn_ir_size)
                        .sum();
                    let cleanup_jobs = Self::take_functions_in_order(all_fns, &ordered_names);
                    let cleanup_results = scheduler.map_stage(
                        CompilerParallelStage::TachyonInlineCleanup,
                        cleanup_jobs,
                        cleanup_total_ir,
                        |(name, mut fn_ir)| {
                            let mut local_profile = TachyonRunProfile::default();
                            if fn_ir.requires_conservative_optimization() {
                                Self::maybe_verify(
                                    &fn_ir,
                                    "After Inline Cleanup (Skipped: ConservativeInterop)",
                                );
                                return (name, fn_ir, local_profile);
                            }
                            let inline_sc_changed = Self::timed_bool_pass(
                                &mut local_profile.pass_timings,
                                "simplify_cfg",
                                || self.simplify_cfg(&mut fn_ir),
                            );
                            let inline_dce_changed = Self::timed_bool_pass(
                                &mut local_profile.pass_timings,
                                "dce",
                                || self.dce(&mut fn_ir),
                            );
                            if inline_sc_changed || inline_dce_changed {
                                local_profile.pulse_stats.inline_cleanup_hits += 1;
                            }
                            if inline_sc_changed {
                                local_profile.pulse_stats.simplify_hits += 1;
                            }
                            if inline_dce_changed {
                                local_profile.pulse_stats.dce_hits += 1;
                            }
                            Self::maybe_verify(&fn_ir, "After Inline Cleanup");
                            (name, fn_ir, local_profile)
                        },
                    );
                    let mut restored_cleanup = Vec::with_capacity(cleanup_results.len());
                    for (name, fn_ir, local_profile) in cleanup_results {
                        stats.accumulate(local_profile.pulse_stats);
                        pass_timings.accumulate(local_profile.pass_timings);
                        restored_cleanup.push((name, fn_ir));
                    }
                    Self::restore_functions(all_fns, restored_cleanup);
                }
            }
        }

        let fresh_user_calls =
            fresh_alias::collect_fresh_returning_user_functions_for_parallel(all_fns);
        // Proof correspondence:
        // `ProgramPostTierStagesSoundness.fresh_alias_stage_*` fixes the
        // reduced stage boundary for the fresh-alias cleanup pass applied
        // across the restored program map here.
        let fresh_alias_names = Self::sorted_fn_names(all_fns);
        let fresh_alias_total_ir: usize = fresh_alias_names
            .iter()
            .filter_map(|name| all_fns.get(name))
            .map(Self::fn_ir_size)
            .sum();
        let fresh_alias_jobs = Self::take_functions_in_order(all_fns, &fresh_alias_names);
        let fresh_alias_results = scheduler.map_stage(
            CompilerParallelStage::TachyonFreshAlias,
            fresh_alias_jobs,
            fresh_alias_total_ir,
            |(name, mut fn_ir)| {
                let mut local_timings = TachyonPassTimings::default();
                let _changed = Self::timed_bool_pass(&mut local_timings, "fresh_alias", || {
                    fresh_alias::optimize_function_with_fresh_user_calls(
                        &mut fn_ir,
                        &fresh_user_calls,
                    )
                });
                (name, fn_ir, local_timings)
            },
        );
        let mut restored_fresh_alias = Vec::with_capacity(fresh_alias_results.len());
        for (name, fn_ir, local_timings) in fresh_alias_results {
            pass_timings.accumulate(local_timings);
            restored_fresh_alias.push((name, fn_ir));
        }
        Self::restore_functions(all_fns, restored_fresh_alias);

        // 3. De-SSA (Phi elimination via parallel copy) before codegen.
        // Proof correspondence:
        // `DeSsaBoundarySoundness` models the reduced redundant-copy
        // elimination boundary here, while `OptimizerPipelineSoundness`
        // exposes the staged theorem family that crosses this point:
        // `program_post_dessa_*` for the `de_ssa` + cleanup boundary and
        // `prepare_for_codegen_*` for the full pre-emission normalization
        // slice. `DeSsaSubset` remains the lower-level reduced theorem for the
        // canonical copy-boundary matcher itself.
        // `ProgramPostTierStagesSoundness.de_ssa_program_stage_*` then lifts
        // this same reduced boundary into the post-heavy, program-level tail
        // stage family used by `run_program_with_profile_inner`.
        let ordered_names = Self::sorted_fn_names(all_fns);
        let de_ssa_total = ordered_names.len();
        let de_ssa_total_ir: usize = ordered_names
            .iter()
            .filter_map(|name| all_fns.get(name))
            .map(Self::fn_ir_size)
            .sum();
        let de_ssa_jobs = Self::take_functions_in_order(all_fns, &ordered_names);
        let de_ssa_results = scheduler.map_stage(
            CompilerParallelStage::TachyonDeSsa,
            de_ssa_jobs,
            de_ssa_total_ir,
            |(name, mut fn_ir)| {
                let mut local_profile = TachyonRunProfile::default();
                let de_ssa_changed =
                    Self::timed_bool_pass(&mut local_profile.pass_timings, "de_ssa", || {
                        de_ssa::run(&mut fn_ir)
                    });
                if de_ssa_changed {
                    local_profile.pulse_stats.de_ssa_hits += 1;
                }
                let copy_cleanup_changed = if fn_ir.requires_conservative_optimization() {
                    false
                } else {
                    Self::timed_bool_pass(&mut local_profile.pass_timings, "copy_cleanup", || {
                        copy_cleanup::optimize(&mut fn_ir)
                    })
                };
                if copy_cleanup_changed {
                    local_profile.pulse_stats.simplify_hits += 1;
                }
                if !fn_ir.requires_conservative_optimization() {
                    let sc_changed = Self::timed_bool_pass(
                        &mut local_profile.pass_timings,
                        "simplify_cfg",
                        || self.simplify_cfg(&mut fn_ir),
                    );
                    let dce_changed =
                        Self::timed_bool_pass(&mut local_profile.pass_timings, "dce", || {
                            self.dce(&mut fn_ir)
                        });
                    if sc_changed {
                        local_profile.pulse_stats.simplify_hits += 1;
                    }
                    if dce_changed {
                        local_profile.pulse_stats.dce_hits += 1;
                    }
                }
                let _ = Self::verify_or_reject(&mut fn_ir, "After De-SSA");
                (name, fn_ir, local_profile)
            },
        );
        let mut restored_de_ssa = Vec::with_capacity(de_ssa_results.len());
        for (idx, (name, fn_ir, local_profile)) in de_ssa_results.into_iter().enumerate() {
            stats.accumulate(local_profile.pulse_stats);
            pass_timings.accumulate(local_profile.pass_timings);
            Self::emit_progress(
                &mut progress,
                TachyonProgressTier::DeSsa,
                idx + 1,
                de_ssa_total,
                &name,
            );
            restored_de_ssa.push((name, fn_ir));
        }
        Self::restore_functions(all_fns, restored_de_ssa);
        profile
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
        let floor_helpers = FxHashSet::default();
        self.run_function_with_proven_index_slots(
            fn_ir,
            callmap_user_whitelist,
            None,
            &floor_helpers,
        )
    }

    fn run_function_with_stats_with_proven(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        proven_param_slots: Option<&FxHashSet<usize>>,
    ) -> TachyonPulseStats {
        let phase_plan = self.build_legacy_function_phase_plan(&fn_ir.name);
        self.run_function_with_phase_plan_with_proven(
            fn_ir,
            callmap_user_whitelist,
            proven_param_slots,
            &phase_plan,
        )
    }

    fn run_function_with_phase_plan_with_proven(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        proven_param_slots: Option<&FxHashSet<usize>>,
        phase_plan: &FunctionPhasePlan,
    ) -> TachyonPulseStats {
        self.run_function_with_phase_plan_with_proven_profile(
            fn_ir,
            callmap_user_whitelist,
            proven_param_slots,
            phase_plan,
        )
        .pulse_stats
    }

    fn run_function_with_phase_plan_with_proven_profile(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        proven_param_slots: Option<&FxHashSet<usize>>,
        phase_plan: &FunctionPhasePlan,
    ) -> TachyonRunProfile {
        let floor_helpers = FxHashSet::default();
        self.run_function_with_proven_index_slots_with_phase_plan(
            fn_ir,
            callmap_user_whitelist,
            proven_param_slots,
            &floor_helpers,
            phase_plan,
        )
    }

    fn run_function_with_proven_index_slots(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        proven_param_slots: Option<&FxHashSet<usize>>,
        floor_helpers: &FxHashSet<String>,
    ) -> TachyonPulseStats {
        let phase_plan = self.build_legacy_function_phase_plan(&fn_ir.name);
        self.run_function_with_proven_index_slots_with_phase_plan(
            fn_ir,
            callmap_user_whitelist,
            proven_param_slots,
            floor_helpers,
            &phase_plan,
        )
        .pulse_stats
    }

    fn run_function_with_proven_index_slots_with_phase_plan(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        proven_param_slots: Option<&FxHashSet<usize>>,
        floor_helpers: &FxHashSet<String>,
        phase_plan: &FunctionPhasePlan,
    ) -> TachyonRunProfile {
        let mut profile = TachyonRunProfile::default();
        let stats = &mut profile.pulse_stats;
        let pass_timings = &mut profile.pass_timings;
        match phase_plan.profile {
            types::PhaseProfileKind::Balanced => stats.phase_profile_balanced_functions += 1,
            types::PhaseProfileKind::ComputeHeavy => {
                stats.phase_profile_compute_heavy_functions += 1
            }
            types::PhaseProfileKind::ControlFlowHeavy => {
                stats.phase_profile_control_flow_heavy_functions += 1
            }
        }
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
            self.configured_max_opt_iterations()
        };
        let heavy_pass_budgeted = fn_ir_size > self.configured_heavy_pass_fn_ir();

        // Initial Verify
        if !Self::verify_or_reject(fn_ir, "Start") {
            return profile;
        }
        Self::debug_stage_dump(fn_ir, "Start");
        let canonicalized_index_params =
            Self::canonicalize_floor_index_params(fn_ir, proven_param_slots, floor_helpers);
        if canonicalized_index_params {
            Self::maybe_verify(fn_ir, "After ParamIndexCanonicalize");
            Self::debug_stage_dump(fn_ir, "After ParamIndexCanonicalize");
        }
        seen_hashes.insert(Self::fn_ir_fingerprint(fn_ir));
        let mut current_schedule = phase_plan.schedule;
        let mut fallback_used = false;
        let mut control_flow_structural_skipped = false;

        while changed && iterations < max_iters {
            if start_time.elapsed().as_millis() > self.configured_max_fn_opt_ms() {
                break;
            }
            changed = false;
            iterations += 1;
            let before_hash = Self::fn_ir_fingerprint(fn_ir);
            let pre_iteration_fn_ir =
                if matches!(current_schedule, PhaseScheduleId::ControlFlowHeavy)
                    && iterations == 1
                    && !fallback_used
                {
                    Some(fn_ir.clone())
                } else {
                    None
                };
            let pre_iteration_stats = if pre_iteration_fn_ir.is_some() {
                Some(*stats)
            } else {
                None
            };

            let run_budgeted_passes = !(heavy_pass_budgeted && iterations > 1);
            let iteration_result = self.run_heavy_phase_schedule_iteration(
                current_schedule,
                fn_ir,
                callmap_user_whitelist,
                &loop_opt,
                stats,
                pass_timings,
                run_budgeted_passes,
            );
            changed |= iteration_result.changed;
            control_flow_structural_skipped |= iteration_result.skipped_structural;
            // check_elimination remains disabled.

            let after_hash = Self::fn_ir_fingerprint(fn_ir);
            if matches!(current_schedule, PhaseScheduleId::ControlFlowHeavy)
                && iterations == 1
                && !fallback_used
                && Self::control_flow_should_fallback_to_balanced(iteration_result)
            {
                if let Some(saved_fn_ir) = pre_iteration_fn_ir {
                    *fn_ir = saved_fn_ir;
                }
                if let Some(saved_stats) = pre_iteration_stats {
                    *stats = saved_stats;
                }
                if phase_plan.trace_requested {
                    eprintln!(
                        "   [phase-order] {} fallback control-flow-heavy -> balanced non_structural_changes={} structural_progress={} skipped_structural={}",
                        fn_ir.name,
                        iteration_result.non_structural_changes,
                        iteration_result.structural_progress,
                        iteration_result.skipped_structural
                    );
                }
                current_schedule = PhaseScheduleId::Balanced;
                fallback_used = true;
                stats.phase_schedule_fallbacks += 1;
                changed = true;
                continue;
            }
            if after_hash == before_hash {
                break;
            }
            if !seen_hashes.insert(after_hash) {
                // Degenerate oscillation guard.
                break;
            }
            changed |= after_hash != before_hash;
        }
        if control_flow_structural_skipped {
            stats.control_flow_structural_skip_functions += 1;
        }

        // Final polishing pass
        let mut polishing = true;
        let mut polish_guard = 0usize;
        let mut polish_seen: FxHashSet<u64> = FxHashSet::default();
        while polishing && polish_guard < 16 {
            if start_time.elapsed().as_millis() > self.configured_max_fn_opt_ms() {
                break;
            }
            polish_guard += 1;
            let before_polish = Self::fn_ir_fingerprint(fn_ir);
            polishing =
                Self::timed_bool_pass(pass_timings, "simplify_cfg", || self.simplify_cfg(fn_ir));
            if polishing {
                stats.simplify_hits += 1;
            }
            let dce_changed = Self::timed_bool_pass(pass_timings, "dce", || self.dce(fn_ir));
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
        Self::debug_stage_dump(fn_ir, "End");
        profile
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
        callmap::collect_callmap_user_whitelist(all_fns)
    }

    fn is_callmap_vector_safe_user_fn(
        name: &str,
        fn_ir: &FnIR,
        user_whitelist: &FxHashSet<String>,
    ) -> bool {
        callmap::is_callmap_vector_safe_user_fn(name, fn_ir, user_whitelist)
    }

    fn is_vector_safe_user_expr(
        fn_ir: &FnIR,
        vid: ValueId,
        user_whitelist: &FxHashSet<String>,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        callmap::is_vector_safe_user_expr(fn_ir, vid, user_whitelist, seen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::Span;
    use std::fs;

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
            all.insert(name.clone(), dummy_fn(&name, 2_100));
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
        let floor_helpers = FxHashSet::default();
        let stats = TachyonEngine::new().run_always_tier_with_stats(&mut f, None, &floor_helpers);
        assert_eq!(stats.always_tier_functions, 1);
        assert!(crate::mir::verify::verify_ir(&f).is_ok());
    }

    #[test]
    fn copy_cleanup_skips_conservative_functions() {
        let mut fn_ir = FnIR::new("opaque_alias".to_string(), vec!["a".to_string()]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let load_a = fn_ir.add_value(
            ValueKind::Load {
                var: "a".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("a".to_string()),
        );
        let load_tmp = fn_ir.add_value(
            ValueKind::Load {
                var: "tmp".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("tmp".to_string()),
        );

        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "tmp".to_string(),
            src: load_a,
            span: Span::default(),
        });
        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "out".to_string(),
            src: load_tmp,
            span: Span::default(),
        });
        fn_ir.blocks[entry].term = Terminator::Return(None);
        fn_ir.mark_opaque_interop("preserve raw aliasing".to_string());

        let mut all = FxHashMap::default();
        all.insert(fn_ir.name.clone(), fn_ir);
        let _ = TachyonEngine::new().run_program_with_stats(&mut all);

        let fn_ir = all
            .get("opaque_alias")
            .expect("function should remain present");
        let Instr::Assign { src, .. } = &fn_ir.blocks[entry].instrs[1] else {
            panic!("expected alias assignment to remain in place");
        };
        match &fn_ir.values[*src].kind {
            ValueKind::Load { var } => assert_eq!(var, "tmp"),
            other => panic!("expected load(tmp) to be preserved, got {:?}", other),
        }
    }

    #[test]
    fn run_program_applies_tco_before_recursive_functions_are_skipped() {
        let mut fn_ir = FnIR::new("recur".to_string(), vec!["n".to_string()]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let n = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("n".to_string()),
        );
        let recur = fn_ir.add_value(
            ValueKind::Call {
                callee: "recur".to_string(),
                args: vec![n],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(recur));

        let mut all = FxHashMap::default();
        all.insert(fn_ir.name.clone(), fn_ir);
        let stats = TachyonEngine::new().run_program_with_stats(&mut all);

        assert!(
            stats.tco_hits > 0,
            "tail recursion should be rewritten before skip logic"
        );
        let fn_ir = all.get("recur").expect("function should remain present");
        assert_ne!(
            fn_ir.body_head, entry,
            "TCO should split a dedicated body_head when entry would otherwise become cyclic"
        );
        assert!(matches!(
            fn_ir.blocks[entry].term,
            Terminator::Goto(target) if target == fn_ir.body_head
        ));
        assert!(matches!(
            fn_ir.blocks[fn_ir.body_head].term,
            Terminator::Goto(target) if target == fn_ir.body_head
        ));
        assert!(crate::mir::verify::verify_ir(fn_ir).is_ok());
    }

    #[test]
    fn floor_helper_detection_and_rewrite_use_builtin_floor() {
        let mut helper = FnIR::new("floorish".to_string(), vec!["x".to_string()]);
        let helper_entry = helper.add_block();
        helper.entry = helper_entry;
        helper.body_head = helper_entry;
        let param = helper.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let one = helper.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let modulo = helper.add_value(
            ValueKind::Binary {
                op: BinOp::Mod,
                lhs: param,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let floorish = helper.add_value(
            ValueKind::Binary {
                op: BinOp::Sub,
                lhs: param,
                rhs: modulo,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        helper.blocks[helper_entry].term = Terminator::Return(Some(floorish));

        let mut caller = FnIR::new("caller".to_string(), vec!["y".to_string()]);
        let caller_entry = caller.add_block();
        caller.entry = caller_entry;
        caller.body_head = caller_entry;
        let caller_param = caller.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("y".to_string()),
        );
        let call = caller.add_value(
            ValueKind::Call {
                callee: "floorish".to_string(),
                args: vec![caller_param],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[caller_entry].term = Terminator::Return(Some(call));

        let mut all = FxHashMap::default();
        all.insert(helper.name.clone(), helper);
        all.insert(caller.name.clone(), caller);

        let helpers = TachyonEngine::collect_floor_helpers(&all);
        assert!(helpers.contains("floorish"));
        let rewrites = TachyonEngine::rewrite_floor_helper_calls(&mut all, &helpers);
        assert_eq!(rewrites, 1);

        let caller = all.get("caller").expect("caller should exist");
        let ValueKind::Call { callee, .. } = &caller.values[call].kind else {
            panic!("caller value should remain a call");
        };
        assert_eq!(callee, "floor");
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

    #[test]
    fn verify_failure_dump_writes_stage_and_function_snapshot() {
        let fn_ir = dummy_fn("dump_target", 8);
        let out_dir = std::env::temp_dir().join(format!(
            "rr-verify-dump-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        TachyonEngine::dump_verify_failure_to(&out_dir, &fn_ir, "vectorize/post", "bad phi");
        let dump_path = out_dir.join("vectorize_post__dump_target.mir.txt");
        let dump = fs::read_to_string(&dump_path).expect("verify dump should be written");
        assert!(dump.contains("stage: vectorize/post"));
        assert!(dump.contains("function: dump_target"));
        assert!(dump.contains("reason: bad phi"));
        assert!(dump.contains("FnIR"));
        let _ = fs::remove_file(&dump_path);
        let _ = fs::remove_dir(&out_dir);
    }

    #[test]
    fn dce_removes_shadowed_dead_assign() {
        let mut fn_ir = FnIR::new("dead_assign".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let two = fn_ir.add_value(
            ValueKind::Const(Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_x = fn_ir.add_value(
            ValueKind::Load {
                var: ".tachyon_x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some(".tachyon_x".to_string()),
        );
        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: ".tachyon_x".to_string(),
            src: one,
            span: Span::default(),
        });
        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: ".tachyon_x".to_string(),
            src: two,
            span: Span::default(),
        });
        fn_ir.blocks[entry].term = Terminator::Return(Some(load_x));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(changed);
        assert_eq!(fn_ir.blocks[entry].instrs.len(), 1);
        match &fn_ir.blocks[entry].instrs[0] {
            Instr::Assign { dst, src, .. } => {
                assert_eq!(dst, ".tachyon_x");
                assert_eq!(*src, two);
            }
            other => panic!("unexpected instruction after DCE: {:?}", other),
        }
    }

    #[test]
    fn dce_preserves_side_effectful_rhs_as_eval() {
        let mut fn_ir = FnIR::new("dead_assign_call".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let side_effect_call = fn_ir.add_value(
            ValueKind::Call {
                callee: "unknown_effect".to_string(),
                args: Vec::new(),
                names: Vec::new(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: ".tachyon_tmp".to_string(),
            src: side_effect_call,
            span: Span::default(),
        });
        fn_ir.blocks[entry].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(changed);
        assert_eq!(fn_ir.blocks[entry].instrs.len(), 1);
        match &fn_ir.blocks[entry].instrs[0] {
            Instr::Eval { val, .. } => assert_eq!(*val, side_effect_call),
            other => panic!(
                "side-effectful dead assign should become Eval, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn parse_phase_ordering_mode_defaults_to_off() {
        assert_eq!(
            TachyonEngine::parse_phase_ordering_mode(None),
            super::types::PhaseOrderingMode::Off
        );
        assert_eq!(
            TachyonEngine::parse_phase_ordering_mode(Some("")),
            super::types::PhaseOrderingMode::Off
        );
        assert_eq!(
            TachyonEngine::parse_phase_ordering_mode(Some("unknown")),
            super::types::PhaseOrderingMode::Off
        );
    }

    #[test]
    fn parse_phase_ordering_mode_accepts_supported_values() {
        assert_eq!(
            TachyonEngine::parse_phase_ordering_mode(Some("off")),
            super::types::PhaseOrderingMode::Off
        );
        assert_eq!(
            TachyonEngine::parse_phase_ordering_mode(Some("balanced")),
            super::types::PhaseOrderingMode::Balanced
        );
        assert_eq!(
            TachyonEngine::parse_phase_ordering_mode(Some("AUTO")),
            super::types::PhaseOrderingMode::Auto
        );
    }

    #[test]
    fn legacy_phase_plan_keeps_balanced_schedule_for_all_modes() {
        for mode in [
            super::types::PhaseOrderingMode::Off,
            super::types::PhaseOrderingMode::Balanced,
            super::types::PhaseOrderingMode::Auto,
        ] {
            let plan = super::types::FunctionPhasePlan::legacy("demo".to_string(), mode, false);
            assert_eq!(plan.function, "demo");
            assert_eq!(plan.mode, mode);
            assert_eq!(plan.profile, super::types::PhaseProfileKind::Balanced);
            assert_eq!(plan.schedule, super::types::PhaseScheduleId::Balanced);
            assert!(plan.features.is_none());
            assert!(!plan.trace_requested);
        }
    }

    #[test]
    fn phase_ordering_opt_level_default_maps_o1_and_o2() {
        assert_eq!(
            TachyonEngine::phase_ordering_opt_level_default(crate::compiler::OptLevel::O0),
            super::types::PhaseOrderingMode::Off
        );
        assert_eq!(
            TachyonEngine::phase_ordering_opt_level_default(crate::compiler::OptLevel::O1),
            super::types::PhaseOrderingMode::Balanced
        );
        assert_eq!(
            TachyonEngine::phase_ordering_opt_level_default(crate::compiler::OptLevel::O2),
            super::types::PhaseOrderingMode::Auto
        );
    }
}
