# Contributing Audit Checklist

Current compiler line: `RR Tachyon v9.0.0`.

Use this checklist after meaningful compiler changes to verify that the code still matches [`CONTRIBUTING.md`](https://github.com/Feralthedogg/RR/blob/main/CONTRIBUTING.md).

## Scope

This page is the post-change verification contract, not a replacement for
`CONTRIBUTING.md`. Use it after changes land in your worktree and before you
trust a result enough to ship or merge it.

## Current Status

Manual audit status as of `2026-03-31`:

- No open `MUST` violations were found on active production paths under `src/**`.
- Baseline verification passed:
  - `cargo check`
  - `cargo clippy --all-targets -- -D warnings`
  - `bash scripts/test_tier.sh tier0`
  - `bash scripts/test_tier.sh tier1`
  - `bash scripts/optimizer_suite.sh legality`
  - `FUZZ_SECONDS=1 ./scripts/fuzz_smoke.sh`
- Extended validation has also been exercised recently:
  - deterministic `O0/O1/O2` differential tests
  - optimizer legality/heavy suites
  - library package suite
  - performance gate and example perf smoke
  - pass-by-pass verifier smoke
  - nightly soak triage/promotion pipeline
  - recommended-package coverage reporting

Recent items that were explicitly closed during audit/refactor work:

- production-path `panic!/unwrap()/expect()` cleanup on active and legacy compiler paths
- hot-path `HashMap/HashSet` cleanup in analysis/codegen scratch paths
- vectorization apply-path decomposition
- origin-phi materialization decomposition
- emitted R/runtime option consistency checks
- incremental cache key and verification correctness fixes

## Fast Audit

Run these commands from the repository root:

```bash
scripts/contributing_audit.sh
cargo check
cargo clippy --all-targets -- -D warnings
bash scripts/test_tier.sh tier0
bash scripts/test_tier.sh tier1
bash scripts/optimizer_suite.sh legality
FUZZ_SECONDS=1 ./scripts/fuzz_smoke.sh
```

`scripts/contributing_audit.sh` also runs
`RR_VERIFY_EACH_PASS=1 cargo test -q --test pass_verify_examples` when the
scanned scope touches pass-sensitive compiler files such as HIR/MIR/pipeline,
incremental, or `src/codegen/mir_emit.rs`. Use `--skip-pass-verify` when you
are only wiring the audit script itself and want to avoid that extra smoke step.

For a quick heuristic-only pass over the current worktree without running cargo/fuzz:

```bash
scripts/contributing_audit.sh --scan-only
```

For a strict clean-checkout style pass that removes ambiguity from an already
dirty worktree, run:

```bash
scripts/verify_cleanroom.sh
scripts/verify_cleanroom.sh --files src/mir/opt/v_opt/mod.rs src/mir/opt/v_opt/planning.rs src/mir/opt/v_opt/reconstruct.rs src/mir/opt/v_opt/transform.rs tests/vectorization_phi_ifelse.rs
scripts/verify_cleanroom.sh --fast --files scripts/verify_cleanroom.sh
```

`verify_cleanroom.sh` creates a detached worktree at `HEAD`, overlays only the
selected current-tree files, then runs `fmt`, `check`, `clippy`, the tiered test
stack, pass-by-pass verifier smoke, the contributing audit, fuzz smoke, and the
docs build in that clean environment. Use `--files` whenever unrelated dirty
changes are present in the source worktree. Use `--fast` when you want to verify
the cleanroom wiring itself before paying for the full strict stack.

CI is now tiered rather than relying on one monolithic compiler job:

- `tier0` fast gates
- `tier1` library/package coverage
- optimizer legality / heavy gates
- perf gate
- `tier2` runtime/example/differential lanes
- nightly soak for fuzz, triage, and recommended-package coverage

## When To Do More

Run a deeper pass when you change any of these:

- parser, lowering, MIR verification
- optimizer passes or vectorization
- runtime injection or emitted helper behavior
- incremental compile or cache logic
- benchmark or perf-gated example paths

Recommended extended checks:

```bash
bash scripts/optimizer_suite.sh heavy
bash scripts/perf_gate.sh
FUZZ_SECONDS=5 ./scripts/fuzz_smoke.sh
cargo test -q --test example_perf_smoke -- --ignored --nocapture
```

## Manual Review Checklist

- `scripts/contributing_audit.sh` reports no static `error[...]` findings on the intended scope.
- No new production `panic!`, `unwrap()`, or `expect()` on normal compiler paths.
- No new `unsafe` in `src/**` without adjacent `// SAFETY:` rationale and a
  clear explanation of why safe alternatives were insufficient.
- Deterministic traversal is preserved where output order matters, including
  stable tie-break rules when multiple valid orders exist.
- Hot loops do not add hidden allocation, path work, regex compilation,
  avoidable cloning, or hidden API cost.
- No mutable global state now affects compilation results.
- Compilation results do not depend on wall-clock time, system randomness, or
  environment-specific paths without explicit normalization.
- IR invariants still hold across touched stages, and relevant `validate_*`/verifier hooks were exercised.
- Persisted IR/debug dumps have explicit compatibility expectations when IR
  structure changes.
- Long-lived compiler data does not add avoidable copy churn when interning or arena allocation would be the clearer cost model.
- Compiler faults use ICE/internal-error paths, not user-facing diagnostics.
- Expected failure paths do not silently fall back in ways that mask
  correctness or suppress diagnostics.
- Constant folding and evaluator helpers still follow RR numeric and overflow semantics rather than host-language accident.
- New emitted R behavior is covered by regression tests.
- Incremental cache keys change when output mode or compiler/runtime salt
  changes, and any new cache key material captures all correctness-affecting
  inputs.
- Persisted cache keys include compiler version/build identity and
  semantic-affecting flags when relevant.
- `--no-runtime` behavior stays aligned with CLI/docs wording and selective-helper injection.
- Native backend resolution stays anchored to the intended project root.
- New dependencies with material performance, portability, or determinism cost
  were explicitly approved.
- Public APIs and non-obvious transform entrypoints document their design intent when local code shape is insufficient for review.

## Ongoing Watch Items

These are not current rule violations, but they are the first places likely to regress if new work lands quickly:

- `src/mir/opt/v_opt/`: vector materialization and interop lowering remain structurally complex even after recent splits.
- `src/codegen/mir_emit.rs`: emitted R construction is sensitive to hidden allocation regressions.
- `src/mir/analyze/range.rs`: analysis precision and hot-loop cost need to stay balanced.
- `tests/common/random_rr.rs` and differential harnesses: generator growth should stay deterministic and easy to shrink.

## Review Focus Areas

If you are reviewing RR-to-R compilation changes, inspect these first:

- `src/compiler/pipeline.rs`
- `src/compiler/incremental.rs`
- `src/mir/verify.rs`
- `src/mir/opt.rs`
- `src/mir/opt/`
- `src/codegen/mir_emit.rs`
- `src/runtime/mod.rs`

## Notes

- `cargo test -q` is the baseline gate, not proof of correctness.
- Fuzz smoke is intended to catch fast regressions; longer fuzz runs are still valuable before release work.
- The nightly verification pipeline is the stronger validation tier for:
  - fuzz crash triage
  - differential mismatch triage
  - pass-verify failure triage
  - promote-ready regression candidate generation
