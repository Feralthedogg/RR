use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RawRewriteStage {
    Fragment,
    FullProgram,
    PostAssemblyFinalize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RawRewritePassId {
    TrivialClamp,
    FloorFedParticleClampPair,
    TrivialDotProductWrappers,
    MountainDxTemp,
    Sym287MeltRateBranch,
    GrayScottClampPair,
    StripUnusedHelperParams,
    HelperExprReuse,
    DotProductHelperCalls,
    Sym119HelperCalls,
    TrivialFillHelperCalls,
    IdenticalZeroFillAlias,
    DuplicateSym183Calls,
    ParticleStateRebinds,
    StaleParticleStateReplays,
    AdjacentDirNeighborRows,
    ExactSafeLoopIndexWrite,
    CgLoopCarriedUpdates,
    BufferSwapAfterTempCopy,
    WenoFullRangeGatherReplay,
    DeadWenoTopologySeed,
    MissingCseRangeAliases,
    DeadZeroLoopSeeds,
    PruneUnreachableHelpers,
    NormalizePostAssemblyWhitespace,
}

pub(crate) struct RawRewriteContext {
    pub(crate) preserve_all_defs: bool,
}

impl RawRewriteContext {
    pub(crate) const fn new(output_opts: CompileOutputOptions) -> Self {
        Self {
            preserve_all_defs: output_opts.preserve_all_defs,
        }
    }
}

pub(crate) type RawRewriteEnabledFn = fn(&RawRewriteContext) -> bool;
pub(crate) type RawRewriteRunner = fn(&str) -> String;

#[derive(Clone, Copy)]
pub(crate) struct RawRewritePassSpec {
    pub(crate) id: RawRewritePassId,
    pub(crate) stage: RawRewriteStage,
    pub(crate) proof_key: &'static str,
    pub(crate) enabled: RawRewriteEnabledFn,
    pub(crate) run: RawRewriteRunner,
}

pub(crate) struct RawRewritePassManager {
    pub(crate) stage: RawRewriteStage,
    pub(crate) passes: &'static [RawRewritePassSpec],
}

impl RawRewritePassManager {
    pub(crate) const fn new(stage: RawRewriteStage, passes: &'static [RawRewritePassSpec]) -> Self {
        Self { stage, passes }
    }

    pub(crate) fn run(&self, mut output: String, ctx: &RawRewriteContext) -> String {
        for spec in self.passes {
            if spec.stage != self.stage || !(spec.enabled)(ctx) {
                continue;
            }
            let _metadata = (spec.id, spec.proof_key);
            output = (spec.run)(&output);
        }
        output
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HermesStageId {
    FragmentRawRewrite,
    FullProgramRawRewrite,
    PostAssemblyFinalize,
}

#[derive(Clone, Copy)]
pub(crate) struct HermesStageSpec {
    pub(crate) id: HermesStageId,
    pub(crate) order: u8,
    pub(crate) raw_stage: RawRewriteStage,
    pub(crate) proof_key: &'static str,
    pub(crate) passes: &'static [RawRewritePassSpec],
}

pub(crate) struct HermesPassManager {
    pub(crate) spec: &'static HermesStageSpec,
}

pub(crate) fn always_enabled(_: &RawRewriteContext) -> bool {
    true
}

pub(crate) fn prune_enabled(ctx: &RawRewriteContext) -> bool {
    !ctx.preserve_all_defs
}

pub(crate) const fn raw_pass(
    id: RawRewritePassId,
    stage: RawRewriteStage,
    proof_key: &'static str,
    enabled: RawRewriteEnabledFn,
    run: RawRewriteRunner,
) -> RawRewritePassSpec {
    RawRewritePassSpec {
        id,
        stage,
        proof_key,
        enabled,
        run,
    }
}

pub(crate) fn normalize_post_assembly_whitespace(output: &str) -> String {
    let mut kept = Vec::new();
    let mut prev_blank = false;
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed == "# rr-cse-pruned" {
            continue;
        }
        if trimmed.is_empty() {
            if prev_blank {
                continue;
            }
            prev_blank = true;
            kept.push(String::new());
            continue;
        }
        prev_blank = false;
        kept.push(line.to_string());
    }
    let mut rewritten = kept.join("\n");
    if !rewritten.is_empty() {
        rewritten.push('\n');
    }
    rewritten
}

pub(crate) const FRAGMENT_RAW_REWRITE_PASSES: &[RawRewritePassSpec] = &[
    raw_pass(
        RawRewritePassId::TrivialClamp,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_trivial_clamp",
        always_enabled,
        rewrite_trivial_clamp_helper_calls_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::FloorFedParticleClampPair,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_floor_fed_particle_clamp_pair_1",
        always_enabled,
        collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::TrivialDotProductWrappers,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_trivial_dot_product_wrappers_1",
        always_enabled,
        collapse_trivial_dot_product_wrappers_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::MountainDxTemp,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_mountain_dx_temp_1",
        always_enabled,
        rewrite_mountain_dx_temp_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::Sym287MeltRateBranch,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_sym287_melt_rate_branch_1",
        always_enabled,
        collapse_sym287_melt_rate_branch_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::FloorFedParticleClampPair,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_floor_fed_particle_clamp_pair_2",
        always_enabled,
        collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::GrayScottClampPair,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_gray_scott_clamp_pair_1",
        always_enabled,
        collapse_gray_scott_clamp_pair_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::TrivialDotProductWrappers,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_trivial_dot_product_wrappers_2",
        always_enabled,
        collapse_trivial_dot_product_wrappers_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::Sym287MeltRateBranch,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_sym287_melt_rate_branch_2",
        always_enabled,
        collapse_sym287_melt_rate_branch_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::FloorFedParticleClampPair,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_floor_fed_particle_clamp_pair_3",
        always_enabled,
        collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::GrayScottClampPair,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_gray_scott_clamp_pair_2",
        always_enabled,
        collapse_gray_scott_clamp_pair_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::HelperExprReuse,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_helper_expr_reuse",
        always_enabled,
        rewrite_helper_expr_reuse_calls_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::DotProductHelperCalls,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_dot_product_helper_calls",
        always_enabled,
        rewrite_dot_product_helper_calls_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::Sym119HelperCalls,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_sym119_helper_calls",
        always_enabled,
        rewrite_sym119_helper_calls_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::TrivialFillHelperCalls,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_trivial_fill_helper_calls",
        always_enabled,
        rewrite_trivial_fill_helper_calls_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::IdenticalZeroFillAlias,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_identical_zero_fill_alias",
        always_enabled,
        rewrite_identical_zero_fill_pairs_to_aliases_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::DuplicateSym183Calls,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_duplicate_sym183_calls",
        always_enabled,
        rewrite_duplicate_sym183_calls_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::ParticleStateRebinds,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_particle_state_rebinds",
        always_enabled,
        restore_particle_state_rebinds_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::StaleParticleStateReplays,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_stale_particle_state_replays",
        always_enabled,
        strip_stale_particle_state_replays_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::AdjacentDirNeighborRows,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_adjacent_dir_neighbor_rows",
        always_enabled,
        collapse_adjacent_dir_neighbor_row_branches_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::ExactSafeLoopIndexWrite,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_exact_safe_loop_index_write_1",
        always_enabled,
        rewrite_exact_safe_loop_index_write_calls_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::FloorFedParticleClampPair,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_floor_fed_particle_clamp_pair_4",
        always_enabled,
        collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::Sym287MeltRateBranch,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_sym287_melt_rate_branch_3",
        always_enabled,
        collapse_sym287_melt_rate_branch_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::CgLoopCarriedUpdates,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_cg_loop_carried_updates_1",
        always_enabled,
        restore_cg_loop_carried_updates_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::BufferSwapAfterTempCopy,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_buffer_swap_after_temp_copy",
        always_enabled,
        restore_buffer_swaps_after_temp_copy_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::WenoFullRangeGatherReplay,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_weno_full_range_gather_replay_1",
        always_enabled,
        collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::ExactSafeLoopIndexWrite,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_exact_safe_loop_index_write_2",
        always_enabled,
        rewrite_exact_safe_loop_index_write_calls_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::MountainDxTemp,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_mountain_dx_temp_2",
        always_enabled,
        rewrite_mountain_dx_temp_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::WenoFullRangeGatherReplay,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_weno_full_range_gather_replay_2",
        always_enabled,
        collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::DeadWenoTopologySeed,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_dead_weno_topology_seed",
        always_enabled,
        strip_dead_weno_topology_seed_i_before_direct_adj_gather_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::CgLoopCarriedUpdates,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_cg_loop_carried_updates_2",
        always_enabled,
        restore_cg_loop_carried_updates_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::MissingCseRangeAliases,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_missing_cse_range_aliases",
        always_enabled,
        repair_missing_cse_range_aliases_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::DeadZeroLoopSeeds,
        RawRewriteStage::Fragment,
        "RawRewriteSoundness.fragment_dead_zero_loop_seeds",
        always_enabled,
        strip_dead_zero_loop_seeds_before_for_in_raw_emitted_r,
    ),
];

pub(crate) const FULL_PROGRAM_RAW_REWRITE_PASSES: &[RawRewritePassSpec] = &[
    raw_pass(
        RawRewritePassId::TrivialClamp,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_trivial_clamp",
        always_enabled,
        rewrite_trivial_clamp_helper_calls_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::FloorFedParticleClampPair,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_floor_fed_particle_clamp_pair_1",
        always_enabled,
        collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::TrivialDotProductWrappers,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_trivial_dot_product_wrappers_1",
        always_enabled,
        collapse_trivial_dot_product_wrappers_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::MountainDxTemp,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_mountain_dx_temp_1",
        always_enabled,
        rewrite_mountain_dx_temp_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::Sym287MeltRateBranch,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_sym287_melt_rate_branch_1",
        always_enabled,
        collapse_sym287_melt_rate_branch_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::FloorFedParticleClampPair,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_floor_fed_particle_clamp_pair_2",
        always_enabled,
        collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::GrayScottClampPair,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_gray_scott_clamp_pair_1",
        always_enabled,
        collapse_gray_scott_clamp_pair_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::StripUnusedHelperParams,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_strip_unused_helper_params",
        always_enabled,
        strip_unused_helper_params_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::TrivialDotProductWrappers,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_trivial_dot_product_wrappers_2",
        always_enabled,
        collapse_trivial_dot_product_wrappers_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::Sym287MeltRateBranch,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_sym287_melt_rate_branch_2",
        always_enabled,
        collapse_sym287_melt_rate_branch_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::FloorFedParticleClampPair,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_floor_fed_particle_clamp_pair_3",
        always_enabled,
        collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::GrayScottClampPair,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_gray_scott_clamp_pair_2",
        always_enabled,
        collapse_gray_scott_clamp_pair_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::HelperExprReuse,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_helper_expr_reuse",
        always_enabled,
        rewrite_helper_expr_reuse_calls_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::DotProductHelperCalls,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_dot_product_helper_calls",
        always_enabled,
        rewrite_dot_product_helper_calls_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::Sym119HelperCalls,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_sym119_helper_calls",
        always_enabled,
        rewrite_sym119_helper_calls_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::TrivialFillHelperCalls,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_trivial_fill_helper_calls",
        always_enabled,
        rewrite_trivial_fill_helper_calls_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::IdenticalZeroFillAlias,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_identical_zero_fill_alias",
        always_enabled,
        rewrite_identical_zero_fill_pairs_to_aliases_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::DuplicateSym183Calls,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_duplicate_sym183_calls",
        always_enabled,
        rewrite_duplicate_sym183_calls_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::ParticleStateRebinds,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_particle_state_rebinds",
        always_enabled,
        restore_particle_state_rebinds_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::StaleParticleStateReplays,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_stale_particle_state_replays",
        always_enabled,
        strip_stale_particle_state_replays_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::AdjacentDirNeighborRows,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_adjacent_dir_neighbor_rows",
        always_enabled,
        collapse_adjacent_dir_neighbor_row_branches_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::ExactSafeLoopIndexWrite,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_exact_safe_loop_index_write_1",
        always_enabled,
        rewrite_exact_safe_loop_index_write_calls_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::FloorFedParticleClampPair,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_floor_fed_particle_clamp_pair_4",
        always_enabled,
        collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::Sym287MeltRateBranch,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_sym287_melt_rate_branch_3",
        always_enabled,
        collapse_sym287_melt_rate_branch_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::CgLoopCarriedUpdates,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_cg_loop_carried_updates_1",
        always_enabled,
        restore_cg_loop_carried_updates_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::BufferSwapAfterTempCopy,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_buffer_swap_after_temp_copy",
        always_enabled,
        restore_buffer_swaps_after_temp_copy_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::WenoFullRangeGatherReplay,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_weno_full_range_gather_replay_1",
        always_enabled,
        collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::ExactSafeLoopIndexWrite,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_exact_safe_loop_index_write_2",
        always_enabled,
        rewrite_exact_safe_loop_index_write_calls_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::MountainDxTemp,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_mountain_dx_temp_2",
        always_enabled,
        rewrite_mountain_dx_temp_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::WenoFullRangeGatherReplay,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_weno_full_range_gather_replay_2",
        always_enabled,
        collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::DeadWenoTopologySeed,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_dead_weno_topology_seed",
        always_enabled,
        strip_dead_weno_topology_seed_i_before_direct_adj_gather_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::PruneUnreachableHelpers,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_prune_unreachable_helpers_1",
        prune_enabled,
        prune_unreachable_raw_helper_definitions,
    ),
    raw_pass(
        RawRewritePassId::CgLoopCarriedUpdates,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_cg_loop_carried_updates_2",
        always_enabled,
        restore_cg_loop_carried_updates_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::MissingCseRangeAliases,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_missing_cse_range_aliases",
        always_enabled,
        repair_missing_cse_range_aliases_in_raw_emitted_r,
    ),
    raw_pass(
        RawRewritePassId::PruneUnreachableHelpers,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_prune_unreachable_helpers_2",
        prune_enabled,
        prune_unreachable_raw_helper_definitions,
    ),
    raw_pass(
        RawRewritePassId::DeadZeroLoopSeeds,
        RawRewriteStage::FullProgram,
        "RawRewriteSoundness.full_dead_zero_loop_seeds",
        always_enabled,
        strip_dead_zero_loop_seeds_before_for_in_raw_emitted_r,
    ),
];

pub(crate) const POST_ASSEMBLY_FINALIZE_PASSES: &[RawRewritePassSpec] = &[raw_pass(
    RawRewritePassId::NormalizePostAssemblyWhitespace,
    RawRewriteStage::PostAssemblyFinalize,
    "RawRewriteSoundness.post_assembly_whitespace",
    always_enabled,
    normalize_post_assembly_whitespace,
)];

pub(crate) const fn hermes_stage(
    id: HermesStageId,
    order: u8,
    raw_stage: RawRewriteStage,
    proof_key: &'static str,
    passes: &'static [RawRewritePassSpec],
) -> HermesStageSpec {
    HermesStageSpec {
        id,
        order,
        raw_stage,
        proof_key,
        passes,
    }
}

pub(crate) const HERMES_STAGE_CATALOG: &[HermesStageSpec] = &[
    hermes_stage(
        HermesStageId::FragmentRawRewrite,
        0,
        RawRewriteStage::Fragment,
        "HermesEmitPipelineSoundness.fragment_raw_rewrite",
        FRAGMENT_RAW_REWRITE_PASSES,
    ),
    hermes_stage(
        HermesStageId::FullProgramRawRewrite,
        1,
        RawRewriteStage::FullProgram,
        "HermesEmitPipelineSoundness.full_program_raw_rewrite",
        FULL_PROGRAM_RAW_REWRITE_PASSES,
    ),
    hermes_stage(
        HermesStageId::PostAssemblyFinalize,
        2,
        RawRewriteStage::PostAssemblyFinalize,
        "HermesEmitPipelineSoundness.post_assembly_finalize",
        POST_ASSEMBLY_FINALIZE_PASSES,
    ),
];

pub(crate) fn hermes_stage_spec(id: HermesStageId) -> &'static HermesStageSpec {
    match id {
        HermesStageId::FragmentRawRewrite => &HERMES_STAGE_CATALOG[0],
        HermesStageId::FullProgramRawRewrite => &HERMES_STAGE_CATALOG[1],
        HermesStageId::PostAssemblyFinalize => &HERMES_STAGE_CATALOG[2],
    }
}

pub(crate) fn hermes_stage_catalog_is_well_formed() -> bool {
    let mut previous_order = None;
    for spec in HERMES_STAGE_CATALOG {
        let _metadata = (spec.raw_stage, spec.proof_key);
        if spec.passes.is_empty() {
            return false;
        }
        if previous_order.is_some_and(|previous| previous >= spec.order) {
            return false;
        }
        if spec.passes.iter().any(|pass| pass.stage != spec.raw_stage) {
            return false;
        }
        previous_order = Some(spec.order);
    }
    true
}

impl HermesPassManager {
    pub(crate) fn for_stage(id: HermesStageId) -> Self {
        debug_assert!(hermes_stage_catalog_is_well_formed());
        let spec = hermes_stage_spec(id);
        Self { spec }
    }

    pub(crate) fn run(&self, output: String, ctx: &RawRewriteContext) -> String {
        let _stage_metadata = (self.spec.id, self.spec.order, self.spec.proof_key);
        RawRewritePassManager::new(self.spec.raw_stage, self.spec.passes).run(output, ctx)
    }
}

pub(crate) fn run_fragment_raw_rewrite_passes(
    output: String,
    output_opts: CompileOutputOptions,
) -> String {
    let ctx = RawRewriteContext::new(output_opts);
    HermesPassManager::for_stage(HermesStageId::FragmentRawRewrite).run(output, &ctx)
}

pub(crate) fn run_full_program_raw_rewrite_passes(
    output: String,
    output_opts: CompileOutputOptions,
) -> String {
    let ctx = RawRewriteContext::new(output_opts);
    HermesPassManager::for_stage(HermesStageId::FullProgramRawRewrite).run(output, &ctx)
}

pub(crate) fn run_post_assembly_finalize_passes(output: String) -> String {
    let ctx = RawRewriteContext::new(CompileOutputOptions::default());
    HermesPassManager::for_stage(HermesStageId::PostAssemblyFinalize).run(output, &ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hermes_stage_catalog_is_ordered_and_stage_consistent() {
        assert!(hermes_stage_catalog_is_well_formed());
    }

    #[test]
    fn hermes_stage_specs_map_to_distinct_raw_stages() {
        let fragment = hermes_stage_spec(HermesStageId::FragmentRawRewrite);
        let full = hermes_stage_spec(HermesStageId::FullProgramRawRewrite);
        let finalize = hermes_stage_spec(HermesStageId::PostAssemblyFinalize);

        assert_ne!(fragment.raw_stage, full.raw_stage);
        assert_ne!(full.raw_stage, finalize.raw_stage);
        assert!(fragment.passes.len() < full.passes.len());
        assert_eq!(finalize.passes.len(), 1);
    }
}
