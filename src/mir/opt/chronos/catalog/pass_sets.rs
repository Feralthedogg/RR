use super::*;
pub(in crate::mir::opt) const PHASE_ORDER_CONTROL_PRELUDE_PASSES: &[ChronosPassSpec] = &[
    phase_order_control_prelude_spec(
        ChronosPassId::SimplifyCfg,
        "After SimplifyCFG",
        always_enabled,
        run_simplify_cfg,
        "PhaseOrderIterationSoundness.control_flow_heavy_simplify_cfg",
    ),
    phase_order_control_prelude_spec(
        ChronosPassId::Sccp,
        "After SCCP",
        always_enabled,
        run_sccp,
        "PhaseOrderIterationSoundness.control_flow_heavy_sccp",
    ),
    phase_order_control_prelude_spec(
        ChronosPassId::Intrinsics,
        "After Intrinsics",
        always_enabled,
        run_intrinsics,
        "PhaseOrderIterationSoundness.control_flow_heavy_intrinsics",
    ),
    phase_order_control_prelude_spec(
        ChronosPassId::TypeSpecialize,
        "After TypeSpecialize",
        always_enabled,
        run_type_specialize,
        "PhaseOrderIterationSoundness.control_flow_heavy_type_specialize",
    ),
    phase_order_control_prelude_spec(
        ChronosPassId::Simplify,
        "After Simplify",
        always_enabled,
        run_simplify,
        "PhaseOrderIterationSoundness.control_flow_heavy_simplify",
    ),
    phase_order_control_prelude_spec(
        ChronosPassId::Sroa,
        "After SROA",
        always_enabled,
        run_sroa,
        "PhaseOrderIterationSoundness.control_flow_heavy_sroa",
    ),
    phase_order_control_prelude_spec(
        ChronosPassId::Dce,
        "After DCE",
        always_enabled,
        run_dce,
        "PhaseOrderIterationSoundness.control_flow_heavy_dce",
    ),
    phase_order_control_prelude_spec(
        ChronosPassId::Tco,
        "After TCO",
        always_enabled,
        run_tco,
        "PhaseOrderIterationSoundness.control_flow_heavy_tco",
    ),
    phase_order_control_prelude_spec(
        ChronosPassId::Gvn,
        "After GVN",
        gvn_enabled,
        run_gvn,
        "PhaseOrderIterationSoundness.control_flow_heavy_gvn",
    ),
];

pub(in crate::mir::opt) const PHASE_ORDER_BUDGET_PREFIX_PASSES: &[ChronosPassSpec] = &[
    phase_order_budget_prefix_spec(
        ChronosPassId::LoopOpt,
        "After LoopOpt",
        always_enabled,
        run_loop_opt,
        "PhaseOrderClusterSoundness.budget_prefix_loop_opt",
    ),
    phase_order_budget_prefix_spec(
        ChronosPassId::Licm,
        "After LICM",
        licm_enabled,
        run_licm,
        "PhaseOrderClusterSoundness.budget_prefix_licm",
    ),
];

pub(in crate::mir::opt) const PHASE_ORDER_CONTROL_BUDGET_PREFIX_PASSES: &[ChronosPassSpec] = &[
    phase_order_budget_prefix_spec(
        ChronosPassId::LoopOpt,
        "After LoopOpt",
        always_enabled,
        run_loop_opt,
        "PhaseOrderClusterSoundness.control_budget_prefix_loop_opt",
    ),
    phase_order_budget_prefix_spec(
        ChronosPassId::Licm,
        "After LICM",
        control_licm_enabled,
        run_licm,
        "PhaseOrderClusterSoundness.control_budget_prefix_licm",
    ),
];

pub(in crate::mir::opt) const PHASE_ORDER_BUDGET_TAIL_PASSES: &[ChronosPassSpec] = &[
    phase_order_budget_tail_spec(
        ChronosPassId::FreshAlloc,
        "After FreshAlloc",
        always_enabled,
        run_fresh_alloc,
        "PhaseOrderClusterSoundness.budget_tail_fresh_alloc",
    ),
    phase_order_budget_tail_spec(
        ChronosPassId::Bce,
        "After BCE",
        always_enabled,
        run_bce,
        "PhaseOrderClusterSoundness.budget_tail_bce",
    ),
];

pub(in crate::mir::opt) const PHASE_ORDER_BALANCED_STRUCTURAL_PASSES: &[ChronosPassSpec] = &[
    balanced_structural_spec(
        ChronosPassId::TypeSpecialize,
        "After TypeSpecialize",
        always_enabled,
        run_type_specialize,
        "PhaseOrderClusterSoundness.structural_cluster_type_specialize",
    ),
    balanced_structural_spec(
        ChronosPassId::Poly,
        "After Poly",
        always_enabled,
        run_poly,
        "PhaseOrderClusterSoundness.structural_cluster_poly",
    ),
    balanced_structural_spec(
        ChronosPassId::Vectorize,
        "After Vectorization",
        always_enabled,
        run_vectorize,
        "PhaseOrderClusterSoundness.structural_cluster_vectorize",
    ),
    balanced_structural_spec(
        ChronosPassId::Unroll,
        "After FullUnroll",
        always_enabled,
        run_unroll,
        "PhaseOrderClusterSoundness.structural_cluster_unroll",
    ),
    balanced_structural_spec(
        ChronosPassId::TypeSpecialize,
        "After TypeSpecialize(PostVec)",
        always_enabled,
        run_type_specialize,
        "PhaseOrderClusterSoundness.structural_cluster_post_vectorize_type_specialize",
    ),
    balanced_structural_spec(
        ChronosPassId::Tco,
        "After TCO",
        always_enabled,
        run_tco,
        "PhaseOrderClusterSoundness.structural_cluster_tco",
    ),
];

pub(in crate::mir::opt) const PHASE_ORDER_CONTROL_STRUCTURAL_PASSES: &[ChronosPassSpec] = &[
    control_structural_spec(
        ChronosPassId::Poly,
        "After Poly(Control)",
        always_enabled,
        run_poly,
        "PhaseOrderClusterSoundness.control_structural_cluster_poly",
    ),
    control_structural_spec(
        ChronosPassId::Vectorize,
        "After Vectorization",
        always_enabled,
        run_vectorize,
        "PhaseOrderClusterSoundness.control_structural_cluster_vectorize",
    ),
    control_structural_spec(
        ChronosPassId::Unroll,
        "After FullUnroll",
        always_enabled,
        run_unroll,
        "PhaseOrderClusterSoundness.control_structural_cluster_unroll",
    ),
    control_structural_spec(
        ChronosPassId::TypeSpecialize,
        "After TypeSpecialize(PostVec)",
        always_enabled,
        run_type_specialize,
        "PhaseOrderClusterSoundness.control_structural_cluster_post_vectorize_type_specialize",
    ),
];

pub(in crate::mir::opt) const PHASE_ORDER_FAST_DEV_VECTORIZE_PASSES: &[ChronosPassSpec] = &[
    fast_dev_vectorize_spec(
        ChronosPassId::TypeSpecialize,
        "After TypeSpecialize(FastDevVec)",
        always_enabled,
        run_type_specialize,
        "PhaseOrderIterationSoundness.fast_dev_subpath_type_specialize",
    ),
    fast_dev_vectorize_spec(
        ChronosPassId::Vectorize,
        "After Vectorization(FastDev)",
        always_enabled,
        run_vectorize,
        "PhaseOrderIterationSoundness.fast_dev_subpath_vectorize",
    ),
    fast_dev_vectorize_spec(
        ChronosPassId::TypeSpecialize,
        "After TypeSpecialize(PostVecFastDev)",
        always_enabled,
        run_type_specialize,
        "PhaseOrderIterationSoundness.fast_dev_subpath_post_vectorize_type_specialize",
    ),
];

pub(in crate::mir::opt) const PHASE_ORDER_STRUCTURAL_CLEANUP_PASSES: &[ChronosPassSpec] = &[
    structural_cleanup_spec(
        ChronosPassId::SimplifyCfg,
        "After Structural SimplifyCFG",
        always_enabled,
        run_simplify_cfg,
        "PhaseOrderClusterSoundness.cleanup_cluster_simplify_cfg",
    ),
    structural_cleanup_spec(
        ChronosPassId::Sroa,
        "After Structural SROA",
        always_enabled,
        run_sroa,
        "PhaseOrderClusterSoundness.cleanup_cluster_sroa",
    ),
    structural_cleanup_spec(
        ChronosPassId::Dce,
        "After Structural DCE",
        always_enabled,
        run_dce,
        "PhaseOrderClusterSoundness.cleanup_cluster_dce",
    ),
];

pub(in crate::mir::opt) const FUNCTION_FINAL_POLISH_PASSES: &[ChronosPassSpec] = &[
    function_final_polish_spec(
        ChronosPassId::SimplifyCfg,
        "FinalPolish/SimplifyCFG",
        always_enabled,
        run_simplify_cfg,
        "PhaseOrderClusterSoundness.final_polish_simplify_cfg",
    ),
    function_final_polish_spec(
        ChronosPassId::Dce,
        "FinalPolish/DCE",
        always_enabled,
        run_dce,
        "PhaseOrderClusterSoundness.final_polish_dce",
    ),
];

pub(in crate::mir::opt) const PROGRAM_INLINE_PASSES: &[ChronosProgramPassSpec] =
    &[program_inline_spec(
        ChronosPassId::Inline,
        "After Inlining",
        program_always_enabled,
        run_inline_program,
        "ProgramPostTierStagesSoundness.inline_stage",
    )];

pub(in crate::mir::opt) const PROGRAM_RECORD_SPECIALIZATION_PASSES: &[ChronosProgramPassSpec] = &[
    program_record_specialization_spec(
        ChronosPassId::RecordCallSpecialize,
        "After SROA Record Call Specialization",
        program_always_enabled,
        run_record_call_specialize,
        "ProgramPostTierStagesSoundness.record_call_specialization_stage",
    ),
    program_record_specialization_spec(
        ChronosPassId::RecordReturnSpecialize,
        "After SROA Record Return Specialization",
        program_always_enabled,
        run_record_return_specialize,
        "ProgramPostTierStagesSoundness.record_return_specialization_stage",
    ),
];

pub(in crate::mir::opt) const PROGRAM_OUTLINE_PASSES: &[ChronosProgramPassSpec] =
    &[program_outline_spec(
        ChronosPassId::Outline,
        "After Function Outlining",
        program_always_enabled,
        run_outline_program,
        "ProgramPostTierStagesSoundness.outline_stage",
    )];

pub(in crate::mir::opt) const PROGRAM_INLINE_CLEANUP_PASSES: &[ChronosPassSpec] = &[
    program_inline_cleanup_spec(
        ChronosPassId::SimplifyCfg,
        "After Inline Cleanup/SimplifyCFG",
        non_conservative_enabled,
        run_simplify_cfg,
        "ProgramPostTierStagesSoundness.inline_cleanup_simplify_cfg",
    ),
    program_inline_cleanup_spec(
        ChronosPassId::Sroa,
        "After Inline Cleanup/SROA",
        non_conservative_enabled,
        run_sroa,
        "ProgramPostTierStagesSoundness.inline_cleanup_sroa",
    ),
    program_inline_cleanup_spec(
        ChronosPassId::Sccp,
        "After Inline Cleanup/SCCP",
        non_conservative_enabled,
        run_sccp,
        "ProgramPostTierStagesSoundness.inline_cleanup_sccp",
    ),
    program_inline_cleanup_spec(
        ChronosPassId::Simplify,
        "After Inline Cleanup/Simplify",
        non_conservative_enabled,
        run_simplify,
        "ProgramPostTierStagesSoundness.inline_cleanup_simplify",
    ),
    program_inline_cleanup_spec(
        ChronosPassId::Dce,
        "After Inline Cleanup/DCE",
        non_conservative_enabled,
        run_dce,
        "ProgramPostTierStagesSoundness.inline_cleanup_dce",
    ),
];

pub(in crate::mir::opt) const PROGRAM_FRESH_ALIAS_PASSES: &[ChronosPassSpec] =
    &[program_fresh_alias_spec(
        ChronosPassId::FreshAlias,
        "After FreshAlias",
        always_enabled,
        run_fresh_alias,
        "ProgramPostTierStagesSoundness.fresh_alias_stage",
    )];

pub(in crate::mir::opt) const PROGRAM_POST_DESSA_PASSES: &[ChronosPassSpec] = &[
    program_post_de_ssa_spec(
        ChronosPassId::DeSsa,
        "After De-SSA",
        always_enabled,
        run_de_ssa,
        "ProgramPostTierStagesSoundness.de_ssa_program_stage",
    ),
    program_post_de_ssa_spec(
        ChronosPassId::CopyCleanup,
        "After De-SSA/CopyCleanup",
        non_conservative_enabled,
        run_copy_cleanup,
        "ProgramPostTierStagesSoundness.copy_cleanup_stage",
    ),
    program_post_de_ssa_spec(
        ChronosPassId::SimplifyCfg,
        "After De-SSA/SimplifyCFG",
        non_conservative_enabled,
        run_simplify_cfg,
        "ProgramPostTierStagesSoundness.post_de_ssa_simplify_cfg",
    ),
    program_post_de_ssa_spec(
        ChronosPassId::Dce,
        "After De-SSA/DCE",
        non_conservative_enabled,
        run_dce,
        "ProgramPostTierStagesSoundness.post_de_ssa_dce",
    ),
];

pub(in crate::mir::opt) const PREPARE_FOR_CODEGEN_DESSA_PASSES: &[ChronosPassSpec] =
    &[prepare_for_codegen_spec(
        ChronosPassId::DeSsa,
        "PrepareForCodegen/DeSSA",
        always_enabled,
        run_de_ssa,
        "OptimizerPipelineSoundness.prepare_for_codegen_de_ssa",
    )];

pub(in crate::mir::opt) const PREPARE_FOR_CODEGEN_CLEANUP_PASSES: &[ChronosPassSpec] = &[
    prepare_for_codegen_spec(
        ChronosPassId::SimplifyCfg,
        "PrepareForCodegen/SimplifyCFG",
        non_conservative_enabled,
        run_simplify_cfg,
        "OptimizerPipelineSoundness.prepare_for_codegen_simplify_cfg",
    ),
    prepare_for_codegen_spec(
        ChronosPassId::Dce,
        "PrepareForCodegen/DCE",
        non_conservative_enabled,
        run_dce,
        "OptimizerPipelineSoundness.prepare_for_codegen_dce",
    ),
];
