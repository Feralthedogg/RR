use crate::compiler::peephole::PeepholeProfile;
use std::time::Instant;

pub(crate) struct PipelineLinearScanOutcome {
    pub(crate) lines: Vec<String>,
    pub(crate) elapsed_ns: u128,
}

#[derive(Clone, Copy)]
pub(crate) struct PrimaryPipelineConfig {
    pub(crate) fast_dev: bool,
    pub(crate) direct_builtin_call_map: bool,
    pub(crate) preserve_all_defs: bool,
    pub(crate) aggressive_o3: bool,
    pub(crate) expression_controlled: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct SecondaryPipelineConfig {
    pub(crate) direct_builtin_call_map: bool,
    pub(crate) preserve_all_defs: bool,
    pub(crate) aggressive_o3: bool,
    pub(crate) expression_controlled: bool,
}

pub(crate) struct PrimaryPipelineOutcome {
    pub(crate) lines: Vec<String>,
    pub(crate) line_map: Vec<u32>,
    pub(crate) rewrite_elapsed_ns: u128,
    pub(crate) flow_elapsed_ns: u128,
    pub(crate) inline_elapsed_ns: u128,
    pub(crate) reuse_elapsed_ns: u128,
    pub(crate) loop_cleanup_elapsed_ns: u128,
    pub(crate) loop_dead_zero_elapsed_ns: u128,
    pub(crate) loop_normalize_elapsed_ns: u128,
    pub(crate) loop_hoist_elapsed_ns: u128,
    pub(crate) loop_repeat_to_for_elapsed_ns: u128,
    pub(crate) loop_tail_cleanup_elapsed_ns: u128,
    pub(crate) loop_guard_cleanup_elapsed_ns: u128,
    pub(crate) loop_helper_cleanup_elapsed_ns: u128,
    pub(crate) loop_exact_cleanup_elapsed_ns: u128,
    pub(crate) loop_exact_pre_elapsed_ns: u128,
    pub(crate) loop_exact_reuse_elapsed_ns: u128,
    pub(crate) loop_exact_reuse_prepare_elapsed_ns: u128,
    pub(crate) loop_exact_reuse_forward_elapsed_ns: u128,
    pub(crate) loop_exact_reuse_pure_call_elapsed_ns: u128,
    pub(crate) loop_exact_reuse_expr_elapsed_ns: u128,
    pub(crate) loop_exact_reuse_vector_alias_elapsed_ns: u128,
    pub(crate) loop_exact_reuse_rebind_elapsed_ns: u128,
    pub(crate) loop_exact_fixpoint_elapsed_ns: u128,
    pub(crate) loop_exact_fixpoint_prepare_elapsed_ns: u128,
    pub(crate) loop_exact_fixpoint_forward_elapsed_ns: u128,
    pub(crate) loop_exact_fixpoint_pure_call_elapsed_ns: u128,
    pub(crate) loop_exact_fixpoint_expr_elapsed_ns: u128,
    pub(crate) loop_exact_fixpoint_rebind_elapsed_ns: u128,
    pub(crate) loop_exact_fixpoint_rounds: usize,
    pub(crate) loop_exact_finalize_elapsed_ns: u128,
    pub(crate) loop_dead_temp_cleanup_elapsed_ns: u128,
}

impl PrimaryPipelineOutcome {
    pub(crate) fn apply_to_profile(&self, profile: &mut PeepholeProfile) {
        profile.primary_rewrite_elapsed_ns = self.rewrite_elapsed_ns;
        profile.primary_flow_elapsed_ns = self.flow_elapsed_ns;
        profile.primary_inline_elapsed_ns = self.inline_elapsed_ns;
        profile.primary_reuse_elapsed_ns = self.reuse_elapsed_ns;
        profile.primary_loop_cleanup_elapsed_ns = self.loop_cleanup_elapsed_ns;
        profile.primary_loop_dead_zero_elapsed_ns = self.loop_dead_zero_elapsed_ns;
        profile.primary_loop_normalize_elapsed_ns = self.loop_normalize_elapsed_ns;
        profile.primary_loop_hoist_elapsed_ns = self.loop_hoist_elapsed_ns;
        profile.primary_loop_repeat_to_for_elapsed_ns = self.loop_repeat_to_for_elapsed_ns;
        profile.primary_loop_tail_cleanup_elapsed_ns = self.loop_tail_cleanup_elapsed_ns;
        profile.primary_loop_guard_cleanup_elapsed_ns = self.loop_guard_cleanup_elapsed_ns;
        profile.primary_loop_helper_cleanup_elapsed_ns = self.loop_helper_cleanup_elapsed_ns;
        profile.primary_loop_exact_cleanup_elapsed_ns = self.loop_exact_cleanup_elapsed_ns;
        profile.primary_loop_exact_pre_elapsed_ns = self.loop_exact_pre_elapsed_ns;
        profile.primary_loop_exact_reuse_elapsed_ns = self.loop_exact_reuse_elapsed_ns;
        profile.primary_loop_exact_reuse_prepare_elapsed_ns =
            self.loop_exact_reuse_prepare_elapsed_ns;
        profile.primary_loop_exact_reuse_forward_elapsed_ns =
            self.loop_exact_reuse_forward_elapsed_ns;
        profile.primary_loop_exact_reuse_pure_call_elapsed_ns =
            self.loop_exact_reuse_pure_call_elapsed_ns;
        profile.primary_loop_exact_reuse_expr_elapsed_ns = self.loop_exact_reuse_expr_elapsed_ns;
        profile.primary_loop_exact_reuse_vector_alias_elapsed_ns =
            self.loop_exact_reuse_vector_alias_elapsed_ns;
        profile.primary_loop_exact_reuse_rebind_elapsed_ns =
            self.loop_exact_reuse_rebind_elapsed_ns;
        profile.primary_loop_exact_fixpoint_elapsed_ns = self.loop_exact_fixpoint_elapsed_ns;
        profile.primary_loop_exact_fixpoint_prepare_elapsed_ns =
            self.loop_exact_fixpoint_prepare_elapsed_ns;
        profile.primary_loop_exact_fixpoint_forward_elapsed_ns =
            self.loop_exact_fixpoint_forward_elapsed_ns;
        profile.primary_loop_exact_fixpoint_pure_call_elapsed_ns =
            self.loop_exact_fixpoint_pure_call_elapsed_ns;
        profile.primary_loop_exact_fixpoint_expr_elapsed_ns =
            self.loop_exact_fixpoint_expr_elapsed_ns;
        profile.primary_loop_exact_fixpoint_rebind_elapsed_ns =
            self.loop_exact_fixpoint_rebind_elapsed_ns;
        profile.primary_loop_exact_fixpoint_rounds = self.loop_exact_fixpoint_rounds;
        profile.primary_loop_exact_finalize_elapsed_ns = self.loop_exact_finalize_elapsed_ns;
        profile.primary_loop_dead_temp_cleanup_elapsed_ns = self.loop_dead_temp_cleanup_elapsed_ns;
    }
}

pub(crate) struct SecondaryPipelineOutcome {
    pub(crate) lines: Vec<String>,
    pub(crate) final_compact_map: Vec<u32>,
    pub(crate) rewrite_elapsed_ns: u128,
    pub(crate) inline_elapsed_ns: u128,
    pub(crate) inline_branch_hoist_elapsed_ns: u128,
    pub(crate) inline_immediate_scalar_elapsed_ns: u128,
    pub(crate) inline_named_index_elapsed_ns: u128,
    pub(crate) inline_named_expr_elapsed_ns: u128,
    pub(crate) inline_scalar_region_elapsed_ns: u128,
    pub(crate) inline_immediate_index_elapsed_ns: u128,
    pub(crate) inline_adjacent_dedup_elapsed_ns: u128,
    pub(crate) exact_elapsed_ns: u128,
    pub(crate) helper_cleanup_elapsed_ns: u128,
    pub(crate) helper_wrapper_elapsed_ns: u128,
    pub(crate) helper_metric_elapsed_ns: u128,
    pub(crate) helper_alias_elapsed_ns: u128,
    pub(crate) helper_simple_expr_elapsed_ns: u128,
    pub(crate) helper_full_range_elapsed_ns: u128,
    pub(crate) helper_named_copy_elapsed_ns: u128,
    pub(crate) record_sroa_elapsed_ns: u128,
    pub(crate) finalize_cleanup_elapsed_ns: u128,
    pub(crate) finalize_bundle_elapsed_ns: u128,
    pub(crate) finalize_dead_temp_elapsed_ns: u128,
    pub(crate) finalize_dead_temp_facts_elapsed_ns: u128,
    pub(crate) finalize_dead_temp_mark_elapsed_ns: u128,
    pub(crate) finalize_dead_temp_reverse_elapsed_ns: u128,
    pub(crate) finalize_dead_temp_compact_elapsed_ns: u128,
}

impl SecondaryPipelineOutcome {
    pub(crate) fn apply_to_profile(&self, profile: &mut PeepholeProfile) {
        profile.secondary_rewrite_elapsed_ns = self.rewrite_elapsed_ns;
        profile.secondary_inline_elapsed_ns = self.inline_elapsed_ns;
        profile.secondary_inline_branch_hoist_elapsed_ns = self.inline_branch_hoist_elapsed_ns;
        profile.secondary_inline_immediate_scalar_elapsed_ns =
            self.inline_immediate_scalar_elapsed_ns;
        profile.secondary_inline_named_index_elapsed_ns = self.inline_named_index_elapsed_ns;
        profile.secondary_inline_named_expr_elapsed_ns = self.inline_named_expr_elapsed_ns;
        profile.secondary_inline_scalar_region_elapsed_ns = self.inline_scalar_region_elapsed_ns;
        profile.secondary_inline_immediate_index_elapsed_ns =
            self.inline_immediate_index_elapsed_ns;
        profile.secondary_inline_adjacent_dedup_elapsed_ns = self.inline_adjacent_dedup_elapsed_ns;
        profile.secondary_exact_elapsed_ns = self.exact_elapsed_ns;
        profile.secondary_helper_cleanup_elapsed_ns = self.helper_cleanup_elapsed_ns;
        profile.secondary_helper_wrapper_elapsed_ns = self.helper_wrapper_elapsed_ns;
        profile.secondary_helper_metric_elapsed_ns = self.helper_metric_elapsed_ns;
        profile.secondary_helper_alias_elapsed_ns = self.helper_alias_elapsed_ns;
        profile.secondary_helper_simple_expr_elapsed_ns = self.helper_simple_expr_elapsed_ns;
        profile.secondary_helper_full_range_elapsed_ns = self.helper_full_range_elapsed_ns;
        profile.secondary_helper_named_copy_elapsed_ns = self.helper_named_copy_elapsed_ns;
        profile.secondary_record_sroa_elapsed_ns = self.record_sroa_elapsed_ns;
        profile.secondary_finalize_cleanup_elapsed_ns = self.finalize_cleanup_elapsed_ns;
        profile.secondary_finalize_bundle_elapsed_ns = self.finalize_bundle_elapsed_ns;
        profile.secondary_finalize_dead_temp_elapsed_ns = self.finalize_dead_temp_elapsed_ns;
        profile.secondary_finalize_dead_temp_facts_elapsed_ns =
            self.finalize_dead_temp_facts_elapsed_ns;
        profile.secondary_finalize_dead_temp_mark_elapsed_ns =
            self.finalize_dead_temp_mark_elapsed_ns;
        profile.secondary_finalize_dead_temp_reverse_elapsed_ns =
            self.finalize_dead_temp_reverse_elapsed_ns;
        profile.secondary_finalize_dead_temp_compact_elapsed_ns =
            self.finalize_dead_temp_compact_elapsed_ns;
    }
}

pub(crate) struct SecondaryInlineProfile {
    pub(crate) elapsed_ns: u128,
    pub(crate) branch_hoist_elapsed_ns: u128,
    pub(crate) immediate_scalar_elapsed_ns: u128,
    pub(crate) named_index_elapsed_ns: u128,
    pub(crate) named_expr_elapsed_ns: u128,
    pub(crate) scalar_region_elapsed_ns: u128,
    pub(crate) immediate_index_elapsed_ns: u128,
    pub(crate) adjacent_dedup_elapsed_ns: u128,
}

pub(crate) struct SecondaryHelperProfile {
    pub(crate) cleanup_elapsed_ns: u128,
    pub(crate) wrapper_elapsed_ns: u128,
    pub(crate) metric_elapsed_ns: u128,
    pub(crate) alias_elapsed_ns: u128,
    pub(crate) simple_expr_elapsed_ns: u128,
    pub(crate) full_range_elapsed_ns: u128,
    pub(crate) named_copy_elapsed_ns: u128,
    pub(crate) record_sroa_elapsed_ns: u128,
}

pub(crate) fn build_primary_pipeline_outcome(
    started: Instant,
    flow_elapsed_ns: u128,
    inline_elapsed_ns: u128,
    reuse_elapsed_ns: u128,
    loop_cleanup: PrimaryLoopCleanupStageOutcome,
) -> PrimaryPipelineOutcome {
    let PrimaryLoopCleanupStageOutcome {
        lines,
        line_map,
        elapsed_ns,
        dead_zero_elapsed_ns,
        normalize_elapsed_ns,
        hoist_elapsed_ns,
        repeat_to_for_elapsed_ns,
        tail_cleanup_elapsed_ns,
        guard_cleanup_elapsed_ns,
        helper_cleanup_elapsed_ns,
        exact_cleanup_elapsed_ns,
        exact_pre_elapsed_ns,
        exact_reuse_elapsed_ns,
        exact_reuse_prepare_elapsed_ns,
        exact_reuse_forward_elapsed_ns,
        exact_reuse_pure_call_elapsed_ns,
        exact_reuse_expr_elapsed_ns,
        exact_reuse_vector_alias_elapsed_ns,
        exact_reuse_rebind_elapsed_ns,
        exact_fixpoint_elapsed_ns,
        exact_fixpoint_prepare_elapsed_ns,
        exact_fixpoint_forward_elapsed_ns,
        exact_fixpoint_pure_call_elapsed_ns,
        exact_fixpoint_expr_elapsed_ns,
        exact_fixpoint_rebind_elapsed_ns,
        exact_fixpoint_rounds,
        exact_finalize_elapsed_ns,
        dead_temp_cleanup_elapsed_ns,
    } = loop_cleanup;
    PrimaryPipelineOutcome {
        lines,
        line_map,
        rewrite_elapsed_ns: started.elapsed().as_nanos(),
        flow_elapsed_ns,
        inline_elapsed_ns,
        reuse_elapsed_ns,
        loop_cleanup_elapsed_ns: elapsed_ns,
        loop_dead_zero_elapsed_ns: dead_zero_elapsed_ns,
        loop_normalize_elapsed_ns: normalize_elapsed_ns,
        loop_hoist_elapsed_ns: hoist_elapsed_ns,
        loop_repeat_to_for_elapsed_ns: repeat_to_for_elapsed_ns,
        loop_tail_cleanup_elapsed_ns: tail_cleanup_elapsed_ns,
        loop_guard_cleanup_elapsed_ns: guard_cleanup_elapsed_ns,
        loop_helper_cleanup_elapsed_ns: helper_cleanup_elapsed_ns,
        loop_exact_cleanup_elapsed_ns: exact_cleanup_elapsed_ns,
        loop_exact_pre_elapsed_ns: exact_pre_elapsed_ns,
        loop_exact_reuse_elapsed_ns: exact_reuse_elapsed_ns,
        loop_exact_reuse_prepare_elapsed_ns: exact_reuse_prepare_elapsed_ns,
        loop_exact_reuse_forward_elapsed_ns: exact_reuse_forward_elapsed_ns,
        loop_exact_reuse_pure_call_elapsed_ns: exact_reuse_pure_call_elapsed_ns,
        loop_exact_reuse_expr_elapsed_ns: exact_reuse_expr_elapsed_ns,
        loop_exact_reuse_vector_alias_elapsed_ns: exact_reuse_vector_alias_elapsed_ns,
        loop_exact_reuse_rebind_elapsed_ns: exact_reuse_rebind_elapsed_ns,
        loop_exact_fixpoint_elapsed_ns: exact_fixpoint_elapsed_ns,
        loop_exact_fixpoint_prepare_elapsed_ns: exact_fixpoint_prepare_elapsed_ns,
        loop_exact_fixpoint_forward_elapsed_ns: exact_fixpoint_forward_elapsed_ns,
        loop_exact_fixpoint_pure_call_elapsed_ns: exact_fixpoint_pure_call_elapsed_ns,
        loop_exact_fixpoint_expr_elapsed_ns: exact_fixpoint_expr_elapsed_ns,
        loop_exact_fixpoint_rebind_elapsed_ns: exact_fixpoint_rebind_elapsed_ns,
        loop_exact_fixpoint_rounds: exact_fixpoint_rounds,
        loop_exact_finalize_elapsed_ns: exact_finalize_elapsed_ns,
        loop_dead_temp_cleanup_elapsed_ns: dead_temp_cleanup_elapsed_ns,
    }
}

pub(crate) fn build_secondary_pipeline_outcome(
    started: Instant,
    inline: SecondaryInlineProfile,
    exact_elapsed_ns: u128,
    helper: SecondaryHelperProfile,
    finalize: SecondaryFinalizeCleanupStageOutcome,
) -> SecondaryPipelineOutcome {
    SecondaryPipelineOutcome {
        lines: finalize.lines,
        final_compact_map: finalize.final_compact_map,
        rewrite_elapsed_ns: started.elapsed().as_nanos(),
        inline_elapsed_ns: inline.elapsed_ns,
        inline_branch_hoist_elapsed_ns: inline.branch_hoist_elapsed_ns,
        inline_immediate_scalar_elapsed_ns: inline.immediate_scalar_elapsed_ns,
        inline_named_index_elapsed_ns: inline.named_index_elapsed_ns,
        inline_named_expr_elapsed_ns: inline.named_expr_elapsed_ns,
        inline_scalar_region_elapsed_ns: inline.scalar_region_elapsed_ns,
        inline_immediate_index_elapsed_ns: inline.immediate_index_elapsed_ns,
        inline_adjacent_dedup_elapsed_ns: inline.adjacent_dedup_elapsed_ns,
        exact_elapsed_ns,
        helper_cleanup_elapsed_ns: helper.cleanup_elapsed_ns,
        helper_wrapper_elapsed_ns: helper.wrapper_elapsed_ns,
        helper_metric_elapsed_ns: helper.metric_elapsed_ns,
        helper_alias_elapsed_ns: helper.alias_elapsed_ns,
        helper_simple_expr_elapsed_ns: helper.simple_expr_elapsed_ns,
        helper_full_range_elapsed_ns: helper.full_range_elapsed_ns,
        helper_named_copy_elapsed_ns: helper.named_copy_elapsed_ns,
        record_sroa_elapsed_ns: helper.record_sroa_elapsed_ns,
        finalize_cleanup_elapsed_ns: finalize.elapsed_ns,
        finalize_bundle_elapsed_ns: finalize.bundle_elapsed_ns,
        finalize_dead_temp_elapsed_ns: finalize.dead_temp_elapsed_ns,
        finalize_dead_temp_facts_elapsed_ns: finalize.dead_temp_facts_elapsed_ns,
        finalize_dead_temp_mark_elapsed_ns: finalize.dead_temp_mark_elapsed_ns,
        finalize_dead_temp_reverse_elapsed_ns: finalize.dead_temp_reverse_elapsed_ns,
        finalize_dead_temp_compact_elapsed_ns: finalize.dead_temp_compact_elapsed_ns,
    }
}

pub(crate) fn build_pipeline_profile(
    linear_scan_elapsed_ns: u128,
    primary: &PrimaryPipelineOutcome,
    secondary: Option<&SecondaryPipelineOutcome>,
    finalize_elapsed_ns: u128,
) -> PeepholeProfile {
    let mut profile = PeepholeProfile {
        linear_scan_elapsed_ns,
        finalize_elapsed_ns,
        ..PeepholeProfile::default()
    };
    primary.apply_to_profile(&mut profile);
    if let Some(secondary) = secondary {
        secondary.apply_to_profile(&mut profile);
    }
    profile
}

#[derive(Default)]
pub(crate) struct ExactFixpointProfile {
    pub(crate) prepare_elapsed_ns: u128,
    pub(crate) forward_elapsed_ns: u128,
    pub(crate) pure_call_elapsed_ns: u128,
    pub(crate) expr_elapsed_ns: u128,
    pub(crate) rebind_elapsed_ns: u128,
    pub(crate) rounds: usize,
}

pub(crate) struct PeepholeLineStageOutcome {
    pub(crate) lines: Vec<String>,
    pub(crate) elapsed_ns: u128,
}

pub(crate) struct PrimaryLoopCleanupStageOutcome {
    pub(crate) lines: Vec<String>,
    pub(crate) line_map: Vec<u32>,
    pub(crate) elapsed_ns: u128,
    pub(crate) dead_zero_elapsed_ns: u128,
    pub(crate) normalize_elapsed_ns: u128,
    pub(crate) hoist_elapsed_ns: u128,
    pub(crate) repeat_to_for_elapsed_ns: u128,
    pub(crate) tail_cleanup_elapsed_ns: u128,
    pub(crate) guard_cleanup_elapsed_ns: u128,
    pub(crate) helper_cleanup_elapsed_ns: u128,
    pub(crate) exact_cleanup_elapsed_ns: u128,
    pub(crate) exact_pre_elapsed_ns: u128,
    pub(crate) exact_reuse_elapsed_ns: u128,
    pub(crate) exact_reuse_prepare_elapsed_ns: u128,
    pub(crate) exact_reuse_forward_elapsed_ns: u128,
    pub(crate) exact_reuse_pure_call_elapsed_ns: u128,
    pub(crate) exact_reuse_expr_elapsed_ns: u128,
    pub(crate) exact_reuse_vector_alias_elapsed_ns: u128,
    pub(crate) exact_reuse_rebind_elapsed_ns: u128,
    pub(crate) exact_fixpoint_elapsed_ns: u128,
    pub(crate) exact_fixpoint_prepare_elapsed_ns: u128,
    pub(crate) exact_fixpoint_forward_elapsed_ns: u128,
    pub(crate) exact_fixpoint_pure_call_elapsed_ns: u128,
    pub(crate) exact_fixpoint_expr_elapsed_ns: u128,
    pub(crate) exact_fixpoint_rebind_elapsed_ns: u128,
    pub(crate) exact_fixpoint_rounds: usize,
    pub(crate) exact_finalize_elapsed_ns: u128,
    pub(crate) dead_temp_cleanup_elapsed_ns: u128,
}

pub(crate) struct SecondaryInlineStageOutcome {
    pub(crate) lines: Vec<String>,
    pub(crate) elapsed_ns: u128,
    pub(crate) branch_hoist_elapsed_ns: u128,
    pub(crate) immediate_scalar_elapsed_ns: u128,
    pub(crate) named_index_elapsed_ns: u128,
    pub(crate) named_expr_elapsed_ns: u128,
    pub(crate) scalar_region_elapsed_ns: u128,
    pub(crate) immediate_index_elapsed_ns: u128,
    pub(crate) adjacent_dedup_elapsed_ns: u128,
}

pub(crate) struct SecondaryHelperCleanupStageOutcome {
    pub(crate) lines: Vec<String>,
    pub(crate) elapsed_ns: u128,
    pub(crate) wrapper_elapsed_ns: u128,
    pub(crate) metric_elapsed_ns: u128,
    pub(crate) alias_elapsed_ns: u128,
    pub(crate) simple_expr_elapsed_ns: u128,
    pub(crate) full_range_elapsed_ns: u128,
    pub(crate) named_copy_elapsed_ns: u128,
    pub(crate) record_sroa_elapsed_ns: u128,
}

pub(crate) struct SecondaryFinalizeCleanupStageOutcome {
    pub(crate) lines: Vec<String>,
    pub(crate) final_compact_map: Vec<u32>,
    pub(crate) elapsed_ns: u128,
    pub(crate) bundle_elapsed_ns: u128,
    pub(crate) dead_temp_elapsed_ns: u128,
    pub(crate) dead_temp_facts_elapsed_ns: u128,
    pub(crate) dead_temp_mark_elapsed_ns: u128,
    pub(crate) dead_temp_reverse_elapsed_ns: u128,
    pub(crate) dead_temp_compact_elapsed_ns: u128,
}

pub(crate) struct LoopCanonicalizationSubstageOutcome {
    pub(crate) lines: Vec<String>,
    pub(crate) dead_zero_elapsed_ns: u128,
    pub(crate) normalize_elapsed_ns: u128,
    pub(crate) hoist_elapsed_ns: u128,
    pub(crate) repeat_to_for_elapsed_ns: u128,
}

pub(crate) struct PrimaryExactPreSubstageOutcome {
    pub(crate) lines: Vec<String>,
    pub(crate) pre_elapsed_ns: u128,
    pub(crate) reuse_prepare_elapsed_ns: u128,
}

pub(crate) struct PrimaryExactReuseSubstageOutcome {
    pub(crate) lines: Vec<String>,
    pub(crate) elapsed_ns: u128,
    pub(crate) prepare_elapsed_ns: u128,
    pub(crate) forward_elapsed_ns: u128,
    pub(crate) pure_call_elapsed_ns: u128,
    pub(crate) expr_elapsed_ns: u128,
    pub(crate) vector_alias_elapsed_ns: u128,
    pub(crate) rebind_elapsed_ns: u128,
}
