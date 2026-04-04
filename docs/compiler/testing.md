<!-- GENERATED FILE: DO NOT EDIT DIRECTLY -->
<!-- Source: policy/contributing_rules.toml -->

This file is generated from `policy/contributing_rules.toml`. Edit the policy file, not the rendered Markdown.

# Testing and Quality Gates

This page is the verification manual for RR.

## Audience

Read this page when you need to choose:

- which test layer should catch a regression
- which local command matches CI
- when to use cleanroom, audit, determinism, cache, fallback, or fuzz flows

The goal is not just “did it compile?” but:

- did meaning stay the same?
- did emitted R keep the expected shape?
- did runtime helper policy stay intact?
- did optimization stay within budget?

## Prerequisites

- Most Rust-only tests need only `cargo`.
- Tests that execute generated R require `Rscript` in `PATH`.

## Primary Commands

Run the standard local verification stack:

```bash
cargo fmt --all --check
cargo test -q
cargo clippy --all-targets -- -D warnings
```

Run one focused suite:

```bash
cargo test -q --test vectorization_extended
cargo test -q --test case_regressions
```

Audit helper:

```bash
perl scripts/contributing_audit.pl
perl scripts/contributing_audit.pl --scan-only
```

On non-scan runs, `perl scripts/contributing_audit.pl` also escalates into scope-driven semantic smoke for cache correctness, determinism, numeric semantics, and fallback/runtime behavior. Use `--skip-semantic-smoke` only when changing the audit wiring itself.

Cleanroom strict verification helper:

```bash
scripts/verify_cleanroom.sh
scripts/verify_cleanroom.sh --files src/syntax/parse.rs tests/statement_boundaries.rs
scripts/verify_cleanroom.sh --fast --files scripts/verify_cleanroom.sh
```

## Local vs CI

RR CI does not replace local verification. The intended model is:

- focused local regression first
- standard local stack second
- diff-scoped static audit and full-scope semantic CI audit as confirmation

## Test Families

### Frontend and Syntax

- `syntax_errors.rs`
- `parse_multi_errors.rs`
- `statement_boundaries.rs`

Parsing, syntax diagnostics, and recovery boundaries.

### Optimization Correctness

- `vectorization_extended.rs`
- `vectorization_phi_ifelse.rs`
- `benchmark_vectorization.rs`
- `sccp_overflow_regression.rs`
- `opt_level_equivalence.rs`
- `rr_logic_equivalence_matrix.rs`
- `numeric_property_differential.rs`

Optimizer legality, numeric semantics, and emitted-artifact parity.

### Incremental Compile and Cache Correctness

- `incremental_phase1.rs`
- `incremental_phase2.rs`
- `incremental_phase3.rs`
- `incremental_auto.rs`
- `incremental_strict_verify.rs`
- `cli_incremental_default.rs`
- `cache_equivalence_matrix.rs`

Artifact cache invalidation, emit-cache reuse, strict verify, and output-mode separation.

### Runtime, Fallback, and Environment Independence

- `runtime_contract.rs`
- `runtime_semantics_regression.rs`
- `hybrid_fallback.rs`
- `parallel_optional_fallback_semantics.rs`
- `native_optional_fallback.rs`
- `fallback_correctness_matrix.rs`
- `hermetic_determinism.rs`

Fallback correctness, runtime subset injection, and hermetic determinism behavior.

### Stress and Determinism

- `commercial_determinism.rs`
- `random_differential.rs`
- `pass_verify_examples.rs`
- `compiler_parallel_equivalence.rs`

Determinism-sensitive, generated-program differential, and pass-by-pass verifier fences.

### Verification Tooling

- `contributing_audit_smoke.rs`
- `recommended_package_coverage_smoke.rs`
- `verification_summary_smoke.rs`
- `triage_reduce_smoke.rs`

Audit, reporting, and triage-tool behavior.

For the normative contributor rule set, see
[`CONTRIBUTING.md`](https://github.com/Feralthedogg/RR/blob/main/CONTRIBUTING.md).
