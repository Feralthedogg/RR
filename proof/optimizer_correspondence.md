# Optimizer Proof ↔ Rust Correspondence

This note ties the reduced optimizer proof layers in `proof/` to the concrete
Rust hardening work in `src/mir/opt/`.

It is not a full 1:1 verification of the production optimizer. The point is
more pragmatic:

- identify the smallest proof artifact that matches a real implementation guard
  or regression
- make explicit which Rust tests a proof layer is intended to approximate
- keep the next extension point obvious

## Claim Boundary

This file should be read with one rule in mind:

- matching a Rust stage boundary to a theorem name does **not** by itself mean
  the production pass is fully mechanized

What it does mean is:

- the reduced proof workspace now names that boundary explicitly
- the reduced theorem chain is intended to approximate that Rust slice
- the exact strength of the claim is further qualified in
  [optimizer_proof_gap_audit.md](/Users/feral/Desktop/Programming/RR/proof/optimizer_proof_gap_audit.md:1)

Use this note to answer:

- “which proof file corresponds to this Rust stage?”

Use the audit note to answer:

- “how strong is that correspondence?”

## Optimizer-Wide Soundness Target

Scaffolding files:
- [MirSemanticsLite.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/MirSemanticsLite.lean:1)
- [MirSemanticsLite.v](/Users/feral/Desktop/Programming/RR/proof/coq/MirSemanticsLite.v:1)
- [MirInvariantBundle.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/MirInvariantBundle.lean:1)
- [MirInvariantBundle.v](/Users/feral/Desktop/Programming/RR/proof/coq/MirInvariantBundle.v:1)

Current role:
- `MirSemanticsLite` fixes the reduced MIR execution domain that future
  optimizer soundness layers will compare before/after rewrites in
  the same semantic space
- `MirInvariantBundle` packages the reduced `verify_ir`-style assumptions and
  optimizer-eligibility side conditions (`unsupported_dynamic = false`,
  `opaque_interop = false`)
- both files currently prove only identity-pass preservation lemmas; they are
  scaffolding for a future optimizer theorem rather than a completed optimizer
  proof

First soundness layer on top of that spine:
- [DataflowOptSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/DataflowOptSoundness.lean:1)
- [DataflowOptSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/DataflowOptSoundness.v:1)

Current role:
- fixes a reduced dataflow optimizer slice over `MirSemanticsLite`
- proves three reusable preservation facts:
  - expression canonicalization preserves evaluation
  - constant propagation under environment agreement preserves evaluation
  - erasing a last dead pure assignment in a straight-line block preserves the
    returned value

Next CFG layer:
- [CfgOptSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/CfgOptSoundness.lean:1)
- [CfgOptSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/CfgOptSoundness.v:1)

Current role:
- introduces a reduced multi-block MIR runner over the same semantic domain
- proves a reduced empty-entry-goto retarget theorem approximating entry
  normalization / jump threading
- proves invariant preservation for appending a dead unreachable block shape
  and for retargeting the entry to an existing block

Next loop layer:
- [LoopOptSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/LoopOptSoundness.lean:1)
- [LoopOptSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/LoopOptSoundness.v:1)

Current role:
- reuses the existing reduced LICM graph/small-step proof chain as the first
  loop-optimization soundness layer inside the optimizer-only spine
- fixes theorem names for:
  - zero-trip LICM preservation
  - one-trip LICM preservation
  - loop-carried unsoundness witness
  - reduced BCE read-check elimination preservation
  - reduced TCO recursion-to-loop preservation

De-SSA boundary layer:
- [DeSsaBoundarySoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/DeSsaBoundarySoundness.lean:1)
- [DeSsaBoundarySoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/DeSsaBoundarySoundness.v:1)

Current role:
- reuses the reduced `DeSsaSubset` copy-boundary theorem
- exposes explicit stage-boundary theorem names for redundant move elimination
- supports the optimizer-wide stage family:
  - `program_post_dessa_preserves_verify_ir`
  - `program_post_dessa_preserves_semantics`
  - `prepare_for_codegen_preserves_verify_ir`
  - `prepare_for_codegen_preserves_semantics`

Composition layer:
- [OptimizerPipelineSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/OptimizerPipelineSoundness.lean:1)
- [OptimizerPipelineSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/OptimizerPipelineSoundness.v:1)

Current role:
- composes the reduced `DataflowOptSoundness`, `CfgOptSoundness`, and
  `LoopOptSoundness` layers
- fixes pass-group-shaped theorem names mirroring the Rust optimizer schedule:
  - `always_tier_preserves_verify_ir`
  - `always_tier_preserves_semantics`
  - `program_inner_pre_dessa_preserves_verify_ir`
  - `program_inner_pre_dessa_preserves_semantics`
- and exposes the top-level optimizer-only theorem names:
  - `optimizer_pipeline_preserves_verify_ir`
  - `optimizer_pipeline_preserves_semantics`

Phase-order refinement:
- [PhaseOrderOptimizerSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhaseOrderOptimizerSoundness.lean:1)
- [PhaseOrderOptimizerSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/PhaseOrderOptimizerSoundness.v:1)

Current role:
- introduces a reduced schedule enum mirroring Rust phase-order profiles
  (`Balanced`, `ComputeHeavy`, `ControlFlowHeavy`)
- fixes theorem family names for each profile:
  - `phase_schedule_balanced_preserves_verify_ir`
  - `phase_schedule_balanced_preserves_semantics`
  - `phase_schedule_compute_heavy_preserves_verify_ir`
  - `phase_schedule_compute_heavy_preserves_semantics`
  - `phase_schedule_control_flow_heavy_preserves_verify_ir`
  - `phase_schedule_control_flow_heavy_preserves_semantics`

Phase-order subcluster refinement:
- [PhaseOrderClusterSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhaseOrderClusterSoundness.lean:1)
- [PhaseOrderClusterSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/PhaseOrderClusterSoundness.v:1)

Current role:
- fixes theorem family names for the internal cluster boundaries that appear in
  Rust `phase_order.rs`:
  - `structural_cluster_preserves_verify_ir`
  - `structural_cluster_preserves_semantics`
  - `standard_cluster_preserves_verify_ir`
  - `standard_cluster_preserves_semantics`
  - `cleanup_cluster_preserves_verify_ir`
  - `cleanup_cluster_preserves_semantics`

Phase-order guard refinement:
- [PhaseOrderGuardSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhaseOrderGuardSoundness.lean:1)
- [PhaseOrderGuardSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/PhaseOrderGuardSoundness.v:1)

Current role:
- fixes a reduced guard record for Rust enable/skip boundaries:
  - `run_budgeted_passes`
  - `structural_enabled`
  - `control_flow_gate`
  - `fast_dev_vectorize`
  - `licm_allowed`
  - `bce_allowed`
- connects those guards to theorem families:
  - `balanced_guarded_preserves_*`
  - `control_flow_guarded_preserves_*`
  - `cleanup_guarded_preserves_*`

Phase-order feature-gate refinement:
- [PhaseOrderFeatureGateSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhaseOrderFeatureGateSoundness.lean:1)
- [PhaseOrderFeatureGateSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/PhaseOrderFeatureGateSoundness.v:1)

Current role:
- fixes a reduced feature record matching the Rust gate inputs:
  - `ir_size`
  - `block_count`
  - `loop_count`
  - `canonical_loop_count`
  - `branch_terms`
  - `call_values`
  - `side_effecting_calls`
  - `store_instrs`
- exposes direct gate/selection lemmas:
  - `control_flow_gate_enables_structural_cluster`
  - `control_flow_gate_false_falls_back_to_standard_cluster`
  - `fast_dev_gate_enables_structural_cluster_when_structural_disabled`
  - `fast_dev_gate_false_falls_back_to_standard_cluster`
  - `budget_disabled_falls_back_to_standard_cluster`

Phase-order iteration-entry refinement:
- [PhaseOrderIterationSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhaseOrderIterationSoundness.lean:1)
- [PhaseOrderIterationSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/PhaseOrderIterationSoundness.v:1)

Current role:
- fixes reduced theorem family names for the actual heavy-iteration entrypoints
  used by Rust `phase_order.rs`:
  - `balanced_iteration_preserves_*`
  - `compute_heavy_iteration_preserves_*`
  - `control_flow_heavy_iteration_preserves_*`
  - `fast_dev_subpath_preserves_*`
- composes the previously introduced cluster/guard/feature-gate layers into
  entrypoint-shaped theorems that match:
  - `run_balanced_heavy_phase_iteration`
  - `run_compute_heavy_phase_iteration`
  - `run_control_flow_heavy_phase_iteration`
  - `run_fast_dev_vectorize_subpath`

Phase-order fallback refinement:
- [PhaseOrderFallbackSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhaseOrderFallbackSoundness.lean:1)
- [PhaseOrderFallbackSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/PhaseOrderFallbackSoundness.v:1)

Current role:
- fixes a reduced heavy-iteration result record with:
  - `structural_progress`
  - `non_structural_changes`
- exposes an explicit theorem boundary for Rust's
  `control_flow_should_fallback_to_balanced` predicate:
  - `control_flow_fallback_preserves_verify_ir`
  - `control_flow_fallback_preserves_semantics`

Phase-plan selection refinement:
- [PhasePlanSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhasePlanSoundness.lean:1)
- [PhasePlanSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/PhasePlanSoundness.v:1)

Current role:
- fixes reduced plan-level enums and records for:
  - `PhaseOrderingMode`
  - `PhaseProfileKind`
  - `PhaseScheduleId`
  - `FunctionPhasePlan`
- exposes reduced theorem family for:
  - `classify_phase_profile`
  - `choose_phase_schedule`
  - `build_function_phase_plan_from_features`
  - plan-selected schedule soundness via `phaseScheduledPipeline`

Phase-plan collection refinement:
- [PhasePlanCollectionSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhasePlanCollectionSoundness.lean:1)
- [PhasePlanCollectionSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/PhasePlanCollectionSoundness.v:1)

Current role:
- fixes reduced collection/eligibility boundaries for Rust
  `collect_function_phase_plans`
- models the same skip reasons:
  - missing function entry
  - conservative optimization required
  - self-recursive function
  - selected-function filter miss
- exposes reduced theorem family for:
  - `collect_single_skips_missing`
  - `collect_single_skips_conservative`
  - `collect_single_skips_self_recursive`
  - `collect_single_skips_unselected`
  - collected-plan preservation via `planSelectedPipeline`

Phase-plan lookup refinement:
- [PhasePlanLookupSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhasePlanLookupSoundness.lean:1)
- [PhasePlanLookupSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/PhasePlanLookupSoundness.v:1)

Current role:
- fixes a reduced lookup boundary for consuming collected plans by function id
- models the same retrieval shape as Rust `plans.get(name)`
- exposes theorem family for:
  - singleton lookup hit/miss regressions
  - `lookup_collected_plan_preserves_verify_ir`
  - `lookup_collected_plan_preserves_semantics`

Phase-plan summary refinement:
- [PhasePlanSummarySoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhasePlanSummarySoundness.lean:1)
- [PhasePlanSummarySoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/PhasePlanSummarySoundness.v:1)

Current role:
- fixes a reduced ordered-summary consumption boundary for `plan_summary_lines`
- models ordered function-id traversal together with lookup hit/miss
- exposes theorem family for:
  - `summary_lookup_hit_emits_entry`
  - `summary_lookup_miss_skips_entry`
  - summary entry exposure of `schedule/profile/pass_groups`
  - summary-lookup preservation via `planSelectedPipeline`

Program-budget refinement:
- [ProgramOptPlanSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/ProgramOptPlanSoundness.lean:1)
- [ProgramOptPlanSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/ProgramOptPlanSoundness.v:1)

Current role:
- fixes a reduced `ProgramOptPlan` boundary for Rust `build_opt_plan_with_profile`
- models the same three high-level cases:
  - under-budget: select all safe functions
  - over-budget: selective mode with within-budget prefix
  - empty selective set: fallback to smallest eligible function

Program-level heavy-tier composition refinement:
- [ProgramPhasePipelineSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/ProgramPhasePipelineSoundness.lean:1)
- [ProgramPhasePipelineSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/ProgramPhasePipelineSoundness.v:1)

Current role:
- fixes a reduced composition boundary for the program-level heavy-tier flow:
  `ProgramOptPlan -> selected_functions -> collect_function_phase_plans ->
  plan_summary`
- exposes theorem family for:
  - heavy-tier disabled yields no collected plans / no summary
  - program-level lookup preserves selected schedule soundness
  - program-level summary hit/miss follows the reduced lookup boundary

Program-level heavy-tier execution refinement:
- [ProgramTierExecutionSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/ProgramTierExecutionSoundness.lean:1)
- [ProgramTierExecutionSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/ProgramTierExecutionSoundness.v:1)

Current role:
- fixes the reduced per-function execution boundary inside
  `run_program_with_profile_inner`
- models the same top-level branches:
  - conservative skip
  - self-recursive skip
  - heavy-tier disabled skip
  - budget skip
  - collected-plan hit
  - legacy-plan fallback

Program tail-stage refinement:
- [ProgramPostTierStagesSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/ProgramPostTierStagesSoundness.lean:1)
- [ProgramPostTierStagesSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/ProgramPostTierStagesSoundness.v:1)

Current role:
- fixes reduced theorem family for the remaining post-heavy stages inside
  `run_program_with_profile_inner`
- names the three stage boundaries directly:
  - `inline_cleanup_stage_*`
  - `fresh_alias_stage_*`
  - `de_ssa_program_stage_*`
- and exposes a composed tail theorem:
  - `program_post_tier_pipeline_preserves_verify_ir`
  - `program_post_tier_pipeline_preserves_semantics`

Tail-stage actual reduced rewrite companions:
- [InlineCleanupRefinementSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/InlineCleanupRefinementSoundness.lean:1)
- [InlineCleanupRefinementSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/InlineCleanupRefinementSoundness.v:1)
- [FreshAliasRewriteSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/FreshAliasRewriteSoundness.lean:1)
- [FreshAliasRewriteSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/FreshAliasRewriteSoundness.v:1)

Current role:
- `InlineCleanupRefinementSoundness` fixes an actual non-identity reduced
  cleanup rewrite via entry retargeting on an empty-entry-goto shape
- `FreshAliasRewriteSoundness` fixes an actual alias-rename reduced rewrite
  showing that replacing a fresh alias load with its source load preserves
  evaluation under alias agreement

Program wrapper refinement:
- [ProgramRunProfileInnerSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/ProgramRunProfileInnerSoundness.lean:1)
- [ProgramRunProfileInnerSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/ProgramRunProfileInnerSoundness.v:1)

Current role:
- fixes the reduced wrapper theorem family for the whole
  `run_program_with_profile_inner` flow
- composes:
  - always-tier execution
  - heavy-tier plan flow and per-function execution
  - plan summary emission
  - post-tier cleanup / de-ssa tail
- exposes wrapper theorem names:
  - `run_program_inner_function_preserves_verify_ir`
  - `run_program_inner_function_preserves_semantics`
  - `run_program_inner_summary_hit_emits_singleton`
  - `run_program_inner_summary_miss_skips_singleton`

Public optimizer API wrapper refinement:
- [ProgramApiWrapperSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/ProgramApiWrapperSoundness.lean:1)
- [ProgramApiWrapperSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/ProgramApiWrapperSoundness.v:1)

Current role:
- fixes reduced shell theorem names for the public optimizer entrypoints around
  `run_program_with_profile_and_scheduler`, `run_program_with_scheduler`,
  `run_program_with_stats`, and `run_program`
- makes explicit that these wrappers are orchestration shells around the
  already-proved `run_program_with_profile_inner` boundary

Reduced compiler end-to-end refinement:
- [CompilerEndToEndSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/CompilerEndToEndSoundness.lean:1)
- [CompilerEndToEndSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/CompilerEndToEndSoundness.v:1)

Current role:
- pairs the optimizer wrapper theorem family with a reduced frontend/backend
  observable theorem
- the Lean theorem reuses `PipelineStmtSubset`; the Coq theorem currently uses a
  tiny self-contained expression model, so this is not a synchronized
  Lean/Coq statement over the same frontend artifact
- exposes a top-level reduced observable statement:
  frontend lowered/emitted evaluation matches the source result, and the
  optimized MIR witness preserves its execution result

## GVN

Proof layers:
- [GvnSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/GvnSubset.lean:1)
- [GvnSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/GvnSubset.v:1)

Core proof claim:
- commutative `add` canonicalization preserves evaluation
- a reduced intrinsic-abs wrapper preserves evaluation through the same
  canonicalization
- a reduced `fieldset -> field` read preserves evaluation through the same
  canonicalization
- if two expressions have the same canonical form, replacing one with the other
  preserves evaluation

Primary Rust correspondence:
- [gvn.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/gvn.rs:258)
  commutative operand canonicalization
- [gvn.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/gvn.rs:273)
  nested canonicalization / replacement propagation

Concrete Rust regressions:
- [gvn_canonicalizes_commutative_binary_operands](/Users/feral/Desktop/Programming/RR/src/mir/opt/gvn.rs:978)
- [gvn_propagates_record_literal_cse_into_field_gets](/Users/feral/Desktop/Programming/RR/src/mir/opt/gvn.rs:905)
- [gvn_cse_duplicate_intrinsics](/Users/feral/Desktop/Programming/RR/src/mir/opt/gvn.rs:1035)
- [gvn_propagates_fieldset_cse_into_field_gets](/Users/feral/Desktop/Programming/RR/src/mir/opt/gvn.rs:1086)

Current gap:
- proof is expression-level only; it does not yet model block dominance,
  availability, or mutation barriers

## Inline

Proof layers:
- [InlineSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/InlineSubset.lean:1)
- [InlineSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/InlineSubset.v:1)

Core proof claim:
- a reduced pure helper shape
  - `arg`
  - `addConst`
  - `field`
  - `fieldAddConst`
  can be expression-inlined without changing evaluation

Primary Rust correspondence:
- [inline.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/inline.rs:716)
  expr-inline `clone_rec()` coverage
- [inline.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/inline.rs:653)
  side-effect guard for expr-inline
- [inline.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/inline.rs:877)
  full-inline remap coverage

Concrete Rust regressions:
- [inline_value_calls_rejects_store_index3d_side_effect_helpers](/Users/feral/Desktop/Programming/RR/src/mir/opt/inline.rs:1309)
- [perform_inline_remaps_record_field_value_ids](/Users/feral/Desktop/Programming/RR/src/mir/opt/inline.rs:1460)
- [inline_value_calls_supports_record_field_helpers](/Users/feral/Desktop/Programming/RR/src/mir/opt/inline.rs:1669)
- [inline_value_calls_supports_intrinsic_helpers](/Users/feral/Desktop/Programming/RR/src/mir/opt/inline.rs:1763)
- [inline_value_calls_supports_fieldset_helpers](/Users/feral/Desktop/Programming/RR/src/mir/opt/inline.rs:1846)
- [inline_value_calls_supports_index3d_helpers](/Users/feral/Desktop/Programming/RR/src/mir/opt/inline.rs:1924)

Current gap:
- proof only covers pure helper semantics; it does not yet model side effects,
  call graph structure, or full caller/callee block rewrites

## De-SSA

Proof layers:
- [DeSsaSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/DeSsaSubset.lean:1)
- [DeSsaSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/DeSsaSubset.v:1)

Core proof claim:
- a reduced canonical fingerprint is enough to decide that an incoming
  predecessor value already matches an existing predecessor assignment
- in that case, adding a redundant move is unnecessary

Primary Rust correspondence:
- [de_ssa.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/de_ssa.rs:204)
  canonical value fingerprint before instruction
- [de_ssa.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/de_ssa.rs:318)
  same canonical value before instruction

Concrete Rust regressions:
- [critical_edge_is_not_split_when_phi_input_matches_existing_field_get_shape](/Users/feral/Desktop/Programming/RR/src/mir/opt/de_ssa.rs:1266)
- [critical_edge_is_not_split_when_phi_input_matches_existing_intrinsic_shape](/Users/feral/Desktop/Programming/RR/src/mir/opt/de_ssa.rs:1365)
- [critical_edge_is_not_split_when_phi_input_matches_existing_fieldset_shape](/Users/feral/Desktop/Programming/RR/src/mir/opt/de_ssa.rs:1448)

Current gap:
- proof does not yet model parallel-copy scheduling or full CFG mutation

## DCE

Proof layers:
- [DceSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/DceSubset.lean:1)
- [DceSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/DceSubset.v:1)

Core proof claim:
- a pure dead assignment may be erased
- an effectful dead assignment must be demoted to `eval`
- nested wrappers preserve the total effect count seen by this reduced DCE

Primary Rust correspondence:
- [cfg_cleanup.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/cfg_cleanup.rs:340)
  recursive `has_side_effect_val()`

Concrete Rust regressions:
- [dce_preserves_eval_with_nested_side_effect_inside_pure_call](/Users/feral/Desktop/Programming/RR/src/mir/opt/cfg_cleanup.rs:421)
- [dce_preserves_eval_with_nested_side_effect_inside_intrinsic](/Users/feral/Desktop/Programming/RR/src/mir/opt/cfg_cleanup.rs:518)
- [dce_preserves_eval_with_nested_side_effect_inside_index1d](/Users/feral/Desktop/Programming/RR/src/mir/opt/cfg_cleanup.rs:628)
- [dce_preserves_eval_with_nested_side_effect_inside_index2d](/Users/feral/Desktop/Programming/RR/src/mir/opt/cfg_cleanup.rs:716)
- [dce_preserves_eval_with_nested_side_effect_inside_index3d](/Users/feral/Desktop/Programming/RR/src/mir/opt/cfg_cleanup.rs:804)
- [dce_preserves_eval_with_nested_side_effect_inside_phi](/Users/feral/Desktop/Programming/RR/src/mir/opt/cfg_cleanup.rs:892)
- [dce_preserves_eval_with_nested_side_effect_inside_len](/Users/feral/Desktop/Programming/RR/src/mir/opt/cfg_cleanup.rs:980)
- [dce_preserves_eval_with_nested_side_effect_inside_indices](/Users/feral/Desktop/Programming/RR/src/mir/opt/cfg_cleanup.rs:1068)
- plus matching dead-assign-to-eval regressions in the same file

Current gap:
- proof tracks reduced effect count, not full R observable behavior

## Vectorize

Proof layers:
- [VectorizeSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VectorizeSubset.lean:1)
- [VectorizeSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VectorizeSubset.v:1)

Core proof claim:
- expr-map certification must reject effectful loop bodies
- conditional map/reduction certification must accept only store-only branch
  shapes

Primary Rust correspondence:
- [planning.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/planning.rs:1760)
- [planning_expr_map.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/planning_expr_map.rs:344)
- [analysis_vectorization.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/analysis_vectorization.rs:148)
- [proof.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:562)
- [proof_reduction.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof_reduction.rs:499)

Concrete Rust regressions:
- [expr_map_matcher_rejects_loop_with_eval_side_effect](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:2549)
- [scatter_matcher_rejects_loop_with_eval_side_effect](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:2591)
- [cond_map_certification_rejects_branch_eval_side_effect](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:2806)
- [cond_reduction_certification_rejects_branch_eval_side_effect](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:2849)
- [cond_map_certification_rejects_branch_assign_side_effect](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:2865)
- [classify_store_3d_rejects_block_with_eval_side_effect](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/analysis_vectorization.rs:2529)

Current gap:
- proof covers only certification guards, not the transactional rewrite itself

## Vectorize Apply

Proof layers:
- [VectorizeApplySubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VectorizeApplySubset.lean:1)
- [VectorizeApplySubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VectorizeApplySubset.v:1)

Core proof claim:
- rejected plans roll back to the scalar original
- certified result-preserving plans may commit without changing the scalar
  result

Primary Rust correspondence:
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:3531)
  transactional apply entry point
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:3493)
  vector apply site selection before transactional rewrite

Concrete Rust regressions:
- [enabled_config_certifies_simple_map_and_plan_applies_transactionally](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:2491)
- [enabled_config_certifies_simple_expr_map_and_plan_applies_transactionally](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:2520)
- [enabled_config_certifies_simple_cond_map_and_plan_applies_transactionally](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:2757)
- [enabled_config_certifies_simple_cond_reduction_and_plan_applies_transactionally](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:2792)
- [enabled_config_certifies_simple_sum_reduction_and_plan_applies_transactionally](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:2910)

Current gap:
- proof models the transactional contract, but not the internal CFG/value
  rewrites that establish result preservation for real plans

## Vectorize Rewrite

Proof layers:
- [VectorizeRewriteSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VectorizeRewriteSubset.lean:1)
- [VectorizeRewriteSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VectorizeRewriteSubset.v:1)

Core proof claim:
- the reduced exit-`Phi` merge after a vectorized apply/fallback split rejoins
  to the original scalar exit value
- both fallback and result-preserving apply paths are covered explicitly

Primary Rust correspondence:
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:666)
  preheader guard split
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:681)
  exit-`Phi` construction for preserved scalar semantics

Concrete Rust regressions:
- [enabled_config_certifies_simple_map_and_plan_applies_transactionally](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:2491)
- [enabled_config_certifies_simple_expr_map_and_plan_applies_transactionally](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:2520)
- [enabled_config_certifies_simple_cond_map_and_plan_applies_transactionally](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:2757)

Current gap:
- proof still abstracts away the concrete MIR block/value mutation performed by
  the production rewrite, but it now models the scalar exit merge itself

## Vectorize MIR Rewrite

Proof layers:
- [VectorizeMirRewriteSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VectorizeMirRewriteSubset.lean:1)
- [VectorizeMirRewriteSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VectorizeMirRewriteSubset.v:1)

Core proof claim:
- a tiny MIR machine with `preheader -> apply/fallback -> exit` preserves the
  original scalar result
- the reduced block/value rewrite is now modeled as an explicit machine rather
  than only as an exit-`Phi` equation

Primary Rust correspondence:
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:657)
  `apply_bb` materialization
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:666)
  preheader guard split into `apply_bb` / fallback
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:677)
  exit merge using scalar loads plus vector out values

Concrete Rust regressions:
- [enabled_config_certifies_simple_map_and_plan_applies_transactionally](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:2577)
- [enabled_config_certifies_simple_expr_map_and_plan_applies_transactionally](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:2606)
- [enabled_config_certifies_simple_cond_map_and_plan_applies_transactionally](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/proof.rs:2825)

Current gap:
- proof still uses a reduced machine with pre-computed scalar/vector slots; it
  does not yet model concrete MIR value ids, load nodes, or reachable-use
  rewriting

## Vectorize Value Rewrite

Proof layers:
- [VectorizeValueRewriteSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VectorizeValueRewriteSubset.lean:1)
- [VectorizeValueRewriteSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VectorizeValueRewriteSubset.v:1)

Core proof claim:
- recursively rewriting exit-region `Load var` uses with a replacement
  expression preserves return meaning whenever the replacement evaluates to the
  same scalar value as the original load

Primary Rust correspondence:
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:121)
  `rewrite_reachable_value_uses_for_var_after`
- [analysis_vectorization.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/analysis_vectorization.rs:118)
  `rewrite_returns_for_var`

Concrete Rust correspondence points:
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:702)
  exit-region reachable-use rewrite after exit-`Phi` creation
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:704)
  return rewrite when no later assignment to the destination survives

Current gap:
- proof is scalar-expression-level only; it does not yet model concrete MIR
  value ids, memoization, or cycle-breaking in the production tree rewrite

## Vectorize Use Rewrite

Proof layers:
- [VectorizeUseRewriteSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VectorizeUseRewriteSubset.lean:1)
- [VectorizeUseRewriteSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VectorizeUseRewriteSubset.v:1)

Core proof claim:
- the scalar load-rewrite theorem is lifted to id-tagged reachable use sets
- rewriting all reachable uses after the exit preserves their meanings
  pointwise

Primary Rust correspondence:
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:121)
  `rewrite_reachable_value_uses_for_var_after`

Concrete Rust correspondence points:
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:141)
  memoized reachable-use rewriting
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:148)
  `Load var` / `origin_var` rewrite boundary

Current gap:
- proof now carries explicit ids, but still abstracts away concrete MIR value
  allocation and memo-table behavior

## Vectorize Origin/Memo

Proof layers:
- [VectorizeOriginMemoSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VectorizeOriginMemoSubset.lean:1)
- [VectorizeOriginMemoSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VectorizeOriginMemoSubset.v:1)

Core proof claim:
- exact `Load var` roots stay anchored
- non-load nodes carrying `origin_var = var` redirect to the replacement
  boundary
- memo hits reuse the existing rewritten value id
- unchanged rewrites reuse the original id, while changed rewrites may use a
  fresh id

Primary Rust correspondence:
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:141)
  memo hit reuse
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:150)
  `origin_var` boundary behavior
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:164)
  fresh value-id allocation on changed rewrites

Current gap:
- proof isolates the local decision logic, but still does not model the full
  recursive tree walk and allocation sequence together

## Vectorize Decision

Proof layers:
- [VectorizeDecisionSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VectorizeDecisionSubset.lean:1)
- [VectorizeDecisionSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VectorizeDecisionSubset.v:1)

Core proof claim:
- the local decision step for
  - `origin_var` boundary handling
  - memo-hit reuse
  - fresh-id allocation
  - reachable-use rewriting
  can be composed into one reduced rewrite decision without changing scalar use
  meaning

Primary Rust correspondence:
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:141)
  memo/origin local decision point
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:164)
  fresh value-id allocation on changed rewrites
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:121)
  reachable-use rewrite entry point

Current gap:
- proof composes the local contracts, but still abstracts away the full
  recursive traversal order and concrete mutable state updates

## Vectorize Tree Rewrite

Proof layers:
- [VectorizeTreeRewriteSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VectorizeTreeRewriteSubset.lean:1)
- [VectorizeTreeRewriteSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VectorizeTreeRewriteSubset.v:1)

Core proof claim:
- the local vectorize rewrite decision is lifted into a reduced recursive tree
  rewrite with explicit traversal order and allocation state
- sample properties cover
  - unchanged roots reusing their original ids
  - changed trees allocating a fresh id
  - scalar evaluation staying unchanged after the rewrite

Primary Rust correspondence:
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:134)
  recursive tree rewrite entry
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:141)
  memo reuse during traversal
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:164)
  fresh-id allocation on changed rewrites

Current gap:
- proof now has traversal order and allocation state, but remains sample-driven
  rather than a full generic proof over the entire reduced tree space

## Vectorize Allocation State

Proof layers:
- [VectorizeAllocStateSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VectorizeAllocStateSubset.lean:1)
- [VectorizeAllocStateSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VectorizeAllocStateSubset.v:1)

Core proof claim:
- multiple rewritten trees can be threaded through a single allocation state
- fresh ids and scalar meanings compose correctly across a list of reachable
  roots

Primary Rust correspondence:
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:193)
  repeated recursive rewriting across field lists / argument lists
- [transform.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/v_opt/transform.rs:262)
  repeated argument rewriting in calls / intrinsics

Current gap:
- proof now carries allocation state across multiple roots, but still remains
  sample-driven rather than generic over arbitrary reachable root sets

## Recommended Next Extensions

1. Lift `GvnSubset` from expression equality to block-local availability and
   dominance.
2. Lift `InlineSubset` from helper shapes to reduced caller/callee CFGs.
3. Lift `DeSsaSubset` from “no move needed” to reduced parallel-copy soundness.
4. Lift `DceSubset` from effect-count preservation to reduced evaluation trace
   preservation.
5. Lift `VectorizeAllocStateSubset` from sample-driven multi-root allocation
   state to a generic reduced theorem over arbitrary reachable root sets.
