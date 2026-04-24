<!-- GENERATED FILE: DO NOT EDIT DIRECTLY -->
<!-- Source: policy/contributing_rules.toml -->

This file is generated from `policy/contributing_rules.toml`. Edit the policy file, not the rendered Markdown.

# Contributing Audit Checklist

Current compiler line: `RR Tachyon v1.3.0`.

Use this checklist after meaningful compiler changes to verify that the code still matches [`CONTRIBUTING.md`](https://github.com/Feralthedogg/RR/blob/main/CONTRIBUTING.md).

## Scope

This page is the post-change verification contract. It complements `CONTRIBUTING.md`; it does not replace it.

## Current Status

Automation baseline as of `2026-04-04`:

- Static audit enforces deterministic-path hazards such as `panic!`, `unwrap()`, `dbg!`, structured comment prefixes, `unsafe` safety notes, `static mut`, mutable globals, wall-clock access, cwd and temp path sensitivity, and hash-order review heuristics.
- Semantic smoke lanes cover cache correctness, determinism, numeric semantics, fallback correctness, pass verification, and fuzz smoke.
- CI now runs diff-scoped static audit, full-scope semantic audit, subsystem-specific matrices, commit-series/process gates, owner-review checks, required-check contract validation, failure bundles, perf governance reporting, and base-ref perf delta checks.

## Fast Audit

Run these commands from the repository root:

```bash
perl scripts/contributing_audit.pl
cargo check
cargo clippy --all-targets -- -D warnings
bash scripts/test_tier.sh tier0
bash scripts/test_tier.sh tier1
bash scripts/optimizer_suite.sh legality
FUZZ_SECONDS=1 ./scripts/fuzz_smoke.sh
```

`perl scripts/contributing_audit.pl` also runs `RR_VERIFY_EACH_PASS=1 cargo test -q --test pass_verify_examples` when the scanned scope touches pass-sensitive compiler files.

For a quick heuristic-only pass without running cargo or fuzz:

```bash
perl scripts/contributing_audit.pl --scan-only
```

For a strict clean-checkout-style pass:

```bash
scripts/verify_cleanroom.sh
scripts/verify_cleanroom.sh --files src/syntax/parse.rs tests/statement_boundaries.rs
scripts/verify_cleanroom.sh --fast --files scripts/verify_cleanroom.sh
```

Semantic smoke lanes triggered by non-scan audits:

- incremental/cache correctness: `incremental_phase1`, `incremental_phase2`, `incremental_phase3`, `incremental_auto`, `incremental_strict_verify`, `cli_incremental_default`
- numeric semantics and optimizer meaning: `sccp_overflow_regression`, `rr_logic_equivalence_matrix`, `opt_level_equivalence`, `numeric_property_differential`
- fallback/runtime correctness: `hybrid_fallback`, `parallel_optional_fallback_semantics`, `native_optional_fallback`, `poly_vopt_fallback`, `runtime_semantics_regression`, `fallback_correctness_matrix`
- determinism and environment independence: `commercial_determinism`, `compiler_parallel_equivalence`, `hermetic_determinism`, and dual-seed `random_differential`

Use `--skip-semantic-smoke` only when wiring the audit itself and you want to avoid re-running meaning-preservation suites.

## When To Do More

Recommended extended checks:

```bash
bash scripts/optimizer_suite.sh heavy
bash scripts/perf_gate.sh
perl scripts/perf_governance.pl
perl scripts/perf_delta_gate.pl
FUZZ_SECONDS=5 ./scripts/fuzz_smoke.sh
cargo test -q --test example_perf_smoke -- --ignored --nocapture
```

## Manual Review Checklist

- Confirm externally visible behavior is still deterministic anywhere ordering, hashing, or parallel scheduling could matter.
- Confirm touched semantic areas such as cache behavior, fallback behavior, numeric semantics, and IR invariants still match intent.
- Confirm hot paths did not pick up hidden allocation, formatting, clone cost, or other non-obvious work beyond what automation can prove.
- Confirm `unsafe`, mutable global state, wall-clock access, cwd access, and temp-path usage cannot change compilation correctness.
- Confirm docs, benchmark evidence, and any exception notes are still accurate rather than placeholder text.

## Ongoing Watch Items

- `src/mir/opt/v_opt/`: vector materialization and interop lowering remain structurally complex.
- `src/codegen/mir_emit.rs`: emitted-R construction is sensitive to hidden allocation regressions.
- `src/compiler/incremental.rs`: cache keys, strict verify, and source-map drift remain high-value failure points.

## Review Focus Areas

- `src/compiler/pipeline.rs`
- `src/compiler/incremental.rs`
- `src/mir/verify.rs`
- `src/mir/opt.rs`
- `src/mir/opt/`
- `src/codegen/mir_emit.rs`
- `src/runtime/mod.rs`

## Notes

- `cargo test -q` is the baseline gate, not proof of correctness.
- `--skip-semantic-smoke` exists only for audit-wiring work.
- Use `docs/compiler/testing.md` for suite selection details and `CONTRIBUTING.md` for rule intent.
