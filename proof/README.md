# RR Proof Workspace

This directory contains a pragmatic formal-methods workspace for RR.

It does not claim to fully verify the entire compiler. That would require a
much larger effort across parsing, typing, MIR lowering, optimization, and R
code generation.

Instead, this workspace formalizes one concrete safety slice of the current
compiler:

- loop-carried state must not be treated as loop-invariant by LICM
- expressions whose free variables are disjoint from loop-written variables may
  be hoisted safely
- expressions that depend on loop-carried variables, such as `time + dt`, are
  not hoistable

This directly matches the recent RR regression where LICM incorrectly hoisted a
loop-carried update.

## Claim Boundary

The current workspace supports the following claim:

- the optimizer path now has a continuous **reduced** proof spine from
  pass-local rewrites up through phase ordering, program-level budgeting, the
  `run_program_with_profile_inner` wrapper, the public `run_program*` shell,
  and a reduced compiler-level theorem

The workspace does **not** currently support the following stronger claim:

- that the production Rust compiler is proven line-by-line or pass-by-pass in a
  1:1 mechanized sense
- that the full production source-level trait solver is formally verified.
  `TraitDispatchSoundness` models only the reduced target-preservation spine for
  static trait dispatch and monomorphized calls, plus reduced lemmas for
  negative-impl exclusion, operator-to-trait mapping, and public trait metadata
  filtering. Associated-type substitution, default method materialization,
  supertrait obligations, impl-coherence checks, exact-over-generic
  specialization, explicit turbofish calls, generic impl matching, and full
  cross-module cache replay remain implementation features covered primarily by
  Rust regression tests today. The proof claim also does not cover Rust-level
  heterogeneous `dyn Trait` vtables, borrow/lifetime or HRTB solving, full GAT
  projection normalization, arbitrary const-generic evaluation, or Rust's
  unstable specialization semantics.

The exact split between

- proved
- approximated
- not modeled

is tracked in
[optimizer_proof_gap_audit.md](/Users/feral/Desktop/Programming/RR/proof/optimizer_proof_gap_audit.md:1).

## Layout

- `lean/`
  - Lean 4 mechanization
- `coq/`
  - Coq mechanization
- `optimizer_correspondence.md`
  - reduced optimizer proof ↔ Rust implementation mapping
- `optimizer_proof_gap_audit.md`
  - reduced optimizer proof vs production optimizer gap audit
- `runtime_safety_correspondence.md`
  - reduced range/runtime-safety proof ↔ Rust implementation mapping
- `pipeline_correspondence.md`
  - reduced lowering/codegen/pipeline proof ↔ Rust implementation mapping
- `verify_correspondence.md`
  - reduced verifier proof ↔ Rust implementation mapping

## Optimizer Soundness Spine

The workspace now carries a reduced optimizer-only MIR soundness spine:

- `MirSemanticsLite`
  - reduced MIR execution object shared by future optimizer proofs
- `MirInvariantBundle`
  - reduced optimizer-eligibility / well-formedness bundle

These files define the shared semantic and invariant objects used by the
reduced optimizer theorem family. The theorem family is continuous through the
optimizer and public wrapper layers, but it is still a reduced model rather than
a line-by-line mechanization of the production Rust optimizer.

- in a `DataflowOptSoundness` layer, reduced expression canonicalization,
  reduced constant propagation under environment agreement, and a reduced
  last-dead-assign elimination step for straight-line blocks are proved
  semantics-preserving on top of `MirSemanticsLite`
- in a `CfgOptSoundness` layer, a reduced multi-block runner is introduced and
  two CFG-style transformations are modeled directly:
  appending a dead unreachable block preserves the invariant bundle, and
  retargeting an empty entry `goto` preserves the reduced MIR execution result
- in a `LoopOptSoundness` layer, the existing reduced LICM graph/small-step
  soundness results are lifted into the optimizer-only spine, and actual
  reduced rewrite theorems are fixed for BCE/TCO slots:
  redundant in-bounds bounds-check elimination preserves the same read result,
  and a reduced tail-recursive step function is equivalent to its loop-style
  form
- in a `DeSsaBoundarySoundness` layer, the reduced `DeSsaSubset` theorem is
  turned into an explicit stage-boundary proof for redundant move elimination
- in an `OptimizerPipelineSoundness` layer, the reduced dataflow / cfg / loop
  layers are composed into optimizer-wide theorem names, now including
  `program_post_dessa_*` and `prepare_for_codegen_*` stage families
  `optimizer_pipeline_preserves_verify_ir` and
  `optimizer_pipeline_preserves_semantics`
- in a `PhaseOrderOptimizerSoundness` layer, reduced phase-order schedule
  profiles (`balanced`, `compute-heavy`, `control-flow-heavy`) are fixed as a
  theorem family over the optimizer pipeline
- in a `PhaseOrderClusterSoundness` layer, reduced `structural / standard /
  cleanup` subcluster theorem names are fixed so that phase-order proofs can
  reference the same internal schedule boundaries as `phase_order.rs`
- in a `PhaseOrderGuardSoundness` layer, reduced guard records are fixed for
  `run_budgeted_passes`, structural gates, control-flow structural gates, and
  fast-dev vectorization gates so that phase-order schedule proofs can follow
  the same enable/skip boundary style as Rust
- in a `PhaseOrderFeatureGateSoundness` layer, reduced feature records are
  fixed for branch density, canonical loop count, side-effect ratios, and
  fast-dev vectorization limits, and those feature gates are tied directly to
  cluster selection lemmas
- in a `PhaseOrderIterationSoundness` layer, reduced iteration-entry theorems
  are fixed for `balanced`, `compute-heavy`, `control-flow-heavy`, and
  `fast-dev` subpaths so the proof spine can name the same heavy-iteration
  entrypoints that Rust `phase_order.rs` dispatches through
- in a `PhaseOrderFallbackSoundness` layer, the reduced
  `control_flow_should_fallback_to_balanced` predicate is fixed as an explicit
  fallback theorem boundary from the control-flow-heavy path back to the
  balanced path
- in a `PhasePlanSoundness` layer, reduced `classify -> choose schedule ->
  build function plan` theorems are fixed so the proof spine can name the same
  plan-selection boundary as Rust `build_function_phase_plan_from_features`
- in a `PhasePlanCollectionSoundness` layer, reduced collection/eligibility
  theorems are fixed for missing-function skips, conservative/self-recursive
  skips, selected-function filtering, and per-plan preservation after
  `collect_function_phase_plans`
- in a `PhasePlanLookupSoundness` layer, reduced lookup theorems are fixed for
  retrieving a collected plan by function id and reusing the selected schedule
  soundness at the actual `plans.get(name)` consumption boundary
- in a `PhasePlanSummarySoundness` layer, reduced summary-entry theorems are
  fixed for ordered plan consumption and lookup-hit/miss summary emission at
  the `plan_summary_lines()` boundary
- in a `ProgramOptPlanSoundness` layer, reduced program-budget theorems are
  fixed for under-budget all-safe selection, over-budget selective mode, and
  fallback-to-smallest selection at `build_opt_plan_with_profile()`
- in a `ProgramPhasePipelineSoundness` layer, reduced program-level composition
  theorems are fixed for `ProgramOptPlan -> selected_functions ->
  collect_function_phase_plans -> plan_summary` so the proof spine names the
  same heavy-tier plan flow as `run_program_with_profile_inner()`
- in a `ProgramTierExecutionSoundness` layer, reduced per-function heavy-tier
  execution theorems are fixed for conservative/self-recursive skips,
  heavy-tier-disabled or budget skips, collected-plan hits, and legacy-plan
  fallback inside `run_program_with_profile_inner()`
- in a `ProgramPostTierStagesSoundness` layer, reduced tail-stage theorems are
  fixed for inline cleanup, fresh-alias, and de-ssa/post-cleanup so the proof
  spine names the remaining post-heavy stages inside `run_program_with_profile_inner()`
- companion actual reduced rewrite files now pin two previously identity-like
  stage slices more directly:
  - `InlineCleanupRefinementSoundness` for entry-retarget cleanup
  - `FreshAliasRewriteSoundness` for alias-rename preservation
- in a `ProgramRunProfileInnerSoundness` layer, the reduced program-level
  wrapper theorems compose always-tier, heavy-tier execution, plan summary, and
  post-tier stages into one `run_program_with_profile_inner()`-shaped boundary
- in a `ProgramApiWrapperSoundness` layer, the public `run_program*` shell is
  tied directly to the inner wrapper theorem family
- in a `CompilerEndToEndSoundness` layer, the optimizer spine is paired with a
  reduced frontend/backend observable theorem to expose one reduced
  compiler-level preservation statement; the Lean side reuses
  `PipelineStmtSubset`, while the Coq side uses a tiny self-contained expression
  model

## Checked Theorems

Both Lean 4 and Coq currently prove:

- updating a variable that does not occur free in an expression preserves the
  expression's value
- applying a list of such irrelevant updates also preserves the expression's
  value
- if a candidate hoist expression is disjoint from the body write-set and the
  hoisted temporary is fresh, hoisting across a concrete body-update trace is
  semantics-preserving
- loop-local `Phi` values depend on carried state after the entry iteration
- any expression built from such a `Phi` remains non-hoistable
- in a tiny MIR subset with assignments and a loop body, expressions with empty
  carried deps and a local-dependency set disjoint from the body write-set are
  hoist-safe over that body
- in a tiny function model with a hoisted temp and loop body, the hoisted and
  non-hoisted executions are equivalent under the same sufficient condition
- a concrete `phi(time0, time)` loop-carried example is proved unsound to hoist
- in a tiny CFG-style loop wrapper with preheader/header/body/exit structure,
  zero-trip and one-trip hoist equivalence are proved under the same condition
- in a reduced `FnIR`-style record with blocks and terminators, the same
  zero-trip / one-trip soundness is lifted one layer closer to RR MIR
- in a deterministic small-step CFG machine, 3-step original and hoisted runs
  are equivalent under the same safety condition
- in an SSA/Phi graph model, loop header entry/latch arm selection is modeled
  directly and self-backedge phis are proved non-invariant when entry and
  carried values differ
- in a predecessor-graph model of a well-formed loop header, entry/latch phi
  arm selection is tied directly to graph structure
- in a graph-level LICM soundness packaging layer, reduced `FnIR`, CFG
  execution, and predecessor-graph phi facts are composed into one reusable
  case record
- in an RR-style well-formedness layer, graph-level LICM facts are lifted under
  explicit header-pred and body-shape assumptions closer to `verify_ir`
- in a `LoweringSubset` layer, a small source expression fragment
  (`const / unary neg / binary add / record literal / field access`) is lowered
  to a MIR-like target expression language and proved semantics-preserving
- in a `LoweringIfPhiSubset` layer, a source `if` expression is lowered to a
  target `phi-join` form, with concrete Coq branch regressions for true/false
  and nested record-field cases
- a reduced Coq `LoweringIfPhiGenericSubset` companion now carries the generic
  `if -> phi` lowering theorems in a separate stable file
- in a `LoweringLetSubset` layer, reduced `let / local read / local field /
  local add` expressions are lowered to a MIR-like local-binding language and
  proved semantics-preserving, including nested field chains
- in a `LoweringAssignPhiSubset` layer, reduced branch-local reassignment is
  lowered to a `phi`-style merged binding and proved semantics-preserving,
  including merged record-field reads
- in a `CodegenSubset` layer, a reduced MIR expression fragment is emitted into
  an R-like expression language and proved semantics-preserving, including
  nested list-field reads
- a reduced Coq `CodegenGenericSubset` companion now carries the generic
  codegen theorems in a separate stable file
- in a `GvnSubset` layer, a reduced MIR expression fragment is canonicalized
  with a small GVN-style normalizer covering commutative `add`, a reduced
  intrinsic-abs wrapper, and `fieldset -> field` reads, and both
  canonicalization itself and canonical-form CSE are proved
  semantics-preserving
- in a `RuntimeSafetyFieldRangeSubset` layer, reduced record-field interval
  propagation is modeled directly, showing that negative singleton intervals
  survive plain field reads, nested field reads, and negative fieldset
  overrides, while positive overrides clear the `< 1` runtime-safety hazard
- in an `InlineSubset` layer, a reduced value-call inliner for pure helper
  shapes (`arg`, `field`, `field + const`) is proved semantics-preserving
- in a `DeSsaSubset` layer, a reduced predecessor-copy matcher is modeled with
  canonical fingerprints, showing that structurally identical incoming values
  can reuse an existing predecessor assignment without adding a redundant move
- in a `DceSubset` layer, a reduced dead-code eliminator is modeled over
  nested wrappers, proving that pure dead assigns erase while effectful dead
  assigns demote to `eval` and preserve total nested side effects
- in a `VectorizeSubset` layer, reduced loop and branch classifiers prove the
  safety bar used by vectorization certification: expression maps reject
  effectful loop bodies, and conditional map/reduction branches accept only
  store-only shapes
- in a `VectorizeApplySubset` layer, a reduced transactional vectorization
  apply step is modeled, proving the key contract used by the Rust optimizer:
  rejected plans roll back to the scalar original, while certified
  result-preserving plans may commit without changing the scalar result
- in a `VectorizeRewriteSubset` layer, the reduced exit-`Phi` merge created by
  vectorized preheader/apply rewrites is modeled explicitly, proving that both
  fallback and result-preserving apply paths rejoin to the original scalar exit
  value
- in a `VectorizeMirRewriteSubset` layer, a tiny MIR machine with
  preheader/apply/fallback/exit blocks is stepped explicitly, proving that the
  reduced rewrite preserves the original scalar result across the whole block
  sequence
- in a `VectorizeValueRewriteSubset` layer, exit-region load rewriting is
  modeled explicitly, proving that recursively replacing `Load var` with a
  replacement expression preserves return meaning whenever the replacement
  evaluates to the same scalar value as the original load
- in a `VectorizeOriginMemoSubset` layer, the `origin_var` boundary, memo-hit
  reuse, and fresh-id allocation contracts used by the rewrite are modeled
  directly
- in a `VectorizeUseRewriteSubset` layer, that load-rewrite theorem is lifted
  to id-tagged reachable use sets, proving that rewriting all reachable uses
  after the exit preserves their scalar meanings pointwise
- in a `VectorizeDecisionSubset` layer, the local decision logic for
  `origin_var` boundaries, memo reuse, fresh-id allocation, and reachable-use
  rewriting is composed into one reduced decision step
- in a `VectorizeTreeRewriteSubset` layer, that decision logic is lifted into a
  reduced recursive tree rewrite with explicit traversal order and allocation
  state, proving scalar evaluation is preserved across the whole tree rewrite
- in a `VectorizeAllocStateSubset` layer, multiple rewritten trees are threaded
  through one allocation state, proving fresh ids and scalar meanings compose
  correctly across a list of reachable roots
- in a `VectorizeGraphSubset` layer, the reduced MIR rewrite machine and the
  reduced exit-region load/return rewrite are composed, proving end-to-end
  scalar return preservation for fallback and result-preserving apply paths
- a reduced Coq `CodegenSubset` companion currently covers concrete codegen
  regressions for `const / add / record field / nested field`
- in a `PipelineIfPhiSubset` layer, reduced `if -> phi -> R-like codegen`
  composition is proved semantics-preserving end-to-end
- a reduced Coq `PipelineIfPhiSubset` companion currently covers concrete
  `if -> phi -> codegen` regressions for pure/true/false/nested-field cases
- a reduced Coq `PipelineIfPhiGenericSubset` companion now carries a generic
  `lower -> emit` theorem for the reduced `if -> phi -> codegen` pipeline
- a reduced Coq `PipelineLetSubset` companion currently covers concrete
  `let / field / nested field / add -> codegen` regressions, and now also
  exposes a generic `lower -> emit` theorem built from Coq lowering/codegen
  subsets
- a reduced Coq `PipelineLetGenericSubset` companion now carries that generic
  `lower -> emit` theorem in a separate stable file
- a reduced Coq `PipelineAssignPhiSubset` companion currently covers concrete
  branch-assignment regressions for local / field / nested-field cases, and
  now also exposes a generic reduced assign-phi codegen theorem
- a reduced Coq `PipelineAssignPhiGenericSubset` companion now carries that
  generic reduced assign-phi theorem in a separate stable file
- a reduced Coq `PipelineStmtSubset` companion currently covers concrete
  straight-line and branch-assignment program regressions, and now also exposes
  generic stmt/program codegen theorems
- a reduced Coq `PipelineStmtGenericSubset` companion now carries the generic
  stmt/program codegen theorems in a separate stable file
- a reduced Coq `PipelineCfgSubset` companion currently covers concrete
  `then/else/join`-style CFG regressions, and now also exposes a generic
  reduced CFG codegen theorem
- a reduced Coq `PipelineCfgGenericSubset` companion now carries the generic
  `lower -> emit` theorem for the reduced CFG pipeline in a separate stable
  file
- in a `PipelineLetSubset` layer, reduced `let / local field / nested field`
  lowering and R-like codegen are proved semantics-preserving end-to-end
- in a `PipelineAssignPhiSubset` layer, reduced branch-assignment with
  `phi`-merged locals and record-field reads is proved semantics-preserving
  through R-like codegen end-to-end
- in a `PipelineStmtSubset` layer, reduced straight-line statements and
  branch-assignment statements are proved semantics-preserving through
  lowering and R-like codegen at a small program/block level
- in a `PipelineCfgSubset` layer, a tiny explicit `then/else/join` CFG-style
  program is proved semantics-preserving through lowering and R-like codegen
- in a `PipelineBlockEnvSubset` layer, a reduced explicit block shell with
  block id, incoming local environment, ordered statements, and return
  expression is proved semantics-preserving through lowering and R-like codegen
- in a `PipelineFnEnvSubset` layer, a reduced explicit function shell with
  function metadata plus an ordered list of block/env shells is shown to
  preserve per-block results through lowering and R-like codegen
- in a `PipelineFnCfgSubset` layer, that reduced function shell is refined
  again with a predecessor map, bringing the proof one step closer to a real
  `FnIR` block graph while still preserving the reduced per-block results
- in a `PipelineFnCfgExecSubset` layer, that predecessor-aware function shell
  is refined again with an execution path witness, showing that reduced
  selected-path results are preserved through lowering and R-like codegen
- in a `PipelineFnCfgSmallStepSubset` layer, that selected-path witness is
  refined again into a tiny small-step trace machine, showing that reduced CFG
  execution traces are preserved through lowering and R-like codegen
- in a `PipelineFnCfgBranchExecSubset` layer, that selected-path story is
  refined again with an explicit branch choice over `then` / `else` paths,
  showing that chosen branch traces are preserved through lowering and R-like
  codegen
- in a `PipelineFnCfgPhiExecSubset` layer, that explicit branch-choice story
  is refined again with a reduced `phi`/join merge result, showing that the
  chosen merged result is preserved through lowering and R-like codegen
- in a `PipelineFnCfgJoinStateSubset` layer, that reduced `phi`/join merge
  result is threaded into an explicit join-local environment, showing that
  join-state results are preserved through lowering and R-like codegen
- in a `PipelineFnCfgJoinExecSubset` layer, that join-local state is refined
  again into explicit join block execution (`join stmts + join ret`), showing
  that join-block results are preserved through lowering and R-like codegen
- in a `PipelineFnCfgPostJoinSubset` layer, that explicit join-block result is
  threaded once more into a follow-up continuation block, showing that
  post-join continuation results are preserved through lowering and R-like
  codegen
- in a `PipelineFnCfgIterExecSubset` layer, that post-join continuation story
  is refined again into an ordered list of continuation blocks, showing that a
  reduced iterative post-join execution preserves the final continuation
  result through lowering and R-like codegen
- in a `PipelineFnCfgControlStateSubset` layer, that ordered continuation
  story is refined again into an explicit control-state transition machine,
  showing that reduced whole-CFG post-join transitions preserve the final
  continuation result through lowering and R-like codegen
- in a `PipelineFnCfgGraphStateSubset` layer, that control-state story is
  refined again into an explicit `pc + step-table` graph shell, showing that
  reduced graph-state execution preserves the final continuation result
  through lowering and R-like codegen
- in a `PipelineFnCfgReentrySubset` layer, that graph-state story is refined
  again into an explicit re-entry trace over step indices, showing that
  reduced revisit/cycling executions preserve the final continuation result
  through lowering and R-like codegen
- in a `PipelineFnCfgLoopCycleSubset` layer, that explicit re-entry story is
  refined again into repeated branch/join cycles with an accumulator-like
  loop state, showing that reduced cycle iteration preserves the final result
  through lowering and R-like codegen
- in a `PipelineFnCfgLoopFixpointSubset` layer, that repeated cycle story is
  refined again with an explicit stability witness, showing that a reduced
  fixed-point condition is preserved through lowering and R-like codegen
- in a `PipelineFnCfgLoopDiscoverSubset` layer, that fixed-point witness story
  is refined again into an explicit worklist/selection shell, showing that a
  reduced discovery result is preserved through lowering and R-like codegen
- in a `PipelineFnCfgLoopWorklistSubset` layer, that selection story is
  refined again into an explicit `pending -> done` update shell, showing that
  reduced worklist state updates are preserved through lowering and R-like
  codegen
- in a `PipelineFnCfgLoopQueueSubset` layer, that single-update story is
  refined again into an ordered queue drain shell, showing that reduced
  multi-round worklist updates are preserved through lowering and R-like
  codegen
- in a `PipelineFnCfgLoopSchedulerSubset` layer, that queue drain story is
  refined again into an ordered batch scheduler shell, showing that reduced
  multi-batch worklist execution is preserved through lowering and R-like
  codegen
- in a `PipelineFnCfgLoopDynamicSchedulerSubset` layer, that scheduler story
  is refined again with explicit reinsertion batches, showing that reduced
  dynamic scheduling preserves the resulting batch-evaluation trace through
  lowering and R-like codegen
- in a `PipelineFnCfgLoopPrioritySubset` layer, that dynamic scheduler story
  is refined again with explicit priority tags, showing that reduced
  priority-labeled scheduling traces are preserved through lowering and
  R-like codegen
- in a `PipelineFnCfgLoopPolicySubset` layer, that priority-labeled story is
  refined again with explicit priority-rewrite rules, showing that reduced
  policy-normalized scheduling traces are preserved through lowering and
  R-like codegen
- in a `PipelineFnCfgLoopAdaptivePolicySubset` layer, that policy-normalized
  story is refined again with explicit feedback-driven rule recomputation,
  showing that reduced adaptive policy traces are preserved through lowering
  and R-like codegen
- in a `PipelineFnCfgLoopClosedLoopSubset` layer, that adaptive policy story
  is refined again into repeated feedback rounds, showing that reduced
  closed-loop adaptive traces are preserved through lowering and R-like
  codegen
- in a `PipelineFnCfgLoopMetaIterSubset` layer, that closed-loop story is
  refined again into a last-summary/fixed-point discovery shell, showing that
  reduced meta-iteration summaries are preserved through lowering and R-like
  codegen
- in a `PipelineFnCfgSummaryProtocolSubset` layer, that meta-iteration story is
  refined again into an explicit summary-protocol shell, showing that reduced
  stable-summary traces are preserved through lowering and R-like codegen
- in a `PipelineFnCfgConvergenceProtocolSubset` layer, that summary-protocol
  story is refined again with an explicit `summary unchanged => halt` witness,
  showing that reduced convergence-protocol results are preserved through
  lowering and R-like codegen
- in a `PipelineFnCfgHaltDiscoverSubset` layer, that convergence-protocol
  story is refined again with an explicit halt-search shell, showing that
  reduced discovered-halt summaries are preserved through lowering and
  R-like codegen
- in a `PipelineFnCfgOpenSearchSubset` layer, that halt-discovery story is
  refined again into an explicit `completed + frontier` worklist shell,
  showing that reduced open-ended search/worklist discovery results are
  preserved through lowering and R-like codegen
- in a `PipelineFnCfgDynamicOpenSearchSubset` layer, that open-search story is
  refined again with explicit frontier growth, showing that reduced dynamic
  open-search results are preserved through lowering and R-like codegen
- in a `PipelineFnCfgOpenSearchSchedulerSubset` layer, that dynamic
  open-search story is refined again into repeated scheduled rounds, showing
  that reduced open-search scheduler results are preserved through lowering
  and R-like codegen
- in a `PipelineFnCfgOpenSearchPrioritySubset` layer, that scheduler story is
  refined again with explicit priority-labeled rounds, showing that reduced
  open-search priority results are preserved through lowering and R-like
  codegen
- in a `PipelineFnCfgOpenSearchPolicySubset` layer, that priority story is
  refined again with explicit priority-rewrite rules, showing that reduced
  open-search policy-normalized results are preserved through lowering and
  R-like codegen
- in a `PipelineFnCfgOpenSearchAdaptivePolicySubset` layer, that policy story
  is refined again with feedback-driven rule recomputation, showing that
  reduced adaptive open-search policy results are preserved through lowering
  and R-like codegen
- in a `PipelineFnCfgOpenSearchClosedLoopSubset` layer, that adaptive policy
  story is refined again into repeated closed-loop rounds, showing that
  reduced closed-loop open-search traces are preserved through lowering and
  R-like codegen
- in a `PipelineFnCfgOpenSearchMetaIterSubset` layer, that closed-loop story
  is refined again into a last-summary extraction shell, showing that reduced
  open-search meta-iteration summaries are preserved through lowering and
  R-like codegen
- in a `PipelineFnCfgOpenSearchSummaryProtocolSubset` layer, that
  meta-iteration story is refined again into an explicit summary-protocol
  shell, showing that reduced stable-summary traces are preserved through
  lowering and R-like codegen
- in a `PipelineFnCfgOpenSearchConvergenceProtocolSubset` layer, that
  summary-protocol story is refined again with an explicit
  `summary unchanged => halt` witness, showing that reduced convergence
  results are preserved through lowering and R-like codegen
- in a `PipelineFnCfgOpenSearchHaltDiscoverSubset` layer, that convergence
  story is refined again with an explicit halt-search shell, showing that
  reduced discovered-halt summaries are preserved through lowering and
  R-like codegen
- in a `PipelineFnCfgOpenSearchFrontierSubset` layer, that halt-discovery
  story is refined again into an explicit `completed + frontier` shell,
  showing that reduced reopened open-search frontier results are preserved
  through lowering and R-like codegen
- in a `PipelineFnCfgOpenSearchDynamicFrontierSubset` layer, that reopened
  frontier story is refined again with explicit frontier growth, showing that
  reduced dynamic frontier results are preserved through lowering and
  R-like codegen
- in a `PipelineFnCfgOpenSearchFrontierSchedulerSubset` layer, that dynamic
  frontier story is refined again into repeated scheduled rounds, showing
  that reduced reopened-frontier scheduler results are preserved through
  lowering and R-like codegen
- in a `PipelineFnCfgOpenSearchFrontierPrioritySubset` layer, that scheduler
  story is refined again with explicit priority-labeled reopened-frontier
  rounds, showing that reduced frontier-priority results are preserved
  through lowering and R-like codegen
- in a `PipelineFnCfgOpenSearchFrontierPolicySubset` layer, that priority
  story is refined again with explicit priority-rewrite rules, showing that
  reduced frontier-policy results are preserved through lowering and
  R-like codegen
- in a `PipelineFnCfgOpenSearchFrontierAdaptivePolicySubset` layer, that
  frontier-policy story is refined again with feedback-driven rule
  recomputation, showing that reduced adaptive frontier-policy results are
  preserved through lowering and R-like codegen
- in a `PipelineFnCfgOpenSearchFrontierClosedLoopSubset` layer, that
  frontier-adaptive-policy story is refined again into repeated closed-loop
  rounds, showing that reduced reopened-frontier traces are preserved through
  lowering and R-like codegen
- in a `PipelineFnCfgOpenSearchFrontierMetaIterSubset` layer, that
  frontier-closed-loop story is refined again into a last-summary extraction
  shell, showing that reduced reopened-frontier meta-iteration summaries are
  preserved through lowering and R-like codegen
- in a `PipelineFnCfgOpenSearchFrontierSummaryProtocolSubset` layer, that
  frontier meta-iteration story is refined again into an explicit
  summary-protocol shell, showing that reduced reopened-frontier stable
  summary traces are preserved through lowering and R-like codegen
- in a `PipelineFnCfgOpenSearchFrontierConvergenceProtocolSubset` layer, that
  frontier summary-protocol story is refined again with an explicit
  `summary unchanged => halt` witness, showing that reduced reopened-frontier
  convergence results are preserved through lowering and R-like codegen
- in a `PipelineFnCfgOpenSearchFrontierHaltDiscoverSubset` layer, that
  frontier convergence story is refined again with an explicit halt-search
  shell, showing that reduced reopened-frontier discovered-halt summaries are
  preserved through lowering and R-like codegen
- in a `PipelineFnCfgOpenSearchFrontierReopenSubset` layer, that
  frontier halt-discovery story is refined again into an explicit
  `completed + frontier` reopen shell, showing that reduced reopened-frontier
  reopen results are preserved through lowering and R-like codegen
- in a `PipelineFnCfgOpenSearchFrontierReopenDynamicSubset` layer, that
  reopened frontier story is refined again with explicit frontier growth,
  showing that reduced reopened-frontier dynamic results are preserved
  through lowering and R-like codegen
- in a `PipelineFnCfgOpenSearchFrontierReopenSchedulerSubset` layer, that
  reopened-frontier dynamic-growth story is refined again into an explicit
  scheduled-round shell, showing that reduced reopened-frontier scheduler
  results are preserved through lowering and R-like codegen
- in a `PipelineFnCfgOpenSearchFrontierReopenPrioritySubset` layer, that
  reopened-frontier scheduler story is refined again with explicit priority
  labels, showing that reduced reopened-frontier priority results are
  preserved through lowering and R-like codegen
- in a `PipelineFnCfgOpenSearchFrontierReopenPolicySubset` layer, that
  reopened-frontier priority story is refined again with explicit policy
  rewrite rules, showing that reduced reopened-frontier policy-normalized
  results are preserved through lowering and R-like codegen
- in a `VerifyLite` layer, the main obligations are packaged as a single
  reduced validator: safe candidates preserve results, self-backedge phis are
  rejected by semantic non-invariance
- in a `VerifyIrLite` layer, a reduced error algebra mirrors key
  `verify_ir`-style failures: `UndefinedVar`, `InvalidPhiSource`, and
  `ReachablePhi`
- in a `VerifyIrStructLite` layer, structural phi ownership is also checked:
  `body_head` must be reachable from `entry`,
  if `body_head != entry` then `entry` must jump directly to it,
  and that entry prologue must remain param-copy-only,
  `entry` itself must remain predecessor-free as the unique CFG root,
  `entry` must also be executable rather than `Unreachable`,
  branch targets must remain distinct for real CFG splits,
  loop headers must split into exactly one body successor and one exit successor,
  `Phi` owners must live in blocks that actually have predecessors to merge,
  and those predecessor arms must be genuinely distinct rather than aliased to
  a single incoming edge,
  `Phi` operands must also be edge-available rather than current-block `Phi`
  definitions,
  `Phi` values must have an owning block, and non-`Phi` values must not carry
  `phi_block` metadata, any `Phi` owner block must itself be valid, and
  `Param` values must reference an in-range parameter index, while `Call`
  values must not carry more argument labels than positional arguments; direct
  self-references and non-`Phi` dependency cycles are also rejected as
  malformed value-graph structure
- in a `VerifyIrValueEnvSubset` layer, explicit `ValueId`/`BlockId`-indexed
  environments are used to model `Phi` edge selection directly, showing that
  rewriting a consumer from a merged `Phi` id to the predecessor-selected
  source id preserves evaluation
- in a `VerifyIrMustDefSubset` layer, reduced predecessor-intersection and
  local-assign extension are modeled directly, showing that a variable present
  in every predecessor out-set remains must-defined at the join and therefore
  avoids reduced `UseBeforeDef` rejection
- in a `VerifyIrMustDefFixedPointSubset` layer, that must-defined reasoning is
  lifted into an explicit reduced CFG-step/iteration model with reachable
  predecessor filtering, showing that one reduced fixed-point step preserves
  join must-defined facts into the next out-set map
- in a `VerifyIrMustDefConvergenceSubset` layer, reduced stable out-set maps
  are shown to remain unchanged under further iteration, giving a small
  fixed-point/convergence witness closer to the Rust verifier's worklist loop
- in a `VerifyIrUseTraversalSubset` layer, reduced recursive wrapper/load
  traversal is modeled directly, showing that must-defined loads stay invisible
  to undefined-load scanning while `Phi` arguments may still be skipped or
  followed depending on the traversal mode
- in a `VerifyIrValueKindTraversalSubset` layer, that traversal is refined into
  reduced `ValueKind`-named cases such as `Intrinsic`, `RecordLit`,
  `FieldSet`, `Index*`, `Range`, and `Binary`, showing that concrete wrapper
  shapes also preserve the absence of undefined loads under must-defined inputs
- in a `VerifyIrArgListTraversalSubset` layer, reduced arg-list and named
  field-list traversal is modeled for `Call`, `Intrinsic`, and `RecordLit`,
  showing that must-defined inputs also keep list-argument scanning free of
  undefined loads
- in a `VerifyIrArgEnvSubset` layer, predecessor-selected `Phi` environments
  are composed with reduced arg-list and field-list consumers, showing that
  rewriting those consumers from a merged `Phi` id to the selected incoming id
  preserves list evaluation
- in a `VerifyIrArgEnvTraversalSubset` layer, that same predecessor-selected
  rewrite is lifted to reduced generic missing-use scans over arg lists and
  field lists, proving that selected-edge rewriting preserves scan cleanliness
  (`= none`) at that list-consumer level
- in a `VerifyIrEnvScanComposeSubset` layer, those env-selected scan facts are
  packaged together with the reduced `ValueKind` arg/field scan facts under a
  reusable compose-case record, and reduced generic list/field composition
  theorems now quantify directly over selected-env clean facts and value-kind
  clean facts, so the same concrete call/record examples are instances of a
  reusable bridge rather than isolated packaging
- in a `VerifyIrConsumerMetaSubset` layer, those reduced env/value-kind clean
  theorems are lifted into explicit heterogeneous consumer constructors for
  `Call`, `Intrinsic`, and `RecordLit`, bringing the proof one step closer to
  the real branch split inside `first_undefined_load_in_value`
- in a `VerifyIrConsumerGraphSubset` layer, those heterogeneous consumer
  constructors are lifted again into a reduced `node-id + seen + fuel` graph,
  so shared child consumers and wrapper parents are modeled one step closer to
  the real recursive `ValueId` traversal used by `first_undefined_load_in_value`
- in a `VerifyIrChildDepsSubset` layer, reduced child-edge extraction now
  mirrors the exact non-`Phi` helper shape used in Rust for `Const/Param/Load`
  leaves, unary wrappers, binary/range pairs, `Call/Intrinsic` arg lists,
  `RecordLit` field values, and `Index*` nodes
- in a `VerifyIrValueDepsWalkSubset` layer, that child-edge story is extended
  to full `value_dependencies`, including `Phi` arg lists, and then lifted into
  a reduced stack walk that approximates `depends_on_phi_in_block_except`
- in a `VerifyIrValueTableWalkSubset` layer, that reduced `value_dependencies`
  walk is rephrased over an explicit `ValueId -> table row` lookup model with
  stored `phi_block` metadata, bringing the proof one step closer to the real
  `FnIR.values` table used by the verifier
- in a `VerifyIrValueKindTableSubset` layer, those table rows are refined
  again to actual `ValueKind`-named payload constructors, so the table-driven
  walk no longer speaks only in generic dependency tags
- in a `VerifyIrValueRecordSubset` layer, those rows are lifted once more to a
  reduced `Value` record carrying `id`, `kind`, `origin_var`, `phi_block`, and
  `escape`, bringing the proof one step closer to the real `FnIR.values`
  entries
- in a `VerifyIrValueFullRecordSubset` layer, that reduced `Value` record is
  extended again with `span`, `facts`, `value_ty`, and `value_term`, so the
  proof row now approximates nearly all fields of the real `Value` record
- in a `VerifyIrFnRecordSubset` layer, those reduced full `Value` rows are
  finally packaged into a small `FnIR`-style record carrying `name`, `params`,
  `values`, `blocks`, `entry`, and `body_head`
- in a `VerifyIrFnMetaSubset` layer, that small `FnIR` shell is refined again
  with reduced `user_name`, return-hint, inferred-return, and
  fallback/interop metadata, while the current verifier-facing value/table walk
  theorems still project straight back to the smaller shell
- in a `VerifyIrFnParamMetaSubset` layer, that reduced function shell is
  refined again with `param_default_r_exprs`, `param_spans`, `param_ty_hints`,
  `param_term_hints`, and `param_hint_spans`, again without changing the
  current verifier-facing value/table walk theorems
- in a `VerifyIrFnHintMapSubset` layer, that reduced function shell is
  refined once more with reduced `call_semantics` and
  `memory_layout_hints` maps, still projecting the current verifier-facing
  value/table walk theorems onto the same smaller shell
- in a `VerifyIrBlockRecordSubset` layer, that reduced function shell is
  refined again with reduced `Block`/`Terminator` payloads carrying explicit
  instruction lists and terminator operands, still projecting the current
  verifier-facing value/table walk theorems onto the same smaller shell
- in a `VerifyIrBlockFlowSubset` layer, those reduced block payloads are
  connected back to `VerifyIrFlowLite` through `origin_var` lookup over the
  reduced value table, so explicit instruction/terminator operands again
  induce reduced `UseBeforeDef` obligations
- in a `VerifyIrBlockMustDefSubset` layer, that block-flow bridge is composed
  directly with the reduced must-defined chain, so a join fact such as
  `example_join_contains_x` can certify an explicit block payload as
  `UseBeforeDef`-clean
- in a `VerifyIrBlockMustDefComposeSubset` layer, that same bridge is lifted
  again to generic `required ⊆ defs -> verifyFlow = none` packaging, plus a
  multi-read block example where reduced join facts for both `x` and `y`
  certify an explicit block payload as clean
- in a `VerifyIrBlockAssignFlowSubset` layer, local `assign` writes are shown
  to satisfy later reads inside the same reduced block, so an incoming
  must-defined fact for only the source var `y` is enough to certify the
  `assign x <- y; eval x; return x` shape as clean
- in a `VerifyIrBlockAssignChainSubset` layer, that story is extended to a
  two-step local def chain, so an incoming must-defined fact for `y` alone is
  enough to certify `assign loop <- y; assign x <- loop; eval x; return x`
  as clean
- in a `VerifyIrBlockAssignBranchSubset` layer, that same local def chain is
  extended through a branch terminator, so an incoming must-defined fact for
  `y` alone is enough to certify `assign loop <- y; assign x <- loop; if x`
  as clean
- in a `VerifyIrBlockAssignStoreSubset` layer, that same local def chain is
  extended through `StoreIndex1D/2D/3D`, so an incoming must-defined fact for
  `y` alone is enough to certify store operand bundles after local writes
- in a `VerifyIrBlockDefinedHereSubset` layer, the sequential `defined_here`
  growth itself is packaged as a reusable theorem: folding `stepInstrFlow`
  over a block has the same first component as simply appending per-instruction
  writes, and incoming defs are preserved throughout the scan
- in a `VerifyIrBlockExecutableSubset` layer, those reusable block-local
  flow and `defined_here` theorems are packaged back into a single-block
  `VerifyIrFlowLite` executable theorem, so reduced block acceptance is no
  longer only example-driven
- in a `VerifyIrTwoBlockExecutableSubset` layer, that executable packaging is
  extended to an ordered two-block case, so reduced predecessor-selected
  `in_defs` facts and block-local acceptance theorems compose beyond one block
- in a `VerifyIrJoinExecutableSubset` layer, that packaging is extended again
  to a join-shaped three-block case with left/right sibling blocks and a join
  block, so reduced predecessor-selected `in_defs` facts compose across the
  small ordered bundle
- in a `VerifyIrCfgExecutableSubset` layer, that join packaging is lifted into
  an explicit CFG witness record carrying `joinPreds` and `blockOrder`, so the
  proof-side acceptance story speaks in a small predecessor/order shell rather
  than only positional arguments
- in a `VerifyIrCfgReachabilitySubset` layer, that CFG witness is tied one step
  closer to reduced `reachable/preds/outDefs` data, so the join block's
  incoming defs can be justified directly via reduced `stepInDefs`
- in a `VerifyIrFlowLite` layer, block-local/use-path obligations are packaged
  as reduced `UseBeforeDef` checks: required loads must already be in the
  current defined set before an instruction or terminator may use them
- in a `VerifyIrClosureLite` layer, nested wrapper nodes such as `Intrinsic`
  and `Record` are shown to preserve reachability of inner `Phi` values, which
  matches the verifier's transitive used-value closure requirement, including
  multi-step wrapper chains such as `Record -> Intrinsic -> Phi`
- in a `VerifyIrExecutableLite` layer, the reduced verifier is packaged as an
  order-sensitive executable spec that mirrors the current Rust
  `verify_ir`/`verify_emittable_ir` phase ordering at a coarse structural level
- in a `VerifyIrRustErrorLite` layer, the reduced proof errors are projected
  onto a Rust-enum-shaped error algebra so the proof-side verifier can be read
  as a name-level approximation of `src/mir/verify.rs`
- in a `VerifyIrPhaseOrderLite` layer, the reduced verifier is additionally
  packaged with an explicit phase-priority ordering that mirrors the coarse
  first-failure order of the current Rust verifier
- in a `VerifyIrCheckOrderLite` layer, the proof-side verifier is split into
  more fine-grained ordered check slots, closer to the individual check groups
  in `src/mir/verify.rs`
- in a `VerifyIrCfgConvergenceSubset` layer, reduced CFG acceptance is further
  tied to a stable reduced out-map witness, showing that once the must-defined
  fixed-point has stabilized, iterated out-def maps can be re-used directly in
  the reduced CFG executable theorem
- in a `VerifyIrCfgWorklistSubset` layer, a reduced join-focused worklist
  change bit is added on top of that stable out-map witness, showing that a
  stable seed produces `changed = false` at the join update site while reduced
  CFG acceptance still holds
- in a `VerifyIrCfgOrderWorklistSubset` layer, that join-focused worklist bit
  is lifted to a small block-order aggregation over left/right/join, showing
  the reduced ordered worklist reports no change across the whole small CFG
  when the seed is stable
- in a `VerifyIrCfgFixedPointSubset` layer, those reduced struct / worklist /
  flow facts are packaged as one small CFG fixed-point checker, proving the
  checker returns `none` when the reduced must-defined iteration has stabilized
  and the CFG flow obligations are discharged
- `time + dt` is not LICM-hoistable when `time` is loop-carried
- replacing the loop-carried `time` with a hoisted constant changes the result
  whenever `time` actually changes

## Build

Lean 4:

```bash
cd proof/lean
lake build
```

Coq:

```bash
cd proof/coq
coqc LicmLoopCarried.v
```

## Next Steps

The natural follow-up work is:

1. model a larger RR MIR fragment
2. formalize SSA/Phi semantics more directly
3. connect the formal LICM criterion to the Rust implementation shape
4. prove additional optimization passes sound over the same core semantics
