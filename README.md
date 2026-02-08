![RR-Logo](./image/RR_banner.png)

RR is an R-oriented compiler written in Rust.
It compiles `.rr` sources into self-contained `.R` output, with a MIR-based optimizer (`Tachyon Engine`) and runtime safety guards.

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
- `--keep-r` (keep generated `.gen.R` after `run`)
- `--no-runtime` (compile only)

## Tests

Run all tests:

```bash
cargo test -q
```

Golden tests compare RR output against R execution (`tests/golden`).
If `Rscript` is unavailable, those tests are skipped.

## Performance Gate

`tests/perf_regression_gate.rs` enforces compile-time budgets:

- `RR_PERF_GATE_MS` (default `12000`)
- `RR_PERF_O2_O1_RATIO` (default `12`)

## Fuzzing

Targets:

- `parser`
- `pipeline`

Run:

```bash
cargo install cargo-fuzz --locked
cargo fuzz run parser fuzz/corpus/parser -- -max_total_time=60
cargo fuzz run pipeline fuzz/corpus/pipeline -- -max_total_time=60
```

## Documentation

Full documentation is in [docs](./docs/README.md).

## License
[MIT](LICENSE)
