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

## Rules

### 1) Control Flow and Function Shape

- `MUST` keep normal-path nesting shallow (prefer early returns/continues).
- `SHOULD` keep one function focused on one concern.
- `SHOULD` split very long functions before adding more branches.
- `MUST NOT` hide side effects inside chained combinators when a `for` loop is clearer.

### 2) Deterministic Output and Traversal

- `MUST` produce deterministic externally visible behavior from identical input.
- `MUST NOT` depend on hash iteration order for:
  - emitted code order
  - diagnostics order
  - cache key materialization order
- `SHOULD` sort keys or use deterministic structures when order matters.

### 3) Hot Path Discipline

Typical hot paths in RR:

- lexer/parser scanning loops
- HIR/MIR lowering and analysis passes
- optimization pass inner loops
- codegen traversal loops

Rules:

- `MUST` avoid hidden allocation/work inside loop bodies where possible.
- `MUST` avoid repeated regex compilation, path canonicalization, or heavyweight formatting in hot loops.
- `SHOULD` pre-allocate with `with_capacity` when growth is predictable.
- `MUST NOT` add `#[inline(always)]` without benchmark evidence.

### 4) Ownership and Allocation

- `MUST` default to borrowing (`&T`, `&str`, slices) for read-only paths.
- `MUST` avoid unnecessary `clone()` in performance-sensitive code.
- `SHOULD` keep large data structures out of tight loop copies.
- `SHOULD` choose contiguous layouts (`Vec`) for sequential pass-heavy workloads.

### 5) Error Model (User Error vs Compiler Fault)

- `MUST` use structured diagnostics (`RRException`) for user-facing errors.
- `MUST` include precise span/location for user-code errors when available.
- `MUST` use internal-fault path (`InternalCompilerError`/`ICE9001`) for compiler bugs or invariant breaks.
- `MUST NOT` use `panic!` for expected user-input failure paths.
- `SHOULD` attach actionable notes for recovery/debugging.

### 6) Numeric and Conversion Safety

- `MUST` make overflow behavior explicit (`checked_*`, `saturating_*`, `wrapping_*`) where relevant.
- `MUST` avoid unchecked casts when truncation/sign changes are possible.
- `SHOULD` keep integer widths intentional in IR and optimizer logic.

### 7) Module Boundaries and Naming

- `MUST` keep module boundaries coherent with pipeline stages.
- `SHOULD` use existing naming conventions:
  - `*_from_env`
  - `*_with_configs`
  - `run_*_phase`
  - `validate_*`
- `MUST` avoid introducing aliases that hide stage semantics.

### 8) Testing and Validation Requirements

For non-trivial changes, contributors `MUST` run:

- `cargo check`
- `cargo clippy --all-targets -- -D warnings`
- targeted tests for touched subsystem

When touching parser/pipeline/type solver logic, contributors `SHOULD` run fuzz smoke:

- `cargo +nightly fuzz run parser ...`
- `cargo +nightly fuzz run pipeline ...`
- `cargo +nightly fuzz run type_solver ...`

For performance-sensitive changes, contributors `SHOULD` attach benchmark evidence in PR:

- `cargo bench` and/or `hyperfine` results
- baseline vs changed command/results
- brief environment note (CPU/OS/build mode)

### 9) IR Debuggability and Developer Ergonomics

- `SHOULD` keep IR structs readable via `Debug` output suitable for troubleshooting.
- `SHOULD` add or maintain pretty-print paths for complex IR dumps used in debugging/review.
- `MUST` keep IR dump ordering deterministic when used by tests or regression artifacts.

### 10) Unsafe Code Policy

- `MUST NOT` use `unsafe` unless it is necessary and justified.
- `MUST` include an adjacent `// SAFETY:` comment explaining the invariant contract.
- `MUST` keep `unsafe` blocks minimal and narrowly scoped.
- `SHOULD` add targeted tests for invariants relied on by `unsafe`.

### 11) Review Readability

- `MUST` favor code that a reviewer can reason about quickly.
- `SHOULD` add brief comments only for non-obvious invariants or performance constraints.
- `MUST NOT` add comments that restate obvious syntax.

## Exception Process

If you must break a `MUST`/`SHOULD` rule:

1. Document why in code comment or PR text.
2. Include expected impact (correctness/perf/maintainability).
3. Add a focused test or benchmark evidence when relevant.

## PR Checklist

- Behavior is deterministic for same input/config.
- Control flow remains readable and explicit.
- No accidental hot-path allocation regressions.
- Benchmark evidence is attached when touching hot-path performance.
- IR dumps/Debug output remain readable and deterministic for debugging.
- `unsafe` blocks (if any) include `// SAFETY:` rationale and narrow scope.
- Error type/category is correct (user error vs ICE).
- Relevant tests and checks were executed.
- Docs updated if CLI/runtime/error semantics changed.

For a concrete post-change verification pass, use
[`docs/contributing-audit.md`](docs/contributing-audit.md).
