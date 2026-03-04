# Tachyon Engine

`TachyonEngine` is the MIR optimizer (`src/mir/opt.rs`).

## Optimization Levels

- `-O0`: no aggressive optimization, but still runs mandatory codegen stabilization (including De-SSA)
- `-O1`: optimized pipeline
- `-O2`: same pipeline with stronger opportunities from analysis/rewrites

## Program-Level Strategy

1. Optimize each function independently.
2. Run inter-procedural inlining rounds (bounded).
3. Run De-SSA globally before emission.
4. Cleanup after De-SSA.

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

## Inlining Controls

Inlining is cost-model driven and constrained by environment policy.

Defaults from `src/mir/opt/inline.rs`:

- `RR_INLINE_MAX_BLOCKS=24`
- `RR_INLINE_MAX_INSTRS=160`
- `RR_INLINE_MAX_COST=220`
- `RR_INLINE_MAX_CALLER_INSTRS=480`
- `RR_INLINE_MAX_TOTAL_INSTRS=900`
- `RR_INLINE_ALLOW_LOOPS=false`
- `RR_DISABLE_INLINE=false` (set true-like value to disable)
- `RR_INLINE_MAX_ROUNDS=3` (from `src/mir/opt.rs`)

## Vectorization Coverage (current implementation)

Implemented pattern families include:

- elementwise map
- conditional map
- shifted map
- recurrence add-constant
- reduction (sum/prod/min/max)
- call-map with builtin/user whitelist
- selected 2D row/column map and reduction patterns

Vectorization remains pattern-based, not arbitrary polyhedral scheduling.

## De-SSA and Parallel Copy

De-SSA is mandatory before codegen:

- phi elimination via parallel copy
- critical-edge handling
- sequentialization with temporaries for cycles

Codegen assumes phi-free MIR and will error on remaining phi nodes.
