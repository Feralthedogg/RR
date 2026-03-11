# Contributing Audit Checklist

Current compiler line: `RR Tachyon v5.0.0`.

Use this checklist after meaningful compiler changes to verify that the code still matches [`CONTRIBUTING.md`](https://github.com/Feralthedogg/RR/blob/main/CONTRIBUTING.md).

## Current Status

Manual audit status as of `2026-03-08`:

- No open `MUST` violations were found on active production paths under `src/**`.
- Baseline verification passed:
  - `cargo check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test -q`
  - `FUZZ_SECONDS=1 ./scripts/fuzz_smoke.sh`
- Extended validation has also been exercised recently:
  - deterministic `O0/O1/O2` differential tests
  - pass-by-pass verifier smoke
  - nightly soak triage/promotion pipeline

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
cargo test -q
FUZZ_SECONDS=1 ./scripts/fuzz_smoke.sh
```

For a quick heuristic-only pass over the current worktree without running cargo/fuzz:

```bash
scripts/contributing_audit.sh --scan-only
```

For a strict clean-checkout style pass that removes ambiguity from an already
dirty worktree, run:

```bash
scripts/verify_cleanroom.sh
scripts/verify_cleanroom.sh --files src/mir/opt/v_opt.rs tests/vectorization_phi_ifelse.rs
scripts/verify_cleanroom.sh --fast --files scripts/verify_cleanroom.sh
```

`verify_cleanroom.sh` creates a detached worktree at `HEAD`, overlays only the
selected current-tree files, then runs `fmt`, `check`, `clippy`, the full test
suite, pass-by-pass verifier smoke, the contributing audit, fuzz smoke, and the
docs build in that clean environment. Use `--files` whenever unrelated dirty
changes are present in the source worktree. Use `--fast` when you want to verify
the cleanroom wiring itself before paying for the full strict stack.

CI runs the same audit in `--scan-only` mode against the diff for each PR/push.
It does not run `--all` yet because the repository still has historical whole-tree
debt that is being paid down incrementally.

## When To Do More

Run a deeper pass when you change any of these:

- parser, lowering, MIR verification
- optimizer passes or vectorization
- runtime injection or emitted helper behavior
- incremental compile or cache logic
- benchmark or perf-gated example paths

Recommended extended checks:

```bash
FUZZ_SECONDS=5 ./scripts/fuzz_smoke.sh
cargo test -q --test example_perf_smoke -- --ignored --nocapture
```

## Manual Review Checklist

- `scripts/contributing_audit.sh` reports no static `error[...]` findings on the intended scope.
- No new production `panic!`, `unwrap()`, or `expect()` on normal compiler paths.
- No new `unsafe` in `src/**` without adjacent `// SAFETY:` rationale.
- Deterministic traversal is preserved where output order matters.
- Hot loops do not add hidden allocation, path work, regex compilation, or avoidable cloning.
- Compiler faults use ICE/internal-error paths, not user-facing diagnostics.
- New emitted R behavior is covered by regression tests.
- Incremental cache keys change when output mode or compiler/runtime salt changes.
- `--no-runtime` behavior stays aligned with CLI/docs wording and selective-helper injection.
- Native backend resolution stays anchored to the intended project root.

## Ongoing Watch Items

These are not current rule violations, but they are the first places likely to regress if new work lands quickly:

- `src/mir/opt/v_opt.rs`: vector materialization and interop lowering remain structurally complex even after recent splits.
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
