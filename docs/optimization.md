# Tachyon Engine

`TachyonEngine` is the MIR optimizer (`src/mir/opt.rs`).

## Optimization Levels

- `-O0`: no aggressive optimization, but still runs mandatory codegen stabilization (including De-SSA)
- `-O1`: optimized pipeline
- `-O2`: same pipeline with stronger opportunities from analysis/rewrites

## Program-Level Strategy

1. Tier A (always): run low-cost canonical passes on every safe function.
2. Tier B (selective-heavy): run full per-function pipeline on budget-selected targets.
3. Tier C (full-program): run bounded inter-procedural inlining only when heavy tier is enabled.
4. Run De-SSA globally before emission.
5. Cleanup after De-SSA.

Budget policy:

- Adaptive IR budgeting is enabled by default.
- RR first estimates a program-level and function-level budget from total IR, max function size, and operation density.
- If the workload still exceeds the adaptive cap, Tier B falls back to deterministic selective mode.
- `RR_HEAVY_PASS_FN_IR` is a soft threshold for compact heavy-tier candidates, not the global hard ceiling for the compilation unit.

## Function-Level Iterative Passes

Core loop (bounded by `RR_OPT_MAX_ITERS`):

1. Structural transforms
- type-based specialization (`type_specialize`)
- vectorization (`v_opt`)
- tail-call optimization (`tco`)
- immediate cleanup (`simplify_cfg` + `dce`) if changed

2. Canonical optimization passes
- `simplify_cfg`
- SCCP (`sccp`)
- intrinsics rewrite (`intrinsics`)
- GVN/CSE (`gvn`)
- simplify (`simplify`)
- DCE (`dce`)
- loop optimizer (`loop_opt`)
- LICM (`licm`)
- fresh allocation tuning (`fresh_alloc`)
- bounds-check elimination (`bce`)

GVN policy:

- GVN is enabled by default.
- It currently runs only on loop-free, store-free functions and skips known unsafe runtime helpers.
- `RR_ENABLE_GVN=0` disables GVN globally.

LICM policy:

- LICM is enabled by default.
- It only runs on compact loop-bearing functions.
- Current guardrails skip very large functions/CFGs so compile time stays bounded on workloads such as `tesseract`.
- `RR_ENABLE_LICM=0` disables LICM globally.

3. Always verify MIR invariants at key boundaries.

Type-specialization policy:

- Guard/intrinsic rewrites are proof-based only.
- If static proof is missing, optimizer keeps the original safe runtime path.

## SCCP Safety Contract

SCCP constant folding is intentionally fail-safe:

- integer folds (`+`, `-`, `*`, `/`, `%`) use checked arithmetic
- overflow or invalid arithmetic (`div/mod by zero`, `i64::MIN / -1`) is treated as "not foldable"
- range-length/index folds use checked length/index math and checked integer casts
- float-to-int fold is accepted only when finite, integral, and within `i64` range

If any proof step fails, SCCP preserves runtime evaluation instead of panicking or emitting an invalid constant.

## Inlining Controls and Growth Safety

Inlining is cost-model driven and constrained by environment policy.

Defaults from `src/mir/opt/inline.rs`:

- `RR_INLINE_MAX_BLOCKS=24`
- `RR_INLINE_MAX_INSTRS=160`
- `RR_INLINE_MAX_COST=220`
- `RR_INLINE_MAX_CALLSITE_COST=240`
- `RR_INLINE_MAX_CALLER_INSTRS=480`
- `RR_INLINE_MAX_TOTAL_INSTRS=900`
- `RR_INLINE_MAX_UNIT_GROWTH_PCT=25`
- `RR_INLINE_MAX_FN_GROWTH_PCT=35`
- `RR_INLINE_ALLOW_LOOPS=false`
- `RR_DISABLE_INLINE=false` (set true-like value to disable)
- `RR_INLINE_MAX_ROUNDS=3` (from `src/mir/opt.rs`)

Growth limits are enforced both per function and per compilation unit; predicted overshoot skips the inline site.

## Vectorization Coverage (current implementation)

Implemented pattern families include:

- elementwise map
- conditional map
- expression map with staged temporaries
- multi-destination slice map when stores are independent
- shifted map
- recurrence add-constant
- reduction (sum/prod/min/max)
- call-map with builtin/user whitelist
- gather-style indirect index map
- indirect scatter map via runtime helper lowering
- cube-index helper rewrite/lowering (`rr_idx_cube_vec_i`)
- selected 2D row/column map and reduction patterns

Vectorization remains pattern-based, not arbitrary polyhedral scheduling.

Current lowering helpers commonly emitted by `v_opt` include:

- `rr_assign_slice(...)`
- `rr_assign_index_vec(...)`
- `rr_index1_read_vec(...)`
- `rr_wrap_index_vec_i(...)`
- `rr_idx_cube_vec_i(...)`
- `rr_ifelse_strict(...)`

## Vectorization Diagnostics

O1/O2 CLI output reports vectorization summary counters:

- `Vectorized`
- `Reduced`
- `Simplified`
- `VecSkip`

`VecSkip` is broken down by dominant reject reason:

- `no-iv`
- `bound`
- `cfg`
- `indirect`
- `store`
- `no-pattern`

This is intended to guide pass work. For example:

- `no-iv`: induction variable recognition / floor-alias normalization is missing
- `indirect`: gather/scatter pattern recognized structurally but not yet supported safely
- `store`: loop has conflicting or non-canonical writes

For per-loop tracing, enable `RR_VECTORIZE_TRACE=1`.

## Current Limits

Known hard cases are still conservative:

- nested loops with branch-merged indirect scatter
- outer-loop state carried through multiple origin-phi chains
- loops whose safety proof depends on non-canonical bound/index reconstruction

In those cases RR prefers a clean skip over speculative lowering.

## De-SSA and Parallel Copy

De-SSA is mandatory before codegen:

- phi elimination via parallel copy
- critical-edge handling
- sequentialization with temporaries for cycles

Codegen assumes phi-free MIR and will error on remaining phi nodes.
