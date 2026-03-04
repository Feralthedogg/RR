# Testing and Quality Gates

RR includes unit/integration/golden/perf/fuzz coverage.

## Run All Tests

```bash
cargo test -q
```

## Test Families

Representative integration suites under `tests/`:

- syntax and parser recovery:
  - `syntax_errors.rs`
  - `parse_multi_errors.rs`
  - `semicolon_required.rs`
- semantic/runtime static validation:
  - `semantic_errors.rs`
  - `runtime_static_errors.rs`
  - `multi_errors.rs`
  - `commercial_negative_corpus.rs`
- language support:
  - `support_expansion.rs`
  - `lambda_closure.rs`
  - `mir_lowering_loop_match.rs`
- optimization correctness:
  - `vectorization_extended.rs`
  - `benchmark_vectorization.rs`
  - `bce_shifted_index.rs`
  - `sccp_overflow_regression.rs`
  - `opt_level_equivalence.rs`
  - `r_output_optimization_audit.rs`
  - `rr_logic_equivalence_matrix.rs`
  - `parallel_codegen.rs`
  - `parallel_cli_flags.rs`
  - `parallel_optional_fallback_semantics.rs`
- CLI behavior:
  - `cli_commands.rs`
- stress and determinism:
  - `commercial_determinism.rs`
  - `commercial_stress_differential.rs`
- performance gate:
  - `perf_regression_gate.rs`

## Golden Semantics

`tests/golden.rs` compares RR-compiled output against R execution for `.rr` cases in `tests/golden/`.

Requirements:

- `Rscript` available in PATH

If unavailable, golden tests skip automatically.

## RR Logic Equivalence Matrix

`tests/rr_logic_equivalence_matrix.rs` validates semantic equivalence on multiple
hand-written RR programs by comparing against reference R scripts.

Matrix axes:

- optimization level: `-O0`, `-O1`, `-O2`
- type mode: `strict`, `gradual`
- native backend mode: `off`, `optional`

Comparison criteria:

- process exit code
- normalized stdout
- normalized stderr

`tests/r_output_optimization_audit.rs` adds stricter output-contract checks for:

- typed condition guard elision (`rr_truthy1` wrapper removal when proven)
- intrinsic helper emission (`rr_intrinsic_vec_*`) and native backend marker injection
- optional native backend fallback behavior under missing shared library
- O0 vs O2 index-guard wrapper count reduction (`rr_index1_read/write`)
- nested generic type-hint program equivalence against reference R

SCCP overflow hardening regression coverage:

- `tests/sccp_overflow_regression.rs`:
  - compiler-level no-panic checks for integer overflow expressions under `-O2`
  - verifies overflowing folds are left as runtime expressions when not provably safe
- `src/mir/opt/sccp.rs` unit tests:
  - `test_div_overflow_is_not_folded`
  - `test_range_len_overflow_is_not_folded`

## Fuzzing

Targets:

- `fuzz/fuzz_targets/parser.rs`
- `fuzz/fuzz_targets/pipeline.rs`
- `fuzz/fuzz_targets/type_solver.rs`

Dictionary:

- `fuzz/dictionaries/rr.dict`

Run:

```bash
cargo install cargo-fuzz --locked
cargo +nightly fuzz run parser fuzz/corpus/parser -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
cargo +nightly fuzz run pipeline fuzz/corpus/pipeline -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
cargo +nightly fuzz run type_solver fuzz/corpus/type_solver -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
```

Crash artifact replay (pipeline target):

```bash
cargo +nightly fuzz run pipeline fuzz/artifacts/pipeline/crash-<hash> -- -runs=1 -rss_limit_mb=2048
cargo +nightly fuzz tmin pipeline fuzz/artifacts/pipeline/crash-<hash>
```

## CI

GitHub Actions workflow `/.github/workflows/ci.yml` runs:

- full test job with R installed
- parser/pipeline/type_solver fuzz smoke job
