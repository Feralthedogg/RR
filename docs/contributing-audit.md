# Contributing Audit Checklist

Current compiler line: `RR Tachyon v4.0.0`.

Use this checklist after meaningful compiler changes to verify that the code still matches [`CONTRIBUTING.md`](https://github.com/Feralthedogg/RR/blob/main/CONTRIBUTING.md).

## Fast Audit

Run these commands from the repository root:

```bash
cargo check
cargo clippy --all-targets -- -D warnings
cargo test -q
FUZZ_SECONDS=1 ./scripts/fuzz_smoke.sh
```

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

- No new production `panic!`, `unwrap()`, or `expect()` on normal compiler paths.
- No new `unsafe` in `src/**` without adjacent `// SAFETY:` rationale.
- Deterministic traversal is preserved where output order matters.
- Hot loops do not add hidden allocation, path work, regex compilation, or avoidable cloning.
- Compiler faults use ICE/internal-error paths, not user-facing diagnostics.
- New emitted R behavior is covered by regression tests.
- Incremental cache keys change when output mode or compiler/runtime salt changes.
- `--no-runtime` behavior stays aligned with CLI/docs wording.
- Native backend resolution stays anchored to the intended project root.

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
