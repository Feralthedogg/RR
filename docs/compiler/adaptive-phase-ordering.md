# Adaptive Phase Ordering Design

This page started as the design for function-sensitive phase ordering in
Tachyon. It now documents the implemented scheduler surface and the remaining
design boundary.

Today the heavy optimizer in `src/mir/opt.rs` runs one hard-coded pass order
for every non-conservative function selected by the heavy tier. That keeps the
engine simple and deterministic, but it also means RR treats:

- loop-dense arithmetic kernels
- branch-heavy guard wrappers
- mixed scalar/control helper functions

as if they should all be prepared for optimization in the same way.

That is the wrong long-term shape for RR. The compiler already chooses which
functions deserve the heavy tier through `ProgramOptPlan`; it should also be
able to choose which safe pass schedule best fits each function.

## Goal

Add deterministic, low-overhead phase-ordering automation that:

- keeps RR semantics and output determinism unchanged
- changes heavy-tier pass order per function, not globally
- uses cheap MIR features rather than expensive pass-search
- improves pattern exposure for compute-heavy functions
- avoids wasting structural-pass budget on control-flow-heavy functions

## Current Status

The current tree now implements:

- explicit heavy-tier schedule execution
- function feature extraction after Tier A
- `Balanced` / `ComputeHeavy` / `ControlFlowHeavy` classification
- `ComputeHeavy` and `ControlFlowHeavy` schedule activation in `auto` mode
- control-heavy structural gating and single-fallback behavior
- aggregated phase-ordering stats and trace output

Default behavior remains conservative:

- `RR_PHASE_ORDERING` still overrides the scheduler explicitly
- `-O1` now defaults to `Balanced`
- `-O2` now defaults to `Auto`
- `-O0` still leaves phase ordering effectively off

## Non-Goals

This design does not try to add:

- exhaustive pass-order search
- profile-trained or randomized scheduling
- dynamic de-SSA/inlining movement across major phase boundaries
- pass legality changes
- a compile-time autotuner in the normal build path

The right V1 for RR is template selection, not generic pass scheduling.

## Current Optimizer Shape

The important current structure in `src/mir/opt.rs` is:

1. Tier A, always:
   - `SimplifyCFG`
   - light `SCCP`
   - `Intrinsics`
   - `TypeSpecialize`
   - `TCO`
   - `LoopOpt`
   - `DCE`
   - one bounded `BCE` sweep after convergence
2. Tier B, selective heavy:
   - `TypeSpecialize`
   - `poly`
   - `v_opt`
   - `TypeSpecialize` again
   - `TCO`
   - structural cleanup (`SimplifyCFG` + `DCE`)
   - `SimplifyCFG`
   - `SCCP`
   - `Intrinsics`
   - `GVN`
   - `simplify`
   - `DCE`
   - `LoopOpt`
   - `LICM`
   - `FreshAlloc`
   - `BCE`
3. Tier C, full-program inline:
   - bounded inlining rounds
   - per-function cleanup
4. Final barriers:
   - fresh-alias cleanup
   - de-SSA
   - copy cleanup
   - final `SimplifyCFG` + `DCE`

Tier A already gives RR a useful normalization barrier. The new phase-ordering
logic should sit after Tier A, not before it.

## Why Fixed Heavy-Tier Order Is Leaving Wins on the Table

Two examples motivate the change:

### Compute-Heavy Functions

Loop-heavy kernels often need loop canonicalization and value simplification
before structural passes do their best work. In the current heavy tier,
`poly` and `v_opt` run before the main `SCCP`/`GVN`/`LICM` cleanup wave.

That can leave potential vector or scheduling opportunities hidden behind:

- dead guard paths
- non-canonical induction updates
- redundant arithmetic
- load/index noise that a dataflow pass would have collapsed

### Control-Flow-Heavy Functions

Guard wrappers and branch-heavy helpers often benefit most from:

- CFG pruning
- constant folding
- dead-path elimination
- TCO

Running expensive structural loop passes too early on these functions burns
budget without exposing much new optimization surface.

## Design Summary

The recommended architecture is:

1. Keep major tier boundaries fixed.
2. After Tier A, extract cheap per-function MIR features.
3. Classify each heavy-tier candidate as:
   - `Balanced`
   - `ComputeHeavy`
   - `ControlFlowHeavy`
4. Map the classification to a deterministic schedule template.
5. Execute that template through a small phase-order interpreter inside the
   heavy-tier loop.
6. If the chosen template makes no useful progress, fall back to the baseline
   `Balanced` template.

This keeps RR explicit and debuggable. The compiler is still running a known
set of passes in a known order; it is only selecting between a few curated
orders.

## Fixed Barriers and Anchored Passes

Not every pass should move.

The following constraints should stay fixed in V1:

| Boundary / pass | Keep fixed because |
| --- | --- |
| `canonicalize_floor_index_params()` | must run before the heavy iteration starts |
| Tier A before classification | Tier A normalizes MIR and makes feature extraction more stable |
| `poly` before `v_opt` | RR currently couples the structural pipeline in that direction and already tracks legacy poly fallback in vector stats |
| `TypeSpecialize` immediately before structural rewrites | structural matchers benefit from sharpened type/state facts |
| `TypeSpecialize` once after structural rewrites | structural changes can create new value/state facts |
| structural cleanup (`SimplifyCFG` + `DCE`) after any structural cluster | removes dead residue and keeps verifier churn bounded |
| inline tier after per-function heavy tier | inlining still rewrites the whole call graph |
| de-SSA at the end | codegen must still see phi-free MIR |

This means V1 is adaptive inside the heavy tier, not across the whole compiler.

## Function Feature Extraction

The feature extractor should use only cheap, deterministic MIR queries that RR
already has or can compute in one pass.

Recommended input struct:

```rust
pub(super) struct FunctionPhaseFeatures {
    pub ir_size: usize,
    pub block_count: usize,
    pub loop_count: usize,
    pub canonical_loop_count: usize,
    pub branch_terms: usize,
    pub phi_count: usize,
    pub arithmetic_values: usize,
    pub intrinsic_values: usize,
    pub call_values: usize,
    pub side_effecting_calls: usize,
    pub index_values: usize,
    pub store_instrs: usize,
}
```

Recommended feature sources:

- `loop_count`, `canonical_loop_count`
  - use `loop_analysis::LoopAnalyzer`
  - count all loops and separately count loops with recoverable IV/bound shape
- `branch_terms`
  - count `Terminator::If`
- `phi_count`
  - count `ValueKind::Phi`
- `arithmetic_values`
  - count `ValueKind::Binary` and `ValueKind::Unary`
- `intrinsic_values`
  - count `ValueKind::Intrinsic`
- `call_values`
  - count `ValueKind::Call`
- `side_effecting_calls`
  - count calls that are not statically pure according to existing effect
    helpers
- `index_values`
  - count `Index1D/2D/3D`
- `store_instrs`
  - count `StoreIndex1D/2D/3D`

The extractor should run after Tier A has been restored into `all_fns`, because
Tier A already simplifies obvious dead paths and canonicalizes some loop/index
shapes.

## Classification

V1 should keep the classifier simple and explicit.

Recommended profiles:

```rust
pub(super) enum PhaseProfileKind {
    Balanced,
    ComputeHeavy,
    ControlFlowHeavy,
}
```

Recommended scoring model:

- `compute_score`
  - weighted by `canonical_loop_count`, `loop_count`, arithmetic density,
    intrinsic density, and index/store activity
- `control_score`
  - weighted by `branch_terms`, `phi_count`, and side-effecting call density

Suggested decision rule:

- `ComputeHeavy`
  - at least one loop
  - `compute_score >= control_score + threshold`
  - side-effecting call density stays below a small cap
- `ControlFlowHeavy`
  - `control_score >= compute_score + threshold`
  - or branch density crosses a hard threshold
- `Balanced`
  - everything else

Weights should live in named constants, not magic literals inside the
classifier. RR should also expose a trace mode so those weights can be tuned
with actual benchmark data.

## Schedule Templates

V1 only needs three templates.

### 1. Balanced

This template must be behavior-compatible with the current heavy-tier order.
It is the baseline schedule and the fallback path.

| Order | Pass / cluster |
| --- | --- |
| 1 | `TypeSpecialize` |
| 2 | `poly` |
| 3 | `v_opt` |
| 4 | `TypeSpecialize` (post-structural) |
| 5 | `TCO` |
| 6 | structural cleanup: `SimplifyCFG`, `DCE` |
| 7 | `SimplifyCFG` |
| 8 | `SCCP` |
| 9 | `Intrinsics` |
| 10 | `GVN` |
| 11 | `simplify` |
| 12 | `DCE` |
| 13 | `LoopOpt` |
| 14 | `LICM` |
| 15 | `FreshAlloc` |
| 16 | `BCE` |

### 2. ComputeHeavy

This template should front-load the passes that expose cleaner loop structure
before RR pays for pattern-sensitive structural rewrites.

| Order | Pass / cluster |
| --- | --- |
| 1 | `SimplifyCFG` |
| 2 | `SCCP` |
| 3 | `Intrinsics` |
| 4 | `GVN` |
| 5 | `simplify` |
| 6 | `DCE` |
| 7 | `LoopOpt` |
| 8 | `LICM` |
| 9 | `TypeSpecialize` |
| 10 | `poly` |
| 11 | `v_opt` |
| 12 | `TypeSpecialize` (post-structural) |
| 13 | `TCO` |
| 14 | structural cleanup: `SimplifyCFG`, `DCE` |
| 15 | `FreshAlloc` |
| 16 | `BCE` |

Rationale:

- dataflow and loop cleanup run early enough to expose canonical loop shape
- `poly` and `v_opt` still stay in their existing relative order
- `FreshAlloc` and `BCE` remain late, after the main loop shape has settled

### 3. ControlFlowHeavy

This template should prioritize branch pruning and cheap cleanup. Structural
loop passes should only run after the CFG has been simplified enough to make
them worth paying for.

| Order | Pass / cluster |
| --- | --- |
| 1 | `SimplifyCFG` |
| 2 | `SCCP` |
| 3 | `Intrinsics` |
| 4 | `TypeSpecialize` |
| 5 | `simplify` |
| 6 | `DCE` |
| 7 | `TCO` |
| 8 | `GVN` |
| 9 | `LoopOpt` if loops remain |
| 10 | `LICM` if loop canonicality passes a small threshold |
| 11 | `poly` if canonical loops remain and branch pressure is no longer high |
| 12 | `v_opt` if the same structural gate passes |
| 13 | `TypeSpecialize` (post-structural if structural work ran) |
| 14 | structural cleanup: `SimplifyCFG`, `DCE` |
| 15 | `FreshAlloc` |
| 16 | `BCE` |

Rationale:

- branch-heavy functions should first get smaller and simpler
- expensive structural work should be gated, not unconditional
- control cleanup still has a path to structural optimization when it reveals a
  cleaner post-cleanup loop

## Structural Gates

The control-heavy template should not blindly run `poly` and `v_opt`.

Recommended structural gate inputs:

- loop count after the early CFG cleanup
- canonical-loop count after `LoopOpt`
- branch density after `SimplifyCFG` and `DCE`
- side-effecting call density

Recommended V1 rule:

- run structural passes only if:
  - at least one canonical loop remains
  - branch density is below a threshold
  - side-effecting call density is not dominant

This keeps RR from paying structural-pass cost on clearly branch-dominated
functions while still allowing promotion when cleanup exposes a valid loop
kernel.

## Fallback Rule

Adaptive scheduling must have a safe escape hatch.

Recommended V1 fallback:

- the selected template runs for iteration 1
- if iteration 1 records no structural progress and very little non-structural
  progress, switch the function to `Balanced`
- once the fallback happens, do not switch again

This rule avoids oscillation and keeps failure behavior easy to reason about.

Useful progress signals:

- `poly_schedule_applied`
- `vector_applied_total`
- `simplified_loops`
- `sccp_hits`
- `gvn_hits`
- `simplify_hits`
- `dce_hits`

## Execution Refactor

The current hard-coded heavy-tier loop in
`run_function_with_proven_index_slots()` should become a schedule interpreter.

Recommended implementation split:

- `src/mir/opt/phase_order.rs`
  - feature extraction
  - profile classification
  - schedule template definitions
  - structural gates
- `src/mir/opt/types.rs`
  - `FunctionPhaseFeatures`
  - `PhaseProfileKind`
  - `PhaseScheduleId`
  - small per-function telemetry structs if needed
- `src/mir/opt/config.rs`
  - env/config toggles for rollout and tracing
- `src/mir/opt.rs`
  - heavy-tier loop rewritten to execute a selected template

Recommended executor shape:

```rust
fn run_function_with_schedule(
    &self,
    fn_ir: &mut FnIR,
    schedule: PhaseScheduleId,
    ctx: &mut PhaseExecutionContext,
) -> TachyonPulseStats
```

The executor should stay enum-based and explicit. Do not introduce a generic
boxed pass interface; RR needs the current direct control over verifier labels,
stats accounting, and pass-specific gates.

## Program-Level Integration

The phase-ordering plan should be computed after Tier A, not inside the current
`build_opt_plan()` budget planner.

Recommended flow inside `run_program_with_stats_inner()`:

1. build `ProgramOptPlan`
2. run Tier A for all functions
3. build `FunctionPhasePlan` for heavy-tier candidates using post-Tier-A MIR
4. run Tier B using the selected schedule for each function
5. keep Tier C and de-SSA unchanged

This separation matters:

- `ProgramOptPlan` answers "which functions get heavy optimization?"
- `FunctionPhasePlan` answers "which heavy schedule should each selected
  function use?"

Those are related, but they are not the same decision.

## Telemetry and Debuggability

Adaptive scheduling is only maintainable if RR can explain what it chose.

Recommended additions:

- `RR_PHASE_ORDERING=off|balanced|auto`
  - `off`: keep legacy static order
  - `balanced`: force the baseline template
  - `auto`: enable classification and template selection
- `RR_PHASE_ORDERING_TRACE=1`
  - print one line per heavy-tier function with:
    - profile kind
    - key feature counts
    - chosen schedule
    - whether fallback happened

Recommended `TachyonPulseStats` counters:

- number of `ComputeHeavy` functions
- number of `ControlFlowHeavy` functions
- number of `Balanced` functions
- number of schedule fallbacks
- number of control-heavy functions that skipped structural passes

These should stay aggregated and deterministic like the current stats.

## Rollout Plan

The safest rollout is incremental.

### Phase 0: Refactor Only

- extract the current heavy-tier order into the `Balanced` template
- keep behavior identical
- land no functional change

### Phase 1: Add Features and Trace

- add feature extraction and profile classification
- keep execution pinned to `Balanced`
- validate that the classifier is stable and cheap

### Phase 2: Opt-In Auto Mode

- enable `ComputeHeavy` and `ControlFlowHeavy` behind
  `RR_PHASE_ORDERING=auto`
- use trace output and benchmark slices to tune thresholds

### Phase 3: Make `-O2` the Adaptive Tier

Once stable:

- keep `-O1` on `Balanced`
- make `-O2` use adaptive phase ordering by default

Current status:

- completed in the current tree

This is the cleanest way to finally give `-O2` a real optimizer identity while
keeping `-O1` conservative.

## Testing Plan

### Unit Tests

Add classifier tests for:

- loop-dense arithmetic kernels -> `ComputeHeavy`
- branch-heavy guard wrappers -> `ControlFlowHeavy`
- small mixed helpers -> `Balanced`

### Behavior Regression Tests

Keep the existing optimizer correctness matrix, including:

- `cargo test`
- `bash scripts/optimizer_suite.sh legality`
- `bash scripts/optimizer_suite.sh heavy`

### Differential and Fuzz Validation

Because this changes pass order rather than pass legality, RR should also keep:

- optimizer differential tests
- pipeline fuzz smoke
- generated-pipeline fuzz smoke

### Performance Validation

Use existing benchmark slices as decision points:

- `signal_pipeline`
- diffusion benchmarks
- representative `poly_*` tests
- branch/guard-heavy examples where compile time previously paid for little
  structural gain

The feature is only worth enabling by default if:

- compute-heavy kernels improve or stay flat
- control-heavy functions get cheaper to optimize
- correctness coverage stays clean

## Risks

| Risk | Mitigation |
| --- | --- |
| misclassification hides a vectorization opportunity | fallback to `Balanced`; keep `poly` -> `v_opt` relative order |
| compile-time overhead from feature extraction | use one-pass counters plus existing `LoopAnalyzer`; run after Tier A only for heavy-tier candidates |
| schedule oscillation | allow at most one fallback and keep existing fingerprint/time guards |
| debugging becomes opaque | add explicit trace mode and aggregated stats |
| `-O1/-O2` behavior split becomes too abrupt | keep adaptive mode opt-in first, then move only `-O2` once validated |

## Recommendation

RR should implement adaptive phase ordering as a deterministic template
selection layer on top of the existing heavy tier.

The most important design choices are:

- classify after Tier A, not before
- keep major phase barriers fixed
- preserve `poly` -> `v_opt` ordering
- use explicit templates instead of generic pass-search
- roll the feature out behind tracing and an opt-in mode before making it the
  default for `-O2`

That gives RR the benefit of function-sensitive ordering without turning the
optimizer into a search problem.
