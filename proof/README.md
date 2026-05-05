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
  filtering, repeated generic pattern discrimination, and owner-qualified
  associated-type projection lookup. Production associated-type substitution,
  default method materialization, supertrait obligations, impl-coherence checks,
  exact-over-generic specialization, explicit turbofish calls, generic impl
  matching, and full cross-module cache replay remain implementation features
  covered primarily by Rust regression tests today. The proof claim also does
  not cover Rust-level
  heterogeneous `dyn Trait` vtables, borrow/lifetime or HRTB solving, full GAT
  projection normalization, arbitrary const-generic evaluation, or Rust's
  unstable specialization semantics.
- that production SROA is fully mechanized. The current proof boundary treats
  trait-driven static call targets and record-field rewrites as reduced
  preservation slices. It does not yet model the complete production SROA use
  graph, materialization-boundary recovery, cross-call record-argument
  specialization, record-return specialization, or codegen-time scalar-temp
  emission line by line.

The exact split between proved, approximated, and not-modeled claims is consolidated below in [Optimizer Proof Gap Audit](#optimizer-proof-gap-audit). The split between Chronos-owned pass specs and remaining legacy orchestration is consolidated in [Chronos Pass Catalog Audit](#chronos-pass-catalog-audit).

## Layout

- `lean/`
  - Lean 4 mechanization
- `coq/`
  - Coq mechanization
- `README.md`
  - the single Markdown proof manual; correspondence notes, catalog audits,
    proof-gap notes, and runtime/verifier mappings are consolidated below
- `lean/RRProofs/PeepholeLineSemantics.lean` and
  `coq/PeepholeLineSemantics.v`
  - mechanized reduced line-stream observation theorem for peephole stages

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
- the production Rust optimizer now routes full-program inlining and those
  post-heavy boundaries through Chronos pass specs, so the code-level stage
  names, verification labels, and proof-key metadata line up with the reduced
  theorem family
- in a `ChronosPassManagerSoundness` layer, reduced theorem names are fixed for
  Chronos stage dispatch and for the composed Chronos schedule boundary; this
  layer proves the reduced scheduler boundary, not the production pass-manager
  implementation line by line
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

## Integrated Proof Correspondence and Audits

All Markdown proof correspondence and audit material is kept in this README. The Lean and Coq mechanizations remain in `proof/lean/` and `proof/coq/`.


<a id="optimizer-proof-gap-audit"></a>
## Optimizer Proof Gap Audit

This note is the explicit trust-boundary document for the reduced optimizer
proof spine.

It answers one question only:

- what is actually proved
- what is only approximated by a reduced model
- what is not modeled yet

It should be read alongside:

- [Optimizer Proof ↔ Rust Correspondence](#optimizer-correspondence)
- [Chronos Pass Catalog Audit](#chronos-pass-catalog-audit)

### Reading Rule

`proved` means:
- a theorem exists in Lean/Coq for the reduced object at that boundary
- the theorem is wired into the current proof spine

`approximated` means:
- theorem names and stage boundaries match Rust structure
- but the reduced transform is weaker, simpler, or more stylized than the real Rust pass

`not modeled` means:
- no corresponding reduced theorem currently carries that behavior

### Summary

The current workspace proves a **reduced end-to-end optimizer spine**, not a
line-by-line mechanization of `src/mir/opt.rs`.

The strongest honest claim is:

- the current optimizer pipeline structure, phase-ordering structure, and
  program-level orchestration now have a continuous reduced theorem family
- many real Rust stage boundaries are named directly
- several stage implementations are still represented by simplified or partial
  reduced transforms

### Short Answer

If someone asks “is the optimizer proven?”, the precise answer is:

- **yes, as a reduced continuous proof spine**
- **no, not as a production 1:1 mechanization**

If someone asks “what may I safely claim?”, the safe wording is:

- the repository contains a reduced formal optimizer correctness argument with
  explicit stage, phase-order, program-level, and top-level wrapper theorems

Unsafe wording would be:

- the production Rust optimizer is fully formally verified

### Core Spine

| Rust slice | Proof file | Proved | Approximated | Not modeled |
| --- | --- | --- | --- | --- |
| `src/mir/opt.rs` optimizer-wide stage composition | [OptimizerPipelineSoundness.lean](lean/RRProofs/OptimizerPipelineSoundness.lean) | stage composition, verify-ir preservation, semantic preservation for reduced stage functions | `alwaysTierDataflowStage`, `alwaysTierLoopStage`, `postDeSsaBoundaryStage`, parts of `postDeSsaCleanupStage` remain simplified wrappers | full production `always tier` pass-by-pass behavior |
| Chronos pass-manager stage dispatch | [ChronosPassManagerSoundness.lean](lean/RRProofs/ChronosPassManagerSoundness.lean) / [ChronosPassManagerSoundness.v](coq/ChronosPassManagerSoundness.v) | reduced Chronos stage dispatch and composed schedule boundary preserve reduced verify-ir and semantics; the reduced catalog now names outlining, unroll dispatch, and fuel-exhausted skip boundaries | stage bodies reuse reduced optimizer/tail theorem functions or identity-style conservative skips rather than production pass specs directly | production timing/stats/progress effects and line-by-line pass-manager implementation |
| `src/mir/opt.rs` public optimizer shell | [ProgramApiWrapperSoundness.lean](lean/RRProofs/ProgramApiWrapperSoundness.lean) | wrapper theorem names for `run_program*` | wrapper semantics are pure shell composition over inner theorem | actual stats/progress side effects |
| reduced compiler observable theorem | [CompilerEndToEndSoundness.lean](lean/RRProofs/CompilerEndToEndSoundness.lean) / [CompilerEndToEndSoundness.v](coq/CompilerEndToEndSoundness.v) | reduced frontend observable theorem + reduced optimizer theorem in one top-level statement | frontend/backend side is still toy/reduced; Lean reuses `PipelineStmtSubset`, while Coq uses a tiny self-contained expression model | full production RR frontend and R runtime, plus a synchronized Lean/Coq frontend artifact |

### Function-Local Pass Layers

| Rust slice | Proof file | Proved | Approximated | Not modeled |
| --- | --- | --- | --- | --- |
| `simplify_cfg` / entry retarget / dead block cleanup | [CfgOptSoundness.lean](lean/RRProofs/CfgOptSoundness.lean) | reduced runner, dead-block append invariant theorem, empty-entry-goto retarget theorem, canonical dead-block append theorem | real CFG normalization set is much broader | full jump-threading / unreachable elimination catalog |
| `sccp` / `gvn` / `dce` reduced layer | [DataflowOptSoundness.lean](lean/RRProofs/DataflowOptSoundness.lean) | expression canonicalization, const-prop under env agreement, dead last-assign elimination | does not model dominance, availability, alias barriers, whole-block sparse propagation | full SCCP lattice / global fixed-point |
| `sroa` record-field and call-boundary scalarization | [SroaRecordReturnSubset.lean](lean/RRProofs/SroaRecordReturnSubset.lean) / [SroaRecordReturnSubset.v](coq/SroaRecordReturnSubset.v) | reduced record-return field projection preservation and static target preservation at the modeled boundary | production SROA has richer use-graph, materialization, record-arg specialization, record-return specialization, and scalar-temp emission logic | full line-by-line production SROA mechanization |
| `licm` / `bce` / `tco` | [LoopOptSoundness.lean](lean/RRProofs/LoopOptSoundness.lean) | reduced LICM zero/one-trip, reduced BCE, reduced TCO | actual loop optimizer side conditions are richer | full production loop optimizer state space |
| `de_ssa` and post-cleanup boundary | [DeSsaBoundarySoundness.lean](lean/RRProofs/DeSsaBoundarySoundness.lean) | reduced de-ssa boundary theorem | reduced copy-boundary matcher only | full parallel-copy scheduling |

### Phase Ordering and Plan Flow

| Rust slice | Proof file | Proved | Approximated | Not modeled |
| --- | --- | --- | --- | --- |
| phase profiles / schedule selection | [PhasePlanSoundness.lean](lean/RRProofs/PhasePlanSoundness.lean) | reduced `classify -> choose -> build plan` theorem family | score model is reduced and sample-based | exact production feature extraction impact on every threshold |
| selected plan collection | [PhasePlanCollectionSoundness.lean](lean/RRProofs/PhasePlanCollectionSoundness.lean) | skip/filter theorems for missing/conservative/self-recursive/unselected | uses reduced list collection instead of actual map | exact `FxHashMap` behavior not modeled |
| collected plan lookup | [PhasePlanLookupSoundness.lean](lean/RRProofs/PhasePlanLookupSoundness.lean) | lookup hit/miss and preservation after lookup | list-based lookup rather than hash-map lookup | map collision/overwrite behavior |
| ordered summary emission | [PhasePlanSummarySoundness.lean](lean/RRProofs/PhasePlanSummarySoundness.lean) | ordered summary hit/miss and payload exposure | summary entries only, not full strings | actual formatted summary text |
| phase-order schedule family | [PhaseOrderOptimizerSoundness.lean](lean/RRProofs/PhaseOrderOptimizerSoundness.lean) | schedule theorem family for balanced / compute-heavy / control-flow-heavy | schedule bodies still reduced relative to Rust | every per-pass delta between schedules |
| cluster boundaries | [PhaseOrderClusterSoundness.lean](lean/RRProofs/PhaseOrderClusterSoundness.lean) | structural / standard / cleanup theorem family | clusters are compressed abstractions | exact production cluster internals |
| guards / feature gates / fallback / iteration | [PhaseOrderGuardSoundness.lean](lean/RRProofs/PhaseOrderGuardSoundness.lean), [PhaseOrderFeatureGateSoundness.lean](lean/RRProofs/PhaseOrderFeatureGateSoundness.lean), [PhaseOrderFallbackSoundness.lean](lean/RRProofs/PhaseOrderFallbackSoundness.lean), [PhaseOrderIterationSoundness.lean](lean/RRProofs/PhaseOrderIterationSoundness.lean) | theorem families for guards, gates, fallback predicate, and heavy-iteration entrypoints | reduced booleans and reduced heavy-iteration state | exact per-pass statistics/progress impact |

### Program-Level Orchestration

| Rust slice | Proof file | Proved | Approximated | Not modeled |
| --- | --- | --- | --- | --- |
| adaptive budget plan | [ProgramOptPlanSoundness.lean](lean/RRProofs/ProgramOptPlanSoundness.lean) | under-budget, selective, fallback-to-smallest cases | selection order is reduced/sample-based | exact profile weighting and full sort tie-break semantics |
| program heavy-tier plan flow | [ProgramPhasePipelineSoundness.lean](lean/RRProofs/ProgramPhasePipelineSoundness.lean) | `ProgramOptPlan -> selected_functions -> collect -> summary` theorem family | reduced collection/list model | actual scheduler/external helper rewrites |
| per-function heavy-tier execution | [ProgramTierExecutionSoundness.lean](lean/RRProofs/ProgramTierExecutionSoundness.lean) | conservative/self-recursive/heavy-disabled/budget/collected-plan/legacy-plan split | branch actions are reduced to reduced stage calls | real local stats accumulation / verification side effects |
| post-heavy tail stages | [ProgramPostTierStagesSoundness.lean](lean/RRProofs/ProgramPostTierStagesSoundness.lean) | wrapper theorem family for inlining, inline cleanup, fresh-alias, de-ssa tail; Rust now routes these boundaries through Chronos stage specs | `freshAliasStage`, full inlining, and full inline cleanup remain reduced; de-ssa tail is still bundled | exact alias analysis, inliner growth behavior, scheduler effects, and copy-cleanup internals |
| outlining / unroll / fuel budget control | [ChronosPassManagerSoundness.lean](lean/RRProofs/ChronosPassManagerSoundness.lean) / [ChronosPassManagerSoundness.v](coq/ChronosPassManagerSoundness.v) | reduced identity-style preservation for conservative outlining dispatch, unroll dispatch, and fuel-exhausted skip | treats accepted transformations as reduced no-op boundaries and proves the skip/dispatch shell, not the production rewrite internals | region extraction correctness, full/partial unroll body cloning, exact fuel accounting, and cache-fingerprint behavior |
| program wrapper | [ProgramRunProfileInnerSoundness.lean](lean/RRProofs/ProgramRunProfileInnerSoundness.lean) | one reduced wrapper theorem for `run_program_with_profile_inner` | scheduler/progress/stats are abstracted out | full side-effectful orchestration |

### Actual Reduced Rewrite Companions

These were added specifically to reduce the number of pure identity-style
placeholders.

| Rust flavor | Proof file | Proved | Remaining gap |
| --- | --- | --- | --- |
| inline-cleanup shape | [InlineCleanupRefinementSoundness.lean](lean/RRProofs/InlineCleanupRefinementSoundness.lean) | reduced entry-retarget cleanup witness with verify-ir + eval preservation | does not model the whole production inline cleanup loop |
| fresh-alias shape | [FreshAliasRewriteSoundness.lean](lean/RRProofs/FreshAliasRewriteSoundness.lean) | reduced alias-rename theorem under explicit alias agreement; production stage now carries the matching Chronos proof key | main `ProgramPostTierStages` transform is still reduced relative to the Rust implementation |

### Highest-Value Remaining Gaps

These are the next places where the reduced spine is still materially weaker
than the production compiler.

1. `alwaysTierDataflowStage`
- still reduced more as a wrapper than as a whole-function SCCP/GVN/DCE stage

2. outlining and unroll internals
- dispatch and conservative skip shells are now named in the reduced Chronos
  proof, but region extraction, helper-call reconstruction, loop-body cloning,
  and partial-unroll cost behavior remain test-backed rather than
  mechanically proved line by line

3. `freshAliasStage`
- companion actual theorem exists and the Rust stage now has a Chronos proof-key
  boundary, but the main stage in
  [ProgramPostTierStagesSoundness.lean](lean/RRProofs/ProgramPostTierStagesSoundness.lean)
  is still simplified relative to production alias analysis

4. top-level compiler theorem
- [CompilerEndToEndSoundness.lean](lean/RRProofs/CompilerEndToEndSoundness.lean)
  is now a continuous reduced theorem
- but it is still not a 1:1 production theorem for the actual RR frontend,
  full MIR, and full emitted R/runtime semantics

### Bottom Line

The honest status is:

- **optimizer reduced proof spine: complete enough to claim a continuous reduced proof**
- **production optimizer 1:1 mechanization: not complete**
- **public API reduced shell: covered**
- **reduced compiler-level theorem: covered**

So the remaining work is no longer “build a proof spine at all”.
It is “tighten the reduced-to-production gap”.


<a id="chronos-pass-catalog-audit"></a>
## Chronos Pass Catalog Audit

This document records the production pass boundaries currently owned by the
Chronos pass manager and the optimizer work that still remains outside Chronos.

It is a catalog audit, not a formal proof. Formal reduced boundary names live in
`ChronosPassManagerSoundness.{lean,v}` and the broader claim strength is tracked
in `Optimizer Proof Gap Audit`.

### Reading Rule

`Chronos-owned` means:
- the stage is declared in `src/mir/opt/chronos/catalog.rs`
- the pass has an explicit `ChronosPassId`, `ChronosStage`, verification label,
  invalidated analysis set, and proof key
- the stage is dispatched through `ChronosPassManager` or
  `ChronosProgramPassManager`

`Legacy-owned` means:
- the code still runs from optimizer orchestration directly
- it may still be correct and tested, but it is not yet represented as a
  Chronos pass spec

### Chronos-Owned Function Stages

| Chronos stage | Production pass group | Current pass ids | Proof boundary |
| --- | --- | --- | --- |
| `FunctionEntryCanonicalization` | function-entry index metadata/canonicalization | `IndexCanonicalize` | reduced identity-style boundary through `ChronosPassManagerSoundness`; production pass carries explicit invalidation metadata |
| `AlwaysTier` | required small-function stabilization | `IndexCanonicalize`, `SimplifyCfg`, `Sccp`, `Intrinsics`, `TypeSpecialize`, `Tco`, `LoopOpt`, `Sroa`, `Dce`, bounded `Bce` | `OptimizerPipelineSoundness.always_tier_*`, `ChronosPassManagerSoundness.chronos_stage_*` |
| `PhaseOrderStandard` | standard heavy-tier core and budgeted tail | `SimplifyCfg`, `Sccp`, `Intrinsics`, `Gvn`, `Simplify`, `Sroa`, `Dce`, `LoopOpt`, `Licm`, `FreshAlloc`, `Bce` | `PhaseOrderClusterSoundness.*`, `ChronosPassManagerSoundness.chronos_stage_*` |
| `PhaseOrderComputePrelude` | compute-heavy prelude | `SimplifyCfg`, `Sccp`, `Intrinsics`, `Gvn`, `Simplify`, `Sroa`, `Dce` | `PhaseOrderIterationSoundness.compute_heavy_*` |
| `PhaseOrderControlPrelude` | control-heavy prelude | `SimplifyCfg`, `Sccp`, `Intrinsics`, `TypeSpecialize`, `Simplify`, `Sroa`, `Dce`, `Tco`, `Gvn` | `PhaseOrderIterationSoundness.control_flow_heavy_*` |
| `PhaseOrderBudgetPrefix` | budget-gated loop prefix | `LoopOpt`, `Licm` | `PhaseOrderClusterSoundness.budget_prefix_*` |
| `PhaseOrderControlBudgetPrefix` | control-flow budget prefix | `LoopOpt`, `Licm` | `PhaseOrderClusterSoundness.control_budget_prefix_*` |
| `PhaseOrderBudgetTail` | budget-gated allocation/bounds tail | `FreshAlloc`, `Bce` | `PhaseOrderClusterSoundness.budget_tail_*` |
| `PhaseOrderBalancedStructural` | balanced structural subcluster | `TypeSpecialize`, `Poly`, `Vectorize`, `Unroll`, `TypeSpecialize`, `Tco` | `PhaseOrderClusterSoundness.structural_cluster_*`; reduced unroll dispatch also has a `ChronosPassManagerSoundness` identity-style stage boundary |
| `PhaseOrderControlStructural` | control-heavy structural subcluster | `Poly`, `Vectorize`, `Unroll`, `TypeSpecialize` | `PhaseOrderClusterSoundness.control_structural_cluster_*`; reduced unroll dispatch also has a `ChronosPassManagerSoundness` identity-style stage boundary |
| `PhaseOrderFastDevVectorize` | fast-dev vectorization subpath | `TypeSpecialize`, `Vectorize`, `TypeSpecialize` | `PhaseOrderIterationSoundness.fast_dev_subpath_*` |
| `PhaseOrderStructuralCleanup` | post-structural cleanup | `SimplifyCfg`, `Sroa`, `Dce` | `PhaseOrderClusterSoundness.cleanup_cluster_*` |
| `FunctionFinalPolish` | function-local final cleanup | `SimplifyCfg`, `Dce` | reduced final-polish boundary through `ChronosPassManagerSoundness` |
| `PrepareForCodegen` | final de-SSA and cleanup | `DeSsa`, `SimplifyCfg`, `Dce` | `OptimizerPipelineSoundness.prepare_for_codegen_*` |

### Chronos-Owned Program Stages

| Chronos stage | Production pass group | Current pass ids | Proof boundary |
| --- | --- | --- | --- |
| `ProgramOutlining` | bounded MIR function outlining before inline/heavy reuse | `Outline` | reduced identity-style boundary through `ChronosPassManagerSoundness`; production pass is conservative and skips opaque/dynamic regions |
| `ProgramInline` | full-program inlining | `Inline` | reduced identity-style program-inline boundary plus `ProgramPostTierStagesSoundness` tail composition |
| `ProgramRecordSpecialization` | cross-function SROA record call/return specialization | `RecordCallSpecialize`, `RecordReturnSpecialize` | reduced identity-style boundary through `ChronosPassManagerSoundness`; production pass carries Chronos metadata/timing/verification |
| `ProgramInlineCleanup` | cleanup after program inlining | `SimplifyCfg`, `Sroa`, `Dce` | `ProgramPostTierStagesSoundness.inline_cleanup_*`, companion `InlineCleanupRefinementSoundness` |
| `ProgramFreshAlias` | fresh alias cleanup with user-call set | `FreshAlias` | `ProgramPostTierStagesSoundness.fresh_alias_*`, companion `FreshAliasRewriteSoundness` |
| `ProgramPostDeSsa` | post-heavy de-SSA cleanup | `DeSsa`, `CopyCleanup`, `SimplifyCfg`, `Dce` | `ProgramPostTierStagesSoundness.de_ssa_program_stage_*` |

### Analysis Cache Status

Chronos now owns a small reusable analysis cache for function-local gates:

| Cached analysis | Current production use | Invalidation boundary |
| --- | --- | --- |
| loop discovery | phase-feature extraction for Chronos gate decisions; cached loop view passed to `LoopOpt`, `LICM`, `Poly`, `Vectorize`, `BCE`, and `FreshAlloc` on Chronos paths | cleared when a changed pass invalidates control-flow or loop info |
| phase features | control-heavy LICM gate | cleared whenever a changed pass invalidates any analysis family |

This is intentionally narrow. The production rewrite passes still compute their
own deeper non-loop analyses until their APIs are explicitly changed to accept
borrowed analysis views.

### Legacy-Owned Optimizer Work

| Legacy slice | Why it remains outside Chronos | Suggested next boundary |
| --- | --- | --- |
| program-level floor-index proof collection | analyzes all functions to compute proven parameter slots before per-function optimization | keep as analysis prelude unless it starts mutating MIR |
| program plan construction, schedule selection, and plan summary emission | analysis/orchestration, not a MIR rewrite pass | keep as plan layer; do not force into pass catalog unless it mutates MIR |
| scheduler/progress/stats shell | side-effectful orchestration only | keep outside correctness pass catalog, document as shell side effects |
| source emit raw R rewrites and helper rewrites | backend text rewrite layer, not MIR | owned by the Hermes backend emit pass manager; keep outside Chronos proof spine |
| runtime/package/native bridge setup | backend/runtime boundary | separate runtime/codegen policy, not Chronos MIR |

### Completeness Claim

The current production optimizer has Chronos coverage for the main MIR rewrite
spine from always-tier through heavy-tier clusters, cross-function record
specialization before heavy tier, program inlining cleanup, fresh-alias cleanup,
post-de-SSA cleanup, and prepare-for-codegen cleanup.

It does not yet have Chronos ownership for all pre-tier canonicalization,
backend raw text rewrites, or side-effectful progress/stat/reporting shells.
Backend raw text rewrites are intentionally not a Chronos target; their current
production manager boundary is Hermes and is tracked separately in
`Hermes Raw Rewrite Pass Catalog Audit`.


<a id="optimizer-correspondence"></a>
## Optimizer Proof ↔ Rust Correspondence

This note ties the reduced optimizer proof layers in `proof/` to the concrete
Rust hardening work in `src/mir/opt/`.

It is not a full 1:1 verification of the production optimizer. The point is
more pragmatic:

- identify the smallest proof artifact that matches a real implementation guard
  or regression
- make explicit which Rust tests a proof layer is intended to approximate
- keep the next extension point obvious

### Claim Boundary

This file should be read with one rule in mind:

- matching a Rust stage boundary to a theorem name does **not** by itself mean
  the production pass is fully mechanized

What it does mean is:

- the reduced proof workspace now names that boundary explicitly
- the reduced theorem chain is intended to approximate that Rust slice
- the exact strength of the claim is further qualified in
  [Optimizer Proof Gap Audit](#optimizer-proof-gap-audit)

Use this note to answer:

- “which proof file corresponds to this Rust stage?”

Use the audit note to answer:

- “how strong is that correspondence?”

### Optimizer-Wide Soundness Target

Scaffolding files:
- [MirSemanticsLite.lean](lean/RRProofs/MirSemanticsLite.lean)
- [MirSemanticsLite.v](coq/MirSemanticsLite.v)
- [MirInvariantBundle.lean](lean/RRProofs/MirInvariantBundle.lean)
- [MirInvariantBundle.v](coq/MirInvariantBundle.v)

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
- [DataflowOptSoundness.lean](lean/RRProofs/DataflowOptSoundness.lean)
- [DataflowOptSoundness.v](coq/DataflowOptSoundness.v)

Current role:
- fixes a reduced dataflow optimizer slice over `MirSemanticsLite`
- proves three reusable preservation facts:
  - expression canonicalization preserves evaluation
  - constant propagation under environment agreement preserves evaluation
  - erasing a last dead pure assignment in a straight-line block preserves the
    returned value

Next CFG layer:
- [CfgOptSoundness.lean](lean/RRProofs/CfgOptSoundness.lean)
- [CfgOptSoundness.v](coq/CfgOptSoundness.v)

Current role:
- introduces a reduced multi-block MIR runner over the same semantic domain
- proves a reduced empty-entry-goto retarget theorem approximating entry
  normalization / jump threading
- proves invariant preservation for appending a dead unreachable block shape
  and for retargeting the entry to an existing block

Next loop layer:
- [LoopOptSoundness.lean](lean/RRProofs/LoopOptSoundness.lean)
- [LoopOptSoundness.v](coq/LoopOptSoundness.v)

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
- [DeSsaBoundarySoundness.lean](lean/RRProofs/DeSsaBoundarySoundness.lean)
- [DeSsaBoundarySoundness.v](coq/DeSsaBoundarySoundness.v)

Current role:
- reuses the reduced `DeSsaSubset` copy-boundary theorem
- exposes explicit stage-boundary theorem names for redundant move elimination
- supports the optimizer-wide stage family:
  - `program_post_dessa_preserves_verify_ir`
  - `program_post_dessa_preserves_semantics`
  - `prepare_for_codegen_preserves_verify_ir`
  - `prepare_for_codegen_preserves_semantics`

Composition layer:
- [OptimizerPipelineSoundness.lean](lean/RRProofs/OptimizerPipelineSoundness.lean)
- [OptimizerPipelineSoundness.v](coq/OptimizerPipelineSoundness.v)

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
- [PhaseOrderOptimizerSoundness.lean](lean/RRProofs/PhaseOrderOptimizerSoundness.lean)
- [PhaseOrderOptimizerSoundness.v](coq/PhaseOrderOptimizerSoundness.v)

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
- [PhaseOrderClusterSoundness.lean](lean/RRProofs/PhaseOrderClusterSoundness.lean)
- [PhaseOrderClusterSoundness.v](coq/PhaseOrderClusterSoundness.v)

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
- [PhaseOrderGuardSoundness.lean](lean/RRProofs/PhaseOrderGuardSoundness.lean)
- [PhaseOrderGuardSoundness.v](coq/PhaseOrderGuardSoundness.v)

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
- [PhaseOrderFeatureGateSoundness.lean](lean/RRProofs/PhaseOrderFeatureGateSoundness.lean)
- [PhaseOrderFeatureGateSoundness.v](coq/PhaseOrderFeatureGateSoundness.v)

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
- [PhaseOrderIterationSoundness.lean](lean/RRProofs/PhaseOrderIterationSoundness.lean)
- [PhaseOrderIterationSoundness.v](coq/PhaseOrderIterationSoundness.v)

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
- [PhaseOrderFallbackSoundness.lean](lean/RRProofs/PhaseOrderFallbackSoundness.lean)
- [PhaseOrderFallbackSoundness.v](coq/PhaseOrderFallbackSoundness.v)

Current role:
- fixes a reduced heavy-iteration result record with:
  - `structural_progress`
  - `non_structural_changes`
- exposes an explicit theorem boundary for Rust's
  `control_flow_should_fallback_to_balanced` predicate:
  - `control_flow_fallback_preserves_verify_ir`
  - `control_flow_fallback_preserves_semantics`

Phase-plan selection refinement:
- [PhasePlanSoundness.lean](lean/RRProofs/PhasePlanSoundness.lean)
- [PhasePlanSoundness.v](coq/PhasePlanSoundness.v)

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
- [PhasePlanCollectionSoundness.lean](lean/RRProofs/PhasePlanCollectionSoundness.lean)
- [PhasePlanCollectionSoundness.v](coq/PhasePlanCollectionSoundness.v)

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
- [PhasePlanLookupSoundness.lean](lean/RRProofs/PhasePlanLookupSoundness.lean)
- [PhasePlanLookupSoundness.v](coq/PhasePlanLookupSoundness.v)

Current role:
- fixes a reduced lookup boundary for consuming collected plans by function id
- models the same retrieval shape as Rust `plans.get(name)`
- exposes theorem family for:
  - singleton lookup hit/miss regressions
  - `lookup_collected_plan_preserves_verify_ir`
  - `lookup_collected_plan_preserves_semantics`

Phase-plan summary refinement:
- [PhasePlanSummarySoundness.lean](lean/RRProofs/PhasePlanSummarySoundness.lean)
- [PhasePlanSummarySoundness.v](coq/PhasePlanSummarySoundness.v)

Current role:
- fixes a reduced ordered-summary consumption boundary for `plan_summary_lines`
- models ordered function-id traversal together with lookup hit/miss
- exposes theorem family for:
  - `summary_lookup_hit_emits_entry`
  - `summary_lookup_miss_skips_entry`
  - summary entry exposure of `schedule/profile/pass_groups`
  - summary-lookup preservation via `planSelectedPipeline`

Program-budget refinement:
- [ProgramOptPlanSoundness.lean](lean/RRProofs/ProgramOptPlanSoundness.lean)
- [ProgramOptPlanSoundness.v](coq/ProgramOptPlanSoundness.v)

Current role:
- fixes a reduced `ProgramOptPlan` boundary for Rust `build_opt_plan_with_profile`
- models the same three high-level cases:
  - under-budget: select all safe functions
  - over-budget: selective mode with within-budget prefix
  - empty selective set: fallback to smallest eligible function

Program-level heavy-tier composition refinement:
- [ProgramPhasePipelineSoundness.lean](lean/RRProofs/ProgramPhasePipelineSoundness.lean)
- [ProgramPhasePipelineSoundness.v](coq/ProgramPhasePipelineSoundness.v)

Current role:
- fixes a reduced composition boundary for the program-level heavy-tier flow:
  `ProgramOptPlan -> selected_functions -> collect_function_phase_plans ->
  plan_summary`
- exposes theorem family for:
  - heavy-tier disabled yields no collected plans / no summary
  - program-level lookup preserves selected schedule soundness
  - program-level summary hit/miss follows the reduced lookup boundary

Program-level heavy-tier execution refinement:
- [ProgramTierExecutionSoundness.lean](lean/RRProofs/ProgramTierExecutionSoundness.lean)
- [ProgramTierExecutionSoundness.v](coq/ProgramTierExecutionSoundness.v)

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
- [ProgramPostTierStagesSoundness.lean](lean/RRProofs/ProgramPostTierStagesSoundness.lean)
- [ProgramPostTierStagesSoundness.v](coq/ProgramPostTierStagesSoundness.v)

Current role:
- fixes reduced theorem family for the remaining post-heavy stages inside
  `run_program_with_profile_inner`
- names the three stage boundaries directly:
  - `inline_cleanup_stage_*`
  - `fresh_alias_stage_*`
  - `de_ssa_program_stage_*`
- the production Rust side now routes these boundaries through Chronos stage
  specs:
  - `ProgramInline`
  - `ProgramInlineCleanup`
  - `ProgramFreshAlias`
  - `ProgramPostDeSsa`
- and exposes a composed tail theorem:
  - `program_post_tier_pipeline_preserves_verify_ir`
  - `program_post_tier_pipeline_preserves_semantics`

Chronos pass-manager refinement:
- [ChronosPassManagerSoundness.lean](lean/RRProofs/ChronosPassManagerSoundness.lean)
- [ChronosPassManagerSoundness.v](coq/ChronosPassManagerSoundness.v)
- [Chronos Pass Catalog Audit](#chronos-pass-catalog-audit)

Current role:
- fixes a reduced `ChronosStageLite` dispatch boundary for the stage names now
  used by production `src/mir/opt/chronos/catalog.rs`
- exposes reduced theorem names:
  - `chronos_stage_preserves_verify_ir`
  - `chronos_stage_preserves_semantics`
  - `chronos_reduced_schedule_preserves_verify_ir`
  - `chronos_reduced_schedule_preserves_semantics`
- the catalog audit lists which production MIR rewrites are Chronos-owned and
  which optimizer/backend orchestration slices remain legacy-owned
- this layer proves the reduced Chronos scheduling boundary; it does not prove
  production stats/timing/reporting side effects or every pass implementation
  line by line

Tail-stage actual reduced rewrite companions:
- [InlineCleanupRefinementSoundness.lean](lean/RRProofs/InlineCleanupRefinementSoundness.lean)
- [InlineCleanupRefinementSoundness.v](coq/InlineCleanupRefinementSoundness.v)
- [FreshAliasRewriteSoundness.lean](lean/RRProofs/FreshAliasRewriteSoundness.lean)
- [FreshAliasRewriteSoundness.v](coq/FreshAliasRewriteSoundness.v)

Current role:
- `InlineCleanupRefinementSoundness` fixes an actual non-identity reduced
  cleanup rewrite via entry retargeting on an empty-entry-goto shape
- `FreshAliasRewriteSoundness` fixes an actual alias-rename reduced rewrite
  showing that replacing a fresh alias load with its source load preserves
  evaluation under alias agreement
- Chronos attaches those proof keys to the production pass boundaries, but the
  reduced companions still do not mechanize the full Rust alias analysis,
  inliner growth budget, scheduler, or stats/progress side effects

Program wrapper refinement:
- [ProgramRunProfileInnerSoundness.lean](lean/RRProofs/ProgramRunProfileInnerSoundness.lean)
- [ProgramRunProfileInnerSoundness.v](coq/ProgramRunProfileInnerSoundness.v)

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
- [ProgramApiWrapperSoundness.lean](lean/RRProofs/ProgramApiWrapperSoundness.lean)
- [ProgramApiWrapperSoundness.v](coq/ProgramApiWrapperSoundness.v)

Current role:
- fixes reduced shell theorem names for the public optimizer entrypoints around
  `run_program_with_profile_and_scheduler`, `run_program_with_scheduler`,
  `run_program_with_stats`, and `run_program`
- makes explicit that these wrappers are orchestration shells around the
  already-proved `run_program_with_profile_inner` boundary

Reduced compiler end-to-end refinement:
- [CompilerEndToEndSoundness.lean](lean/RRProofs/CompilerEndToEndSoundness.lean)
- [CompilerEndToEndSoundness.v](coq/CompilerEndToEndSoundness.v)

Current role:
- pairs the optimizer wrapper theorem family with a reduced frontend/backend
  observable theorem
- the Lean theorem reuses `PipelineStmtSubset`; the Coq theorem currently uses a
  tiny self-contained expression model, so this is not a synchronized
  Lean/Coq statement over the same frontend artifact
- exposes a top-level reduced observable statement:
  frontend lowered/emitted evaluation matches the source result, and the
  optimized MIR witness preserves its execution result

### SROA

Proof layers:
- [SroaRecordReturnSubset.lean](lean/RRProofs/SroaRecordReturnSubset.lean)
- [SroaRecordReturnSubset.v](coq/SroaRecordReturnSubset.v)

Core proof claim:
- a reduced record-return projection can be replaced by the corresponding
  scalar field value without changing the modeled result
- the theorem is a reduced preservation slice for static record-field lowering,
  not a full production SROA proof

Primary Rust correspondence:
- [sroa.rs](../src/mir/opt/sroa.rs#L1)
  pass facade and shared SROA analysis types
- [core_rewrite/](../src/mir/opt/sroa/core_rewrite/field_maps.rs#L1)
  local record-field maps, materialization boundaries, replacement rewrites,
  and use-graph analysis
- [call_specialization/](../src/mir/opt/sroa/call_specialization/entrypoints.rs#L1)
  cross-call record-argument and record-return specialization

Current gap:
- production SROA handles richer use graphs, alias snapshots,
  materialization-boundary rematerialization, scalar-temp naming, and cross-call
  specialization than the current reduced theorem models

### GVN

Proof layers:
- [GvnSubset.lean](lean/RRProofs/GvnSubset.lean)
- [GvnSubset.v](coq/GvnSubset.v)

Core proof claim:
- commutative `add` canonicalization preserves evaluation
- a reduced intrinsic-abs wrapper preserves evaluation through the same
  canonicalization
- a reduced `fieldset -> field` read preserves evaluation through the same
  canonicalization
- if two expressions have the same canonical form, replacing one with the other
  preserves evaluation

Primary Rust correspondence:
- [gvn.rs](../src/mir/opt/gvn.rs#L258)
  commutative operand canonicalization
- [gvn.rs](../src/mir/opt/gvn.rs#L273)
  nested canonicalization / replacement propagation

Concrete Rust regressions:
- [gvn_canonicalizes_commutative_binary_operands](../src/mir/opt/gvn.rs#L978)
- [gvn_propagates_record_literal_cse_into_field_gets](../src/mir/opt/gvn.rs#L905)
- [gvn_cse_duplicate_intrinsics](../src/mir/opt/gvn.rs#L1035)
- [gvn_propagates_fieldset_cse_into_field_gets](../src/mir/opt/gvn.rs#L1086)

Current gap:
- proof is expression-level only; it does not yet model block dominance,
  availability, or mutation barriers

### Inline

Proof layers:
- [InlineSubset.lean](lean/RRProofs/InlineSubset.lean)
- [InlineSubset.v](coq/InlineSubset.v)

Core proof claim:
- a reduced pure helper shape
  - `arg`
  - `addConst`
  - `field`
  - `fieldAddConst`
  can be expression-inlined without changing evaluation

Primary Rust correspondence:
- [inline.rs](../src/mir/opt/inline.rs#L716)
  expr-inline `clone_rec()` coverage
- [inline.rs](../src/mir/opt/inline.rs#L653)
  side-effect guard for expr-inline
- [inline.rs](../src/mir/opt/inline.rs#L877)
  full-inline remap coverage

Concrete Rust regressions:
- [inline_value_calls_rejects_store_index3d_side_effect_helpers](../src/mir/opt/inline.rs#L1309)
- [perform_inline_remaps_record_field_value_ids](../src/mir/opt/inline.rs#L1460)
- [inline_value_calls_supports_record_field_helpers](../src/mir/opt/inline.rs#L1669)
- [inline_value_calls_supports_intrinsic_helpers](../src/mir/opt/inline.rs#L1763)
- [inline_value_calls_supports_fieldset_helpers](../src/mir/opt/inline.rs#L1846)
- [inline_value_calls_supports_index3d_helpers](../src/mir/opt/inline.rs#L1924)

Current gap:
- proof only covers pure helper semantics; it does not yet model side effects,
  call graph structure, or full caller/callee block rewrites

### De-SSA

Proof layers:
- [DeSsaSubset.lean](lean/RRProofs/DeSsaSubset.lean)
- [DeSsaSubset.v](coq/DeSsaSubset.v)

Core proof claim:
- a reduced canonical fingerprint is enough to decide that an incoming
  predecessor value already matches an existing predecessor assignment
- in that case, adding a redundant move is unnecessary

Primary Rust correspondence:
- [de_ssa.rs](../src/mir/opt/de_ssa.rs#L204)
  canonical value fingerprint before instruction
- [de_ssa.rs](../src/mir/opt/de_ssa.rs#L318)
  same canonical value before instruction

Concrete Rust regressions:
- [critical_edge_is_not_split_when_phi_input_matches_existing_field_get_shape](../src/mir/opt/de_ssa.rs#L1266)
- [critical_edge_is_not_split_when_phi_input_matches_existing_intrinsic_shape](../src/mir/opt/de_ssa.rs#L1365)
- [critical_edge_is_not_split_when_phi_input_matches_existing_fieldset_shape](../src/mir/opt/de_ssa.rs#L1448)

Current gap:
- proof does not yet model parallel-copy scheduling or full CFG mutation

### DCE

Proof layers:
- [DceSubset.lean](lean/RRProofs/DceSubset.lean)
- [DceSubset.v](coq/DceSubset.v)

Core proof claim:
- a pure dead assignment may be erased
- an effectful dead assignment must be demoted to `eval`
- nested wrappers preserve the total effect count seen by this reduced DCE

Primary Rust correspondence:
- [cfg_cleanup.rs](../src/mir/opt/cfg_cleanup.rs#L340)
  recursive `has_side_effect_val()`

Concrete Rust regressions:
- [dce_preserves_eval_with_nested_side_effect_inside_pure_call](../src/mir/opt/cfg_cleanup.rs#L421)
- [dce_preserves_eval_with_nested_side_effect_inside_intrinsic](../src/mir/opt/cfg_cleanup.rs#L518)
- [dce_preserves_eval_with_nested_side_effect_inside_index1d](../src/mir/opt/cfg_cleanup.rs#L628)
- [dce_preserves_eval_with_nested_side_effect_inside_index2d](../src/mir/opt/cfg_cleanup.rs#L716)
- [dce_preserves_eval_with_nested_side_effect_inside_index3d](../src/mir/opt/cfg_cleanup.rs#L804)
- [dce_preserves_eval_with_nested_side_effect_inside_phi](../src/mir/opt/cfg_cleanup.rs#L892)
- [dce_preserves_eval_with_nested_side_effect_inside_len](../src/mir/opt/cfg_cleanup.rs#L980)
- [dce_preserves_eval_with_nested_side_effect_inside_indices](../src/mir/opt/cfg_cleanup.rs#L1068)
- plus matching dead-assign-to-eval regressions in the same file

Current gap:
- proof tracks reduced effect count, not full R observable behavior

### Vectorize

Proof layers:
- [VectorizeSubset.lean](lean/RRProofs/VectorizeSubset.lean)
- [VectorizeSubset.v](coq/VectorizeSubset.v)

Core proof claim:
- expr-map certification must reject effectful loop bodies
- conditional map/reduction certification must accept only store-only branch
  shapes

Primary Rust correspondence:
- [planning.rs](../src/mir/opt/v_opt/planning.rs#L1760)
- [planning_expr_map.rs](../src/mir/opt/v_opt/planning_expr_map.rs#L344)
- [analysis_vectorization.rs](../src/mir/opt/v_opt/analysis_vectorization.rs#L148)
- [proof.rs](../src/mir/opt/v_opt/proof.rs#L562)
- [proof_reduction.rs](../src/mir/opt/v_opt/proof_reduction.rs#L499)

Concrete Rust regressions:
- [expr_map_matcher_rejects_loop_with_eval_side_effect](../src/mir/opt/v_opt/proof.rs#L2549)
- [scatter_matcher_rejects_loop_with_eval_side_effect](../src/mir/opt/v_opt/proof.rs#L2591)
- [cond_map_certification_rejects_branch_eval_side_effect](../src/mir/opt/v_opt/proof.rs#L2806)
- [cond_reduction_certification_rejects_branch_eval_side_effect](../src/mir/opt/v_opt/proof.rs#L2849)
- [cond_map_certification_rejects_branch_assign_side_effect](../src/mir/opt/v_opt/proof.rs#L2865)
- [classify_store_3d_rejects_block_with_eval_side_effect](../src/mir/opt/v_opt/analysis_vectorization.rs#L2529)

Current gap:
- proof covers only certification guards, not the transactional rewrite itself

### Vectorize Apply

Proof layers:
- [VectorizeApplySubset.lean](lean/RRProofs/VectorizeApplySubset.lean)
- [VectorizeApplySubset.v](coq/VectorizeApplySubset.v)

Core proof claim:
- rejected plans roll back to the scalar original
- certified result-preserving plans may commit without changing the scalar
  result

Primary Rust correspondence:
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L3531)
  transactional apply entry point
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L3493)
  vector apply site selection before transactional rewrite

Concrete Rust regressions:
- [enabled_config_certifies_simple_map_and_plan_applies_transactionally](../src/mir/opt/v_opt/proof.rs#L2491)
- [enabled_config_certifies_simple_expr_map_and_plan_applies_transactionally](../src/mir/opt/v_opt/proof.rs#L2520)
- [enabled_config_certifies_simple_cond_map_and_plan_applies_transactionally](../src/mir/opt/v_opt/proof.rs#L2757)
- [enabled_config_certifies_simple_cond_reduction_and_plan_applies_transactionally](../src/mir/opt/v_opt/proof.rs#L2792)
- [enabled_config_certifies_simple_sum_reduction_and_plan_applies_transactionally](../src/mir/opt/v_opt/proof.rs#L2910)

Current gap:
- proof models the transactional contract, but not the internal CFG/value
  rewrites that establish result preservation for real plans

### Vectorize Rewrite

Proof layers:
- [VectorizeRewriteSubset.lean](lean/RRProofs/VectorizeRewriteSubset.lean)
- [VectorizeRewriteSubset.v](coq/VectorizeRewriteSubset.v)

Core proof claim:
- the reduced exit-`Phi` merge after a vectorized apply/fallback split rejoins
  to the original scalar exit value
- both fallback and result-preserving apply paths are covered explicitly

Primary Rust correspondence:
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L666)
  preheader guard split
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L681)
  exit-`Phi` construction for preserved scalar semantics

Concrete Rust regressions:
- [enabled_config_certifies_simple_map_and_plan_applies_transactionally](../src/mir/opt/v_opt/proof.rs#L2491)
- [enabled_config_certifies_simple_expr_map_and_plan_applies_transactionally](../src/mir/opt/v_opt/proof.rs#L2520)
- [enabled_config_certifies_simple_cond_map_and_plan_applies_transactionally](../src/mir/opt/v_opt/proof.rs#L2757)

Current gap:
- proof still abstracts away the concrete MIR block/value mutation performed by
  the production rewrite, but it now models the scalar exit merge itself

### Vectorize MIR Rewrite

Proof layers:
- [VectorizeMirRewriteSubset.lean](lean/RRProofs/VectorizeMirRewriteSubset.lean)
- [VectorizeMirRewriteSubset.v](coq/VectorizeMirRewriteSubset.v)

Core proof claim:
- a tiny MIR machine with `preheader -> apply/fallback -> exit` preserves the
  original scalar result
- the reduced block/value rewrite is now modeled as an explicit machine rather
  than only as an exit-`Phi` equation

Primary Rust correspondence:
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L657)
  `apply_bb` materialization
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L666)
  preheader guard split into `apply_bb` / fallback
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L677)
  exit merge using scalar loads plus vector out values

Concrete Rust regressions:
- [enabled_config_certifies_simple_map_and_plan_applies_transactionally](../src/mir/opt/v_opt/proof.rs#L2577)
- [enabled_config_certifies_simple_expr_map_and_plan_applies_transactionally](../src/mir/opt/v_opt/proof.rs#L2606)
- [enabled_config_certifies_simple_cond_map_and_plan_applies_transactionally](../src/mir/opt/v_opt/proof.rs#L2825)

Current gap:
- proof still uses a reduced machine with pre-computed scalar/vector slots; it
  does not yet model concrete MIR value ids, load nodes, or reachable-use
  rewriting

### Vectorize Value Rewrite

Proof layers:
- [VectorizeValueRewriteSubset.lean](lean/RRProofs/VectorizeValueRewriteSubset.lean)
- [VectorizeValueRewriteSubset.v](coq/VectorizeValueRewriteSubset.v)

Core proof claim:
- recursively rewriting exit-region `Load var` uses with a replacement
  expression preserves return meaning whenever the replacement evaluates to the
  same scalar value as the original load

Primary Rust correspondence:
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L121)
  `rewrite_reachable_value_uses_for_var_after`
- [analysis_vectorization.rs](../src/mir/opt/v_opt/analysis_vectorization.rs#L118)
  `rewrite_returns_for_var`

Concrete Rust correspondence points:
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L702)
  exit-region reachable-use rewrite after exit-`Phi` creation
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L704)
  return rewrite when no later assignment to the destination survives

Current gap:
- proof is scalar-expression-level only; it does not yet model concrete MIR
  value ids, memoization, or cycle-breaking in the production tree rewrite

### Vectorize Use Rewrite

Proof layers:
- [VectorizeUseRewriteSubset.lean](lean/RRProofs/VectorizeUseRewriteSubset.lean)
- [VectorizeUseRewriteSubset.v](coq/VectorizeUseRewriteSubset.v)

Core proof claim:
- the scalar load-rewrite theorem is lifted to id-tagged reachable use sets
- rewriting all reachable uses after the exit preserves their meanings
  pointwise

Primary Rust correspondence:
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L121)
  `rewrite_reachable_value_uses_for_var_after`

Concrete Rust correspondence points:
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L141)
  memoized reachable-use rewriting
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L148)
  `Load var` / `origin_var` rewrite boundary

Current gap:
- proof now carries explicit ids, but still abstracts away concrete MIR value
  allocation and memo-table behavior

### Vectorize Origin/Memo

Proof layers:
- [VectorizeOriginMemoSubset.lean](lean/RRProofs/VectorizeOriginMemoSubset.lean)
- [VectorizeOriginMemoSubset.v](coq/VectorizeOriginMemoSubset.v)

Core proof claim:
- exact `Load var` roots stay anchored
- non-load nodes carrying `origin_var = var` redirect to the replacement
  boundary
- memo hits reuse the existing rewritten value id
- unchanged rewrites reuse the original id, while changed rewrites may use a
  fresh id

Primary Rust correspondence:
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L141)
  memo hit reuse
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L150)
  `origin_var` boundary behavior
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L164)
  fresh value-id allocation on changed rewrites

Current gap:
- proof isolates the local decision logic, but still does not model the full
  recursive tree walk and allocation sequence together

### Vectorize Decision

Proof layers:
- [VectorizeDecisionSubset.lean](lean/RRProofs/VectorizeDecisionSubset.lean)
- [VectorizeDecisionSubset.v](coq/VectorizeDecisionSubset.v)

Core proof claim:
- the local decision step for
  - `origin_var` boundary handling
  - memo-hit reuse
  - fresh-id allocation
  - reachable-use rewriting
  can be composed into one reduced rewrite decision without changing scalar use
  meaning

Primary Rust correspondence:
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L141)
  memo/origin local decision point
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L164)
  fresh value-id allocation on changed rewrites
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L121)
  reachable-use rewrite entry point

Current gap:
- proof composes the local contracts, but still abstracts away the full
  recursive traversal order and concrete mutable state updates

### Vectorize Tree Rewrite

Proof layers:
- [VectorizeTreeRewriteSubset.lean](lean/RRProofs/VectorizeTreeRewriteSubset.lean)
- [VectorizeTreeRewriteSubset.v](coq/VectorizeTreeRewriteSubset.v)

Core proof claim:
- the local vectorize rewrite decision is lifted into a reduced recursive tree
  rewrite with explicit traversal order and allocation state
- sample properties cover
  - unchanged roots reusing their original ids
  - changed trees allocating a fresh id
  - scalar evaluation staying unchanged after the rewrite

Primary Rust correspondence:
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L134)
  recursive tree rewrite entry
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L141)
  memo reuse during traversal
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L164)
  fresh-id allocation on changed rewrites

Current gap:
- proof now has traversal order and allocation state, but remains sample-driven
  rather than a full generic proof over the entire reduced tree space

### Vectorize Allocation State

Proof layers:
- [VectorizeAllocStateSubset.lean](lean/RRProofs/VectorizeAllocStateSubset.lean)
- [VectorizeAllocStateSubset.v](coq/VectorizeAllocStateSubset.v)

Core proof claim:
- multiple rewritten trees can be threaded through a single allocation state
- fresh ids and scalar meanings compose correctly across a list of reachable
  roots

Primary Rust correspondence:
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L193)
  repeated recursive rewriting across field lists / argument lists
- [transform.rs](../src/mir/opt/v_opt/transform.rs#L262)
  repeated argument rewriting in calls / intrinsics

Current gap:
- proof now carries allocation state across multiple roots, but still remains
  sample-driven rather than generic over arbitrary reachable root sets

### Recommended Next Extensions

1. Lift `GvnSubset` from expression equality to block-local availability and
   dominance.
2. Lift `InlineSubset` from helper shapes to reduced caller/callee CFGs.
3. Lift `DeSsaSubset` from “no move needed” to reduced parallel-copy soundness.
4. Lift `DceSubset` from effect-count preservation to reduced evaluation trace
   preservation.
5. Lift `VectorizeAllocStateSubset` from sample-driven multi-root allocation
   state to a generic reduced theorem over arbitrary reachable root sets.


<a id="pipeline-correspondence"></a>
## Lowering / Codegen Proof ↔ Rust Correspondence

This note ties the reduced lowering/codegen/pipeline proof layers in `proof/`
to the concrete Rust compiler pipeline in `src/mir/`, `src/codegen/`, and
`src/compiler/pipeline/`.

As with [Optimizer Proof ↔ Rust Correspondence](#optimizer-correspondence),
the goal is pragmatic rather than grand:

- identify which proof file approximates which Rust stage
- make the current abstraction boundary explicit
- keep the next 1:1 connection target obvious

### Lowering

Proof layers:
- [lean/RRProofs/LoweringSubset.lean](lean/RRProofs/LoweringSubset.lean)
- [coq/LoweringSubset.v](coq/LoweringSubset.v)

Core proof claim:
- a reduced source expression fragment
  - const
  - unary neg
  - binary add
  - record literal
  - field access
  lowers to a MIR-like expression fragment without changing evaluation

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L705)
  `MirLowerer::lower_fn`
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  expression and assignment emission into `FnIR`

Current gap:
- proof is expression-centric
- Rust lowering is statement/CFG-producing and tracks names, blocks, and
  side conditions beyond the subset

### If → Phi Lowering

Proof layers:
- [lean/RRProofs/LoweringIfPhiSubset.lean](lean/RRProofs/LoweringIfPhiSubset.lean)
- [coq/LoweringIfPhiSubset.v](coq/LoweringIfPhiSubset.v)
- [coq/LoweringIfPhiGenericSubset.v](coq/LoweringIfPhiGenericSubset.v)

Core proof claim:
- reduced source `if` lowers to a MIR-like `phi` join form
- generic and concrete true/false/nested-field cases preserve evaluation

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L466)
  branch lowering scaffolding
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering

Current gap:
- proof models a reduced join expression rather than the full Rust block graph
- Rust lowering also carries ownership metadata and later verifier obligations

### Let / Local Lowering

Proof layers:
- [lean/RRProofs/LoweringLetSubset.lean](lean/RRProofs/LoweringLetSubset.lean)
- [coq/PipelineLetSubset.v](coq/PipelineLetSubset.v)
- [coq/PipelineLetGenericSubset.v](coq/PipelineLetGenericSubset.v)

Core proof claim:
- reduced local reads, field reads, nested field reads, and local `add`
  preserve evaluation through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L558)
  local binding / SSA name setup
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L578)
  recursive variable read reconstruction

Current gap:
- proof does not model full local environment mutation over blocks
- Rust lowering includes shadowing, assignment emission, and CFG placement

### Codegen

Proof layers:
- [lean/RRProofs/CodegenSubset.lean](lean/RRProofs/CodegenSubset.lean)
- [coq/CodegenSubset.v](coq/CodegenSubset.v)
- [coq/CodegenGenericSubset.v](coq/CodegenGenericSubset.v)

Core proof claim:
- a reduced MIR-like expression fragment emits to an R-like expression fragment
  without changing evaluation

Primary Rust correspondence:
- [src/codegen/mir_emit.rs](../src/codegen/mir_emit.rs#L259)
  structured code emission entry
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured traversal
- [src/codegen/emit/resolve.rs](../src/codegen/emit/resolve.rs#L499)
  expression rendering for index/call forms

Current gap:
- proof covers expression evaluation only
- Rust codegen also performs statement rendering, structured control-flow
  reconstruction, and emitted-R cleanup

### Assign / Phi Pipeline

Proof layers:
- [lean/RRProofs/PipelineAssignPhiSubset.lean](lean/RRProofs/PipelineAssignPhiSubset.lean)
- [coq/PipelineAssignPhiSubset.v](coq/PipelineAssignPhiSubset.v)
- [coq/PipelineAssignPhiGenericSubset.v](coq/PipelineAssignPhiGenericSubset.v)

Core proof claim:
- reduced branch-local reassignment lowered through a `phi`-merged value and
  then emitted to R-like code still preserves evaluation

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  `Phi` placeholder introduction
- [src/mir/opt/de_ssa.rs](../src/mir/opt/de_ssa.rs#L1)
  later `Phi` elimination before codegen
- [src/mir/opt.rs](../src/mir/opt.rs#L1593)
  de-SSA before codegen

Current gap:
- proof speaks in reduced merged-value semantics
- Rust path spans lowering, verification, de-SSA, and statement emission

### Statement / Program Pipeline

Proof layers:
- [lean/RRProofs/PipelineStmtSubset.lean](lean/RRProofs/PipelineStmtSubset.lean)
- [coq/PipelineStmtSubset.v](coq/PipelineStmtSubset.v)
- [coq/PipelineStmtGenericSubset.v](coq/PipelineStmtGenericSubset.v)

Core proof claim:
- reduced straight-line and branch/program fragments preserve execution through
  lowering and R-like codegen

Primary Rust correspondence:
- [src/compiler/pipeline/compile_api.rs](../src/compiler/pipeline/compile_api.rs#L262)
  `compile_with_pipeline_request`
- [src/compiler/pipeline/phases/source_emit/mir_synthesis.rs](../src/compiler/pipeline/phases/source_emit/mir_synthesis.rs#L1)
  source-to-MIR function collection and lowering
- [src/compiler/pipeline/phases/source_emit/cached_emit.rs](../src/compiler/pipeline/phases/source_emit/cached_emit.rs#L1)
  `emit_r_functions_cached`

Current gap:
- proof uses reduced program fragments
- Rust path includes caching, root selection, emitted-R rewrites, and runtime
  injection
- source emission is now split by module API into source analysis, MIR
  synthesis, cached emission, raw emission helpers, and module-artifact replay;
  the proof still models that path as one reduced lowering/codegen boundary
- Rust source analysis now also performs trait/generic metadata-sensitive module
  ordering, module-artifact source metadata replay, explicit turbofish lowering,
  generic return-type inference from annotated `let` bindings, impl-coherence
  checks, associated-type substitution, default method materialization,
  supertrait obligation checks, exact-over-generic specialization, negative impl
  blocking, operator trait lowering, and generic impl instantiation. The reduced
  `TraitDispatchSoundness` files model static target preservation after
  resolution, negative-impl exclusion, reduced operator-to-trait mapping, and
  public trait metadata filtering; most source/HIR trait-solver behavior is
  still outside the reduced lowering/codegen proof layers. Rust-level
  heterogeneous `dyn Trait` vtables, borrow/lifetime or HRTB solving, full GAT
  projection normalization, arbitrary const-generic evaluation, and unstable
  specialization semantics are explicit non-claims for the current proof spine.

### CFG Pipeline

Proof layers:
- [lean/RRProofs/PipelineCfgSubset.lean](lean/RRProofs/PipelineCfgSubset.lean)
- [coq/PipelineCfgSubset.v](coq/PipelineCfgSubset.v)
- [coq/PipelineCfgGenericSubset.v](coq/PipelineCfgGenericSubset.v)

Core proof claim:
- a tiny explicit `then/else/join` CFG wrapper preserves evaluation through
  lowering and emission

Primary Rust correspondence:
- [src/mir/verify.rs](../src/mir/verify.rs#L272)
  CFG-side structural obligations before emission
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured CFG-to-R reconstruction
- [src/compiler/pipeline/compile_api.rs](../src/compiler/pipeline/compile_api.rs#L444)
  `verify_emittable_program`

Current gap:
- proof uses tiny explicit CFG fragments
- Rust path still includes richer loop/block metadata and emitted-R cleanup

### Block / Env Pipeline

Proof layers:
- [lean/RRProofs/PipelineBlockEnvSubset.lean](lean/RRProofs/PipelineBlockEnvSubset.lean)
- [coq/PipelineBlockEnvSubset.v](coq/PipelineBlockEnvSubset.v)

Core proof claim:
- a reduced explicit block shell carrying block id, incoming local
  environment, ordered statements, and return expression preserves evaluation
  through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L558)
  local binding / read environment setup
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/mir_emit.rs](../src/codegen/mir_emit.rs#L259)
  structured function/block emission entry

Current gap:
- proof still uses one ordered block shell rather than a real CFG of blocks
- Rust path still tracks SSA ids, block ownership, verifier side conditions,
  and emitted-R cleanup beyond the subset

### Function / Env Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnEnvSubset.lean](lean/RRProofs/PipelineFnEnvSubset.lean)
- [coq/PipelineFnEnvSubset.v](coq/PipelineFnEnvSubset.v)

Core proof claim:
- a reduced explicit function shell carrying
  - function name
  - entry/body-head metadata
  - ordered block/env list
  preserves per-block results through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L705)
  `MirLowerer::lower_fn`
- [src/codegen/mir_emit.rs](../src/codegen/mir_emit.rs#L252)
  function emission entry
- [src/compiler/pipeline/compile_api.rs](../src/compiler/pipeline/compile_api.rs#L263)
  top-level compile pipeline entry

Current gap:
- proof still uses ordered block lists instead of a real predecessor graph
- Rust path still includes verifier obligations, SSA ids, and structured CFG
  reconstruction beyond the subset

### Function / CFG Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgSubset.lean](lean/RRProofs/PipelineFnCfgSubset.lean)
- [coq/PipelineFnCfgSubset.v](coq/PipelineFnCfgSubset.v)

Core proof claim:
- a reduced explicit function shell carrying
  - function metadata
  - predecessor map
  - ordered block/env list
  preserves reduced per-block results through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L705)
  `MirLowerer::lower_fn`
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L466)
  branch/block scaffolding
- [src/codegen/mir_emit.rs](../src/codegen/mir_emit.rs#L252)
  function emission entry

Current gap:
- proof now carries predecessor data, but still not a real CFG execution
- Rust path still tracks SSA ids, phi placeholders, verifier obligations, and
  structured CFG reconstruction beyond the subset

### Function / CFG Execution Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgExecSubset.lean](lean/RRProofs/PipelineFnCfgExecSubset.lean)
- [coq/PipelineFnCfgExecSubset.v](coq/PipelineFnCfgExecSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell plus an explicit execution path
  witness preserves selected-path results through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L466)
  branch/block scaffolding
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L705)
  `MirLowerer::lower_fn`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured CFG-to-R reconstruction

Current gap:
- proof now carries a selected path witness, but still not a real small-step
  CFG execution semantics
- Rust path still includes phi placeholders, verifier obligations, and
  structure recovery beyond the subset

### Function / CFG Small-Step Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgSmallStepSubset.lean](lean/RRProofs/PipelineFnCfgSmallStepSubset.lean)
- [coq/PipelineFnCfgSmallStepSubset.v](coq/PipelineFnCfgSmallStepSubset.v)

Core proof claim:
- a reduced tiny trace machine over the selected CFG path preserves execution
  trace results through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L466)
  branch/block scaffolding
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured CFG-to-R reconstruction

Current gap:
- proof now has a reduced small-step trace, but still not the full concrete
  `FnIR` block/value operational semantics
- Rust path still includes phi placeholders, verifier obligations, and richer
  structure recovery beyond the subset

### Function / CFG Branch-Exec Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgBranchExecSubset.lean](lean/RRProofs/PipelineFnCfgBranchExecSubset.lean)
- [coq/PipelineFnCfgBranchExecSubset.v](coq/PipelineFnCfgBranchExecSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit `then` / `else`
  path choice preserves the chosen branch trace through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L466)
  branch/block scaffolding
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured CFG branch reconstruction

Current gap:
- proof now carries explicit branch choice, but still not a reduced `phi` /
  join merge operational semantics for converging paths
- Rust path still includes verifier obligations, phi placeholders, and richer
  structured reconstruction beyond the subset

### Function / CFG Phi-Exec Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgPhiExecSubset.lean](lean/RRProofs/PipelineFnCfgPhiExecSubset.lean)
- [coq/PipelineFnCfgPhiExecSubset.v](coq/PipelineFnCfgPhiExecSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit branch choice and a
  reduced `phi`/join merge result preserves the chosen merged result through
  lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction

Current gap:
- proof now carries a reduced join-merge result, but still not a full reduced
  operational semantics for branch execution plus explicit join block state
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Join-State Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgJoinStateSubset.lean](lean/RRProofs/PipelineFnCfgJoinStateSubset.lean)
- [coq/PipelineFnCfgJoinStateSubset.v](coq/PipelineFnCfgJoinStateSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit branch choice,
  reduced `phi`/join merge, and explicit join-local environment preserves the
  join-state result through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/opt/de_ssa.rs](../src/mir/opt/de_ssa.rs#L1)
  later join-state realization before codegen
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction

Current gap:
- proof now carries explicit join-local state, but still not a reduced
  operational semantics for whole-CFG join block execution after merge
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Join-Exec Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgJoinExecSubset.lean](lean/RRProofs/PipelineFnCfgJoinExecSubset.lean)
- [coq/PipelineFnCfgJoinExecSubset.v](coq/PipelineFnCfgJoinExecSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit join-local state and
  explicit join block execution (`join stmts + join ret`) preserves the
  join-block result through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured join-block reconstruction

Current gap:
- proof now carries explicit join-block execution, but still not a reduced
  whole-CFG operational semantics with explicit post-join continuation
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Post-Join Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgPostJoinSubset.lean](lean/RRProofs/PipelineFnCfgPostJoinSubset.lean)
- [coq/PipelineFnCfgPostJoinSubset.v](coq/PipelineFnCfgPostJoinSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit join block
  execution plus an explicit post-join continuation block preserves the
  continuation result through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured join-block reconstruction and follow-up block emission

Current gap:
- proof now carries explicit post-join continuation, but still not a reduced
  whole-CFG operational semantics for iterative multi-block execution after
  join
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Iterative Post-Join Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgIterExecSubset.lean](lean/RRProofs/PipelineFnCfgIterExecSubset.lean)
- [coq/PipelineFnCfgIterExecSubset.v](coq/PipelineFnCfgIterExecSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit join-block
  execution and an ordered list of post-join continuation blocks preserves the
  final continuation result through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured join-block reconstruction and post-join ordered emission

Current gap:
- proof now carries reduced iterative post-join execution, but still not a
  whole-CFG operational semantics for repeated graph transitions driven by
  explicit branch/join control state
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Control-State Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgControlStateSubset.lean](lean/RRProofs/PipelineFnCfgControlStateSubset.lean)
- [coq/PipelineFnCfgControlStateSubset.v](coq/PipelineFnCfgControlStateSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit join execution and
  an explicit control-state transition machine for post-join continuation
  preserves the final continuation result through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction with follow-up control flow

Current gap:
- proof now carries explicit post-join control-state transitions, but still
  not a richer reduced semantics for arbitrary graph re-entry, loops, or
  repeated branch/join cycling
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Graph-State Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgGraphStateSubset.lean](lean/RRProofs/PipelineFnCfgGraphStateSubset.lean)
- [coq/PipelineFnCfgGraphStateSubset.v](coq/PipelineFnCfgGraphStateSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit join execution and
  an explicit `pc + step-table` graph-state shell preserves the final
  continuation result through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction with ordered follow-up control flow

Current gap:
- proof now carries explicit graph-state execution, but still not a richer
  reduced semantics for loops, arbitrary re-entry, or repeated branch/join
  cycling over non-linear control graphs
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Reentry Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgReentrySubset.lean](lean/RRProofs/PipelineFnCfgReentrySubset.lean)
- [coq/PipelineFnCfgReentrySubset.v](coq/PipelineFnCfgReentrySubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit join execution and
  an explicit re-entry trace over step indices preserves the final
  continuation result through lowering and emission, even when the same
  continuation step is revisited

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction with follow-up control-flow revisits

Current gap:
- proof now carries explicit re-entry / revisit traces, but still not a richer
  reduced semantics for arbitrary loop headers, fixed-point branch cycling, or
  non-trace-driven graph exploration
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Loop-Cycle Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgLoopCycleSubset.lean](lean/RRProofs/PipelineFnCfgLoopCycleSubset.lean)
- [coq/PipelineFnCfgLoopCycleSubset.v](coq/PipelineFnCfgLoopCycleSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit re-entry traces and
  repeated branch/join cycles over an accumulator-like loop state preserves
  the final cycle result through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction with repeated follow-up control flow

Current gap:
- proof now carries repeated branch/join cycle iteration, but still not a
  richer reduced semantics for open-ended fixed-point convergence or general
  loop-header graph execution
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Loop-Fixpoint Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgLoopFixpointSubset.lean](lean/RRProofs/PipelineFnCfgLoopFixpointSubset.lean)
- [coq/PipelineFnCfgLoopFixpointSubset.v](coq/PipelineFnCfgLoopFixpointSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with repeated branch/join cycles
  and an explicit stability witness preserves that fixed-point witness through
  lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit fixed-point witness, but still not a richer
  reduced semantics for automatic convergence discovery or general loop-header
  fixed-point computation
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Loop-Discover Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgLoopDiscoverSubset.lean](lean/RRProofs/PipelineFnCfgLoopDiscoverSubset.lean)
- [coq/PipelineFnCfgLoopDiscoverSubset.v](coq/PipelineFnCfgLoopDiscoverSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with an explicit worklist and a
  selected stable candidate preserves that discovery result through lowering
  and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit discovery/worklist shell, but still not a
  reduced semantics for automatically updating the candidate set or proving
  convergence discovery by iteration
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Loop-Worklist Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgLoopWorklistSubset.lean](lean/RRProofs/PipelineFnCfgLoopWorklistSubset.lean)
- [coq/PipelineFnCfgLoopWorklistSubset.v](coq/PipelineFnCfgLoopWorklistSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with an explicit worklist
  selection and a `pending -> done` update shell preserves that updated
  worklist state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit worklist update shell, but still not a reduced
  semantics for multiple update rounds, candidate insertion, or automatic
  convergence discovery by iteration
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Loop-Queue Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgLoopQueueSubset.lean](lean/RRProofs/PipelineFnCfgLoopQueueSubset.lean)
- [coq/PipelineFnCfgLoopQueueSubset.v](coq/PipelineFnCfgLoopQueueSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with an ordered queue of worklist
  rounds preserves the resulting drained update list through lowering and
  emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit queue drain shell, but still not a reduced
  semantics for candidate insertion, priority changes, or automatic multi-round
  convergence discovery by iteration
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Loop-Scheduler Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgLoopSchedulerSubset.lean](lean/RRProofs/PipelineFnCfgLoopSchedulerSubset.lean)
- [coq/PipelineFnCfgLoopSchedulerSubset.v](coq/PipelineFnCfgLoopSchedulerSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with an ordered scheduler of queue
  batches preserves the resulting batch-evaluation trace through lowering and
  emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit scheduler shell, but still not a reduced
  semantics for dynamic queue growth, priority changes, or automatic
  convergence discovery by repeated scheduling
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Loop-Dynamic-Scheduler Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgLoopDynamicSchedulerSubset.lean](lean/RRProofs/PipelineFnCfgLoopDynamicSchedulerSubset.lean)
- [coq/PipelineFnCfgLoopDynamicSchedulerSubset.v](coq/PipelineFnCfgLoopDynamicSchedulerSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit scheduler reinserts
  preserves the resulting dynamically scheduled batch trace through lowering
  and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries explicit reinsertion batches, but still not a reduced
  semantics for priority-based insertion, dynamic candidate growth, or
  automatic convergence discovery by repeated scheduling
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Loop-Priority Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgLoopPrioritySubset.lean](lean/RRProofs/PipelineFnCfgLoopPrioritySubset.lean)
- [coq/PipelineFnCfgLoopPrioritySubset.v](coq/PipelineFnCfgLoopPrioritySubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit priority labels over
  pending and reinserted batches preserves the resulting priority-labeled
  scheduler trace through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries explicit priority labels, but still not a reduced
  semantics for dynamic priority recomputation or policy-driven candidate
  promotion by repeated scheduling
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Loop-Policy Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgLoopPolicySubset.lean](lean/RRProofs/PipelineFnCfgLoopPolicySubset.lean)
- [coq/PipelineFnCfgLoopPolicySubset.v](coq/PipelineFnCfgLoopPolicySubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit priority-rewrite
  rules preserves the resulting policy-normalized scheduler trace through
  lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries explicit priority-rewrite policy, but still not a reduced
  semantics for policy recomputation driven by newly discovered costs or
  repeated scheduler feedback
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Loop-Adaptive-Policy Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgLoopAdaptivePolicySubset.lean](lean/RRProofs/PipelineFnCfgLoopAdaptivePolicySubset.lean)
- [coq/PipelineFnCfgLoopAdaptivePolicySubset.v](coq/PipelineFnCfgLoopAdaptivePolicySubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with feedback-driven priority rule
  recomputation preserves the resulting adaptive policy trace through lowering
  and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries explicit feedback-driven rule recomputation, but still not
  a reduced semantics for closed-loop policy learning or repeated feedback
  adaptation across multiple scheduler rounds
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Loop-Closed-Loop Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgLoopClosedLoopSubset.lean](lean/RRProofs/PipelineFnCfgLoopClosedLoopSubset.lean)
- [coq/PipelineFnCfgLoopClosedLoopSubset.v](coq/PipelineFnCfgLoopClosedLoopSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with repeated adaptive feedback
  rounds preserves the resulting closed-loop adaptive trace through lowering
  and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries repeated adaptive rounds, but still not a reduced
  semantics for open-ended learning loops, policy saturation, or repeated
  adaptive convergence discovery
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Loop-Meta-Iteration Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgLoopMetaIterSubset.lean](lean/RRProofs/PipelineFnCfgLoopMetaIterSubset.lean)
- [coq/PipelineFnCfgLoopMetaIterSubset.v](coq/PipelineFnCfgLoopMetaIterSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with repeated adaptive rounds and
  an explicit last-summary witness preserves that meta-iteration summary
  through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit meta-iteration summary shell, but still not a
  reduced semantics for discovering that summary by open-ended convergence
  rather than reading it from a bounded closed-loop trace
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Summary-Protocol Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgSummaryProtocolSubset.lean](lean/RRProofs/PipelineFnCfgSummaryProtocolSubset.lean)
- [coq/PipelineFnCfgSummaryProtocolSubset.v](coq/PipelineFnCfgSummaryProtocolSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with repeated meta-iteration
  rounds and an explicit stable-summary protocol preserves that summary trace
  through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit summary protocol shell, but still not a
  reduced semantics for discovering stability by open-ended convergence rather
  than carrying a bounded summary trace
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Convergence-Protocol Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgConvergenceProtocolSubset.lean](lean/RRProofs/PipelineFnCfgConvergenceProtocolSubset.lean)
- [coq/PipelineFnCfgConvergenceProtocolSubset.v](coq/PipelineFnCfgConvergenceProtocolSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with an explicit
  `summary unchanged => halt` witness preserves the resulting convergence
  summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit convergence protocol shell, but still not a
  reduced semantics for discovering the halt condition by open-ended dynamic
  search rather than transporting a bounded witness
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Halt-Discover Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgHaltDiscoverSubset.lean](lean/RRProofs/PipelineFnCfgHaltDiscoverSubset.lean)
- [coq/PipelineFnCfgHaltDiscoverSubset.v](coq/PipelineFnCfgHaltDiscoverSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with an explicit halt-search
  shell preserves the discovered halt summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit halt-discovery shell, but still not a reduced
  semantics for open-ended halt search or dynamic convergence discovery beyond
  a bounded search space
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchSubset.lean)
- [coq/PipelineFnCfgOpenSearchSubset.v](coq/PipelineFnCfgOpenSearchSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with an explicit
  `completed + frontier` worklist shell preserves the discovered halt summary
  through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit pending/completed open-search shell, but still
  not a reduced semantics for dynamic frontier growth or repeated open-ended
  queue discovery beyond one split worklist
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Dynamic Open-Search Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgDynamicOpenSearchSubset.lean](lean/RRProofs/PipelineFnCfgDynamicOpenSearchSubset.lean)
- [coq/PipelineFnCfgDynamicOpenSearchSubset.v](coq/PipelineFnCfgDynamicOpenSearchSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit frontier growth
  preserves the discovered halt summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one explicit dynamic frontier-growth step, but still not a
  reduced semantics for repeated open-ended queue discovery or policy-guided
  expansion beyond one update
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Scheduler Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchSchedulerSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchSchedulerSubset.lean)
- [coq/PipelineFnCfgOpenSearchSchedulerSubset.v](coq/PipelineFnCfgOpenSearchSchedulerSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit scheduled
  open-search rounds preserves the discovered halt summary through lowering
  and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries repeated scheduled open-search rounds, but still not a
  reduced semantics for adaptive queue reordering or policy-guided scheduling
  beyond one fixed schedule shell
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Priority Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchPrioritySubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchPrioritySubset.lean)
- [coq/PipelineFnCfgOpenSearchPrioritySubset.v](coq/PipelineFnCfgOpenSearchPrioritySubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit priority-labeled
  open-search rounds preserves the discovered halt summary through lowering
  and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries priority-labeled open-search rounds, but still not a
  reduced semantics for adaptive reprioritization or feedback-driven
  open-search policy updates beyond one tagged schedule shell
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Policy Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchPolicySubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchPolicySubset.lean)
- [coq/PipelineFnCfgOpenSearchPolicySubset.v](coq/PipelineFnCfgOpenSearchPolicySubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit priority-rewrite
  rules over open-search rounds preserves the discovered halt summary through
  lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one explicit priority-rewrite policy step, but still not a
  reduced semantics for feedback-driven adaptive reprioritization or repeated
  policy updates beyond one rewrite shell
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Adaptive Policy Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchAdaptivePolicySubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchAdaptivePolicySubset.lean)
- [coq/PipelineFnCfgOpenSearchAdaptivePolicySubset.v](coq/PipelineFnCfgOpenSearchAdaptivePolicySubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with feedback-driven
  recomputation of open-search priority rules preserves the discovered halt
  summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one explicit adaptive-rule recomputation step, but still
  not a reduced semantics for repeated closed-loop reprioritization or
  convergence of adaptive open-search policy updates
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Closed-Loop Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchClosedLoopSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchClosedLoopSubset.lean)
- [coq/PipelineFnCfgOpenSearchClosedLoopSubset.v](coq/PipelineFnCfgOpenSearchClosedLoopSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with repeated closed-loop
  adaptive-policy rounds preserves the discovered halt summary trace through
  lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries repeated closed-loop open-search rounds, but still not a
  reduced semantics for meta-iteration or explicit convergence discovery over
  adaptive open-search summaries
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Meta-Iteration Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchMetaIterSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchMetaIterSubset.lean)
- [coq/PipelineFnCfgOpenSearchMetaIterSubset.v](coq/PipelineFnCfgOpenSearchMetaIterSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with last-summary extraction over
  repeated open-search closed-loop rounds preserves the discovered halt
  summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries last-summary extraction, but still not a reduced
  convergence protocol or explicit halt-discovery layer for repeated
  open-search summaries
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Summary-Protocol Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchSummaryProtocolSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchSummaryProtocolSubset.lean)
- [coq/PipelineFnCfgOpenSearchSummaryProtocolSubset.v](coq/PipelineFnCfgOpenSearchSummaryProtocolSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit stable-summary
  protocol rounds preserves the discovered halt summary through lowering and
  emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries stable-summary protocol rounds, but still not a reduced
  convergence protocol or explicit halt witness for repeated open-search
  summaries
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Convergence-Protocol Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchConvergenceProtocolSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchConvergenceProtocolSubset.lean)
- [coq/PipelineFnCfgOpenSearchConvergenceProtocolSubset.v](coq/PipelineFnCfgOpenSearchConvergenceProtocolSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit
  `summary unchanged => halt` witness over repeated open-search summaries
  preserves the discovered halt summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit convergence shell, but still not a reduced
  halt-discovery/search-space layer for repeated open-search summaries
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Halt-Discover Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchHaltDiscoverSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchHaltDiscoverSubset.lean)
- [coq/PipelineFnCfgOpenSearchHaltDiscoverSubset.v](coq/PipelineFnCfgOpenSearchHaltDiscoverSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with an explicit halt-search
  shell over repeated open-search summaries preserves the discovered halt
  summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit halt-discovery shell, but still not a reduced
  open-ended search/worklist layer for repeated open-search summaries
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Frontier Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchFrontierSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchFrontierSubset.lean)
- [coq/PipelineFnCfgOpenSearchFrontierSubset.v](coq/PipelineFnCfgOpenSearchFrontierSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell that reopens the discovered halt
  summary into an explicit `completed + frontier` shell preserves the
  resulting frontier state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries a reopened frontier shell, but still not a reduced
  dynamic-growth/update layer for repeated open-search frontier evolution
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Dynamic-Frontier Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchDynamicFrontierSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchDynamicFrontierSubset.lean)
- [coq/PipelineFnCfgOpenSearchDynamicFrontierSubset.v](coq/PipelineFnCfgOpenSearchDynamicFrontierSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit frontier growth
  after reopening the halt-discovered summary preserves the resulting
  frontier state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one explicit dynamic frontier-growth step, but still not a
  reduced scheduler/policy layer for repeated reopened-frontier evolution
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Frontier-Scheduler Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchFrontierSchedulerSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchFrontierSchedulerSubset.lean)
- [coq/PipelineFnCfgOpenSearchFrontierSchedulerSubset.v](coq/PipelineFnCfgOpenSearchFrontierSchedulerSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit scheduled rounds
  over reopened dynamic frontier states preserves the resulting frontier
  state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries repeated scheduled frontier rounds, but still not a
  reduced priority/policy layer for reopened-frontier evolution
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Frontier-Priority Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchFrontierPrioritySubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchFrontierPrioritySubset.lean)
- [coq/PipelineFnCfgOpenSearchFrontierPrioritySubset.v](coq/PipelineFnCfgOpenSearchFrontierPrioritySubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit priority-labeled
  reopened-frontier rounds preserves the resulting frontier state through
  lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries priority-labeled frontier rounds, but still not a reduced
  policy/adaptive reprioritization layer for reopened-frontier evolution
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Frontier-Policy Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchFrontierPolicySubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchFrontierPolicySubset.lean)
- [coq/PipelineFnCfgOpenSearchFrontierPolicySubset.v](coq/PipelineFnCfgOpenSearchFrontierPolicySubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit priority-rewrite
  rules over reopened-frontier rounds preserves the resulting frontier state
  through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one explicit frontier-policy rewrite step, but still not a
  reduced adaptive reprioritization layer for reopened-frontier evolution
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Frontier-Adaptive-Policy Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchFrontierAdaptivePolicySubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchFrontierAdaptivePolicySubset.lean)
- [coq/PipelineFnCfgOpenSearchFrontierAdaptivePolicySubset.v](coq/PipelineFnCfgOpenSearchFrontierAdaptivePolicySubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with feedback-driven
  recomputation of reopened-frontier priority rules preserves the resulting
  frontier state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one explicit adaptive frontier-policy recomputation step,
  but still not a reduced closed-loop or meta-iteration layer for reopened
  frontier evolution
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Frontier-Closed-Loop Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchFrontierClosedLoopSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchFrontierClosedLoopSubset.lean)
- [coq/PipelineFnCfgOpenSearchFrontierClosedLoopSubset.v](coq/PipelineFnCfgOpenSearchFrontierClosedLoopSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with repeated closed-loop rounds
  over reopened frontier-adaptive-policy states preserves the resulting
  frontier trace through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries repeated closed-loop frontier rounds, but still not a
  reduced meta-iteration or convergence-discovery layer for reopened frontier
  evolution
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Frontier-Meta-Iteration Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchFrontierMetaIterSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchFrontierMetaIterSubset.lean)
- [coq/PipelineFnCfgOpenSearchFrontierMetaIterSubset.v](coq/PipelineFnCfgOpenSearchFrontierMetaIterSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with last-summary extraction over
  repeated reopened-frontier closed-loop rounds preserves the resulting
  frontier summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries last-summary extraction, but still not a reopened-frontier
  summary/convergence protocol or explicit halt-discovery layer
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Frontier-Summary-Protocol Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchFrontierSummaryProtocolSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchFrontierSummaryProtocolSubset.lean)
- [coq/PipelineFnCfgOpenSearchFrontierSummaryProtocolSubset.v](coq/PipelineFnCfgOpenSearchFrontierSummaryProtocolSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit stable-summary
  protocol rounds over reopened frontier evolution preserves the resulting
  frontier summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries stable-summary protocol rounds, but still not a reopened
  frontier convergence protocol or explicit halt-discovery/search-space layer
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Frontier-Convergence-Protocol Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchFrontierConvergenceProtocolSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchFrontierConvergenceProtocolSubset.lean)
- [coq/PipelineFnCfgOpenSearchFrontierConvergenceProtocolSubset.v](coq/PipelineFnCfgOpenSearchFrontierConvergenceProtocolSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit
  `summary unchanged => halt` witness over reopened frontier summaries
  preserves the resulting frontier halt summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit frontier convergence shell, but still not a
  reopened-frontier halt-discovery/search-space layer
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Frontier-Halt-Discover Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchFrontierHaltDiscoverSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchFrontierHaltDiscoverSubset.lean)
- [coq/PipelineFnCfgOpenSearchFrontierHaltDiscoverSubset.v](coq/PipelineFnCfgOpenSearchFrontierHaltDiscoverSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with an explicit halt-search
  shell over reopened frontier summaries preserves the discovered halt
  summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit reopened-frontier halt-discovery shell, but
  still not a reopened-frontier reopen/update layer that starts a new search
  cycle
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Frontier-Reopen Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenSubset.lean)
- [coq/PipelineFnCfgOpenSearchFrontierReopenSubset.v](coq/PipelineFnCfgOpenSearchFrontierReopenSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell that reopens the
  halt-discovered frontier summary into an explicit `completed + frontier`
  shell preserves the resulting frontier state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries a reopened frontier shell after halt discovery, but still
  not the next dynamic-growth/update layer that restarts search over that
  reopened frontier
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Frontier-Reopen-Dynamic Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenDynamicSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenDynamicSubset.lean)
- [coq/PipelineFnCfgOpenSearchFrontierReopenDynamicSubset.v](coq/PipelineFnCfgOpenSearchFrontierReopenDynamicSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit frontier growth
  after reopening the halt-discovered frontier preserves the resulting
  frontier state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one reopened-frontier dynamic-growth step, but still not a
  repeated scheduler/policy layer over the restarted frontier cycle
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Frontier-Reopen-Scheduler Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenSchedulerSubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenSchedulerSubset.lean)
- [coq/PipelineFnCfgOpenSearchFrontierReopenSchedulerSubset.v](coq/PipelineFnCfgOpenSearchFrontierReopenSchedulerSubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with an explicit scheduled-round
  wrapper over reopened-frontier dynamic growth preserves the resulting
  frontier state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one reopened-frontier scheduled-round step, but still not
  priority/policy/adaptive layers over the restarted frontier cycle
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Frontier-Reopen-Priority Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenPrioritySubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenPrioritySubset.lean)
- [coq/PipelineFnCfgOpenSearchFrontierReopenPrioritySubset.v](coq/PipelineFnCfgOpenSearchFrontierReopenPrioritySubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit priority-tagged
  reopened-frontier scheduled rounds preserves the resulting frontier state
  through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one reopened-frontier priority step, but still not
  policy/adaptive layers over the restarted frontier cycle
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Function / CFG Open-Search Frontier-Reopen-Policy Pipeline

Proof layers:
- [lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenPolicySubset.lean](lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenPolicySubset.lean)
- [coq/PipelineFnCfgOpenSearchFrontierReopenPolicySubset.v](coq/PipelineFnCfgOpenSearchFrontierReopenPolicySubset.v)

Core proof claim:
- a reduced predecessor-aware function shell with explicit policy-rewrite
  normalization over reopened-frontier priority rounds preserves the resulting
  frontier state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](../src/codegen/emit/structured.rs#L25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one reopened-frontier policy-normalization step, but still
  not adaptive or closed-loop layers over the restarted frontier cycle
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

### Immediate Next Steps

The most direct next connection points are:

1. attach short Rust source comments at
   [src/mir/lower_hir.rs](../src/mir/lower_hir.rs#L705),
   [src/codegen/mir_emit.rs](../src/codegen/mir_emit.rs#L259),
   and
   [src/compiler/pipeline/compile_api.rs](../src/compiler/pipeline/compile_api.rs#L262)
   that point at the matching proof files
2. lift one reduced proof from expression-level semantics to a small
   block/value environment model closer to real `FnIR`
3. continue moving Coq generic theorems into stable `*GenericSubset.v`
   companions where Rocq compile pathology makes in-file generic proofs brittle


<a id="verify-correspondence"></a>
## Verify Proof ↔ Rust Correspondence

This note ties the reduced verifier proof layers in `proof/` to the concrete
Rust verifier in [src/mir/verify.rs](../src/mir/verify.rs#L1).

The goal is the same as the other correspondence notes:

- identify which proof layer approximates which Rust check group
- make the current abstraction boundary explicit
- keep the next 1:1 refinement target obvious

### Struct Layer

Proof layers:
- [lean/RRProofs/VerifyIrStructLite.lean](lean/RRProofs/VerifyIrStructLite.lean)
- [coq/VerifyIrStructLite.v](coq/VerifyIrStructLite.v)

Core proof claim:
- `body_head` must be reachable
- self-recursive `body_head != entry` functions must have a direct entry edge
  and param-copy-only entry prologue
- entry root / branch target / loop-header shape invariants hold
- `Phi` ownership, predecessor distinctness, and edge-availability hold
- parameter index / call-name / self-reference / non-`Phi` cycle checks hold

Primary Rust correspondence:
- [src/mir/verify.rs](../src/mir/verify.rs#L272)
  entry/body-head and loop-header structural checks
- [src/mir/verify.rs](../src/mir/verify.rs#L526)
  `Phi` shape against CFG predecessors
- [src/mir/verify.rs](../src/mir/verify.rs#L307)
  self-recursive entry prologue restriction

Current gap:
- proof packages booleans/flags rather than the full Rust `FnIR`
- Rust verifier also reasons about exact predecessor sets and inferred owner
  blocks on the concrete CFG

### Flow Layer

Proof layers:
- [lean/RRProofs/VerifyIrMustDefSubset.lean](lean/RRProofs/VerifyIrMustDefSubset.lean)
- [coq/VerifyIrMustDefSubset.v](coq/VerifyIrMustDefSubset.v)
- [lean/RRProofs/VerifyIrMustDefFixedPointSubset.lean](lean/RRProofs/VerifyIrMustDefFixedPointSubset.lean)
- [coq/VerifyIrMustDefFixedPointSubset.v](coq/VerifyIrMustDefFixedPointSubset.v)
- [lean/RRProofs/VerifyIrMustDefConvergenceSubset.lean](lean/RRProofs/VerifyIrMustDefConvergenceSubset.lean)
- [coq/VerifyIrMustDefConvergenceSubset.v](coq/VerifyIrMustDefConvergenceSubset.v)
- [lean/RRProofs/VerifyIrUseTraversalSubset.lean](lean/RRProofs/VerifyIrUseTraversalSubset.lean)
- [coq/VerifyIrUseTraversalSubset.v](coq/VerifyIrUseTraversalSubset.v)
- [lean/RRProofs/VerifyIrValueKindTraversalSubset.lean](lean/RRProofs/VerifyIrValueKindTraversalSubset.lean)
- [coq/VerifyIrValueKindTraversalSubset.v](coq/VerifyIrValueKindTraversalSubset.v)
- [lean/RRProofs/VerifyIrArgListTraversalSubset.lean](lean/RRProofs/VerifyIrArgListTraversalSubset.lean)
- [coq/VerifyIrArgListTraversalSubset.v](coq/VerifyIrArgListTraversalSubset.v)
- [lean/RRProofs/VerifyIrEnvScanComposeSubset.lean](lean/RRProofs/VerifyIrEnvScanComposeSubset.lean)
- [coq/VerifyIrEnvScanComposeSubset.v](coq/VerifyIrEnvScanComposeSubset.v)
- [lean/RRProofs/VerifyIrConsumerMetaSubset.lean](lean/RRProofs/VerifyIrConsumerMetaSubset.lean)
- [coq/VerifyIrConsumerMetaSubset.v](coq/VerifyIrConsumerMetaSubset.v)
- [lean/RRProofs/VerifyIrConsumerGraphSubset.lean](lean/RRProofs/VerifyIrConsumerGraphSubset.lean)
- [coq/VerifyIrConsumerGraphSubset.v](coq/VerifyIrConsumerGraphSubset.v)
- [lean/RRProofs/VerifyIrChildDepsSubset.lean](lean/RRProofs/VerifyIrChildDepsSubset.lean)
- [coq/VerifyIrChildDepsSubset.v](coq/VerifyIrChildDepsSubset.v)
- [lean/RRProofs/VerifyIrValueDepsWalkSubset.lean](lean/RRProofs/VerifyIrValueDepsWalkSubset.lean)
- [coq/VerifyIrValueDepsWalkSubset.v](coq/VerifyIrValueDepsWalkSubset.v)
- [lean/RRProofs/VerifyIrValueTableWalkSubset.lean](lean/RRProofs/VerifyIrValueTableWalkSubset.lean)
- [coq/VerifyIrValueTableWalkSubset.v](coq/VerifyIrValueTableWalkSubset.v)
- [lean/RRProofs/VerifyIrValueKindTableSubset.lean](lean/RRProofs/VerifyIrValueKindTableSubset.lean)
- [coq/VerifyIrValueKindTableSubset.v](coq/VerifyIrValueKindTableSubset.v)
- [lean/RRProofs/VerifyIrValueRecordSubset.lean](lean/RRProofs/VerifyIrValueRecordSubset.lean)
- [coq/VerifyIrValueRecordSubset.v](coq/VerifyIrValueRecordSubset.v)
- [lean/RRProofs/VerifyIrValueFullRecordSubset.lean](lean/RRProofs/VerifyIrValueFullRecordSubset.lean)
- [coq/VerifyIrValueFullRecordSubset.v](coq/VerifyIrValueFullRecordSubset.v)
- [lean/RRProofs/VerifyIrFnRecordSubset.lean](lean/RRProofs/VerifyIrFnRecordSubset.lean)
- [coq/VerifyIrFnRecordSubset.v](coq/VerifyIrFnRecordSubset.v)
- [lean/RRProofs/VerifyIrFnMetaSubset.lean](lean/RRProofs/VerifyIrFnMetaSubset.lean)
- [coq/VerifyIrFnMetaSubset.v](coq/VerifyIrFnMetaSubset.v)
- [lean/RRProofs/VerifyIrFnParamMetaSubset.lean](lean/RRProofs/VerifyIrFnParamMetaSubset.lean)
- [coq/VerifyIrFnParamMetaSubset.v](coq/VerifyIrFnParamMetaSubset.v)
- [lean/RRProofs/VerifyIrFnHintMapSubset.lean](lean/RRProofs/VerifyIrFnHintMapSubset.lean)
- [coq/VerifyIrFnHintMapSubset.v](coq/VerifyIrFnHintMapSubset.v)
- [lean/RRProofs/VerifyIrBlockRecordSubset.lean](lean/RRProofs/VerifyIrBlockRecordSubset.lean)
- [coq/VerifyIrBlockRecordSubset.v](coq/VerifyIrBlockRecordSubset.v)
- [lean/RRProofs/VerifyIrBlockFlowSubset.lean](lean/RRProofs/VerifyIrBlockFlowSubset.lean)
- [coq/VerifyIrBlockFlowSubset.v](coq/VerifyIrBlockFlowSubset.v)
- [lean/RRProofs/VerifyIrBlockMustDefSubset.lean](lean/RRProofs/VerifyIrBlockMustDefSubset.lean)
- [coq/VerifyIrBlockMustDefSubset.v](coq/VerifyIrBlockMustDefSubset.v)
- [lean/RRProofs/VerifyIrBlockMustDefComposeSubset.lean](lean/RRProofs/VerifyIrBlockMustDefComposeSubset.lean)
- [coq/VerifyIrBlockMustDefComposeSubset.v](coq/VerifyIrBlockMustDefComposeSubset.v)
- [lean/RRProofs/VerifyIrBlockAssignFlowSubset.lean](lean/RRProofs/VerifyIrBlockAssignFlowSubset.lean)
- [coq/VerifyIrBlockAssignFlowSubset.v](coq/VerifyIrBlockAssignFlowSubset.v)
- [lean/RRProofs/VerifyIrBlockAssignChainSubset.lean](lean/RRProofs/VerifyIrBlockAssignChainSubset.lean)
- [coq/VerifyIrBlockAssignChainSubset.v](coq/VerifyIrBlockAssignChainSubset.v)
- [lean/RRProofs/VerifyIrBlockAssignBranchSubset.lean](lean/RRProofs/VerifyIrBlockAssignBranchSubset.lean)
- [coq/VerifyIrBlockAssignBranchSubset.v](coq/VerifyIrBlockAssignBranchSubset.v)
- [lean/RRProofs/VerifyIrBlockAssignStoreSubset.lean](lean/RRProofs/VerifyIrBlockAssignStoreSubset.lean)
- [coq/VerifyIrBlockAssignStoreSubset.v](coq/VerifyIrBlockAssignStoreSubset.v)
- [lean/RRProofs/VerifyIrBlockDefinedHereSubset.lean](lean/RRProofs/VerifyIrBlockDefinedHereSubset.lean)
- [coq/VerifyIrBlockDefinedHereSubset.v](coq/VerifyIrBlockDefinedHereSubset.v)
- [lean/RRProofs/VerifyIrBlockExecutableSubset.lean](lean/RRProofs/VerifyIrBlockExecutableSubset.lean)
- [coq/VerifyIrBlockExecutableSubset.v](coq/VerifyIrBlockExecutableSubset.v)
- [lean/RRProofs/VerifyIrTwoBlockExecutableSubset.lean](lean/RRProofs/VerifyIrTwoBlockExecutableSubset.lean)
- [coq/VerifyIrTwoBlockExecutableSubset.v](coq/VerifyIrTwoBlockExecutableSubset.v)
- [lean/RRProofs/VerifyIrJoinExecutableSubset.lean](lean/RRProofs/VerifyIrJoinExecutableSubset.lean)
- [coq/VerifyIrJoinExecutableSubset.v](coq/VerifyIrJoinExecutableSubset.v)
- [lean/RRProofs/VerifyIrCfgExecutableSubset.lean](lean/RRProofs/VerifyIrCfgExecutableSubset.lean)
- [coq/VerifyIrCfgExecutableSubset.v](coq/VerifyIrCfgExecutableSubset.v)
- [lean/RRProofs/VerifyIrCfgReachabilitySubset.lean](lean/RRProofs/VerifyIrCfgReachabilitySubset.lean)
- [coq/VerifyIrCfgReachabilitySubset.v](coq/VerifyIrCfgReachabilitySubset.v)
- [lean/RRProofs/VerifyIrCfgConvergenceSubset.lean](lean/RRProofs/VerifyIrCfgConvergenceSubset.lean)
- [coq/VerifyIrCfgConvergenceSubset.v](coq/VerifyIrCfgConvergenceSubset.v)
- [lean/RRProofs/VerifyIrCfgWorklistSubset.lean](lean/RRProofs/VerifyIrCfgWorklistSubset.lean)
- [coq/VerifyIrCfgWorklistSubset.v](coq/VerifyIrCfgWorklistSubset.v)
- [lean/RRProofs/VerifyIrCfgOrderWorklistSubset.lean](lean/RRProofs/VerifyIrCfgOrderWorklistSubset.lean)
- [coq/VerifyIrCfgOrderWorklistSubset.v](coq/VerifyIrCfgOrderWorklistSubset.v)
- [lean/RRProofs/VerifyIrCfgFixedPointSubset.lean](lean/RRProofs/VerifyIrCfgFixedPointSubset.lean)
- [coq/VerifyIrCfgFixedPointSubset.v](coq/VerifyIrCfgFixedPointSubset.v)
- [lean/RRProofs/VerifyIrFlowLite.lean](lean/RRProofs/VerifyIrFlowLite.lean)
- [coq/VerifyIrFlowLite.v](coq/VerifyIrFlowLite.v)

Core proof claim:
- predecessor out-set intersection computes reduced must-defined join facts
- local assignment extends that must-defined set monotonically
- reachable-predecessor filtering and one reduced fixed-point step preserve
  those join facts into the next out-set map
- stable reduced out-set maps remain unchanged under further iteration
- reduced recursive load/wrapper traversal returns `none` whenever every
  observed load is must-defined
- reduced `ValueKind`-named wrappers such as `Intrinsic`, `RecordLit`,
  `FieldSet`, `Index*`, `Range`, and `Binary` also preserve the absence of
  undefined loads under the same must-defined assumption
- reduced arg-list and named-field-list scans for `Call`, `Intrinsic`, and
  `RecordLit` also preserve the absence of undefined loads under the same
  must-defined assumption
- env-selected scans and `ValueKind` arg/field scans can be packaged together
  under reusable compose-case and cross-case theorems, and reduced generic
  list/field composition theorems now quantify directly over selected-env
  clean facts and value-kind clean facts, with concrete call/record examples
  as instances
- those reduced composition theorems can then be re-packaged under explicit
  heterogeneous consumer constructors for `Call`, `Intrinsic`, and `RecordLit`
- those heterogeneous consumer constructors can then be lifted into a reduced
  `node-id + seen + fuel` graph so shared children and recursive wrapper
  parents approximate the concrete `ValueId` traversal discipline
- reduced child-edge extraction for non-`Phi` nodes now also mirrors the
  exact helper shape for unary wrappers, arg lists, field lists, and `Index*`
  nodes used before recursive traversal in Rust
- full reduced `value_dependencies` now also includes `Phi` arg lists and is
  lifted into a reduced seen/fuel stack walk approximating
  `depends_on_phi_in_block_except`
- that reduced seen/fuel walk is now also rephrased over an explicit
  `ValueId -> table row` lookup with stored `phi_block` metadata, closer to
  the concrete `FnIR.values` table shape
- those explicit table rows are now also refined to actual `ValueKind`-named
  payload constructors, rather than only reduced dependency tags
- those rows are now also lifted again to a reduced `Value` record carrying
  `id`, `kind`, `origin_var`, `phi_block`, and `escape`
- that reduced `Value` record is now also extended with `span`, `facts`,
  `value_ty`, and `value_term`, so nearly all fields of the concrete record are
  represented
- those reduced full `Value` rows are now also packaged into a small
  `FnIR`-style record carrying `name`, `params`, `values`, `blocks`, `entry`,
  and `body_head`
- that small reduced `FnIR` shell is now also refined again with reduced
  `user_name`, return-hint, inferred-return, and fallback/interop metadata
  while keeping the current verifier-facing value/table walk theorems
  projected onto the same smaller shell
- that same reduced function shell is now also refined again with
  `param_default_r_exprs`, `param_spans`, `param_ty_hints`,
  `param_term_hints`, and `param_hint_spans`, still projecting the current
  verifier-facing walks onto the same smaller shell
- that same reduced function shell is now also refined with reduced
  `call_semantics` and `memory_layout_hints` maps, still projecting the
  current verifier-facing walks onto the same smaller shell
- that same reduced function shell is now also refined with reduced
  `Block`/`Terminator` payloads carrying explicit instruction lists and
  terminator operands, still projecting the current verifier-facing walks
  onto the same smaller shell
- those reduced block payloads are now also connected back to reduced
  `UseBeforeDef` obligations by looking operand ids up through the reduced
  value table's `origin_var` field and packaging the resulting requirements
  as `VerifyIrFlowLite` blocks
- that block-flow bridge is now also composed directly with the reduced
  must-defined chain, so reduced join facts can certify explicit block payloads
  as `UseBeforeDef`-clean
- that same bridge is now also lifted to generic `required ⊆ defs`
  packaging, and multi-read block payloads can be certified clean from
  multiple reduced join facts at once
- local `assign` writes are now also packaged explicitly, so a reduced
  block may consume one incoming must-defined source var and then satisfy
  later reads of the destination var from block-local writes
- that same block-local write story is now also extended to a two-step local
  def chain, closer to the concrete `defined_here` growth across several
  `Assign` instructions before a later read
- that same local def-chain story is now also extended through a branch
  terminator, closer to the concrete case where `defined_here` must also
  discharge `If { cond, .. }` after preceding `Assign` instructions
- that same local def-chain story is now also extended through
  `StoreIndex1D/2D/3D`, closer to the concrete case where `defined_here` must
  also discharge store operands after preceding `Assign` instructions
- the sequential `defined_here` growth itself is now also packaged as a
  reusable reduced theorem, closer to the concrete loop that updates
  `defined_here` after each `Assign`
- those reusable block-local flow and `defined_here` theorems are now also
  packaged back into a single-block executable theorem, closer to the concrete
  verifier's ordered per-block acceptance story
- that executable packaging is now also extended to an ordered two-block case,
  closer to the concrete verifier's multi-block acceptance order after
  predecessor-selected `in_defs` are fixed
- that same executable packaging is now also extended to a join-shaped
  three-block case with left/right sibling blocks and a join block, closer to
  the small ordered bundles the concrete verifier reasons about after
  predecessor-selected `in_defs` are fixed
- that same join packaging is now also lifted into an explicit CFG witness
  record carrying reduced predecessor-map and block-order data, closer to the
  concrete verifier's explicit CFG reasoning surface
- that same CFG witness is now also tied directly to reduced
  `reachable/preds/outDefs` data, so the join block's incoming defs are
  justified through reduced `stepInDefs`
- that same reduced CFG witness is now also tied to a stable reduced out-map
  witness, so once the must-defined iteration has converged, iterated out-def
  maps can be re-used directly to justify reduced CFG acceptance
- that same stable reduced out-map witness is now also tied to a reduced
  join-focused worklist `changed` bit, closer to the concrete
  `compute_must_defined_vars` loop that stops once no block update changes the
  current out-def map
- that same reduced worklist story is now also lifted to a small block-order
  aggregation over left/right/join and then packaged as a reduced whole-CFG
  fixed-point checker, closer to the concrete `changed` loop and its
  `if !changed { break; }` exit condition
- required loads must already be defined on the path that reaches an
  instruction or terminator

Primary Rust correspondence:
- [src/mir/def.rs](../src/mir/def.rs#L118)
  concrete `FnIR` record layout
- [src/mir/def.rs](../src/mir/def.rs#L213)
  concrete `Value` record layout
- [src/mir/def.rs](../src/mir/def.rs#L561)
  concrete `value_dependencies`
- [src/mir/verify.rs](../src/mir/verify.rs#L929)
  concrete `non_phi_dependencies`
- [src/mir/verify.rs](../src/mir/verify.rs#L608)
  instruction/terminator use-before-def checks
- [src/mir/verify.rs](../src/mir/verify.rs#L729)
  `Phi` edge availability against predecessor out-def sets
- [src/mir/verify.rs](../src/mir/verify.rs#L1041)
  concrete `compute_must_defined_vars`
- [src/mir/verify.rs](../src/mir/verify.rs#L1124)
  recursive `first_undefined_load_in_value`

Current gap:
- proof now models the core predecessor-intersection / local-assign step, but
  still uses reduced lists/functions rather than the full `FnIR` CFG/worklist
  state and full termination/convergence argument
- wrapper traversal now has reduced `ValueKind`-named cases and reduced
  arg-list forms, but it still does not model exact heterogeneous field/arg
  metadata or the full `ValueId` graph
- Rust verifier computes concrete must-defined sets over the real CFG

### Phi Edge Value Environment

Proof layers:
- [lean/RRProofs/VerifyIrValueEnvSubset.lean](lean/RRProofs/VerifyIrValueEnvSubset.lean)
- [coq/VerifyIrValueEnvSubset.v](coq/VerifyIrValueEnvSubset.v)
- [lean/RRProofs/VerifyIrArgEnvSubset.lean](lean/RRProofs/VerifyIrArgEnvSubset.lean)
- [coq/VerifyIrArgEnvSubset.v](coq/VerifyIrArgEnvSubset.v)
- [lean/RRProofs/VerifyIrArgEnvTraversalSubset.lean](lean/RRProofs/VerifyIrArgEnvTraversalSubset.lean)
- [coq/VerifyIrArgEnvTraversalSubset.v](coq/VerifyIrArgEnvTraversalSubset.v)
- [lean/RRProofs/VerifyIrEnvScanComposeSubset.lean](lean/RRProofs/VerifyIrEnvScanComposeSubset.lean)
- [coq/VerifyIrEnvScanComposeSubset.v](coq/VerifyIrEnvScanComposeSubset.v)
- [lean/RRProofs/VerifyIrConsumerMetaSubset.lean](lean/RRProofs/VerifyIrConsumerMetaSubset.lean)
- [coq/VerifyIrConsumerMetaSubset.v](coq/VerifyIrConsumerMetaSubset.v)
- [lean/RRProofs/VerifyIrConsumerGraphSubset.lean](lean/RRProofs/VerifyIrConsumerGraphSubset.lean)
- [coq/VerifyIrConsumerGraphSubset.v](coq/VerifyIrConsumerGraphSubset.v)
- [lean/RRProofs/VerifyIrChildDepsSubset.lean](lean/RRProofs/VerifyIrChildDepsSubset.lean)
- [coq/VerifyIrChildDepsSubset.v](coq/VerifyIrChildDepsSubset.v)
- [lean/RRProofs/VerifyIrValueDepsWalkSubset.lean](lean/RRProofs/VerifyIrValueDepsWalkSubset.lean)
- [coq/VerifyIrValueDepsWalkSubset.v](coq/VerifyIrValueDepsWalkSubset.v)
- [lean/RRProofs/VerifyIrValueTableWalkSubset.lean](lean/RRProofs/VerifyIrValueTableWalkSubset.lean)
- [coq/VerifyIrValueTableWalkSubset.v](coq/VerifyIrValueTableWalkSubset.v)
- [lean/RRProofs/VerifyIrValueKindTableSubset.lean](lean/RRProofs/VerifyIrValueKindTableSubset.lean)
- [coq/VerifyIrValueKindTableSubset.v](coq/VerifyIrValueKindTableSubset.v)
- [lean/RRProofs/VerifyIrValueRecordSubset.lean](lean/RRProofs/VerifyIrValueRecordSubset.lean)
- [coq/VerifyIrValueRecordSubset.v](coq/VerifyIrValueRecordSubset.v)
- [lean/RRProofs/VerifyIrValueFullRecordSubset.lean](lean/RRProofs/VerifyIrValueFullRecordSubset.lean)
- [coq/VerifyIrValueFullRecordSubset.v](coq/VerifyIrValueFullRecordSubset.v)
- [lean/RRProofs/VerifyIrFnRecordSubset.lean](lean/RRProofs/VerifyIrFnRecordSubset.lean)
- [coq/VerifyIrFnRecordSubset.v](coq/VerifyIrFnRecordSubset.v)
- [lean/RRProofs/VerifyIrFnMetaSubset.lean](lean/RRProofs/VerifyIrFnMetaSubset.lean)
- [coq/VerifyIrFnMetaSubset.v](coq/VerifyIrFnMetaSubset.v)
- [lean/RRProofs/VerifyIrFnParamMetaSubset.lean](lean/RRProofs/VerifyIrFnParamMetaSubset.lean)
- [coq/VerifyIrFnParamMetaSubset.v](coq/VerifyIrFnParamMetaSubset.v)
- [lean/RRProofs/VerifyIrFnHintMapSubset.lean](lean/RRProofs/VerifyIrFnHintMapSubset.lean)
- [coq/VerifyIrFnHintMapSubset.v](coq/VerifyIrFnHintMapSubset.v)
- [lean/RRProofs/VerifyIrBlockRecordSubset.lean](lean/RRProofs/VerifyIrBlockRecordSubset.lean)
- [coq/VerifyIrBlockRecordSubset.v](coq/VerifyIrBlockRecordSubset.v)
- [lean/RRProofs/VerifyIrBlockFlowSubset.lean](lean/RRProofs/VerifyIrBlockFlowSubset.lean)
- [coq/VerifyIrBlockFlowSubset.v](coq/VerifyIrBlockFlowSubset.v)
- [lean/RRProofs/VerifyIrBlockMustDefSubset.lean](lean/RRProofs/VerifyIrBlockMustDefSubset.lean)
- [coq/VerifyIrBlockMustDefSubset.v](coq/VerifyIrBlockMustDefSubset.v)
- [lean/RRProofs/VerifyIrBlockMustDefComposeSubset.lean](lean/RRProofs/VerifyIrBlockMustDefComposeSubset.lean)
- [coq/VerifyIrBlockMustDefComposeSubset.v](coq/VerifyIrBlockMustDefComposeSubset.v)
- [lean/RRProofs/VerifyIrBlockAssignFlowSubset.lean](lean/RRProofs/VerifyIrBlockAssignFlowSubset.lean)
- [coq/VerifyIrBlockAssignFlowSubset.v](coq/VerifyIrBlockAssignFlowSubset.v)
- [lean/RRProofs/VerifyIrBlockAssignChainSubset.lean](lean/RRProofs/VerifyIrBlockAssignChainSubset.lean)
- [coq/VerifyIrBlockAssignChainSubset.v](coq/VerifyIrBlockAssignChainSubset.v)
- [lean/RRProofs/VerifyIrBlockAssignBranchSubset.lean](lean/RRProofs/VerifyIrBlockAssignBranchSubset.lean)
- [coq/VerifyIrBlockAssignBranchSubset.v](coq/VerifyIrBlockAssignBranchSubset.v)
- [lean/RRProofs/VerifyIrBlockAssignStoreSubset.lean](lean/RRProofs/VerifyIrBlockAssignStoreSubset.lean)
- [coq/VerifyIrBlockAssignStoreSubset.v](coq/VerifyIrBlockAssignStoreSubset.v)
- [lean/RRProofs/VerifyIrBlockDefinedHereSubset.lean](lean/RRProofs/VerifyIrBlockDefinedHereSubset.lean)
- [coq/VerifyIrBlockDefinedHereSubset.v](coq/VerifyIrBlockDefinedHereSubset.v)
- [lean/RRProofs/VerifyIrBlockExecutableSubset.lean](lean/RRProofs/VerifyIrBlockExecutableSubset.lean)
- [coq/VerifyIrBlockExecutableSubset.v](coq/VerifyIrBlockExecutableSubset.v)
- [lean/RRProofs/VerifyIrTwoBlockExecutableSubset.lean](lean/RRProofs/VerifyIrTwoBlockExecutableSubset.lean)
- [coq/VerifyIrTwoBlockExecutableSubset.v](coq/VerifyIrTwoBlockExecutableSubset.v)
- [lean/RRProofs/VerifyIrJoinExecutableSubset.lean](lean/RRProofs/VerifyIrJoinExecutableSubset.lean)
- [coq/VerifyIrJoinExecutableSubset.v](coq/VerifyIrJoinExecutableSubset.v)
- [lean/RRProofs/VerifyIrCfgExecutableSubset.lean](lean/RRProofs/VerifyIrCfgExecutableSubset.lean)
- [coq/VerifyIrCfgExecutableSubset.v](coq/VerifyIrCfgExecutableSubset.v)
- [lean/RRProofs/VerifyIrCfgReachabilitySubset.lean](lean/RRProofs/VerifyIrCfgReachabilitySubset.lean)
- [coq/VerifyIrCfgReachabilitySubset.v](coq/VerifyIrCfgReachabilitySubset.v)

Core proof claim:
- an explicit `ValueId`/`BlockId` environment can model predecessor-selected
  `Phi` values directly
- rewriting a consumer from the merged `Phi` id to the predecessor-selected
  source id preserves evaluation
- that same predecessor-selected rewrite also preserves reduced arg-list and
  field-list consumer evaluation
- that same predecessor-selected rewrite also preserves reduced missing-use
  scans over arg lists and field lists
- those env-selected scan facts can also be packaged alongside reduced
  `ValueKind` arg/field scan facts for the same concrete correspondence cases,
  and are now linked by reduced generic list/field composition theorems rather
  than example-only packaging
- those same reduced composition theorems are now also lifted under explicit
  heterogeneous consumer metadata for `Call`, `Intrinsic`, and `RecordLit`
- those heterogeneous consumer metadata cases are now also lifted into a
  reduced graph with shared node ids, seen sets, and fuel-based recursion
  closer to the concrete recursive traversal
- the exact non-`Phi` child extraction shape used to seed that recursion is
  now also modeled directly for unary wrappers, arg lists, field lists, and
  `Index*` nodes
- the full `value_dependencies` shape, including `Phi` arg lists, is now also
  modeled directly and composed into a reduced stack walk for
  `depends_on_phi_in_block_except`
- that reduced stack walk is now also lifted to an explicit lookup-table model
  with stored `phi_block` metadata closer to the concrete `FnIR.values` table
- those explicit table rows are now also refined to actual `ValueKind`-named
  payload constructors, closer to the top-level row kinds stored in
  `FnIR.values`
- those rows are now also lifted to a reduced `Value` record carrying the
  main per-value fields used by the concrete table
- that reduced `Value` record is now also extended with reduced
  `span/facts/value_ty/value_term` fields so nearly all top-level fields of the
  concrete `Value` row are represented
- those reduced rows are now also packaged into a small `FnIR` shell carrying
  `name/params/values/blocks/entry/body_head`
- that small `FnIR` shell is now also refined with reduced `user_name`,
  return-hint, inferred-return, and fallback/interop metadata while still
  projecting the current verifier-relevant walk back onto the same shell
- that same reduced `FnIR` shell is now also refined with reduced parameter
  defaults, per-parameter spans, and per-parameter type/term/hint-span lists
  while still projecting the current verifier-relevant walk back onto the same
  shell
- that same reduced `FnIR` shell is now also refined with reduced
  `call_semantics` and `memory_layout_hints` maps while still projecting the
  current verifier-relevant walk back onto the same shell
- that same reduced `FnIR` shell is now also refined with reduced
  `Block`/`Terminator` payloads carrying explicit instruction lists and
  terminator operands while still projecting the current verifier-relevant
  walk back onto the same shell
- those reduced block payloads are now also connected back to reduced
  `UseBeforeDef` obligations through `origin_var` lookup over the reduced
  value table, closer to the concrete instruction/terminator operand checks
- that same block-flow bridge is now also composed with reduced must-defined
  join facts, closer to the concrete `in_defs` / instruction-operand story
- that same bridge is now also lifted to generic `required ⊆ defs`
  packaging, closer to the concrete story that every operand-derived required
  load must already lie in the block's incoming must-defined set
- block-local writes are now also packaged explicitly, closer to the concrete
  story that `defined_here` grows after each `Assign` and may discharge later
  operand uses within the same block
- that same `defined_here` story is now also extended over a two-step local
  def chain before a later read, closer to the concrete sequential block scan
- that same sequential block story is now also extended to a branch
  terminator condition after the local def chain
- that same sequential block story is now also extended to store operand
  bundles after the local def chain
- the sequential `defined_here` growth itself is now also isolated as a
  reusable reduced theorem over block scans
- those reusable block-local theorems are now also packaged back into a
  single-block executable acceptance theorem over reduced `VerifyIrFlowLite`
- that single-block executable packaging is now also extended to an ordered
  two-block acceptance theorem over reduced `VerifyIrFlowLite`
- that same executable packaging is now also extended to a join-shaped
  three-block acceptance theorem over reduced `VerifyIrFlowLite`
- that same packaging is now also lifted into an explicit CFG witness record
  carrying reduced predecessor/order data
- that same explicit CFG witness is now also tied directly to reduced
  `reachable/preds/outDefs` computation data

Primary Rust correspondence:
- [src/mir/verify.rs](../src/mir/verify.rs#L559)
  reachable predecessor matching for `Phi` args
- [src/mir/verify.rs](../src/mir/verify.rs#L603)
  `Phi` edge-availability and current-block-phi exclusion
- [src/mir/verify.rs](../src/mir/verify.rs#L976)
  `infer_phi_owner_block`
- [src/mir/verify.rs](../src/mir/verify.rs#L949)
  `depends_on_phi_in_block_except`

Current gap:
- proof still abstracts away the full `FnIR` graph and uses a reduced explicit
  environment model
- proof now composes env rewriting with reduced list consumers, but it still
  does not connect those consumers back to the concrete `ValueId` graph used by
  `first_undefined_load_in_value`
- proof now has a reduced table-driven walk with `ValueKind`-named rows, but
  it still does not use the full concrete `Value` payload fields or exact
  verifier stack/update discipline
- the new reduced full `Value` record still compresses
  `span/facts/value_ty/value_term/escape` into small tags rather than the full
  concrete payloads
- the new reduced `FnIR` shell still omits many concrete fields such as
  return/type hints, fallback flags, interop metadata, and call semantics
- Rust verifier also combines owner-block inference, predecessor filtering, and
  use-before-def propagation over the concrete CFG

### Executable Layer

Proof layers:
- [lean/RRProofs/VerifyIrExecutableLite.lean](lean/RRProofs/VerifyIrExecutableLite.lean)
- [coq/VerifyIrExecutableLite.v](coq/VerifyIrExecutableLite.v)

Core proof claim:
- block ids / value ids / intrinsic arities / terminators are structurally
  executable
- emittable MIR must also eliminate reachable `Phi`

Primary Rust correspondence:
- [src/mir/verify.rs](../src/mir/verify.rs#L307)
  operand/id/arity checking during value validation
- [src/mir/verify.rs](../src/mir/verify.rs#L807)
  `verify_emittable_ir`

Current gap:
- proof collapses several Rust check groups into coarse booleans
- Rust verifier still traverses full values/instructions and exact ids

### Rust Error Name Layer

Proof layers:
- [lean/RRProofs/VerifyIrRustErrorLite.lean](lean/RRProofs/VerifyIrRustErrorLite.lean)
- [coq/VerifyIrRustErrorLite.v](coq/VerifyIrRustErrorLite.v)

Core proof claim:
- reduced proof-side failures map onto Rust-enum-shaped verifier names

Primary Rust correspondence:
- [src/mir/verify.rs](../src/mir/verify.rs#L7)
  `VerifyError`
- [src/mir/verify.rs](../src/mir/verify.rs#L91)
  displayed user-facing error categories

Current gap:
- proof maps names, not full diagnostic payloads or spans
- Rust side still contains richer source attribution and staging details

### Immediate Next Steps

The most direct next refinements are:

1. lift one `VerifyIrStructLite` boolean bundle into a reduced explicit CFG
   predecessor map closer to real `FnIR`
2. replace the current reduced lookup-table / seen / fuel walk with a closer
   approximation of the real heterogeneous `FnIR.values` table, exact
   `ValueKind` payloads, and concrete stack discipline used by
   `depends_on_phi_in_block_except` and `first_undefined_load_in_value`
3. document which verifier checks are intentionally restricted to
   self-recursive/TCO shapes versus general emittable MIR


<a id="runtime-safety-correspondence"></a>
## Runtime Safety Proof ↔ Rust Correspondence

This note ties the reduced runtime-safety proof slice in `proof/` to the
concrete range-analysis and diagnostic checks in `src/mir/`.

It is not a full proof of `validate_runtime_safety`. The point is narrower:

- make explicit which reduced theorem matches the current field-range hazard
  story
- identify the Rust helpers that consume those range facts
- keep the remaining proof gap visible

### Field Range Hazard Slice

Proof layers:
- [RuntimeSafetyFieldRangeSubset.lean](lean/RRProofs/RuntimeSafetyFieldRangeSubset.lean)
- [RuntimeSafetyFieldRangeSubset.v](coq/RuntimeSafetyFieldRangeSubset.v)

Core proof claim:
- reduced record-field interval propagation preserves exact singleton intervals
- negative singleton intervals survive
  - plain field reads
  - nested field reads
  - negative `FieldSet` overrides
- positive `FieldSet` overrides clear the reduced `< 1` hazard

Primary Rust correspondence:
- [range.rs](../src/mir/analyze/range.rs#L349)
  `ensure_field_range()` joins the candidate field values collected from
  `RecordLit` / `FieldSet` structure
- [runtime_proofs.rs](../src/mir/semantics/runtime_proofs.rs#L108)
  `interval_guarantees_below_one()` projects reduced range facts into the
  1-based indexing hazard
- [runtime_proofs.rs](../src/mir/semantics/runtime_proofs.rs#L114)
  `interval_guarantees_negative()` projects reduced range facts into the
  negative-length hazard
- [semantics.rs](../src/mir/semantics.rs#L366)
  `validate_function_runtime()` is the concrete consumer that combines those
  range predicates with E2007-style diagnostics

Concrete Rust regressions:
- [field_get_reads_exact_field_interval_from_record_literal](../src/mir/analyze/range.rs#L600)
- [field_get_tracks_fieldset_override_range_precisely](../src/mir/analyze/range.rs#L633)
- [nested_field_get_reads_exact_interval](../src/mir/analyze/range.rs#L676)
- [field_get_reads_exact_interval_through_phi_merged_records](../src/mir/analyze/range.rs#L726)
- [field_get_joins_interval_through_phi_merged_records](../src/mir/analyze/range.rs#L779)
- [runtime_safety_flags_negative_index_through_phi_merged_record_field](../src/mir/semantics.rs#L155)
- [runtime_safety_does_not_treat_unknown_index_as_proven_below_one](../src/mir/semantics.rs#L269)
- [runtime_safety_does_not_treat_unknown_seq_len_arg_as_proven_negative](../src/mir/semantics.rs#L329)
- [runtime_safety_flags_negative_seq_len_through_nested_record_field](../src/mir/semantics.rs#L366)
- [runtime_safety_flags_negative_seq_len_through_fieldset_override](../src/mir/semantics.rs#L435)
- [runtime_safety_does_not_flag_positive_seq_len_after_fieldset_override](../src/mir/semantics.rs#L503)

Current gap:
- proof is still expression/range-level only
- it does not yet model the whole block/dataflow fixed-point used by
  `validate_function_runtime()`
- it also abstracts away unrelated runtime hazards such as NA propagation,
  aliasing, and non-field index arithmetic


<a id="raw-rewrite-pass-catalog-audit"></a>
## Hermes Raw Rewrite Pass Catalog Audit

This document fixes the claim boundary for the emitted-R raw text rewrite
manager under `src/compiler/pipeline/phases/source_emit/raw_emit`.

### Scope

Raw rewrite passes run after MIR lowering/codegen has already emitted R text.
They are backend text rewrites, not MIR optimizer passes, so they are not owned
by Chronos. Hermes is the backend emit pass-manager boundary for this layer. It
exists to make ordering, gating, cache salt coverage, and review boundaries
explicit.

### Manager-Owned Stages

| Stage | Production pass group | Boundary |
| --- | --- | --- |
| `FragmentRawRewrite` | per-function raw emitted-R cleanup before emitted-R peephole optimization | Hermes stage catalog in `raw_pass_manager.rs`; skipped for unsafe R escapes |
| `FullProgramRawRewrite` | assembled-program raw emitted-R cleanup before final peephole/runtime wrapping | Hermes stage catalog in `raw_pass_manager.rs`; `preserve_all_defs` gates unreachable-helper pruning |
| `PostAssemblyFinalize` | final whitespace/comment cleanup after assembly | Hermes stage catalog in `raw_pass_manager.rs` |

### Claim Boundary

The current Hermes manager is a production orchestration boundary. It preserves
the previous pass order and conditionals while making the stage catalog
explicit. It is not a formal proof that every raw text rewrite preserves R
semantics.

The cache salt includes the raw-emission pass manager and its sibling
raw-emission modules, so changing the pass catalog invalidates persisted compile
output caches.

### Remaining Gaps

| Gap | Why it remains |
| --- | --- |
| per-pass raw text semantics proof | raw rewrites operate on emitted R text and need a separate reduced R-text semantics model |
| per-pass timing profile | current public profile schema exposes aggregate `raw_rewrite_elapsed_ns`; splitting it would change profile output |
| MIR-level proof composition | raw rewrite soundness must be attached at backend/codegen level, not Chronos MIR level |


<a id="peephole-stage-catalog-audit"></a>
## Peephole Stage Catalog Audit

This document fixes the current claim boundary for the emitted-R peephole
pipeline under `src/compiler/peephole`.

### Scope

The peephole pipeline runs after raw emitted-R rewrites and before final source
map remapping. It operates over emitted R lines and emitted-IR helper models,
not MIR, so it is outside Chronos.

### Catalog-Owned Stage Boundaries

The compiled stage catalog lives in `src/compiler/peephole/stage_catalog.rs`.
It mirrors the profile/timing boundaries already exposed by `PeepholeProfile`.

| Stage | Mode | Profile boundary | Current claim |
| --- | --- | --- | --- |
| `LinearScan` | always | `linear_scan_elapsed_ns` | line-local fact collection and direct rewrites |
| `PrimaryFlow` | always | `primary_flow_elapsed_ns` | early flow/full-range/index cleanup |
| `PrimaryInline` | always | `primary_inline_elapsed_ns` | first local scalar/index inline bundle |
| `PrimaryReuse` | always | `primary_reuse_elapsed_ns` | first exact expression/pure-call reuse bundle |
| `PrimaryLoopCleanup` | always | `primary_loop_cleanup_elapsed_ns` | loop normalization, exact cleanup, dead-temp cleanup |
| `SecondaryInline` | standard only | `secondary_inline_elapsed_ns` | second scalar/index inline bundle |
| `SecondaryExact` | standard only | `secondary_exact_elapsed_ns` | secondary exact emitted-IR cleanup |
| `SecondaryHelperCleanup` | standard only | `secondary_helper_cleanup_elapsed_ns` | wrapper/helper/metric/alias cleanup |
| `SecondaryRecordSroa` | standard only | `secondary_record_sroa_elapsed_ns` | static record scalarization with independent timing |
| `SecondaryFinalizeCleanup` | standard only | `secondary_finalize_cleanup_elapsed_ns` | empty-else/dead-temp final cleanup |
| `Finalize` | always | `finalize_elapsed_ns` | line-map composition and final text repair |

### Claim Boundary

The catalog is currently a compiled stage boundary. Primary flow, primary
inline, primary reuse, primary loop cleanup, secondary inline, secondary exact,
secondary helper cleanup, secondary record SROA, secondary finalize cleanup, and
finalization have stage-owned entrypoints. Primary loop cleanup and secondary
cleanup still keep some timed substep sequencing inside profile-preserving
helper functions.

### Remaining Gaps

| Gap | Next boundary |
| --- | --- |
| primary loop cleanup substeps are not individually catalog entries | split fast-dev and standard loop cleanup substeps into smaller named helpers only if profile review requires it |
| secondary cleanup substeps are not individually catalog entries | split secondary inline/exact/helper/finalize substeps into smaller named helpers only if profile review requires it |
| emitted-R peephole semantic proof is reduced | strengthen the current Lean/Coq line-stream theorem into per-stage small-step rewrite relations before claiming full semantic preservation |

The reduced emitted-R line semantics boundary is tracked in
[Peephole Line Semantics Boundary](#peephole-line-semantics).


<a id="peephole-line-semantics"></a>
## Peephole Line Semantics Boundary

This document fixes the reduced semantic model used for emitted-R peephole
claims. It is intentionally narrower than a full R interpreter.

### Reduced Object

The peephole layer operates on an ordered vector of emitted R source lines after
raw emitted-R rewrites and before final source-map remapping. The reduced model
treats a line stream as preserving meaning when a rewrite only changes one of
these local representations:

- redundant temporary assignment elimination where the removed name has no live
  later use
- exact expression reuse where the reused expression is pure under the recorded
  `pure_user_calls` set
- helper wrapper cleanup where the wrapper and unwrapped call have the same
  helper contract
- loop-counter normalization where the counted repeat-loop trip shape is
  unchanged
- static record scalarization where scalar fields are rematerialized at the same
  observable record boundary

### Explicit Non-Claims

This is not a proof of all production emitted R. It does not model arbitrary R
side effects, reflection, mutable global state, dynamic helper rebinding, or
source-map formatting. Unsafe R escape blocks are outside this rewrite model and
must remain protected by the unsafe-R rewrite skip gates.

### Mechanized Reduced Claim

The reduced line-stream preservation claim is mechanized in:

- `proof/lean/RRProofs/PeepholeLineSemantics.lean`
- `proof/coq/PeepholeLineSemantics.v`

These files intentionally prove the catalog-level observation boundary only.
They do not claim a full R interpreter or line-by-line production rewrite
mechanization.

### Connection To The Catalog

`src/compiler/peephole/stage_catalog.rs` provides the compiled stage boundary.
Each catalog stage is currently justified by this reduced line semantics plus
Rust regression tests. A future strengthening should turn each production stage
into a small-step relation over this line stream instead of claiming direct MIR
semantics.

## Build

Lean 4:

```bash
cd proof/lean
lake build
```

Coq:

```bash
cd proof/coq
coq_makefile -f _CoqProject -o Makefile.proof
make -f Makefile.proof
```

## Next Steps

The natural follow-up work is:

1. model a larger RR MIR fragment
2. formalize SSA/Phi semantics more directly
3. connect the formal LICM criterion to the Rust implementation shape
4. prove additional optimization passes sound over the same core semantics
