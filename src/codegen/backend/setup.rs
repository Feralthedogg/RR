//! Constructors and lifecycle helpers for `MirEmitter` and `RBackend`.
//!
//! These APIs assemble the grouped backend contexts and reset them between
//! emitted functions so the emission code can stay focused on lowering logic.

use crate::codegen::backend::state::{
    EmitAnalysisContext, EmitScratch, LoopAnalysisContext, MapEntry, RBackend, ValueTracker,
};
use crate::codegen::mir_emit::MirEmitter;
use crate::error::RR;
use crate::mir::def::FnIR;
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;

impl Default for MirEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl MirEmitter {
    pub fn new() -> Self {
        Self::with_options(FxHashSet::default(), true)
    }

    pub fn with_fresh_result_calls(known_fresh_result_calls: FxHashSet<String>) -> Self {
        Self::with_options(known_fresh_result_calls, true)
    }

    pub fn with_options(
        known_fresh_result_calls: FxHashSet<String>,
        direct_builtin_vector_math: bool,
    ) -> Self {
        Self::with_analysis_options(
            known_fresh_result_calls,
            FxHashMap::default(),
            direct_builtin_vector_math,
        )
    }

    pub fn with_analysis_options(
        known_fresh_result_calls: FxHashSet<String>,
        seq_len_param_end_slots_by_fn: FxHashMap<String, FxHashMap<usize, usize>>,
        direct_builtin_vector_math: bool,
    ) -> Self {
        Self::with_shared_analysis_options(
            Arc::new(known_fresh_result_calls),
            Arc::new(seq_len_param_end_slots_by_fn),
            direct_builtin_vector_math,
        )
    }

    pub fn with_shared_analysis_options(
        known_fresh_result_calls: Arc<FxHashSet<String>>,
        seq_len_param_end_slots_by_fn: Arc<FxHashMap<String, FxHashMap<usize, usize>>>,
        direct_builtin_vector_math: bool,
    ) -> Self {
        Self {
            backend: RBackend::with_shared_analysis_options(
                known_fresh_result_calls,
                seq_len_param_end_slots_by_fn,
                direct_builtin_vector_math,
            ),
        }
    }

    pub fn emit(&mut self, fn_ir: &FnIR) -> RR<(String, Vec<MapEntry>)> {
        self.backend.emit_function(fn_ir)
    }
}

impl Default for RBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RBackend {
    pub fn new() -> Self {
        Self::with_options(FxHashSet::default(), true)
    }

    pub fn with_fresh_result_calls(known_fresh_result_calls: FxHashSet<String>) -> Self {
        Self::with_options(known_fresh_result_calls, true)
    }

    pub fn with_options(
        known_fresh_result_calls: FxHashSet<String>,
        direct_builtin_vector_math: bool,
    ) -> Self {
        Self::with_analysis_options(
            known_fresh_result_calls,
            FxHashMap::default(),
            direct_builtin_vector_math,
        )
    }

    pub fn with_analysis_options(
        known_fresh_result_calls: FxHashSet<String>,
        seq_len_param_end_slots_by_fn: FxHashMap<String, FxHashMap<usize, usize>>,
        direct_builtin_vector_math: bool,
    ) -> Self {
        Self::with_shared_analysis_options(
            Arc::new(known_fresh_result_calls),
            Arc::new(seq_len_param_end_slots_by_fn),
            direct_builtin_vector_math,
        )
    }

    pub fn with_shared_analysis_options(
        known_fresh_result_calls: Arc<FxHashSet<String>>,
        seq_len_param_end_slots_by_fn: Arc<FxHashMap<String, FxHashMap<usize, usize>>>,
        direct_builtin_vector_math: bool,
    ) -> Self {
        Self {
            output: String::new(),
            indent: 0,
            current_line: 1,
            current_fn_name: String::new(),
            source_map: Vec::new(),
            value_tracker: ValueTracker::default(),
            emit_scratch: EmitScratch::default(),
            loop_analysis: LoopAnalysisContext::default(),
            analysis: EmitAnalysisContext::new(
                known_fresh_result_calls,
                seq_len_param_end_slots_by_fn,
                direct_builtin_vector_math,
            ),
        }
    }

    pub(crate) fn reset_emit_output_state(&mut self) {
        self.output.clear();
        self.indent = 0;
        self.current_line = 1;
        self.source_map.clear();
        self.current_fn_name.clear();
        self.value_tracker.clear();
        self.emit_scratch.clear();
        self.loop_analysis.clear();
        self.analysis.clear_runtime_state();
    }

    pub(crate) fn prepare_function_emit_state(&mut self, fn_ir: &FnIR) {
        self.current_fn_name = fn_ir.name.clone();
        self.value_tracker.clear();
        self.emit_scratch.clear();
        self.loop_analysis.clear();
        self.analysis.prepare_for_fn(fn_ir.name.as_str());
    }
}
