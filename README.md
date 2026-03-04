![RR-Logo](./image/RR_banner.png)

RR is an R-oriented compiler written in Rust.
It compiles `.rr` sources into self-contained `.R` output, with a MIR-based optimizer (`Tachyon Engine`) and runtime safety guards.

## Architecture

- CLI entry: `src/main.rs`
- Compiler pipeline core: `src/compiler/pipeline.rs`
- Frontend: `src/syntax`, `src/hir`
- Core IR + optimization: `src/mir`
- Backend emission: `src/codegen/mir_emit.rs`
- Runtime and execution: `src/runtime`
- Legacy/experimental path (not production): `src/legacy`

Surface syntax supports both styles:

- R style: `x <- 1`, `add <- function(a, b) { a + b }`, `for (i in 1..n) ...`
- Native style: `x = 1`, `fn add(a, b) = a + b`, `for i in 1..n { x += i }`

## Quick Start

### Prerequisites

- Rust toolchain (`cargo`)
- `Rscript` (required for `run` command and runtime execution)

### Build

```bash
cargo build
```

### Run a project (`main.rr`)

```bash
cargo run -- run . -O2
```

### Compile one file to R

```bash
cargo run -- path/to/input.rr -o out.R --no-runtime -O2
```

### Build all `.rr` files under a directory

```bash
cargo run -- build . --out-dir build -O2
```

## CLI Summary

- `RR <input.rr> [options]`
- `RR run [main.rr|dir|.] [options]`
- `RR build [dir|file.rr] [options]`

Options:

- `-o <file>` (legacy output file mode)
- `--out-dir <dir>` (`build` command output root)
- `-O0 | -O1 | -O2` (also accepts `-o0/-o1/-o2`)
- `--type-mode strict|gradual` (default `strict`)
- `--native-backend off|optional|required` (default `off`)
- `--parallel-mode off|optional|required` (default `off`)
- `--parallel-backend auto|r|openmp` (default `auto`)
- `--parallel-threads <N>` (default `0`, auto-detect)
- `--parallel-min-trip <N>` (default `4096`)
- `--keep-r` (keep generated `.gen.R` after `run`)
- `--no-runtime` (compile only)

Environment knobs:

- `RR_STRICT_LET=1` disallow implicit declarations through assignment
- `RR_WARN_IMPLICIT_DECL=1` print warnings for implicit declarations
- `RR_TYPE_MODE=strict|gradual` type checking policy
- `RR_NATIVE_BACKEND=off|optional|required` intrinsic backend policy
- `RR_NATIVE_LIB=/path/to/librr_native.{so,dylib}` optional native intrinsic library
- `RR_PARALLEL_MODE=off|optional|required` parallel execution policy
- `RR_PARALLEL_BACKEND=auto|r|openmp` parallel backend selector
- `RR_PARALLEL_THREADS=<N>` parallel worker threads (`0` = auto)
- `RR_PARALLEL_MIN_TRIP=<N>` minimum vector length to attempt parallel path

## Tests

Run all tests:

```bash
cargo test -q
```

Golden tests compare RR output against R execution (`tests/golden`).
If `Rscript` is unavailable, those tests are skipped.

Additional semantic-differential coverage:

- `tests/rr_logic_equivalence_matrix.rs`
  - validates RR vs reference R logic on multiple hand-written programs
  - matrix axes: `-O0/-O1/-O2` x `strict/gradual` x `off/optional`
  - compares `exit code`, `stdout`, and `stderr`
- `tests/sccp_overflow_regression.rs`
  - guards against optimizer-stage integer overflow panics in `-O2` path
  - verifies unsafe folds stay as runtime expressions

## Performance Gate

`tests/perf_regression_gate.rs` enforces compile-time budgets:

- `RR_PERF_GATE_MS` (default `12000`)
- `RR_PERF_O2_O1_RATIO` (default `12`)

## Fuzzing

Targets:

- `parser`
- `pipeline`
- `type_solver`

Run:

```bash
cargo install cargo-fuzz --locked
cargo +nightly fuzz run parser fuzz/corpus/parser -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
cargo +nightly fuzz run pipeline fuzz/corpus/pipeline -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
cargo +nightly fuzz run type_solver fuzz/corpus/type_solver -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
```

## Documentation

Full documentation is in [docs](./docs/index.md).

## License
[MIT](LICENSE)
