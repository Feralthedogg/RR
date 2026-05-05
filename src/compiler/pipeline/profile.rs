use crate::compiler::scheduler::CompilerParallelProfile;
use crate::mir::opt::{TachyonPassTimings, TachyonPulseStats};
use std::fmt::Write;

#[derive(Clone, Debug, Default)]
pub struct PhaseProfile {
    pub elapsed_ns: u128,
}

#[derive(Clone, Debug, Default)]
pub struct SourceAnalysisProfile {
    pub elapsed_ns: u128,
    pub parsed_modules: usize,
    pub cached_modules: usize,
}

#[derive(Clone, Debug, Default)]
pub struct MirSynthesisProfile {
    pub elapsed_ns: u128,
    pub lowered_functions: usize,
}

#[derive(Clone, Debug, Default)]
pub struct EmitBreakdownProfile {
    pub fragment_build_elapsed_ns: u128,
    pub cache_load_elapsed_ns: u128,
    pub emitter_elapsed_ns: u128,
    pub cache_store_elapsed_ns: u128,
    pub optimized_fragment_cache_hits: usize,
    pub optimized_fragment_cache_misses: usize,
    pub optimized_fragment_final_artifact_hits: usize,
    pub optimized_fragment_fast_path_direct_hits: usize,
    pub optimized_fragment_fast_path_raw_hits: usize,
    pub optimized_fragment_fast_path_peephole_hits: usize,
    pub quote_wrap_elapsed_ns: u128,
    pub fragment_assembly_elapsed_ns: u128,
    pub raw_rewrite_elapsed_ns: u128,
    pub peephole_elapsed_ns: u128,
    pub peephole_linear_scan_elapsed_ns: u128,
    pub peephole_primary_rewrite_elapsed_ns: u128,
    pub peephole_primary_flow_elapsed_ns: u128,
    pub peephole_primary_inline_elapsed_ns: u128,
    pub peephole_primary_reuse_elapsed_ns: u128,
    pub peephole_primary_loop_cleanup_elapsed_ns: u128,
    pub peephole_primary_loop_dead_zero_elapsed_ns: u128,
    pub peephole_primary_loop_normalize_elapsed_ns: u128,
    pub peephole_primary_loop_hoist_elapsed_ns: u128,
    pub peephole_primary_loop_repeat_to_for_elapsed_ns: u128,
    pub peephole_primary_loop_tail_cleanup_elapsed_ns: u128,
    pub peephole_primary_loop_guard_cleanup_elapsed_ns: u128,
    pub peephole_primary_loop_helper_cleanup_elapsed_ns: u128,
    pub peephole_primary_loop_exact_cleanup_elapsed_ns: u128,
    pub peephole_primary_loop_exact_pre_elapsed_ns: u128,
    pub peephole_primary_loop_exact_reuse_elapsed_ns: u128,
    pub peephole_primary_loop_exact_reuse_prepare_elapsed_ns: u128,
    pub peephole_primary_loop_exact_reuse_forward_elapsed_ns: u128,
    pub peephole_primary_loop_exact_reuse_pure_call_elapsed_ns: u128,
    pub peephole_primary_loop_exact_reuse_expr_elapsed_ns: u128,
    pub peephole_primary_loop_exact_reuse_vector_alias_elapsed_ns: u128,
    pub peephole_primary_loop_exact_reuse_rebind_elapsed_ns: u128,
    pub peephole_primary_loop_exact_fixpoint_elapsed_ns: u128,
    pub peephole_primary_loop_exact_fixpoint_prepare_elapsed_ns: u128,
    pub peephole_primary_loop_exact_fixpoint_forward_elapsed_ns: u128,
    pub peephole_primary_loop_exact_fixpoint_pure_call_elapsed_ns: u128,
    pub peephole_primary_loop_exact_fixpoint_expr_elapsed_ns: u128,
    pub peephole_primary_loop_exact_fixpoint_rebind_elapsed_ns: u128,
    pub peephole_primary_loop_exact_fixpoint_rounds: usize,
    pub peephole_primary_loop_exact_finalize_elapsed_ns: u128,
    pub peephole_primary_loop_dead_temp_cleanup_elapsed_ns: u128,
    pub peephole_secondary_rewrite_elapsed_ns: u128,
    pub peephole_secondary_inline_elapsed_ns: u128,
    pub peephole_secondary_inline_branch_hoist_elapsed_ns: u128,
    pub peephole_secondary_inline_immediate_scalar_elapsed_ns: u128,
    pub peephole_secondary_inline_named_index_elapsed_ns: u128,
    pub peephole_secondary_inline_named_expr_elapsed_ns: u128,
    pub peephole_secondary_inline_scalar_region_elapsed_ns: u128,
    pub peephole_secondary_inline_immediate_index_elapsed_ns: u128,
    pub peephole_secondary_inline_adjacent_dedup_elapsed_ns: u128,
    pub peephole_secondary_exact_elapsed_ns: u128,
    pub peephole_secondary_helper_cleanup_elapsed_ns: u128,
    pub peephole_secondary_helper_wrapper_elapsed_ns: u128,
    pub peephole_secondary_helper_metric_elapsed_ns: u128,
    pub peephole_secondary_helper_alias_elapsed_ns: u128,
    pub peephole_secondary_helper_simple_expr_elapsed_ns: u128,
    pub peephole_secondary_helper_full_range_elapsed_ns: u128,
    pub peephole_secondary_helper_named_copy_elapsed_ns: u128,
    pub peephole_secondary_record_sroa_elapsed_ns: u128,
    pub peephole_secondary_finalize_cleanup_elapsed_ns: u128,
    pub peephole_secondary_finalize_bundle_elapsed_ns: u128,
    pub peephole_secondary_finalize_dead_temp_elapsed_ns: u128,
    pub peephole_secondary_finalize_dead_temp_facts_elapsed_ns: u128,
    pub peephole_secondary_finalize_dead_temp_mark_elapsed_ns: u128,
    pub peephole_secondary_finalize_dead_temp_reverse_elapsed_ns: u128,
    pub peephole_secondary_finalize_dead_temp_compact_elapsed_ns: u128,
    pub peephole_finalize_elapsed_ns: u128,
    pub source_map_remap_elapsed_ns: u128,
    pub quoted_wrapped_functions: usize,
}

#[derive(Clone, Debug, Default)]
pub struct EmitProfile {
    pub elapsed_ns: u128,
    pub emitted_functions: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub breakdown: EmitBreakdownProfile,
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeInjectionProfile {
    pub elapsed_ns: u128,
    pub inject_runtime: bool,
}

#[derive(Clone, Debug, Default)]
pub struct IncrementalProfile {
    pub enabled: bool,
    pub phase1_artifact_hit: bool,
    pub phase2_emit_hits: usize,
    pub phase2_emit_misses: usize,
    pub phase3_memory_hit: bool,
    pub strict_verification_checked: bool,
    pub strict_verification_passed: bool,
    pub miss_reasons: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct TachyonProfile {
    pub elapsed_ns: u128,
    pub optimized_mir_cache_hit: bool,
    pub pulse_stats: TachyonPulseStats,
    pub pass_timings: TachyonPassTimings,
    pub disabled_pass_groups: Vec<String>,
    pub active_pass_groups: Vec<String>,
    pub plan_summary: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct CompileProfile {
    pub compile_mode: String,
    pub incremental: IncrementalProfile,
    pub compiler_parallel: CompilerParallelProfile,
    pub source_analysis: SourceAnalysisProfile,
    pub canonicalization: PhaseProfile,
    pub mir_synthesis: MirSynthesisProfile,
    pub tachyon: TachyonProfile,
    pub emit: EmitProfile,
    pub runtime_injection: RuntimeInjectionProfile,
    pub total_elapsed_ns: u128,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SourceAnalysisMetrics {
    pub(crate) source_analysis_elapsed_ns: u128,
    pub(crate) canonicalization_elapsed_ns: u128,
    pub(crate) parsed_modules: usize,
    pub(crate) cached_modules: usize,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct MirSynthesisMetrics {
    pub(crate) elapsed_ns: u128,
    pub(crate) lowered_functions: usize,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct EmitMetrics {
    pub(crate) elapsed_ns: u128,
    pub(crate) emitted_functions: usize,
    pub(crate) cache_hits: usize,
    pub(crate) cache_misses: usize,
    pub(crate) breakdown: EmitBreakdownProfile,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TachyonPhaseMetrics {
    pub(crate) elapsed_ns: u128,
    pub(crate) optimized_mir_cache_hit: bool,
    pub(crate) pulse_stats: TachyonPulseStats,
    pub(crate) pass_timings: TachyonPassTimings,
    pub(crate) disabled_pass_groups: Vec<String>,
    pub(crate) active_pass_groups: Vec<String>,
    pub(crate) plan_summary: Vec<String>,
}

impl CompileProfile {
    pub fn to_json_string(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str("  \"schema\": \"rr-compile-profile\",\n");
        out.push_str("  \"version\": 4,\n");
        let _ = writeln!(
            out,
            "  \"compile_mode\": \"{}\",",
            json_escape(&self.compile_mode)
        );
        out.push_str("  \"incremental\": {");
        let _ = write!(
            out,
            "\"enabled\": {}, \"phase1_artifact_hit\": {}, \"phase2_emit_hits\": {}, \"phase2_emit_misses\": {}, \"phase3_memory_hit\": {}, \"strict_verification_checked\": {}, \"strict_verification_passed\": {}, \"miss_reasons\": [",
            self.incremental.enabled,
            self.incremental.phase1_artifact_hit,
            self.incremental.phase2_emit_hits,
            self.incremental.phase2_emit_misses,
            self.incremental.phase3_memory_hit,
            self.incremental.strict_verification_checked,
            self.incremental.strict_verification_passed
        );
        for (idx, reason) in self.incremental.miss_reasons.iter().enumerate() {
            if idx > 0 {
                out.push_str(", ");
            }
            out.push('"');
            out.push_str(&json_escape(reason));
            out.push('"');
        }
        out.push_str("]},\n");
        let _ = writeln!(out, "  \"compiler_parallel\": {{");
        let _ = writeln!(
            out,
            "    \"mode\": \"{}\",",
            json_escape(&self.compiler_parallel.mode)
        );
        let _ = writeln!(
            out,
            "    \"configured_threads\": {},",
            self.compiler_parallel.configured_threads
        );
        let _ = writeln!(
            out,
            "    \"active_workers\": {},",
            self.compiler_parallel.active_workers
        );
        let _ = writeln!(
            out,
            "    \"max_jobs\": {},",
            self.compiler_parallel.max_jobs
        );
        let _ = writeln!(
            out,
            "    \"min_functions\": {},",
            self.compiler_parallel.min_functions
        );
        let _ = writeln!(
            out,
            "    \"min_fn_ir\": {},",
            self.compiler_parallel.min_fn_ir
        );
        out.push_str("    \"stages\": [\n");
        for (idx, stage) in self.compiler_parallel.stages.iter().enumerate() {
            out.push_str("      {");
            let _ = write!(
                out,
                "\"stage\": \"{}\", \"invocations\": {}, \"parallel_invocations\": {}, \"serial_invocations\": {}, \"total_jobs\": {}, \"total_ir\": {}, \"max_jobs\": {}, \"max_ir\": {}, \"reason_counts\": {{",
                json_escape(&stage.stage),
                stage.invocations,
                stage.parallel_invocations,
                stage.serial_invocations,
                stage.total_jobs,
                stage.total_ir,
                stage.max_jobs,
                stage.max_ir
            );
            let mut first_reason = true;
            for (reason, count) in &stage.reason_counts {
                if !first_reason {
                    out.push_str(", ");
                }
                first_reason = false;
                let _ = write!(out, "\"{}\": {}", json_escape(reason), count);
            }
            out.push('}');
            out.push('}');
            if idx + 1 != self.compiler_parallel.stages.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("    ]\n");
        out.push_str("  },\n");
        let _ = writeln!(
            out,
            "  \"source_analysis\": {{\"elapsed_ns\": {}, \"parsed_modules\": {}, \"cached_modules\": {}}},",
            self.source_analysis.elapsed_ns,
            self.source_analysis.parsed_modules,
            self.source_analysis.cached_modules
        );
        let _ = writeln!(
            out,
            "  \"canonicalization\": {{\"elapsed_ns\": {}}},",
            self.canonicalization.elapsed_ns
        );
        let _ = writeln!(
            out,
            "  \"mir_synthesis\": {{\"elapsed_ns\": {}, \"lowered_functions\": {}}},",
            self.mir_synthesis.elapsed_ns, self.mir_synthesis.lowered_functions
        );
        let _ = writeln!(out, "  \"tachyon\": {{");
        let _ = writeln!(out, "    \"elapsed_ns\": {},", self.tachyon.elapsed_ns);
        let _ = writeln!(
            out,
            "    \"optimized_mir_cache_hit\": {},",
            self.tachyon.optimized_mir_cache_hit
        );
        out.push_str("    \"disabled_pass_groups\": [");
        for (idx, group) in self.tachyon.disabled_pass_groups.iter().enumerate() {
            if idx > 0 {
                out.push_str(", ");
            }
            out.push('"');
            out.push_str(&json_escape(group));
            out.push('"');
        }
        out.push_str("],\n");
        out.push_str("    \"active_pass_groups\": [");
        for (idx, group) in self.tachyon.active_pass_groups.iter().enumerate() {
            if idx > 0 {
                out.push_str(", ");
            }
            out.push('"');
            out.push_str(&json_escape(group));
            out.push('"');
        }
        out.push_str("],\n");
        out.push_str("    \"plan_summary\": [");
        for (idx, line) in self.tachyon.plan_summary.iter().enumerate() {
            if idx > 0 {
                out.push_str(", ");
            }
            out.push('"');
            out.push_str(&json_escape(line));
            out.push('"');
        }
        out.push_str("],\n");
        out.push_str("    \"passes\": ");
        out.push_str(&self.tachyon.pass_timings.to_json_string());
        out.push_str(",\n");
        out.push_str("    \"pass_decisions\": ");
        out.push_str(&self.tachyon.pass_timings.decisions_to_json_string());
        out.push_str(",\n");
        out.push_str("    \"optimization_opportunities\": ");
        out.push_str(&self.tachyon.pass_timings.opportunities_to_json_string());
        out.push_str(",\n");
        out.push_str("    \"pulse_stats\": ");
        out.push_str(&indent_json_block(
            &self.tachyon.pulse_stats.to_json_string(),
            4,
        ));
        out.push_str("\n  },\n");
        let _ = writeln!(
            out,
            "  \"emit\": {{\"elapsed_ns\": {}, \"emitted_functions\": {}, \"cache_hits\": {}, \"cache_misses\": {}, \"breakdown\": {{\"fragment_build_elapsed_ns\": {}, \"cache_load_elapsed_ns\": {}, \"emitter_elapsed_ns\": {}, \"cache_store_elapsed_ns\": {}, \"optimized_fragment_cache_hits\": {}, \"optimized_fragment_cache_misses\": {}, \"optimized_fragment_final_artifact_hits\": {}, \"optimized_fragment_fast_path_direct_hits\": {}, \"optimized_fragment_fast_path_raw_hits\": {}, \"optimized_fragment_fast_path_peephole_hits\": {}, \"quote_wrap_elapsed_ns\": {}, \"fragment_assembly_elapsed_ns\": {}, \"raw_rewrite_elapsed_ns\": {}, \"peephole_elapsed_ns\": {}, \"peephole_linear_scan_elapsed_ns\": {}, \"peephole_primary_rewrite_elapsed_ns\": {}, \"peephole_primary_flow_elapsed_ns\": {}, \"peephole_primary_inline_elapsed_ns\": {}, \"peephole_primary_reuse_elapsed_ns\": {}, \"peephole_primary_loop_cleanup_elapsed_ns\": {}, \"peephole_primary_loop_dead_zero_elapsed_ns\": {}, \"peephole_primary_loop_normalize_elapsed_ns\": {}, \"peephole_primary_loop_hoist_elapsed_ns\": {}, \"peephole_primary_loop_repeat_to_for_elapsed_ns\": {}, \"peephole_primary_loop_tail_cleanup_elapsed_ns\": {}, \"peephole_primary_loop_guard_cleanup_elapsed_ns\": {}, \"peephole_primary_loop_helper_cleanup_elapsed_ns\": {}, \"peephole_primary_loop_exact_cleanup_elapsed_ns\": {}, \"peephole_primary_loop_exact_pre_elapsed_ns\": {}, \"peephole_primary_loop_exact_reuse_elapsed_ns\": {}, \"peephole_primary_loop_exact_reuse_prepare_elapsed_ns\": {}, \"peephole_primary_loop_exact_reuse_forward_elapsed_ns\": {}, \"peephole_primary_loop_exact_reuse_pure_call_elapsed_ns\": {}, \"peephole_primary_loop_exact_reuse_expr_elapsed_ns\": {}, \"peephole_primary_loop_exact_reuse_vector_alias_elapsed_ns\": {}, \"peephole_primary_loop_exact_reuse_rebind_elapsed_ns\": {}, \"peephole_primary_loop_exact_fixpoint_elapsed_ns\": {}, \"peephole_primary_loop_exact_fixpoint_prepare_elapsed_ns\": {}, \"peephole_primary_loop_exact_fixpoint_forward_elapsed_ns\": {}, \"peephole_primary_loop_exact_fixpoint_pure_call_elapsed_ns\": {}, \"peephole_primary_loop_exact_fixpoint_expr_elapsed_ns\": {}, \"peephole_primary_loop_exact_fixpoint_rebind_elapsed_ns\": {}, \"peephole_primary_loop_exact_fixpoint_rounds\": {}, \"peephole_primary_loop_exact_finalize_elapsed_ns\": {}, \"peephole_primary_loop_dead_temp_cleanup_elapsed_ns\": {}, \"peephole_secondary_rewrite_elapsed_ns\": {}, \"peephole_secondary_inline_elapsed_ns\": {}, \"peephole_secondary_inline_branch_hoist_elapsed_ns\": {}, \"peephole_secondary_inline_immediate_scalar_elapsed_ns\": {}, \"peephole_secondary_inline_named_index_elapsed_ns\": {}, \"peephole_secondary_inline_named_expr_elapsed_ns\": {}, \"peephole_secondary_inline_scalar_region_elapsed_ns\": {}, \"peephole_secondary_inline_immediate_index_elapsed_ns\": {}, \"peephole_secondary_inline_adjacent_dedup_elapsed_ns\": {}, \"peephole_secondary_exact_elapsed_ns\": {}, \"peephole_secondary_helper_cleanup_elapsed_ns\": {}, \"peephole_secondary_helper_wrapper_elapsed_ns\": {}, \"peephole_secondary_helper_metric_elapsed_ns\": {}, \"peephole_secondary_helper_alias_elapsed_ns\": {}, \"peephole_secondary_helper_simple_expr_elapsed_ns\": {}, \"peephole_secondary_helper_full_range_elapsed_ns\": {}, \"peephole_secondary_helper_named_copy_elapsed_ns\": {}, \"peephole_secondary_record_sroa_elapsed_ns\": {}, \"peephole_secondary_finalize_cleanup_elapsed_ns\": {}, \"peephole_secondary_finalize_bundle_elapsed_ns\": {}, \"peephole_secondary_finalize_dead_temp_elapsed_ns\": {}, \"peephole_secondary_finalize_dead_temp_facts_elapsed_ns\": {}, \"peephole_secondary_finalize_dead_temp_mark_elapsed_ns\": {}, \"peephole_secondary_finalize_dead_temp_reverse_elapsed_ns\": {}, \"peephole_secondary_finalize_dead_temp_compact_elapsed_ns\": {}, \"peephole_finalize_elapsed_ns\": {}, \"source_map_remap_elapsed_ns\": {}, \"quoted_wrapped_functions\": {}}}}},",
            self.emit.elapsed_ns,
            self.emit.emitted_functions,
            self.emit.cache_hits,
            self.emit.cache_misses,
            self.emit.breakdown.fragment_build_elapsed_ns,
            self.emit.breakdown.cache_load_elapsed_ns,
            self.emit.breakdown.emitter_elapsed_ns,
            self.emit.breakdown.cache_store_elapsed_ns,
            self.emit.breakdown.optimized_fragment_cache_hits,
            self.emit.breakdown.optimized_fragment_cache_misses,
            self.emit.breakdown.optimized_fragment_final_artifact_hits,
            self.emit.breakdown.optimized_fragment_fast_path_direct_hits,
            self.emit.breakdown.optimized_fragment_fast_path_raw_hits,
            self.emit
                .breakdown
                .optimized_fragment_fast_path_peephole_hits,
            self.emit.breakdown.quote_wrap_elapsed_ns,
            self.emit.breakdown.fragment_assembly_elapsed_ns,
            self.emit.breakdown.raw_rewrite_elapsed_ns,
            self.emit.breakdown.peephole_elapsed_ns,
            self.emit.breakdown.peephole_linear_scan_elapsed_ns,
            self.emit.breakdown.peephole_primary_rewrite_elapsed_ns,
            self.emit.breakdown.peephole_primary_flow_elapsed_ns,
            self.emit.breakdown.peephole_primary_inline_elapsed_ns,
            self.emit.breakdown.peephole_primary_reuse_elapsed_ns,
            self.emit.breakdown.peephole_primary_loop_cleanup_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_dead_zero_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_normalize_elapsed_ns,
            self.emit.breakdown.peephole_primary_loop_hoist_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_repeat_to_for_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_tail_cleanup_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_guard_cleanup_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_helper_cleanup_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_cleanup_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_pre_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_reuse_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_reuse_prepare_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_reuse_forward_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_reuse_pure_call_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_reuse_expr_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_reuse_vector_alias_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_reuse_rebind_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_fixpoint_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_fixpoint_prepare_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_fixpoint_forward_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_fixpoint_pure_call_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_fixpoint_expr_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_fixpoint_rebind_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_fixpoint_rounds,
            self.emit
                .breakdown
                .peephole_primary_loop_exact_finalize_elapsed_ns,
            self.emit
                .breakdown
                .peephole_primary_loop_dead_temp_cleanup_elapsed_ns,
            self.emit.breakdown.peephole_secondary_rewrite_elapsed_ns,
            self.emit.breakdown.peephole_secondary_inline_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_inline_branch_hoist_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_inline_immediate_scalar_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_inline_named_index_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_inline_named_expr_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_inline_scalar_region_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_inline_immediate_index_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_inline_adjacent_dedup_elapsed_ns,
            self.emit.breakdown.peephole_secondary_exact_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_helper_cleanup_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_helper_wrapper_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_helper_metric_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_helper_alias_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_helper_simple_expr_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_helper_full_range_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_helper_named_copy_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_record_sroa_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_finalize_cleanup_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_finalize_bundle_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_finalize_dead_temp_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_finalize_dead_temp_facts_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_finalize_dead_temp_mark_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_finalize_dead_temp_reverse_elapsed_ns,
            self.emit
                .breakdown
                .peephole_secondary_finalize_dead_temp_compact_elapsed_ns,
            self.emit.breakdown.peephole_finalize_elapsed_ns,
            self.emit.breakdown.source_map_remap_elapsed_ns,
            self.emit.breakdown.quoted_wrapped_functions
        );
        let _ = writeln!(
            out,
            "  \"runtime_injection\": {{\"elapsed_ns\": {}, \"inject_runtime\": {}}},",
            self.runtime_injection.elapsed_ns, self.runtime_injection.inject_runtime
        );
        let _ = writeln!(out, "  \"total_elapsed_ns\": {}", self.total_elapsed_ns);
        out.push('}');
        out
    }
}

pub fn json_escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

pub(crate) fn indent_json_block(raw: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    let mut out = String::new();
    for (idx, line) in raw.lines().enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        out.push_str(&indent);
        out.push_str(line);
    }
    out
}
