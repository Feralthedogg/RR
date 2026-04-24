# Optimizer Proof Gap Audit

This note is the explicit trust-boundary document for the reduced optimizer
proof spine.

It answers one question only:

- what is actually proved
- what is only approximated by a reduced model
- what is not modeled yet

It should be read alongside:

- [optimizer_correspondence.md](/Users/feral/Desktop/Programming/RR/proof/optimizer_correspondence.md:1)

## Reading Rule

`proved` means:
- a theorem exists in Lean/Coq for the reduced object at that boundary
- the theorem is wired into the current proof spine

`approximated` means:
- theorem names and stage boundaries match Rust structure
- but the reduced transform is weaker, simpler, or more stylized than the real Rust pass

`not modeled` means:
- no corresponding reduced theorem currently carries that behavior

## Summary

The current workspace proves a **reduced end-to-end optimizer spine**, not a
line-by-line mechanization of `src/mir/opt.rs`.

The strongest honest claim is:

- the current optimizer pipeline structure, phase-ordering structure, and
  program-level orchestration now have a continuous reduced theorem family
- many real Rust stage boundaries are named directly
- several stage implementations are still represented by simplified or partial
  reduced transforms

## Short Answer

If someone asks “is the optimizer proven?”, the precise answer is:

- **yes, as a reduced continuous proof spine**
- **no, not as a production 1:1 mechanization**

If someone asks “what may I safely claim?”, the safe wording is:

- the repository contains a reduced formal optimizer correctness argument with
  explicit stage, phase-order, program-level, and top-level wrapper theorems

Unsafe wording would be:

- the production Rust optimizer is fully formally verified

## Core Spine

| Rust slice | Proof file | Proved | Approximated | Not modeled |
| --- | --- | --- | --- | --- |
| `src/mir/opt.rs` optimizer-wide stage composition | [OptimizerPipelineSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/OptimizerPipelineSoundness.lean:1) | stage composition, verify-ir preservation, semantic preservation for reduced stage functions | `alwaysTierDataflowStage`, `alwaysTierLoopStage`, `postDeSsaBoundaryStage`, parts of `postDeSsaCleanupStage` remain simplified wrappers | full production `always tier` pass-by-pass behavior |
| `src/mir/opt.rs` public optimizer shell | [ProgramApiWrapperSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/ProgramApiWrapperSoundness.lean:1) | wrapper theorem names for `run_program*` | wrapper semantics are pure shell composition over inner theorem | actual stats/progress side effects |
| reduced compiler observable theorem | [CompilerEndToEndSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/CompilerEndToEndSoundness.lean:1) / [CompilerEndToEndSoundness.v](/Users/feral/Desktop/Programming/RR/proof/coq/CompilerEndToEndSoundness.v:1) | reduced frontend observable theorem + reduced optimizer theorem in one top-level statement | frontend/backend side is still toy/reduced; Lean reuses `PipelineStmtSubset`, while Coq uses a tiny self-contained expression model | full production RR frontend and R runtime, plus a synchronized Lean/Coq frontend artifact |

## Function-Local Pass Layers

| Rust slice | Proof file | Proved | Approximated | Not modeled |
| --- | --- | --- | --- | --- |
| `simplify_cfg` / entry retarget / dead block cleanup | [CfgOptSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/CfgOptSoundness.lean:1) | reduced runner, dead-block append invariant theorem, empty-entry-goto retarget theorem, canonical dead-block append theorem | real CFG normalization set is much broader | full jump-threading / unreachable elimination catalog |
| `sccp` / `gvn` / `dce` reduced layer | [DataflowOptSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/DataflowOptSoundness.lean:1) | expression canonicalization, const-prop under env agreement, dead last-assign elimination | does not model dominance, availability, alias barriers, whole-block sparse propagation | full SCCP lattice / global fixed-point |
| `licm` / `bce` / `tco` | [LoopOptSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/LoopOptSoundness.lean:1) | reduced LICM zero/one-trip, reduced BCE, reduced TCO | actual loop optimizer side conditions are richer | full production loop optimizer state space |
| `de_ssa` and post-cleanup boundary | [DeSsaBoundarySoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/DeSsaBoundarySoundness.lean:1) | reduced de-ssa boundary theorem | reduced copy-boundary matcher only | full parallel-copy scheduling |

## Phase Ordering and Plan Flow

| Rust slice | Proof file | Proved | Approximated | Not modeled |
| --- | --- | --- | --- | --- |
| phase profiles / schedule selection | [PhasePlanSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhasePlanSoundness.lean:1) | reduced `classify -> choose -> build plan` theorem family | score model is reduced and sample-based | exact production feature extraction impact on every threshold |
| selected plan collection | [PhasePlanCollectionSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhasePlanCollectionSoundness.lean:1) | skip/filter theorems for missing/conservative/self-recursive/unselected | uses reduced list collection instead of actual map | exact `FxHashMap` behavior not modeled |
| collected plan lookup | [PhasePlanLookupSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhasePlanLookupSoundness.lean:1) | lookup hit/miss and preservation after lookup | list-based lookup rather than hash-map lookup | map collision/overwrite behavior |
| ordered summary emission | [PhasePlanSummarySoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhasePlanSummarySoundness.lean:1) | ordered summary hit/miss and payload exposure | summary entries only, not full strings | actual formatted summary text |
| phase-order schedule family | [PhaseOrderOptimizerSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhaseOrderOptimizerSoundness.lean:1) | schedule theorem family for balanced / compute-heavy / control-flow-heavy | schedule bodies still reduced relative to Rust | every per-pass delta between schedules |
| cluster boundaries | [PhaseOrderClusterSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhaseOrderClusterSoundness.lean:1) | structural / standard / cleanup theorem family | clusters are compressed abstractions | exact production cluster internals |
| guards / feature gates / fallback / iteration | [PhaseOrderGuardSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhaseOrderGuardSoundness.lean:1), [PhaseOrderFeatureGateSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhaseOrderFeatureGateSoundness.lean:1), [PhaseOrderFallbackSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhaseOrderFallbackSoundness.lean:1), [PhaseOrderIterationSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PhaseOrderIterationSoundness.lean:1) | theorem families for guards, gates, fallback predicate, and heavy-iteration entrypoints | reduced booleans and reduced heavy-iteration state | exact per-pass statistics/progress impact |

## Program-Level Orchestration

| Rust slice | Proof file | Proved | Approximated | Not modeled |
| --- | --- | --- | --- | --- |
| adaptive budget plan | [ProgramOptPlanSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/ProgramOptPlanSoundness.lean:1) | under-budget, selective, fallback-to-smallest cases | selection order is reduced/sample-based | exact profile weighting and full sort tie-break semantics |
| program heavy-tier plan flow | [ProgramPhasePipelineSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/ProgramPhasePipelineSoundness.lean:1) | `ProgramOptPlan -> selected_functions -> collect -> summary` theorem family | reduced collection/list model | actual scheduler/external helper rewrites |
| per-function heavy-tier execution | [ProgramTierExecutionSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/ProgramTierExecutionSoundness.lean:1) | conservative/self-recursive/heavy-disabled/budget/collected-plan/legacy-plan split | branch actions are reduced to reduced stage calls | real local stats accumulation / verification side effects |
| post-heavy tail stages | [ProgramPostTierStagesSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/ProgramPostTierStagesSoundness.lean:1) | wrapper theorem family for inline cleanup, fresh-alias, de-ssa tail | `freshAliasStage` still simplified; de-ssa tail bundled | exact alias analysis and copy-cleanup internals |
| program wrapper | [ProgramRunProfileInnerSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/ProgramRunProfileInnerSoundness.lean:1) | one reduced wrapper theorem for `run_program_with_profile_inner` | scheduler/progress/stats are abstracted out | full side-effectful orchestration |

## Actual Reduced Rewrite Companions

These were added specifically to reduce the number of pure identity-style
placeholders.

| Rust flavor | Proof file | Proved | Remaining gap |
| --- | --- | --- | --- |
| inline-cleanup shape | [InlineCleanupRefinementSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/InlineCleanupRefinementSoundness.lean:1) | reduced entry-retarget cleanup witness with verify-ir + eval preservation | does not model the whole production inline cleanup loop |
| fresh-alias shape | [FreshAliasRewriteSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/FreshAliasRewriteSoundness.lean:1) | reduced alias-rename theorem under explicit alias agreement | not yet wired as the main stage transform in `ProgramPostTierStages` |

## Highest-Value Remaining Gaps

These are the next places where the reduced spine is still materially weaker
than the production compiler.

1. `alwaysTierDataflowStage`
- still reduced more as a wrapper than as a whole-function SCCP/GVN/DCE stage

2. `freshAliasStage`
- companion actual theorem exists, but the main stage in
  [ProgramPostTierStages.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/ProgramPostTierStages.lean:1)
  is still simplified

3. top-level compiler theorem
- [CompilerEndToEndSoundness.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/CompilerEndToEndSoundness.lean:1)
  is now a continuous reduced theorem
- but it is still not a 1:1 production theorem for the actual RR frontend,
  full MIR, and full emitted R/runtime semantics

## Bottom Line

The honest status is:

- **optimizer reduced proof spine: complete enough to claim a continuous reduced proof**
- **production optimizer 1:1 mechanization: not complete**
- **public API reduced shell: covered**
- **reduced compiler-level theorem: covered**

So the remaining work is no longer “build a proof spine at all”.
It is “tighten the reduced-to-production gap”.
