![RR-Logo](./image/RR_banner.png)

RR is an optimizing compiler for an R-oriented source language.
It compiles `.rr` programs into self-contained `.R` output, using an SSA-like MIR pipeline,
the `Tachyon` optimizer, and an embedded runtime for checks and helper operations.
Current compiler line: `RR Tachyon v4.0.0`.

## What RR Does

- accepts R-first syntax such as `<-`, `function(...)`, dotted identifiers, and `1..n`
- lowers through HIR and MIR rather than directly emitting R
- optimizes selected loops and expressions at MIR level
- emits standalone `.R` files that can run through `Rscript`

## Quick Start

### Prerequisites

- Rust toolchain (`cargo`)
- `Rscript` in `PATH` if you want to execute generated programs

### Build

```bash
cargo build
```

Use the binary directly:

```bash
target/debug/RR --help
```

Or through Cargo:

```bash
cargo run -- --help
```

Print the compiler version:

```bash
target/debug/RR --version
```

### Compile a file to R

```bash
cargo run -- path/to/input.rr -o out.R -O2
```

Compile without embedding the runtime:

```bash
cargo run -- path/to/input.rr -o out.R --no-runtime -O2
```

### Run a project

```bash
cargo run -- run . -O2
```

`run` resolves `.` or a directory to `main.rr`.

### Build a directory tree

```bash
cargo run -- build . --out-dir build -O2
```

### Watch and recompile

```bash
cargo run -- watch . -O2
```

## Minimal Example

```r
main <- function() {
  x <- 1 + 2
  print(x)
  x
}

print(main())
```

## CLI At a Glance

- `RR <input.rr> [options]`
- `RR --version`
- `RR version`
- `RR run [main.rr|dir|.] [options]`
- `RR build [dir|file.rr] [options]`
- `RR watch [main.rr|dir|.] [options]`

Common options:

- `-O0 | -O1 | -O2`
- `-o <file>`
- `--out-dir <dir>`
- `--no-runtime`
  - omit source/native bootstrap while keeping helper definitions and runtime settings
- `--type-mode strict|gradual`
- `--native-backend off|optional|required`
- `--parallel-mode off|optional|required`
- `--parallel-backend auto|r|openmp`
- `--incremental[=off|1|1,2|1,2,3|all]`

## Project Layout

- CLI entry: `src/main.rs`
- compiler pipeline: `src/compiler/pipeline.rs`
- frontend: `src/syntax`, `src/hir`
- MIR and optimization: `src/mir`
- R emission: `src/codegen/mir_emit.rs`
- runtime and execution: `src/runtime`
- legacy/experimental path: `src/legacy`

## Testing

Run the full Rust test suite:

```bash
cargo test -q
```

Representative coverage includes:

- parser and syntax recovery
- semantic/runtime static validation
- optimizer regressions
- promoted golden semantics cases from `tests/golden/`
- simulation catalog coverage in `example/data_science/` and `example/physics/`
- benchmark workload smoke coverage in `example/benchmarks/`
- RR vs R differential tests
- generated `-O0/-O1/-O2` differential tests against reference R
- per-pass MIR verification smoke with `RR_VERIFY_EACH_PASS=1`
- generator-driven valid-program fuzzing for optimizer/codegen stability
- compile-time performance gates
- fuzz targets for parser, pipeline, type solver, and incremental compilation

If `Rscript` is unavailable, R-dependent integration tests skip automatically.

Benchmark helper:

```bash
scripts/bench_examples.sh
```

Optimizer validation helpers:

```bash
cargo test -q --test random_differential
RR_RANDOM_DIFFERENTIAL_COUNT=12 RR_RANDOM_DIFFERENTIAL_SEED=0xA11CE5EED55AA11C cargo test -q --test random_differential
RR_RANDOM_DIFFERENTIAL_COUNT=12 RR_RANDOM_DIFFERENTIAL_SEED=0x5EED123456789ABC cargo test -q --test random_differential
RR_VERIFY_EACH_PASS=1 cargo test -q --test pass_verify_examples
RR_RANDOM_DIFFERENTIAL_COUNT=72 cargo test -q --test random_differential -- --nocapture
```

The generated differential harness covers loops, recurrences, matrices,
records, and direct `stats`/`base` namespace interop cases.

## Fuzzing

```bash
cargo install cargo-fuzz --locked
cargo +nightly fuzz run parser fuzz/corpus/parser -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
cargo +nightly fuzz run pipeline fuzz/corpus/pipeline -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
cargo +nightly fuzz run type_solver fuzz/corpus/type_solver -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
cargo +nightly fuzz run incremental_compile fuzz/corpus/incremental_compile -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
cargo +nightly fuzz run generated_pipeline fuzz/corpus/generated_pipeline -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
```

Smoke runner:

```bash
scripts/fuzz_smoke.sh
```

Default smoke runs use the small `fuzz/corpus_smoke/` sets.
Use `FUZZ_CORPUS_ROOT=fuzz/corpus` to exercise the full corpus.
Longer soak coverage runs in `.github/workflows/nightly-soak.yml`.
The default CI differential job now runs two deterministic seed slices:

- `RR_RANDOM_DIFFERENTIAL_COUNT=12`, `RR_RANDOM_DIFFERENTIAL_SEED=0xA11CE5EED55AA11C`
- `RR_RANDOM_DIFFERENTIAL_COUNT=12`, `RR_RANDOM_DIFFERENTIAL_SEED=0x5EED123456789ABC`

Nightly soak uploads differential logs plus any fuzz crash artifacts so failures
can be reproduced locally. It also uploads a triage bundle with minimized crash
inputs and replay commands, plus verifier dumps from per-pass verification.
Differential mismatches also persist repro bundles under
`target/tests/random_differential_failures/`, and nightly differential triage
turns those bundles into summary tables plus regression skeletons under
`.artifacts/differential-triage/` and smoke-tests those generated regressions.
Per-pass verifier failures from `pass_verify_examples` likewise persist under
`target/tests/pass_verify_failures/` and are triaged into
`.artifacts/pass-verify-triage/` with the same nightly smoke pass. Fuzz triage
also smoke-tests generated text regressions. These triage bundles now carry
machine-readable `bundle.manifest` files, and the shared
`scripts/triage_driver.sh` validates those manifests before triage/promote
automation proceeds. Triage outputs now also include `summary.json`, and nightly
aggregates differential/pass-verify/fuzz triage into a single
`.artifacts/nightly-soak/verification-summary.json` file so verification
statistics can be consumed without scraping markdown. The paired
`verification-summary.md` also lists promote-ready regression candidates when a
bundle already has a smoke-tested `regression.rs` skeleton. Nightly also emits
`top-promotion-candidates.json` / `.md` for downstream automation that only
needs the highest-priority promote targets.

## Documentation

- docs landing page: [docs/index.md](./docs/index.md)
- getting started: [docs/getting-started.md](./docs/getting-started.md)
- compiler internals: [docs/compiler-pipeline.md](./docs/compiler-pipeline.md)
- optimizer details: [docs/optimization.md](./docs/optimization.md)

## Contributing

Contributor rules and code style are in [CONTRIBUTING.md](./CONTRIBUTING.md).

## License

[MIT](LICENSE)
