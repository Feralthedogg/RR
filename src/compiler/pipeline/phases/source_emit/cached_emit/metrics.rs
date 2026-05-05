use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct OptimizedAssemblyHits {
    pub(crate) final_artifact: usize,
    pub(crate) direct: usize,
    pub(crate) raw: usize,
    pub(crate) peephole: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct EmitStageTimings {
    pub(crate) fragment_assembly_elapsed_ns: u128,
    pub(crate) raw_rewrite_elapsed_ns: u128,
    pub(crate) peephole_elapsed_ns: u128,
    pub(crate) source_map_remap_elapsed_ns: u128,
}

pub(crate) struct EmitMetricContext<'a> {
    pub(crate) emitted_functions: usize,
    pub(crate) cache_hits: usize,
    pub(crate) cache_misses: usize,
    pub(crate) fragment_build_elapsed_ns: u128,
    pub(crate) cache_load_elapsed_ns: &'a Arc<AtomicU64>,
    pub(crate) emitter_elapsed_ns: &'a Arc<AtomicU64>,
    pub(crate) cache_store_elapsed_ns: &'a Arc<AtomicU64>,
    pub(crate) optimized_fragment_cache_hits: usize,
    pub(crate) optimized_fragment_cache_misses: usize,
    pub(crate) quote_wrap_elapsed_ns: &'a Arc<AtomicU64>,
    pub(crate) quoted_wrapped_functions: &'a Arc<AtomicUsize>,
}

impl EmitMetricContext<'_> {
    pub(crate) fn build(
        &self,
        elapsed_ns: u128,
        assembly_hits: OptimizedAssemblyHits,
        timings: EmitStageTimings,
        peephole_profile: Option<&crate::compiler::peephole::PeepholeProfile>,
    ) -> EmitMetrics {
        let peephole_profile = peephole_profile.cloned().unwrap_or_default();
        EmitMetrics {
            elapsed_ns,
            emitted_functions: self.emitted_functions,
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            breakdown: EmitBreakdownProfile {
                fragment_build_elapsed_ns: self.fragment_build_elapsed_ns,
                cache_load_elapsed_ns: self.cache_load_elapsed_ns.load(Ordering::Relaxed) as u128,
                emitter_elapsed_ns: self.emitter_elapsed_ns.load(Ordering::Relaxed) as u128,
                cache_store_elapsed_ns: self.cache_store_elapsed_ns.load(Ordering::Relaxed) as u128,
                optimized_fragment_cache_hits: self.optimized_fragment_cache_hits,
                optimized_fragment_cache_misses: self.optimized_fragment_cache_misses,
                optimized_fragment_final_artifact_hits: assembly_hits.final_artifact,
                optimized_fragment_fast_path_direct_hits: assembly_hits.direct,
                optimized_fragment_fast_path_raw_hits: assembly_hits.raw,
                optimized_fragment_fast_path_peephole_hits: assembly_hits.peephole,
                quote_wrap_elapsed_ns: self.quote_wrap_elapsed_ns.load(Ordering::Relaxed) as u128,
                fragment_assembly_elapsed_ns: timings.fragment_assembly_elapsed_ns,
                raw_rewrite_elapsed_ns: timings.raw_rewrite_elapsed_ns,
                peephole_elapsed_ns: timings.peephole_elapsed_ns,
                peephole_linear_scan_elapsed_ns: peephole_profile.linear_scan_elapsed_ns,
                peephole_primary_rewrite_elapsed_ns: peephole_profile.primary_rewrite_elapsed_ns,
                peephole_primary_flow_elapsed_ns: peephole_profile.primary_flow_elapsed_ns,
                peephole_primary_inline_elapsed_ns: peephole_profile.primary_inline_elapsed_ns,
                peephole_primary_reuse_elapsed_ns: peephole_profile.primary_reuse_elapsed_ns,
                peephole_primary_loop_cleanup_elapsed_ns: peephole_profile
                    .primary_loop_cleanup_elapsed_ns,
                peephole_primary_loop_dead_zero_elapsed_ns: peephole_profile
                    .primary_loop_dead_zero_elapsed_ns,
                peephole_primary_loop_normalize_elapsed_ns: peephole_profile
                    .primary_loop_normalize_elapsed_ns,
                peephole_primary_loop_hoist_elapsed_ns: peephole_profile
                    .primary_loop_hoist_elapsed_ns,
                peephole_primary_loop_repeat_to_for_elapsed_ns: peephole_profile
                    .primary_loop_repeat_to_for_elapsed_ns,
                peephole_primary_loop_tail_cleanup_elapsed_ns: peephole_profile
                    .primary_loop_tail_cleanup_elapsed_ns,
                peephole_primary_loop_guard_cleanup_elapsed_ns: peephole_profile
                    .primary_loop_guard_cleanup_elapsed_ns,
                peephole_primary_loop_helper_cleanup_elapsed_ns: peephole_profile
                    .primary_loop_helper_cleanup_elapsed_ns,
                peephole_primary_loop_exact_cleanup_elapsed_ns: peephole_profile
                    .primary_loop_exact_cleanup_elapsed_ns,
                peephole_primary_loop_exact_pre_elapsed_ns: peephole_profile
                    .primary_loop_exact_pre_elapsed_ns,
                peephole_primary_loop_exact_reuse_elapsed_ns: peephole_profile
                    .primary_loop_exact_reuse_elapsed_ns,
                peephole_primary_loop_exact_reuse_prepare_elapsed_ns: peephole_profile
                    .primary_loop_exact_reuse_prepare_elapsed_ns,
                peephole_primary_loop_exact_reuse_forward_elapsed_ns: peephole_profile
                    .primary_loop_exact_reuse_forward_elapsed_ns,
                peephole_primary_loop_exact_reuse_pure_call_elapsed_ns: peephole_profile
                    .primary_loop_exact_reuse_pure_call_elapsed_ns,
                peephole_primary_loop_exact_reuse_expr_elapsed_ns: peephole_profile
                    .primary_loop_exact_reuse_expr_elapsed_ns,
                peephole_primary_loop_exact_reuse_vector_alias_elapsed_ns: peephole_profile
                    .primary_loop_exact_reuse_vector_alias_elapsed_ns,
                peephole_primary_loop_exact_reuse_rebind_elapsed_ns: peephole_profile
                    .primary_loop_exact_reuse_rebind_elapsed_ns,
                peephole_primary_loop_exact_fixpoint_elapsed_ns: peephole_profile
                    .primary_loop_exact_fixpoint_elapsed_ns,
                peephole_primary_loop_exact_fixpoint_prepare_elapsed_ns: peephole_profile
                    .primary_loop_exact_fixpoint_prepare_elapsed_ns,
                peephole_primary_loop_exact_fixpoint_forward_elapsed_ns: peephole_profile
                    .primary_loop_exact_fixpoint_forward_elapsed_ns,
                peephole_primary_loop_exact_fixpoint_pure_call_elapsed_ns: peephole_profile
                    .primary_loop_exact_fixpoint_pure_call_elapsed_ns,
                peephole_primary_loop_exact_fixpoint_expr_elapsed_ns: peephole_profile
                    .primary_loop_exact_fixpoint_expr_elapsed_ns,
                peephole_primary_loop_exact_fixpoint_rebind_elapsed_ns: peephole_profile
                    .primary_loop_exact_fixpoint_rebind_elapsed_ns,
                peephole_primary_loop_exact_fixpoint_rounds: peephole_profile
                    .primary_loop_exact_fixpoint_rounds,
                peephole_primary_loop_exact_finalize_elapsed_ns: peephole_profile
                    .primary_loop_exact_finalize_elapsed_ns,
                peephole_primary_loop_dead_temp_cleanup_elapsed_ns: peephole_profile
                    .primary_loop_dead_temp_cleanup_elapsed_ns,
                peephole_secondary_rewrite_elapsed_ns: peephole_profile
                    .secondary_rewrite_elapsed_ns,
                peephole_secondary_inline_elapsed_ns: peephole_profile.secondary_inline_elapsed_ns,
                peephole_secondary_inline_branch_hoist_elapsed_ns: peephole_profile
                    .secondary_inline_branch_hoist_elapsed_ns,
                peephole_secondary_inline_immediate_scalar_elapsed_ns: peephole_profile
                    .secondary_inline_immediate_scalar_elapsed_ns,
                peephole_secondary_inline_named_index_elapsed_ns: peephole_profile
                    .secondary_inline_named_index_elapsed_ns,
                peephole_secondary_inline_named_expr_elapsed_ns: peephole_profile
                    .secondary_inline_named_expr_elapsed_ns,
                peephole_secondary_inline_scalar_region_elapsed_ns: peephole_profile
                    .secondary_inline_scalar_region_elapsed_ns,
                peephole_secondary_inline_immediate_index_elapsed_ns: peephole_profile
                    .secondary_inline_immediate_index_elapsed_ns,
                peephole_secondary_inline_adjacent_dedup_elapsed_ns: peephole_profile
                    .secondary_inline_adjacent_dedup_elapsed_ns,
                peephole_secondary_exact_elapsed_ns: peephole_profile.secondary_exact_elapsed_ns,
                peephole_secondary_helper_cleanup_elapsed_ns: peephole_profile
                    .secondary_helper_cleanup_elapsed_ns,
                peephole_secondary_helper_wrapper_elapsed_ns: peephole_profile
                    .secondary_helper_wrapper_elapsed_ns,
                peephole_secondary_helper_metric_elapsed_ns: peephole_profile
                    .secondary_helper_metric_elapsed_ns,
                peephole_secondary_helper_alias_elapsed_ns: peephole_profile
                    .secondary_helper_alias_elapsed_ns,
                peephole_secondary_helper_simple_expr_elapsed_ns: peephole_profile
                    .secondary_helper_simple_expr_elapsed_ns,
                peephole_secondary_helper_full_range_elapsed_ns: peephole_profile
                    .secondary_helper_full_range_elapsed_ns,
                peephole_secondary_helper_named_copy_elapsed_ns: peephole_profile
                    .secondary_helper_named_copy_elapsed_ns,
                peephole_secondary_record_sroa_elapsed_ns: peephole_profile
                    .secondary_record_sroa_elapsed_ns,
                peephole_secondary_finalize_cleanup_elapsed_ns: peephole_profile
                    .secondary_finalize_cleanup_elapsed_ns,
                peephole_secondary_finalize_bundle_elapsed_ns: peephole_profile
                    .secondary_finalize_bundle_elapsed_ns,
                peephole_secondary_finalize_dead_temp_elapsed_ns: peephole_profile
                    .secondary_finalize_dead_temp_elapsed_ns,
                peephole_secondary_finalize_dead_temp_facts_elapsed_ns: peephole_profile
                    .secondary_finalize_dead_temp_facts_elapsed_ns,
                peephole_secondary_finalize_dead_temp_mark_elapsed_ns: peephole_profile
                    .secondary_finalize_dead_temp_mark_elapsed_ns,
                peephole_secondary_finalize_dead_temp_reverse_elapsed_ns: peephole_profile
                    .secondary_finalize_dead_temp_reverse_elapsed_ns,
                peephole_secondary_finalize_dead_temp_compact_elapsed_ns: peephole_profile
                    .secondary_finalize_dead_temp_compact_elapsed_ns,
                peephole_finalize_elapsed_ns: peephole_profile.finalize_elapsed_ns,
                source_map_remap_elapsed_ns: timings.source_map_remap_elapsed_ns,
                quoted_wrapped_functions: self.quoted_wrapped_functions.load(Ordering::Relaxed),
            },
        }
    }
}
