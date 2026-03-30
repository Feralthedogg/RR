# Parallel Compilation Design

This page describes RR's current parallel-compilation architecture and the
design choices behind it.

The core compiler-side scheduler, slot-based `ProgramIR` carrier, SCC-wave type
analysis, function-local Tachyon waves, and parallel function emission are now
implemented in the current tree.

The page describes the architecture as implemented in the current RR pipeline
in `src/compiler/pipeline.rs`, `src/typeck/solver.rs`, `src/mir/opt.rs`, and
`src/codegen/mir_emit.rs`.

## Goal

Reduce wall-clock compile latency without changing:

- emitted `.R` semantics
- deterministic output ordering
- structured diagnostic behavior
- existing incremental cache guarantees

The recommended model is:

- keep full-program boundaries where RR genuinely needs them
- parallelize function-local work aggressively between those boundaries
- preserve deterministic stitching and validation after every barrier

## Why This Shape Fits RR

RR is not a uniform pipeline where every phase can simply become
`par_iter_mut()`.

The current structure falls into two groups:

- function-local work
  - MIR lowering of individual `HirFn`
  - most Tachyon tiers
  - de-SSA cleanup
  - MIR-to-R emission
- full-program work
  - import/module discovery
  - type-return fixed point
  - helper summary collection and helper-driven rewrites
  - interprocedural inlining
  - final raw-emitted-R cleanup

That makes a staged task graph with serial barriers the best fit. A fully
lock-free everywhere design would be much more complex than the current
compiler needs, while a single giant mutex around `all_fns` would leave most of
the speedup on the table.

## Do Not Reuse `ParallelConfig`

Current `ParallelConfig` controls generated runtime/backend policy:

- `parallel_mode`
- `parallel_backend`
- `parallel_threads`
- `parallel_min_trip`

That is artifact semantics, not compiler scheduling.

Compiler parallelism should use a separate config so that:

- emitted runtime policy stays reproducible
- host compile-thread count does not leak into output semantics
- `RR build` can choose outer-file parallelism independently of inner
  function-level parallelism

Recommended API shape:

```rust
pub enum CompilerParallelMode {
    Off,
    Auto,
    On,
}

pub struct CompilerParallelConfig {
    pub mode: CompilerParallelMode,
    pub threads: usize,      // 0 => std::thread::available_parallelism()
    pub min_functions: usize,
    pub min_fn_ir: usize,
    pub max_jobs: usize,     // 0 => active worker count
}
```

`Auto` should stay conservative and only turn on when there is enough work to
amortize scheduling overhead.

Current default behavior:

- compiler parallel mode defaults to `Auto`
- compiler thread count defaults to `0`, which resolves to host parallelism
- users can still override mode, worker count, thresholds, and max concurrent
  jobs from the CLI

## Core Structural Refactor

The current full-program carrier is effectively:

- `FxHashMap<String, FnIR>`
- `emit_order: Vec<String>`
- `emit_roots: Vec<String>`
- `top_level_calls: Vec<String>`

That shape is easy for serial code but awkward for deterministic parallel
mutation. The recommended refactor is to wrap it in a stable slot-based program
container.

```rust
pub type FnSlot = usize;

pub struct ProgramIR {
    pub fns: Vec<FnUnit>,
    pub by_name: FxHashMap<String, FnSlot>,
    pub emit_order: Vec<FnSlot>,
    pub emit_roots: Vec<FnSlot>,
    pub top_level_calls: Vec<FnSlot>,
}

pub struct FnUnit {
    pub name: String,
    pub ir: Option<FnIR>,
    pub is_public: bool,
    pub is_top_level: bool,
}
```

Why this is worth doing first:

- `Vec<FnUnit>` is easy to process in stable order
- function-local waves can use parallel iteration without fighting the borrow
  checker on a hash map
- global passes can still resolve names through `by_name`
- emitted order and diagnostics remain deterministic by slot/order index

Current implementation note:

- `FnUnit.ir` is stored as `Option<FnIR>` so legacy map-shaped phases can
  temporarily take ownership and then restore it without duplicating the
  program IR

Do not try to parallelize the current `FxHashMap<String, FnIR>` in place with
coarse locking. That would make the optimizer and emitter harder to reason
about and would not scale well on uneven workloads.

## Scheduler Choice

Use a dedicated work-stealing pool.

RR currently implements this with a shared cached `rayon` pool keyed by worker
count, reused across compile batches.

`rayon` is the best fit for RR because:

- function sizes vary a lot
- many phases are map-style over a stable function list
- there is no async runtime to integrate with
- the compiler already prefers bounded, CPU-oriented batch work

If dependency policy must stay minimal, a scoped worker pool on
`std::thread::scope` is the fallback. The design below still applies, but the
implementation will be more verbose.

Nested-parallelism note:

- `RR build` is still file-serial today
- if RR adds outer file-level parallel build mode later, file jobs and
  function-local jobs should continue to share this same compiler-side pool
  rather than stacking a second pool on top

Memory-pressure note:

- RR also exposes `max_jobs` on the compiler-parallel config as a safety valve
  for large programs
- this caps the number of simultaneously active compiler jobs even when the
  host has more available threads

## Recommended Stage Plan

| Stage | Recommended unit | Barrier reason | V1 plan |
| --- | --- | --- | --- |
| Source Analysis | keep mostly serial | shared import queue and loader state | serial |
| MIR Synthesis | each `HirFn` | need global arity table first | parallel |
| Type Analysis | SCC wave | return-type fixed point across calls | parallel by SCC wave |
| Tachyon Tier A | each function | uses immutable global summaries | parallel |
| Tachyon Tier B | each selected function | same as above | parallel |
| Tachyon Inline Tier | whole program | caller/callee graph rewrite | serial initially |
| Fresh alias + De-SSA | each function | summary computed before wave | parallel |
| R Emission | each function | final artifact order must be preserved | parallel emit, serial stitch |
| Raw R Cleanup | whole artifact | text rewrites cross function boundaries | serial |
| Runtime Injection | whole artifact | final output assembly | serial |

## Detailed Design

### 1. Source Analysis

Keep `run_source_analysis_and_canonicalization()` mostly serial in V1.

Reasons:

- import discovery currently uses a shared queue
- parser/lowerer diagnostics are easier to aggregate in source order
- this phase is not the cleanest first win compared with emission and Tachyon

Optional later improvement:

- parallelize file reads and parsing after import targets are discovered
- keep lowering/diagnostic collation deterministic by module id

### 2. MIR Synthesis

`run_mir_synthesis()` already has a natural split:

1. serial pre-pass to build `known_fn_arities`
2. per-function MIR lowering
3. per-module top-level wrapper synthesis

Recommended plan:

- collect `HirFn` jobs serially
- lower each function in parallel into `FnUnit`
- lower top-level synthetic `Sym_top_*` wrappers in a second wave
- assemble `ProgramIR` in stable module/item order

This is safe because each `MirLowerer` owns its own `FnIR` and only reads shared
symbol/arity tables.

### 3. Type Analysis

The current type solver iterates the whole function map until return summaries
stabilize. That is correct, but it leaves parallelism unused.

Recommended redesign:

1. build the user-function call graph
2. condense it into SCCs
3. process SCCs in reverse topological order
4. solve each SCC locally to a fixed point
5. run independent ready SCCs in parallel

Important rule:

- callee-summary dependencies flow from callers to callees, so SCC scheduling
  must respect the condensed DAG

For non-recursive SCCs:

- analyze once after all callee SCC summaries are known

For recursive SCCs:

- iterate only inside the SCC until local return summaries stabilize

This keeps the current semantics while removing unnecessary whole-program
re-scans.

### 4. Tachyon

`run_tachyon_phase()` mixes truly global work and function-local work. Split it
into waves.

#### 4.1 Global summary barrier

Keep these serial:

- optimization budget planning
- helper discovery (`floor`, `abs`, `clamp`, wrap/cube helpers)
- callmap whitelist collection
- any summary that inspects the full program before rewriting

These summaries should produce immutable program facts:

```rust
pub struct ProgramOptFacts {
    pub plan: ProgramOptPlan,
    pub floor_helpers: FxHashSet<String>,
    pub proven_floor_param_slots: FxHashMap<FnSlot, FxHashSet<usize>>,
    pub wrap_helpers: FxHashSet<String>,
    pub periodic_helpers: FxHashMap<String, PeriodicSummary>,
    pub cube_helpers: FxHashSet<String>,
    pub callmap_user_whitelist: FxHashSet<String>,
}
```

#### 4.2 Function-local optimization waves

Parallelize:

- Tier A always passes
- Tier B heavy passes for selected functions
- post-inline cleanup for each function
- fresh alias rewrite after fresh-returning summaries are known
- de-SSA and copy cleanup

Each worker should have exclusive access only to one `FnIR` for the wave.
Global summaries must be immutable during the wave.

#### 4.3 Inline tier

Keep bounded interprocedural inlining serial in V1.

Why:

- it mutates callers while reading the full callee set
- it is the easiest place to introduce nondeterministic behavior
- a wrong parallel inline design will be much harder to debug than a serial one

If RR later wants more speed here, the next step is snapshot-based parallel
caller rewriting per round, not ad hoc locks around the current inliner.

### 5. Emission

This is the cleanest first target.

`emit_r_functions_cached()` is currently serial but function emission is mostly
independent once these shared facts are computed:

- `fresh_user_calls`
- `seq_len_param_end_slots_by_fn`
- `quoted_entry_targets`
- `direct_builtin_call_map`

Recommended design:

1. compute shared emission facts serially
2. emit each function in parallel into an `EmittedFnFragment`
3. store fragments by `FnSlot`
4. stitch fragments serially in `emit_order`
5. run whole-artifact raw-R cleanup exactly once at the end

Suggested fragment type:

```rust
pub struct EmittedFnFragment {
    pub slot: FnSlot,
    pub code: String,
    pub map: Vec<MapEntry>,
    pub cache_hit: bool,
    pub line_count: u32,
}
```

Serial stitching must:

- preserve current `emit_order`
- rebase `MapEntry.r_line` by accumulated line offset
- append the same single blank line policy as today

Do not parallelize the final raw-R cleanup in V1. Many of those rewrites reason
about adjacent helper definitions and whole-artifact text shape.

## Incremental Compile and Cache Interaction

The existing incremental design already suggests the right granularity:

- phase 1 caches whole-artifact results
- phase 2 caches per-function emission
- phase 3 caches whole-artifact results in memory

That means compiler parallelism should align with phase 2, not fight it.

Recommended cache adjustments:

- make `EmitFunctionCache` thread-safe
- switch cache methods from `&mut self` to `&self`
- require `Send + Sync` for parallel emission backends

The disk cache is already key-addressed and does not need mutable shared state,
so this is a good fit.

## Determinism Rules

Parallel compile should remain reproducible.

Hard rules:

- maintain stable slot order from source/module order
- aggregate stats in slot order, not worker completion order
- sort diagnostics before emission to the user
- keep full-program barriers explicit
- keep `RR_VERIFY_EACH_PASS=1` meaningful after parallel waves

A compile should never emit different helper order or different source-map line
numbers just because the host had more cores available.

## Error Handling

Use fail-fast cancellation at the wave level:

- if any worker returns an error, stop dispatching new queued work for that wave
- collect already-finished worker errors
- surface one deterministic primary diagnostic and aggregate the rest when useful

The current implementation is cooperative rather than preemptive:

- workers already executing may finish
- workers that have not yet dequeued work stop taking new jobs once cancellation
  is signaled

Partially rewritten program state must not continue into the next phase.

## `RR build` and Outer Parallelism

`cmd_build()` currently compiles multiple `.rr` files serially.

That is a valid second layer of parallelism, but only after inner compiler
parallelism is controlled. The safest policy is:

- one shared compiler worker pool
- either outer-file parallelism or inner-function parallelism dominates
- avoid naive nested pools that oversubscribe the machine

V1 should focus on single-program latency first.

## Recommended Rollout Order

1. Introduce `ProgramIR` without changing behavior.
2. Parallelize function emission and serial stitching.
3. Parallelize MIR lowering.
4. Parallelize Tachyon Tier A, Tier B, fresh-alias, and de-SSA waves.
5. Convert type analysis to SCC-wave scheduling.
6. Consider optional outer-file parallelism for `RR build`.

This order gives the best payoff-to-risk ratio:

- emission is already function-granular and cache-aware
- MIR lowering is naturally isolated
- Tachyon has many function-local passes after global summaries are built
- type analysis is the biggest structural redesign and should come later

## Bottom Line

The best design for RR is not “make the whole compiler parallel.”

It is:

- a stable slot-based `ProgramIR`
- a dedicated compiler-parallel config
- function-local worker waves separated by explicit serial barriers
- serial inline and serial whole-artifact cleanup in V1
- deterministic stitching, validation, and diagnostics

That design matches RR's current architecture, preserves correctness fences, and
lets the compiler reuse its existing per-function emission cache instead of
working around it.
