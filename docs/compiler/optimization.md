# Tachyon Engine

This page is the optimizer manual for RR.

Primary implementation entrypoint:

- `src/mir/opt.rs`

## Audience

Read this page when you need to know:

- why a loop vectorized or skipped
- which pass family owns a rewrite
- what Tachyon considers safe enough to do

## Design Goals

Tachyon is not a speculative “make R fast somehow” pass stack.

Its goals are:

- preserve RR program meaning
- exploit proofs already available in MIR
- emit simpler and more idiomatic R when safe
- keep compile time bounded on large workloads

## Mental Model

Tachyon is not "just pattern matching".

The optimizer has two broad layers:

1. general MIR analysis/simplification passes
   - SCCP
   - GVN/CSE
   - simplify
   - DCE
   - BCE
   - LICM
   - inlining
   - de-SSA
2. proof-driven structural rewrites that are much more pattern-sensitive
   - vectorization
   - reduction rewrites
   - selected scatter/gather/call-map forms

So when a user says "my loop did not optimize", the answer is not always
"the pattern failed". Sometimes the general passes still ran and improved the
program, but the pattern-sensitive layer correctly skipped because the final MIR
shape was not provable enough.

## Safety Rules

Tachyon prefers a clean skip to a risky rewrite.

Important rules:

- no guard elimination without proof
- no phi-sensitive codegen paths after de-SSA
- no vectorization when loop-carried state is ambiguous
- no reduction when the loop carries extra non-accumulator state
- no helper rewrite that changes scalar/vector semantics

## Optimization Levels

- `-O0`
  - stabilization only
  - still performs mandatory helper canonicalization and de-SSA
- `-O1`
  - optimizing pipeline
- `-O2`
  - currently runs the same optimizing pipeline as `-O1`
  - RR currently distinguishes `-O0` from optimized mode, not `-O1` from `-O2`

## Program-Level Strategy

Tachyon uses a tiered budget model.

### Tier A: Always

Run low-cost, safe canonical passes on every eligible function.

### Tier B: Selective Heavy

Run heavier per-function optimization only on budget-selected targets.

### Tier C: Full-Program Inline

Run bounded interprocedural inlining only when the heavy tier is active.

## Core Pass Families

### Canonicalization

- helper call rewrites
- index-floor canonicalization
- wrap/cube helper normalization
- simplification after structural rewrites

### Scalar Analysis and Simplification

- SCCP
- GVN/CSE
- simplify
- DCE
- BCE
- LICM

These are not merely pre-processing helpers. They are substantive optimization
passes in their own right, and they often produce the facts or MIR cleanup that
later pattern-sensitive passes rely on.

### Structural Transformations

- inlining
- TCO
- de-SSA

### Vectorization and Reduction

Implemented pattern families include:

- map
- conditional map
- expr-map
- multi-output expr-map
- call-map
- scatter-map
- shifted map
- recurrence add-constant
- reduction (`sum/prod/min/max`)
- selected 2D row/column map and reduction
- selected 3D map/expr-map/call-map/scatter-map/reduction/shift forms

## What Tachyon Will Not Do

Tachyon remains conservative on:

- arbitrary nested-loop scheduling
- branch-merged indirect scatter with weak proof
- non-canonical bound reconstruction
- loop-carried state that cannot be reconstructed safely

When in doubt, the pass should skip.

## Reading Rule

This page describes optimizer policy, not a promise that every syntactically
similar program will optimize the same way. Proof availability still decides.

## Vectorization Diagnostics

CLI summary reports:

- `Vectorized`
- `Reduced`
- `Simplified`
- `VecSkip`

`VecSkip` is grouped by dominant reject reason:

| Reason | Meaning |
| --- | --- |
| `no-iv` | the loop did not expose one recoverable induction variable |
| `bound` | the trip count or loop bound was not canonical enough to prove safely |
| `cfg` | the loop CFG shape exceeded the currently supported matcher forms |
| `indirect` | the loop used indirect index access that RR could not prove safe to rewrite |
| `store` | the loop had store side effects that block the current vector rewrite families |
| `no-pattern` | the loop was analyzable, but it did not match any supported vector plan |

## Related Manuals

- [Writing RR for Performance and Safety](../writing-rr.md)
- [Compiler Pipeline](pipeline.md)
- [Compatibility and Limits](../compatibility.md)

Use `RR_VECTORIZE_TRACE=1` to see per-loop matcher decisions.

## Cost Model

Tachyon uses a cost model rather than blindly preferring helper-heavy lowering.

Current inputs include:

- loop trip count hints
- helper count and helper family cost
- whole-destination vs partial-range writes
- shadow-state penalties
- direct builtin vector-call opportunities

The main idea is simple:

- prefer direct whole-vector R when it is provably available
- prefer helper-based vector lowering when it reduces loop work and keeps meaning
- prefer scalar fallback when helper overhead dominates

## Runtime-Aware Lowering

Selected vector call paths lower through runtime helpers such as:

- `rr_call_map_whole_auto(...)`
- `rr_call_map_slice_auto(...)`

These helpers may choose scalar fallback or vector evaluation at runtime based
on trip count and helper cost.

Related knobs:

- `RR_VECTOR_FALLBACK_BASE_TRIP`
- `RR_VECTOR_FALLBACK_HELPER_SCALE`

## Backend-Aware Fusion

Backend-aware lowering only pays off when the hot path stays on the
backend-aware path end to end.

The current `signal_pipeline` optimizer-tier benchmark in
[Testing and Quality Gates](testing.md) is now the clearest cautionary example:

- plain emitted R stays close to idiomatic vectorized GNU R
- `-O1/-O2` need to justify themselves through generic MIR/vectorization wins,
  not backend-specific special cases

On the current 2026-03-24 snapshot, the useful comparison is the generic
optimizer tier itself:

- RR O0 emitted R: benchmark-script output
- RR O1 emitted R: benchmark-script output
- RR O2 emitted R: benchmark-script output

The important week-1 change is that the benchmark scripts now also record
optimizer diagnostics for RR artifacts:

- emitted line count and helper residue (`repeat`, `for`, `rr_index1_*`,
  `rr_call_map_*`)
- `TachyonPulseStats` summaries for matched/applied vector plans
- trip-tier and call-map lowering counts

The practical rule is:

- do not assume primitive-by-primitive wrappers are enough
- compare `-O0/-O1/-O2` first and only widen the matrix when a backend path is
  still demonstrably alive
- tie every performance claim back to emitted-shape and pulse diagnostics

On the current week-4 signoff snapshot, the diffusion slice is also a useful
sanity check that the generic path is still doing real work: `heat_diffusion`
and `reaction_diffusion` both stay in the same broad O2 band instead of
needing benchmark-specific backend lowering to look competitive.

The inverse rule matters just as much:

- do not assume every backend-looking kernel should become a fused backend helper
- already-compact emitted vector R can still be the best answer
- irregular gather-heavy kernels may flatten out or regress even after fusion

The current backend-candidate slice shows both sides:

- `orbital_sweep` improves a lot in warm runs after whole-run fusion
- `vector_fusion` gets slower because emitted R was already a compact whole-vector affine expression
- `bootstrap_resample` stays roughly flat because the gather-heavy resample loop does not benefit much from the fused helper boundary

Tachyon should therefore prefer fewer backend crossings with larger,
proof-backed kernels over many small “technically native” calls, but only when
the resulting kernel still matches the actual cost shape of the workload.

## Reduction Rules

Reductions are intentionally narrower than maps.

A reduction candidate must not rely on:

- ambiguous loop-local state
- unstable loop-local state
- extra non-accumulator loop state
- accumulator self-reference hidden inside the candidate RHS

This is the main barrier against “closed-form” miscompiles in stateful loops.

## Debugging Tachyon

Use:

```bash
RR_VECTORIZE_TRACE=1 target/debug/RR file.rr -o out.R -O2 --no-incremental
RR_VERIFY_EACH_PASS=1 target/debug/RR file.rr -o out.R -O2 --no-incremental
```

Useful companion references:

- [Compiler Pipeline](pipeline.md)
- [Runtime and Error Model](runtime-and-errors.md)
- [Testing and Quality Gates](testing.md)
