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
pub mod sccp;
pub mod simplify;
pub mod tco;
pub mod type_specialize;
#[path = "opt/v_opt/mod.rs"]
pub mod v_opt;

use self::types::{FunctionBudgetProfile, ProgramOptPlan};
pub use self::types::{TachyonProgress, TachyonProgressTier, TachyonPulseStats};

pub struct TachyonEngine;

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
    pub fn stabilize_for_codegen(&self, all_fns: &mut FxHashMap<String, FnIR>) {
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
            if !Self::verify_or_reject(fn_ir, "PrepareForCodegen/Start") {
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

    fn run_always_tier_with_stats(
        &self,
        fn_ir: &mut FnIR,
        proven_param_slots: Option<&FxHashSet<usize>>,
        floor_helpers: &FxHashSet<String>,
    ) -> TachyonPulseStats {
        let mut stats = TachyonPulseStats::default();
        if fn_ir.requires_conservative_optimization() {
            return stats;
        }
        if !Self::verify_or_reject(fn_ir, "AlwaysTier/Start") {
            return stats;
        }
        let canonicalized_index_params =
            Self::canonicalize_floor_index_params(fn_ir, proven_param_slots, floor_helpers);
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

            let tco_changed = tco::optimize(fn_ir);
            if tco_changed {
                stats.tco_hits += 1;
                changed = true;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/TCO");

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
        for (idx, name) in ordered_names.iter().enumerate() {
            let Some(fn_ir) = all_fns.get_mut(name) else {
                continue;
            };
            let s = self.run_always_tier_with_stats(
                fn_ir,
                proven_floor_param_slots.get(name),
                &floor_helpers,
            );
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
        for (idx, name) in ordered_names.iter().enumerate() {
            let Some(fn_ir) = all_fns.get_mut(name) else {
                continue;
            };
            if fn_ir.requires_conservative_optimization() {
                stats.skipped_functions += 1;
                let _ = Self::verify_or_reject(fn_ir, "SkipOpt/ConservativeInterop");
                Self::emit_progress(
                    &mut progress,
                    TachyonProgressTier::Heavy,
                    idx + 1,
                    ordered_total,
                    name,
                );
                continue;
            }
            if fn_ir
                .values
                .iter()
                .any(|value| matches!(&value.kind, ValueKind::Call { callee, .. } if callee == &fn_ir.name))
            {
                stats.skipped_functions += 1;
                let _ = Self::verify_or_reject(fn_ir, "SkipOpt/SelfRecursive");
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
                        if fn_ir.requires_conservative_optimization() {
                            Self::maybe_verify(
                                fn_ir,
                                "After Inline Cleanup (Skipped: ConservativeInterop)",
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

        let _ = fresh_alias::optimize_program(all_fns);

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
            let copy_cleanup_changed = if fn_ir.requires_conservative_optimization() {
                false
            } else {
                copy_cleanup::optimize(fn_ir)
            };
            if copy_cleanup_changed {
                stats.simplify_hits += 1;
            }
            // Cleanup after De-SSA to drop dead temps and unreachable blocks.
            if !fn_ir.requires_conservative_optimization() {
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
        let floor_helpers = FxHashSet::default();
        self.run_function_with_proven_index_slots(
            fn_ir,
            callmap_user_whitelist,
            proven_param_slots,
            &floor_helpers,
        )
    }

    fn run_function_with_proven_index_slots(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        proven_param_slots: Option<&FxHashSet<usize>>,
        floor_helpers: &FxHashSet<String>,
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
        Self::debug_stage_dump(fn_ir, "Start");
        let canonicalized_index_params =
            Self::canonicalize_floor_index_params(fn_ir, proven_param_slots, floor_helpers);
        if canonicalized_index_params {
            Self::maybe_verify(fn_ir, "After ParamIndexCanonicalize");
            Self::debug_stage_dump(fn_ir, "After ParamIndexCanonicalize");
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
                Self::debug_stage_dump(fn_ir, "After TypeSpecialize");
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
                stats.vector_candidate_total += v_stats.candidate_total;
                stats.vector_candidate_reductions += v_stats.candidate_reductions;
                stats.vector_candidate_conditionals += v_stats.candidate_conditionals;
                stats.vector_candidate_recurrences += v_stats.candidate_recurrences;
                stats.vector_candidate_shifted += v_stats.candidate_shifted;
                stats.vector_candidate_call_maps += v_stats.candidate_call_maps;
                stats.vector_candidate_expr_maps += v_stats.candidate_expr_maps;
                stats.vector_candidate_scatters += v_stats.candidate_scatters;
                stats.vector_candidate_cube_slices += v_stats.candidate_cube_slices;
                stats.vector_candidate_basic_maps += v_stats.candidate_basic_maps;
                stats.vector_candidate_multi_exprs += v_stats.candidate_multi_exprs;
                stats.vector_candidate_2d += v_stats.candidate_2d;
                stats.vector_candidate_3d += v_stats.candidate_3d;
                stats.vector_candidate_call_map_direct += v_stats.candidate_call_map_direct;
                stats.vector_candidate_call_map_runtime += v_stats.candidate_call_map_runtime;
                stats.vector_applied_total += v_stats.applied_total;
                stats.vector_applied_reductions += v_stats.applied_reductions;
                stats.vector_applied_conditionals += v_stats.applied_conditionals;
                stats.vector_applied_recurrences += v_stats.applied_recurrences;
                stats.vector_applied_shifted += v_stats.applied_shifted;
                stats.vector_applied_call_maps += v_stats.applied_call_maps;
                stats.vector_applied_expr_maps += v_stats.applied_expr_maps;
                stats.vector_applied_scatters += v_stats.applied_scatters;
                stats.vector_applied_cube_slices += v_stats.applied_cube_slices;
                stats.vector_applied_basic_maps += v_stats.applied_basic_maps;
                stats.vector_applied_multi_exprs += v_stats.applied_multi_exprs;
                stats.vector_applied_2d += v_stats.applied_2d;
                stats.vector_applied_3d += v_stats.applied_3d;
                stats.vector_applied_call_map_direct += v_stats.applied_call_map_direct;
                stats.vector_applied_call_map_runtime += v_stats.applied_call_map_runtime;
                stats.vector_trip_tier_tiny += v_stats.trip_tier_tiny;
                stats.vector_trip_tier_small += v_stats.trip_tier_small;
                stats.vector_trip_tier_medium += v_stats.trip_tier_medium;
                stats.vector_trip_tier_large += v_stats.trip_tier_large;
                stats.proof_certified += v_stats.proof_certified;
                stats.proof_applied += v_stats.proof_applied;
                stats.proof_apply_failed += v_stats.proof_apply_failed;
                stats.proof_fallback_pattern += v_stats.proof_fallback_pattern;
                for (dst, src) in stats
                    .proof_fallback_reason_counts
                    .iter_mut()
                    .zip(v_stats.proof_fallback_reason_counts)
                {
                    *dst += src;
                }
                let v_changed = v_stats.changed();
                Self::maybe_verify(fn_ir, "After Vectorization");
                Self::debug_stage_dump(fn_ir, "After Vectorization");
                pass_changed |= v_changed;

                let type_spec_post_vec = type_specialize::optimize(fn_ir);
                Self::maybe_verify(fn_ir, "After TypeSpecialize(PostVec)");
                Self::debug_stage_dump(fn_ir, "After TypeSpecialize(PostVec)");
                pass_changed |= type_spec_post_vec;

                // TCO
                let tco_changed = tco::optimize(fn_ir);
                if tco_changed {
                    stats.tco_hits += 1;
                }
                Self::maybe_verify(fn_ir, "After TCO");
                Self::debug_stage_dump(fn_ir, "After TCO");
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
                Self::debug_stage_dump(fn_ir, "After Structural SimplifyCFG");
                let dce_changed = self.dce(fn_ir);
                if dce_changed {
                    stats.dce_hits += 1;
                }
                Self::maybe_verify(fn_ir, "After Structural DCE");
                Self::debug_stage_dump(fn_ir, "After Structural DCE");
                changed |= sc_changed || dce_changed;
            }

            // 2. Standard optimization passes
            let sc_changed = self.simplify_cfg(fn_ir);
            if sc_changed {
                stats.simplify_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After SimplifyCFG");
            Self::debug_stage_dump(fn_ir, "After SimplifyCFG");
            changed |= sc_changed;

            let sccp_changed = sccp::MirSCCP::new().optimize(fn_ir);
            if sccp_changed {
                stats.sccp_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After SCCP");
            Self::debug_stage_dump(fn_ir, "After SCCP");
            changed |= sccp_changed;

            let intr_changed = intrinsics::optimize(fn_ir);
            if intr_changed {
                stats.intrinsics_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After Intrinsics");
            Self::debug_stage_dump(fn_ir, "After Intrinsics");
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
            Self::debug_stage_dump(fn_ir, "After GVN");
            changed |= gvn_changed;

            let simplify_changed = simplify::optimize(fn_ir);
            if simplify_changed {
                stats.simplify_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After Simplify");
            Self::debug_stage_dump(fn_ir, "After Simplify");
            changed |= simplify_changed;

            let dce_changed = self.dce(fn_ir);
            if dce_changed {
                stats.dce_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After DCE");
            Self::debug_stage_dump(fn_ir, "After DCE");
            changed |= dce_changed;

            if !(heavy_pass_budgeted && iterations > 1) {
                let loop_changed_count = loop_opt.optimize_with_count(fn_ir);
                stats.simplified_loops += loop_changed_count;
                let loop_changed = loop_changed_count > 0;
                Self::maybe_verify(fn_ir, "After LoopOpt");
                Self::debug_stage_dump(fn_ir, "After LoopOpt");
                changed |= loop_changed;

                let licm_changed = if Self::licm_enabled() && Self::licm_allowed_for_fn(fn_ir) {
                    let c = licm::MirLicm::new().optimize(fn_ir);
                    if c {
                        stats.licm_hits += 1;
                    }
                    c
                } else {
                    false
                };
                Self::maybe_verify(fn_ir, "After LICM");
                Self::debug_stage_dump(fn_ir, "After LICM");
                changed |= licm_changed;

                let fresh_changed = fresh_alloc::optimize(fn_ir);
                if fresh_changed {
                    stats.fresh_alloc_hits += 1;
                }
                Self::maybe_verify(fn_ir, "After FreshAlloc");
                Self::debug_stage_dump(fn_ir, "After FreshAlloc");
                changed |= fresh_changed;

                let bce_changed = bce::optimize(fn_ir);
                if bce_changed {
                    stats.bce_hits += 1;
                }
                Self::maybe_verify(fn_ir, "After BCE");
                Self::debug_stage_dump(fn_ir, "After BCE");
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
        Self::debug_stage_dump(fn_ir, "End");
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
        assert!(matches!(fn_ir.blocks[entry].term, Terminator::Goto(target) if target == entry));
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
}
