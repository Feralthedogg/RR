# Contributing to RR (Compiler Code Style)

This guide is for contributors to the RR compiler implementation (`src/**`, `tests/**`).
It is **not** a style guide for RR language users writing `.rr` programs.

The target style is:

- predictable behavior
- explicit control/data flow
- performance-aware by default

Think "Power of Ten spirit, practical compiler edition."

## Scope

Applies to:

- compiler frontend/middle/backend code
- runtime bridge and CLI implementation
- tests, fuzz targets, regression harnesses

Does not apply to:

- user-authored RR source style
- generated R output formatting

## Core Principles

1. Determinism over cleverness.
2. Explicit cost over hidden convenience.
3. Simple control flow over dense abstractions.
4. Measured performance over speculative micro-optimization.

## Rule Levels

- `MUST`: required for new/modified compiler code.
- `SHOULD`: expected unless there is a documented reason.
- `MAY`: optional guidance.
- When a rule is unclear in a specific case, prefer the interpretation that
  preserves determinism, explicitness, and reviewability.

## Rules

### 1) Control Flow and Function Shape

- `MUST` keep normal-path nesting shallow (prefer early returns/continues).
- `SHOULD` keep one function focused on one concern.
- `SHOULD` split very long functions before adding more branches.
- `SHOULD` separate pure computation from mutation/side-effect orchestration
  when doing so improves reviewability.
- `SHOULD` prefer explicit local state transitions over callback-driven control
  flow in pipeline code.
- `MUST NOT` hide side effects inside chained combinators when a `for` loop is clearer.

### 2) Deterministic Output and Traversal

- `MUST` produce deterministic externally visible behavior from identical input.
- `MUST NOT` depend on hash iteration order for:
  - emitted code order
  - diagnostics order
  - cache key materialization order
- `MUST` control or fix RNG seeds in tests, fuzz reductions, and any pipeline
  component whose behavior would otherwise vary between runs.
- `MUST` define stable tie-break rules when multiple valid traversal, emission,
  or diagnostic orders exist.
- `MUST` keep ordering deterministic for golden files, snapshot tests,
  regression artifacts, and IR dumps.
- `SHOULD` sort keys or use deterministic structures when order matters.

### 3) Hot Path Discipline

A path is considered hot when it:

- runs per token, node, instruction, or emitted unit
- performs an input-sized traversal (`O(n)` or worse)
- appears in profile or benchmark results as a meaningful cost center

Typical hot paths in RR:

- lexer/parser scanning loops
- HIR/MIR lowering and analysis passes
- optimization pass inner loops
- codegen traversal loops

Rules:

- `MUST` avoid hidden allocation/work inside loop bodies on hot paths.
- `MUST NOT` introduce non-obvious allocation, hashing, or formatting in code
  that appears constant-time from the call site.
- `MUST` avoid repeated regex compilation, path canonicalization, or heavyweight formatting in hot loops.
- `MUST NOT` allocate temporary `String` values for diagnostic or debug-only
  output inside hot loops unless gated behind a debug or trace condition.
- `SHOULD` hoist invariant computations out of loops.
- `SHOULD` pre-allocate with `with_capacity` when growth is predictable.
- `SHOULD` make non-trivial cost visible in API shape (explicit allocation,
  builder, or precomputed context).
- `MUST NOT` add `#[inline(always)]` without benchmark evidence.

Example:

```rust
// BAD: traversal looks lightweight at the call site, but allocates one String
// per node and materializes a fresh Vec on the hot path.
let names = nodes.iter().map(|n| n.to_string()).collect::<Vec<_>>();
```

```rust
// GOOD: borrow where possible, or make reusable scratch/output explicit.
let names: Vec<&str> = nodes.iter().map(|n| n.as_str()).collect();
format_name_into(node, &mut scratch);
```

### 4) Ownership and Allocation

- `MUST` default to borrowing (`&T`, `&str`, slices) for read-only paths.
- `MUST` justify `clone()` in hot paths or when copying large/long-lived
  compiler data.
- `SHOULD` prefer borrowing, indexing, interning, or arena-backed handles over
  cloning in pass-heavy pipelines.
- `SHOULD` make ownership choices visible at pass boundaries (borrowed view,
  owned transform result, arena-backed handle, interned key).
- `SHOULD` keep large data structures out of tight loop copies.
- `SHOULD` choose contiguous layouts (`Vec`) for sequential pass-heavy workloads.
- `SHOULD` use interning or arena-style allocation for long-lived compiler data
  such as symbols, type metadata, or repeated semantic descriptors when that
  materially reduces copy churn or allocation pressure.

### 5) Pass Ownership and IR Growth

- `MUST` define which pass owns mutation rights for each IR stage.
- `MUST NOT` mutate IR structures outside the owning pass unless explicitly documented.
- `MUST` ensure each compiler pass has a clearly defined responsibility
  (analysis, lowering, optimization, validation, etc.).
- `MUST NOT` mix unrelated transformations in a single pass.
- `MUST` avoid transformations that can cause unbounded or super-linear IR
  growth without explicit justification.
- `MUST` document worst-case complexity for transformations that duplicate or
  expand IR.
- `SHOULD` treat non-owning passes as read-only or as producers of new IR.
- `SHOULD` prefer adding a new pass over overloading an existing pass with
  unrelated responsibilities.
- `SHOULD` include stress tests for transformations that can increase IR size.

### 6) Error Model (User Error vs Compiler Fault)

- `MUST` use structured diagnostics (`RRException`) for user-facing errors.
- `MUST` treat invalid user programs, unsupported user-facing constructs, and
  configuration/input errors as structured diagnostics.
- `MUST` include precise span/location for user-code errors when available.
- `MUST` use internal-fault path (`InternalCompilerError`/`ICE9001`) only for
  compiler bugs, violated invariants, or impossible internal states.
- `MUST NOT` use `panic!` for expected user-input failure paths.
- `MUST NOT` silently fall back to alternative behavior when an expected path fails.
- `MUST` surface explicit diagnostics, error returns, or documented fallback
  handling when fallback occurs.
- `SHOULD` attach at least one actionable note when the user can realistically
  fix the problem from source code or configuration changes.

### 7) Numeric and Conversion Safety

- `MUST` make overflow behavior explicit (`checked_*`, `saturating_*`, `wrapping_*`) where relevant.
- `MUST` avoid unchecked casts when truncation/sign changes are possible.
- `SHOULD` keep integer widths intentional in IR and optimizer logic.
- `MUST` preserve RR language semantics when simulating target-language
  arithmetic in compiler code, including constant folding and evaluator helpers.
- `MUST` add focused tests when changing constant folding, evaluator helpers,
  or overflow-sensitive logic.
- `SHOULD` document when compiler-side arithmetic intentionally differs from
  target-program arithmetic because it is implementing compiler bookkeeping
  rather than RR program semantics.
- `SHOULD` cover boundary cases including min/max values, sign changes, width
  truncation, and division/modulo edge cases.

### 8) Module Boundaries and Naming

- `MUST` keep module boundaries coherent with pipeline stages.
- `SHOULD` keep pass entrypoints, validation steps, and data model definitions
  easy to locate from module names alone.
- `SHOULD` use existing naming conventions:
  - `*_from_env`
  - `*_with_configs`
  - `run_*_phase`
  - `validate_*`
- `MUST` avoid introducing aliases that hide stage semantics.
- `MUST` avoid utility modules that mix unrelated pipeline stages or hide stage ownership.

### 9) Caching and Memoization

- `MUST` ensure cache keys are deterministic and capture all inputs affecting results.
- `MUST NOT` let caches change externally visible behavior except for performance.
- `MUST NOT` allow stale or partially invalid cache entries to influence correctness.
- `MUST` include compiler version/build identity, output mode, and
  semantic-affecting compile flags in persisted cache keys.
- `SHOULD` document invalidation assumptions for non-trivial caches.
- `SHOULD` provide a way to disable or bypass caches for debugging when practical.

### 10) Global State

- `MUST NOT` introduce mutable global state that affects compilation results.
- `SHOULD` pass required context explicitly through function parameters or
  structured contexts.

### 11) Environment Independence

- `MUST NOT` depend on wall-clock time, system randomness, or
  environment-specific paths for compilation results.
- `SHOULD` normalize environment-dependent inputs (paths, locales, and similar
  inputs) before use.

### 12) Testing and Validation Requirements

For non-trivial changes, contributors `MUST` run:

- `cargo check`
- `cargo clippy --all-targets -- -D warnings`
- at least one minimal targeted test that isolates the changed behavior,
  invariant, or regression surface

When touching parser/pipeline/type solver logic, contributors `SHOULD` run fuzz smoke:

- `cargo +nightly fuzz run parser ...`
- `cargo +nightly fuzz run pipeline ...`
- `cargo +nightly fuzz run type_solver ...`

For performance-sensitive changes, contributors `SHOULD` attach benchmark evidence in PR:

- command used (`cargo bench`, `hyperfine`, or equivalent)
- baseline vs changed result
- input/workload description
- brief environment note (CPU/OS/build mode) sufficient for rough reproduction

### 13) IR Debuggability and Developer Ergonomics

- `MUST` preserve stage-specific IR invariants across each pass boundary.
- `MUST` document new IR invariants at the point they are introduced.
- `MUST` use existing `validate_*` or verifier-style checks after non-trivial IR
  rewrites, and add focused validation when a new invariant is introduced.
- `SHOULD` document whether persisted IR/debug dump formats are versioned or are
  intentionally best-effort developer artifacts.
- `SHOULD` make compatibility expectations explicit when IR structure changes
  invalidate previous dumps, snapshots, or persisted intermediates.
- `SHOULD` state whether a verifier is expected before, after, or both before
  and after a non-trivial rewriting pass.
- `SHOULD` keep IR structs readable via `Debug` output suitable for troubleshooting.
- `SHOULD` add or maintain pretty-print paths for complex IR dumps used in debugging/review.
- `SHOULD` make IR dumps easy to diff across revisions.
- `MUST` keep IR dump ordering deterministic when used by tests or regression artifacts.

### 14) Logging and Tracing

- `SHOULD` use structured logging at pass boundaries and major phase transitions.
- `MUST NOT` emit debug/log formatting inside hot loops unless gated behind a
  debug or trace condition.
- `SHOULD` prefer concise phase summaries over noisy per-node logging by default.
- `SHOULD` keep developer-facing debug output deterministic when used in tests
  or bug reports.

### 15) Unsafe Code Policy

- `MUST NOT` use `unsafe` unless it is necessary and justified.
- `MUST` include an adjacent `// SAFETY:` comment explaining the invariant contract.
- `SHOULD` document why safe alternatives were insufficient.
- `MUST` keep unsafe preconditions visible and locally checkable where practical.
- `MUST` keep `unsafe` blocks minimal and narrowly scoped.
- `SHOULD` add targeted tests for invariants relied on by `unsafe`.

### 16) Experimental and Optional Behavior

- `SHOULD` gate experimental optimizations or unstable behavior behind explicit flags.
- `MUST` keep the default pipeline deterministic and production-safe.
- `SHOULD` ensure gated behavior is easy to disable during debugging and regression triage.

### 17) Dependency and Crate Discipline

- `SHOULD` prefer the standard library and existing project utilities over new
  dependencies for small conveniences.
- `MUST` get maintainer approval before adding external crates that can
  materially affect performance, compile time, portability, or determinism.
- `MUST NOT` add dependencies or abstraction layers that hide stage semantics or
  runtime cost without clear payoff.

### 18) Review Readability

- `MUST` favor code that a reviewer can reason about quickly.
- `SHOULD` use Rustdoc comments (`///`) on public APIs and on core IR
  transformation entrypoints whose design intent is not obvious from local code.
- `SHOULD` comment non-obvious invariants, stage assumptions,
  lifetime/ownership constraints, and performance-sensitive tradeoffs.
- `MUST NOT` add comments that restate obvious syntax.

## Exception Process

If you must break a `MUST`/`SHOULD` rule:

1. Document why in code comment or PR text.
2. Include expected impact (correctness/perf/maintainability).
3. Note the affected scope (hot path, determinism, pass boundary, safety, etc.)
   when it is not obvious from local context.
4. Add a focused test or benchmark evidence when relevant.

## PR Checklist

- Behavior is deterministic for same input/config.
- Stable tie-break and ordering rules are defined where multiple valid outputs exist.
- Control flow remains readable and explicit.
- No accidental hot-path allocation regressions or hidden API costs.
- Benchmark evidence is attached when touching hot-path performance.
- Pass mutation ownership and responsibility remain clear for touched IR stages.
- IR invariants still hold and relevant `validate_*`/verifier paths were exercised.
- IR dumps/Debug output remain readable, diffable, and deterministic for debugging.
- Long-lived compiler data does not introduce avoidable clone/allocation churn where interning or arenas are more appropriate.
- New cache or memoization paths have deterministic keys and documented invalidation assumptions.
- Persisted cache keys include compiler version/build identity and
  semantic-affecting flags when relevant.
- No mutable global state now affects compilation results.
- Compilation results do not depend on wall-clock time, randomness, or
  environment-specific paths without explicit normalization.
- No silent fallback changed correctness or masked failure.
- `unsafe` blocks (if any) include `// SAFETY:` rationale, explain why safe alternatives were not sufficient, and keep narrow scope.
- Error type/category is correct (user error vs ICE).
- Compiler-side constant folding, evaluators, and conversion helpers were checked against RR numeric/overflow semantics, including edge cases when relevant.
- Relevant tests and checks were executed, including at least one minimal targeted test for the changed behavior or invariant.
- New dependencies with material perf/determinism/portability impact were
  explicitly approved.
- Public API or core transform docs were updated when design intent changed.
- Docs updated if CLI/runtime/error semantics changed.

For a concrete post-change verification pass, use
[`docs/contributing-audit.md`](docs/contributing-audit.md).
