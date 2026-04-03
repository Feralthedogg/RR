//! Mutable state partitions used by MIR-to-R emission.
//!
//! `RBackend` owns these contexts and coordinates their lifetime, but each
//! struct below captures one concern so the emitter no longer depends on a
//! single flat "god object" state bag.

use crate::typeck::LenSym;
use crate::utils::Span;
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEntry {
    pub r_line: u32,
    pub rr_span: Span,
}

#[derive(Debug)]
pub(crate) struct ValueBindingUndo {
    pub(crate) val_id: usize,
    pub(crate) prev: Option<(String, u64)>,
}

#[derive(Debug)]
pub(crate) struct VarVersionUndo {
    pub(crate) var: String,
    pub(crate) prev: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct VarValueBindingUndo {
    pub(crate) var: String,
    pub(crate) prev: Option<(usize, u64)>,
}

#[derive(Debug)]
pub(crate) struct LastAssignedValueUndo {
    pub(crate) var: String,
    pub(crate) prev: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BranchSnapshot {
    pub(crate) value_binding_log_len: usize,
    pub(crate) var_version_log_len: usize,
    pub(crate) var_value_binding_log_len: usize,
    pub(crate) last_assigned_value_log_len: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct TypedParallelWrapperPlan {
    pub(crate) impl_name: String,
    pub(crate) slice_param_slots: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScalarLoopCmp {
    Lt,
    Le,
}

#[derive(Clone, Debug)]
pub(crate) struct ActiveScalarLoopIndex {
    pub(crate) var: String,
    pub(crate) start_min: i64,
    pub(crate) cmp: ScalarLoopCmp,
}

#[derive(Debug, Default)]
pub(crate) struct ValueTracker {
    pub(crate) value_bindings: FxHashMap<usize, (String, u64)>,
    pub(crate) var_versions: FxHashMap<String, u64>,
    pub(crate) var_value_bindings: FxHashMap<String, (usize, u64)>,
    pub(crate) last_assigned_value_ids: FxHashMap<String, usize>,
    pub(crate) value_binding_log: Vec<ValueBindingUndo>,
    pub(crate) var_version_log: Vec<VarVersionUndo>,
    pub(crate) var_value_binding_log: Vec<VarValueBindingUndo>,
    pub(crate) last_assigned_value_log: Vec<LastAssignedValueUndo>,
    pub(crate) branch_snapshot_depth: usize,
}

impl ValueTracker {
    /// Reset all binding/version state for a fresh function emission.
    pub(crate) fn clear(&mut self) {
        self.value_bindings.clear();
        self.var_versions.clear();
        self.var_value_bindings.clear();
        self.last_assigned_value_ids.clear();
        self.value_binding_log.clear();
        self.var_version_log.clear();
        self.var_value_binding_log.clear();
        self.last_assigned_value_log.clear();
        self.branch_snapshot_depth = 0;
    }
}

#[derive(Debug, Default)]
pub(crate) struct EmitScratch {
    pub(crate) expr_use_counts: FxHashMap<usize, usize>,
    pub(crate) expr_path: FxHashSet<usize>,
    pub(crate) emitted_ids: FxHashSet<usize>,
    pub(crate) emitted_temp_names: Vec<String>,
}

impl EmitScratch {
    /// Clear per-emission scratch structures that do not survive across functions.
    pub(crate) fn clear(&mut self) {
        self.expr_use_counts.clear();
        self.expr_path.clear();
        self.emitted_ids.clear();
        self.emitted_temp_names.clear();
    }
}

#[derive(Debug, Default)]
pub(crate) struct LoopAnalysisContext {
    pub(crate) recent_whole_assign_bases: FxHashSet<String>,
    pub(crate) known_full_end_exprs: FxHashMap<String, String>,
    pub(crate) len_sym_end_exprs: FxHashMap<LenSym, String>,
    pub(crate) active_loop_known_full_end_exprs: Vec<FxHashMap<String, String>>,
    pub(crate) active_loop_mutated_vars: Vec<FxHashSet<String>>,
    pub(crate) active_scalar_loop_indices: Vec<ActiveScalarLoopIndex>,
    pub(crate) active_loop_fallback_vars: Vec<String>,
}

impl LoopAnalysisContext {
    /// Drop all loop-local facts before emitting a new function or wrapper body.
    pub(crate) fn clear(&mut self) {
        self.recent_whole_assign_bases.clear();
        self.known_full_end_exprs.clear();
        self.len_sym_end_exprs.clear();
        self.active_loop_known_full_end_exprs.clear();
        self.active_loop_mutated_vars.clear();
        self.active_scalar_loop_indices.clear();
        self.active_loop_fallback_vars.clear();
    }
}

#[derive(Debug)]
pub(crate) struct EmitAnalysisContext {
    pub(crate) known_fresh_result_calls: Arc<FxHashSet<String>>,
    pub(crate) seq_len_param_end_slots_by_fn: Arc<FxHashMap<String, FxHashMap<usize, usize>>>,
    pub(crate) current_seq_len_param_end_slots: FxHashMap<usize, usize>,
    pub(crate) direct_builtin_vector_math: bool,
}

impl EmitAnalysisContext {
    /// Build the shared analysis context that can be reused across emitted functions.
    pub(crate) fn new(
        known_fresh_result_calls: Arc<FxHashSet<String>>,
        seq_len_param_end_slots_by_fn: Arc<FxHashMap<String, FxHashMap<usize, usize>>>,
        direct_builtin_vector_math: bool,
    ) -> Self {
        Self {
            known_fresh_result_calls,
            seq_len_param_end_slots_by_fn,
            current_seq_len_param_end_slots: FxHashMap::default(),
            direct_builtin_vector_math,
        }
    }

    /// Clear function-local analysis state while preserving shared lookup tables.
    pub(crate) fn clear_runtime_state(&mut self) {
        self.current_seq_len_param_end_slots.clear();
    }

    /// Load any callsite-specific summaries needed for the function being emitted.
    pub(crate) fn prepare_for_fn(&mut self, fn_name: &str) {
        self.current_seq_len_param_end_slots = self
            .seq_len_param_end_slots_by_fn
            .get(fn_name)
            .cloned()
            .unwrap_or_default();
    }
}

/// Coordinator for MIR-to-R emission.
///
/// Child modules in `codegen/emit/*` operate on this state, while the grouped
/// contexts above keep binding/version tracking, scratch storage, loop facts,
/// and shared analysis inputs logically separate.
pub struct RBackend {
    pub(crate) output: String,
    pub(crate) indent: usize,
    pub(crate) current_line: u32,
    pub(crate) current_fn_name: String,
    pub source_map: Vec<MapEntry>,
    pub(crate) value_tracker: ValueTracker,
    pub(crate) emit_scratch: EmitScratch,
    pub(crate) loop_analysis: LoopAnalysisContext,
    pub(crate) analysis: EmitAnalysisContext,
}
