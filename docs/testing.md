# Testing and Quality Gates

RR uses unit tests, integration tests, golden tests, performance gates, and fuzzing.

## Prerequisites

Most Rust-only tests need only `cargo`.
Tests that execute generated R require `Rscript` in `PATH`.

## Common Commands

Run all tests:

```bash
cargo test -q
```

Run one integration suite:

```bash
cargo test -q --test vectorization_extended
```

Run lints:

```bash
cargo clippy --all-targets -- -D warnings
```

## Test Families

### Frontend and Syntax

- `syntax_errors.rs`
- `parse_multi_errors.rs`
- `semicolon_required.rs`

These cover parsing, error recovery, and syntax diagnostics.

### Semantic and Runtime Static Validation

- `semantic_errors.rs`
- `runtime_static_errors.rs`
- `multi_errors.rs`
- `commercial_negative_corpus.rs`

These focus on compile-time rejection and aggregated diagnostics.

### Language and Lowering

- `support_expansion.rs`
- `lambda_closure.rs`
- `mir_lowering_loop_match.rs`

These verify that accepted language forms lower correctly into MIR/codegen.

### Optimization Correctness

- `vectorization_extended.rs`
- `vectorization_phi_ifelse.rs`
- `benchmark_vectorization.rs`
- `bce_shifted_index.rs`
- `sccp_overflow_regression.rs`
- `opt_level_equivalence.rs`
- `r_output_optimization_audit.rs`
- `rr_logic_equivalence_matrix.rs`

These guard optimizer semantics, emitted R shape, and no-panic behavior under aggressive optimization.

### CLI and Execution Behavior

- `cli_commands.rs`
- `parallel_cli_flags.rs`
- `parallel_optional_fallback_semantics.rs`

These cover command wiring, mode flags, and backend fallback behavior.

### Stress and Determinism

- `commercial_determinism.rs`
- `commercial_stress_differential.rs`

These exercise larger workloads and determinism-sensitive paths.

### Performance Gate

- `perf_regression_gate.rs`

This enforces compile-time budget expectations for optimized builds.

### Example Catalog and Bench Workloads

- `example/data_science/*.rr`
- `example/physics/*.rr`
- `tests/example_simulations.rs`
- `example/benchmarks/*.rr`
- `tests/benchmark_examples_smoke.rs`

The simulation catalog is compiled across optimization levels and executed at `-O2`.
The benchmark catalog is intended for repeatable compile-time and runtime comparisons.

Benchmark runner:

```bash
scripts/bench_examples.sh
```

## Golden Semantics

`tests/golden.rs` compares RR-compiled output against reference R behavior for cases in `tests/golden/`.

Requirements:

- `Rscript` available in `PATH`

If `Rscript` is unavailable, golden tests skip automatically.

Promoted example-derived golden cases include:

- `bootstrap_mean`
- `logistic_ensemble`
- `markov_weather_chain`
- `monte_carlo_pi`
- `sir_epidemic`

## RR vs R Differential Matrix

`tests/rr_logic_equivalence_matrix.rs` compares hand-written RR programs against reference R scripts across:

- optimization level: `-O0`, `-O1`, `-O2`
- type mode: `strict`, `gradual`
- native backend mode: `off`, `optional`

Compared outputs:

- process exit code
- normalized stdout
- normalized stderr

## Performance Knobs

`tests/perf_regression_gate.rs` uses:

- `RR_PERF_GATE_MS` (default `12000`)
- `RR_PERF_O2_O1_RATIO` (default `12`)

Adjust these only when you intentionally re-baseline compile-time expectations.

## Fuzzing

Targets:

- `fuzz/fuzz_targets/parser.rs`
- `fuzz/fuzz_targets/pipeline.rs`
- `fuzz/fuzz_targets/type_solver.rs`
- `fuzz/fuzz_targets/incremental.rs` (`incremental_compile`)

Dictionary:

- `fuzz/dictionaries/rr.dict`

Run:

```bash
cargo install cargo-fuzz --locked
cargo +nightly fuzz run parser fuzz/corpus/parser -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
cargo +nightly fuzz run pipeline fuzz/corpus/pipeline -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
cargo +nightly fuzz run type_solver fuzz/corpus/type_solver -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
cargo +nightly fuzz run incremental_compile fuzz/corpus/incremental_compile -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
```

Smoke runner:

```bash
scripts/fuzz_smoke.sh
```

By default this uses `fuzz/corpus_smoke/*` so that smoke runs stay bounded.
Set `FUZZ_CORPUS_ROOT=fuzz/corpus` when you want to run against the full corpus set.

Replay a pipeline crash artifact:

```bash
cargo +nightly fuzz run pipeline fuzz/artifacts/pipeline/crash-<hash> -- -runs=1 -rss_limit_mb=2048
cargo +nightly fuzz tmin pipeline fuzz/artifacts/pipeline/crash-<hash>
```

## CI Expectations

The CI workflow is expected to run:

- Rust test suite
- R-backed integration coverage
- linting
- fuzz smoke coverage for key targets

For compiler changes, targeted regression tests are preferred over broad snapshot updates.
