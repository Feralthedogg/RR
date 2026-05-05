<!-- GENERATED FILE: DO NOT EDIT DIRECTLY -->
<!-- Source: policy/contributing_rules.toml -->

This file is generated from `policy/contributing_rules.toml`. Edit the policy file, not the rendered Markdown.

# Contributing to RR (Compiler Code Style)
This guide applies to RR compiler implementation work under `src/**`, `tests/**`, `scripts/**`, `docs/**`, and verification tooling. It does not define style for user-authored `.rr` programs.
The target style is:
- predictable behavior
- explicit control/data flow
- performance-aware by default
Think Power-of-Ten discipline, practical compiler edition.
## Scope
- Applies: compiler, runtime, CLI, tests, verification tooling, and behavior-defining docs.
- Excludes: user-authored `.rr` style and generated R formatting when semantics stay the same.
## Core Principles
1. Determinism over cleverness.
2. Explicit cost over hidden convenience.
3. Simple control flow over dense abstractions.
4. Measured performance over speculative micro-optimization.
## Rule Levels
- `MUST`: required for new or modified compiler work.
- `SHOULD`: expected unless there is a documented reason to deviate.
- `MAY`: optional guidance when it improves clarity, safety, or speed.
## Rules
### 1) Deterministic Output and Traversal
- `MUST` keep output, diagnostics, cache keys, dumps, and snapshots deterministic; never rely on hash order or unfixed RNG.
### 2) Error Model (User Error vs Compiler Fault)
- `MUST` use `RRException` for user-facing failures and ICE for compiler bugs; expected user failures `MUST NOT` `panic!` or silently fallback.
### 3) Testing and Validation Requirements
- `MUST` make `perl scripts/contributing_audit.pl`, `cargo check`, `cargo clippy --all-targets -- -D warnings`, and at least one minimal targeted test pass; `SHOULD` use `scripts/verify_cleanroom.sh` when you need a cleanroom result.
### 4) Hot Path Discipline
- `MUST` keep hot paths free of hidden allocation such as `to_string()` or `clone()` in loops, and `#[inline(always)]` still `MUST` carry benchmark evidence.
### 5) Unsafe Code Policy
- `MUST` keep `unsafe` directly adjacent to `// SAFETY:` with the invariant, why safe alternatives are insufficient, and the narrow/local scope.
### 6) Commenting Rules
- `MUST` explain why, use structured `TODO/FIXME/NOTE`, and `SHOULD` delete stale comments immediately.
## Exception Process
1. Document why. 2. State impact and scope. 3. Add targeted tests or benchmark evidence when relevant.
## PR Checklist
- Behavior is deterministic for the same input and configuration.
- Relevant tests and checks were executed, including `perl scripts/contributing_audit.pl`, `cargo check`, `cargo clippy --all-targets -- -D warnings`, and at least one minimal targeted test.
- Touched semantic areas such as cache behavior, fallback behavior, numeric semantics, and IR invariants were reviewed for correctness.
- `unsafe`, mutable global state, and environment-sensitive paths were reviewed to confirm they cannot change compilation correctness.
- Docs updated if CLI/runtime/error semantics changed.
For a concrete post-change verification pass, use
[`docs/compiler/contributing-audit.md`](docs/compiler/contributing-audit.md).