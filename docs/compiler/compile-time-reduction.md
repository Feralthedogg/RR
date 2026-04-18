# Compile-Time Reduction Plan

This page is the implementation plan for reducing RR compile latency in a way
that is structural, measurable, and hard to regress by accident.

It is written for compiler work, not user-facing docs.

## Why This Exists

Recent compile-time work improved RR substantially, but the current profile
still shows the same structural problem:

- total cold compile is still dominated by emitted-R cleanup
- emitted-R cleanup is still dominated by repeated peephole scans
- the slowest remaining peephole work is not one giant pass anymore, but a
  cluster of repeated exact-reuse, dead-temp, helper-cleanup, and finalize
  scans over the same function text

One recent single-run `example/tesseract.rr -O2 --no-incremental` profile on
`2026-04-07` looked like:

- total: about `8.2s`
- emit: about `5.9s`
- peephole: about `5.1s`
- `primary_loop_cleanup`: about `2.2s`
- `secondary_rewrite`: about `1.9s`

The important point is not the exact absolute number. Single-run wall-clock is
still noisy. The stable signal is that compile time is being spent re-deriving
the same facts from the same emitted lines in many passes.

## Warm-Path Evidence

As of `2026-04-18`, the current cache stack is no longer hypothetical. Warm
compiles are reusing:

- optimized MIR
- per-function emitted fragments
- exact final optimized emitted-R artifacts when available

Representative isolated cold/warm runs using `target/debug/RR`, `-O2`,
`--no-runtime`, `--no-incremental`, and a dedicated cache root looked like:

| Program | Cold | Warm | Speedup | Warm Reuse Tier |
| --- | ---: | ---: | ---: | --- |
| `example/tesseract.rr` | `7200ms` | `989ms` | `7.3x` | optimized MIR + final artifact |
| `example/benchmarks/signal_pipeline_bench.rr` | `544ms` | `66ms` | `8.2x` | optimized MIR + final artifact |
| `example/data_science/logistic_ensemble.rr` | `327ms` | `93ms` | `3.5x` | optimized MIR + final artifact |
| `example/benchmarks/heat_diffusion_bench.rr` | `501ms` | `113ms` | `4.4x` | optimized MIR + final artifact |
| `example/data_science/bootstrap_mean.rr` | `371ms` | `110ms` | `3.4x` | optimized MIR + final artifact |
| `example/physics/orbital_two_body.rr` | `401ms` | `93ms` | `4.3x` | optimized MIR + final artifact |

Two points matter more than the exact numbers:

- warm compile is now structurally different from cold compile
- several representative non-trivial programs already skip whole-output raw
  rewrite and peephole entirely on the warm path

For example, the `tesseract` warm profile reported:

- total: about `904ms`
- Tachyon: about `220ms`
- emit: about `550ms`
- raw rewrite: `0`
- peephole: `0`
- `optimized_mir_cache_hit=true`
- `optimized_fragment_final_artifact_hits=1`

And in the representative six-program sweep above, every warm profile reported:

- `optimized_mir_cache_hit=true`
- `optimized_fragment_final_artifact_hits=1`
- `raw_rewrite_elapsed_ns=0`
- `peephole_elapsed_ns=0`

So the remaining work is not "make warm path exist". It already exists. The
remaining work is:

- widen hit coverage
- keep invalidation/corruption behavior safe
- make the win easy to explain and hard to regress

## Current Deep Dive

### Why A 1625-Line File Still Takes About 4.8s

As of `2026-04-14`, a representative single-run profile for
`example/tesseract.rr -O2 --no-runtime --no-incremental --profile-compile`
looked like this:

- source size: `1625` lines
- total: about `4.8s`
- Tachyon: about `2.1s`
- emit: about `2.5s`
- raw rewrite: about `0.07s`
- peephole: about `1.56s`
- `secondary_rewrite`: about `0.49s`
- `primary_loop_exact_cleanup`: about `0.28s`

That snapshot matters for two reasons:

- `emit` is now more than half of total wall-clock
- `raw_rewrite` is no longer the main problem

The remaining latency is no longer "one stupid text rewrite". It is the cost of
lowering a non-trivial numeric kernel into many MIR / emitted-R fragments, then
running the still-necessary non-local cleanup families over that lowered form.

### Line Count Is The Wrong Metric

`1625` source lines is a weak predictor for RR compile cost.

What matters much more is:

- how many MIR functions the module lowers into
- how many loops survive into MIR / emitted R
- how many helper-shaped expressions get materialized
- how much branch-local and loop-local cleanup remains after emission

Recent `tesseract` runs reported:

- `41` MIR functions synthesized
- `23` emitted functions
- around `80` loops seen by Tachyon
- `11` vectorized loops

So the backend is not compiling "a 1625-line script". It is compiling a
lowered program graph with dozens of functions and many loop / helper shapes.

### What The 4.8s Is Actually Paying For

At this point the cost splits roughly like this:

1. Tachyon optimization
   RR is still spending about `2.1s` simplifying, vectorizing, and structurally
   preparing MIR before emission even starts.

2. Function emission
   Emission is still expensive because every lowered function is rendered into R
   text, with local canonicalization and source-map work.

3. Non-local emitted-R cleanup
   The big remaining peephole cost is no longer local scalar cleanup. It is:
   - `secondary_rewrite`
   - `primary_loop_exact_cleanup`
   - `secondary_finalize_cleanup`

4. Cold path with no incremental reuse
   `--no-incremental` deliberately disables the warm-path wins we already have:
   - function emit cache
   - whole-output raw rewrite cache
   - whole-output peephole cache

So for cold compiles, RR still has to do the full backend pipeline.

### What Is No Longer The Problem

The following used to matter much more and now do not:

- whole-file raw rewrite
- repeated local scalar alias cleanup
- repeated local named-list / field-get cleanup
- repeated local `.arg_` alias cleanup
- repeated exact-pre parse/setup work
- repeated exact-reuse parse work
- repeated secondary inline named-index / scalar-region scans

This is visible in the current numbers:

- `raw_rewrite` is down near `0.07s`
- many local canonicalizations now happen in emitter postprocess
- many peephole families now run as shared IR bundles instead of separate parses

### The Remaining Structural Problems

The remaining backend cost is mostly one of these:

#### 1. Secondary rewrite still contains non-local work

Even after bundle work, `secondary_rewrite` still owns:

- helper-family rewrites with cross-line dependencies
- finalize work that still needs liveness / compaction reasoning
- some full-range / loop-adjacent cleanup that is still text-oriented

This is why `secondary_rewrite` remains the largest single peephole bucket.

#### 2. Exact cleanup is now smaller, but still real

`primary_loop_exact_cleanup` is much smaller than before, but it is still doing:

- exact-reuse candidate reasoning
- vector alias cleanup
- final rebind cleanup
- one fixpoint round in some shapes

This is no longer catastrophic, but it is still one of the last meaningful
peephole costs.

#### 3. Emission itself now matters more

Because raw rewrite and many local peephole passes have been removed or moved,
the remaining backend wall-clock increasingly comes from:

- building emitted function fragments
- rendering expressions / helpers
- source-map bookkeeping

This is a good sign structurally, but it also means future wins need to target
emission architecture, not just peephole micro-optimizations.

### Conclusion

The answer to "why does a 1625-line file still take about 4.8s" is:

- because it is not behaving like a 1625-line script after lowering
- because cold compiles still pay the full MIR + backend pipeline
- because the remaining backend cost is now concentrated in non-local cleanup
  and function emission, not cheap local rewrites

This means the next wins must come from:

- more `secondary_rewrite` integration
- more exact-family absorption into emitter aliasing / canonicalization
- optimized artifact reuse above the current raw-rewrite / peephole caches

## Problem Statement

Today many peephole passes still work like this:

1. take `Vec<String>`
2. rescan lines
3. re-run regex matching
4. re-run `expr_idents(...)`
5. rediscover the same assignments / uses / helper boundaries
6. rewrite one narrow shape
7. repeat the whole process in the next pass

That design makes compile time scale with:

- number of passes
- number of functions
- number of lines in a function
- number of repeated candidate rescans inside a function

This is the main reason micro-optimizations keep giving uneven results. We are
still paying the same asymptotic cost, only with slightly smaller constants.

## Design Goal

The goal is to replace "pass scans raw text and rediscovers facts" with
"pass consumes precomputed facts and only touches candidate lines".

That is the only direction that reliably reduces compile time without relying
on benchmark luck.

## Non-Goals

This plan is not trying to:

- rewrite the whole backend at once
- remove every peephole pass immediately
- change RR semantics
- make O2 rely on heuristic skipping of correctness-critical cleanup

The first version must be semantics-preserving and easy to verify.

## Core Design

### 1. Promote `FunctionTextIndex` into `FunctionFacts`

RR already has pieces of function-level indexing in the peephole layer. The
next step is to make that the primary substrate for expensive passes.

Add a function-scoped fact bundle in `src/compiler/peephole/facts.rs`:

```rust
struct FunctionFacts {
    fn_range: FunctionRange,
    line_facts: Vec<LineFacts>,
    defs: FxHashMap<String, SmallVec<[LineId; 4]>>,
    uses: FxHashMap<String, SmallVec<[LineId; 8]>>,
    first_use_after: FxHashMap<(LineId, SymbolId), Option<LineId>>,
    next_def_after: FxHashMap<(LineId, SymbolId), Option<LineId>>,
    prologue_arg_aliases: FxHashMap<String, String>,
    candidate_sets: CandidateSets,
}
```

`LineFacts` should contain only information that is re-read frequently:

- line kind: assign / return / control / helper def / blank
- indentation
- function boundary flags
- `lhs`
- normalized `rhs`
- identifier list
- helper-call flags
- `.arg_` alias flags
- `rr_*` helper family flags
- index-read / index-write flags
- brace delta / block boundary flags

The key rule is simple:

- compute line facts once per function version
- share them across many passes

### 2. Introduce Versioned Function-Local Analysis Cache

Each function in emitted R should carry a stable version/fingerprint.

Add:

```rust
struct PeepholeFunctionCache {
    version: u64,
    facts: FunctionFacts,
}

struct PeepholeAnalysisCache {
    functions: FxHashMap<FnId, PeepholeFunctionCache>,
}
```

When a pass mutates a function:

- invalidate only that function
- rebuild facts only for that function
- do not rebuild the whole file

This is the most important invalidation rule in the design.

### 3. Change Pass API from Raw Text to Candidate-Driven Facts

Current style:

```rust
fn rewrite_x(lines: Vec<String>) -> Vec<String>
```

Target style:

```rust
fn rewrite_x(
    file: &mut EmittedFile,
    fn_id: FnId,
    facts: &FunctionFacts,
    scratch: &mut PassScratch,
) -> bool
```

The pass should:

- consult candidate sets
- skip immediately if candidate set is empty
- only inspect relevant lines
- report whether it changed anything

This is a much better fit for RR than trying to make every peephole pass
string-only forever.

### 4. Group Passes by Shared Facts

The current breakdown already shows that cost clusters exist.

We should turn those into explicit pass families:

- `secondary_inline`
- `secondary_exact`
- `secondary_helper_cleanup`
- `secondary_finalize_cleanup`
- `primary_loop_exact_cleanup`

Inside each family, pass ordering stays the same initially, but all passes must
reuse the same facts.

That lets one expensive fact build pay for multiple passes.

## First Implementation Slice

Do not start by rewriting every peephole pass. Start where the profile is
already clear.

### Slice A: Exact Reuse Family

Move the following passes to `FunctionFacts` first:

- `rewrite_forward_exact_pure_call_reuse`
- `rewrite_forward_exact_expr_reuse`
- `strip_redundant_identical_pure_rebinds`
- fixpoint rounds that repeat those same families

Why first:

- they are currently the biggest remaining exact-cleanup cost
- they repeatedly compute `expr_idents`
- they repeatedly search for later defs / uses of the same symbol
- they fit naturally with precomputed `next_def_after` / `first_use_after`

Target algorithm:

1. build function facts once
2. identify candidate assign lines once
3. for each candidate, use precomputed next-def / use-site facts
4. rewrite only candidate use lines
5. invalidate and rebuild facts only if the function changed

Expected effect:

- repeated O(lines * later-lines) scans collapse toward O(lines + candidates)

### Slice B: Dead Temp / Finalize Family

Next move:

- `strip_dead_temps`
- `mark_overwritten_dead_assignments`
- `mark_branch_local_dead_inits`
- `mark_redundant_identical_temp_reassigns`

Why second:

- current finalize work still spends time on global-ish dataflow derived from
  line text
- this family benefits strongly from shared def/use tables

Target algorithm:

- use `defs` and `uses` maps from `FunctionFacts`
- use function-scoped live/read summaries instead of cloning per-line sets
- keep compaction semantics identical

### Slice C: Helper / Alias Cleanup Family

After exact and dead-temp families:

- `rewrite_readonly_param_aliases`
- `rewrite_remaining_readonly_param_shadow_uses`
- `rewrite_index_only_mutated_param_shadow_aliases`
- `strip_unused_arg_aliases`
- `strip_unused_helper_params_with_cache`

Why third:

- these passes already have partial caches
- they still rely too much on rescanning body text
- they want the same alias and use-site facts

Target algorithm:

- precompute `.arg_` defs, alias defs, and alias uses
- precompute helper body / helper params / helper call sites once per function

## Data Structures to Add

### `LineFacts`

Minimum useful fields:

- `lhs: Option<String>`
- `rhs: Option<String>`
- `idents: SmallVec<[String; 8]>`
- `is_assign: bool`
- `is_return: bool`
- `is_control_boundary: bool`
- `is_function_header: bool`
- `contains_arg_alias: bool`
- `contains_sym_call: bool`
- `contains_rr_call: bool`
- `contains_index_read: bool`
- `contains_index_write: bool`

### `CandidateSets`

Minimum useful candidate indexes:

- `exact_pure_call_candidates`
- `exact_expr_candidates`
- `temp_assign_candidates`
- `arg_alias_def_lines`
- `arg_alias_use_lines`
- `helper_call_lines`
- `helper_def_lines`

### `PassScratch`

Pass-local scratch buffers should be reused:

- replacement string buffer
- candidate line list
- temporary symbol set

Avoid allocating new `Vec<String>` inside hot inner loops.

## Pass Invalidation Rules

This is where many compile-time designs fail by becoming too expensive.

Rules:

- mutation invalidates only the touched function
- line-local rewrites do not invalidate other functions
- helper-definition changes invalidate helper facts only for that function
- no global recompute unless a pass adds/removes top-level function boundaries

This is why the design stays function-scoped first.

## Migration Strategy

### Stage 1

No behavior changes.

- add `FunctionFacts`
- add `PeepholeAnalysisCache`
- add function-version invalidation
- add fact builders
- add profile counters for cache hits / misses

### Stage 2

Port exact-reuse family to facts.

- keep old implementation behind a debug fallback if needed
- compare output hashes on test fixtures

### Stage 3

Port dead-temp family to facts.

- verify line map behavior stays identical

### Stage 4

Port alias/helper cleanup family.

- delete redundant whole-body rescans only after new facts path passes

### Stage 5

Only after those succeed:

- consider moving selected peephole rewrites into codegen/MIR

That is a separate semantic cleanup project, not the first compile-time win.

## Measurement Rules

We do not accept compile-time changes based only on one lucky run.

Use:

- structure counters as the primary correctness target
- sequential cold runs for runtime comparison
- profile breakdowns for regression diagnosis

Required profile outputs for this work:

- function-fact cache hits / misses
- exact-reuse candidate count
- dead-temp candidate count
- helper-cleanup candidate count
- per-family elapsed time

## Required Tests

### Existing suites to keep green

- `compile_profile_smoke`
- `dev_fast_profile_smoke`
- `tachyon_pass_plan`
- `r_emit_regressions`

### New tests to add during migration

- `peephole_function_facts_smoke`
- `peephole_exact_reuse_facts_equivalence`
- `peephole_dead_temps_facts_equivalence`
- `peephole_alias_facts_equivalence`
- `peephole_fact_cache_invalidation`

### Performance gates

Do not assert wall-clock in unit tests.

Instead assert:

- fact cache hit/miss counters
- candidate counts
- number of rebuilt functions
- per-family elapsed counters are emitted

## Success Criteria

This design is successful when:

1. exact-reuse and dead-temp families stop rescanning raw function text for the
   same facts
2. function-local cache rebuilds happen only for touched functions
3. `peephole` latency keeps falling without needing many one-off guards
4. profile output makes the next bottleneck obvious

## Next Implementation Step

The first implementation slice should be:

1. add `FunctionFacts` and `PeepholeAnalysisCache`
2. port exact-reuse family first
3. remeasure

That is the highest-confidence path to more compile-time wins from here.
