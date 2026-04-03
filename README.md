![RR-Logo](./image/RR_banner.png)

RR is an optimizing compiler for an R-oriented source language.
It compiles `.rr` programs into self-contained `.R` output, using an SSA-like MIR pipeline,
the `Tachyon` optimizer, and an embedded runtime for checks and helper operations.
Current compiler line: `RR Tachyon v8.0.0`.

## Documentation Sets

RR documentation is split into manual-style sets:

- Guide
  - [Getting Started](./docs/getting-started.md)
  - [Writing RR for Performance and Safety](./docs/writing-rr.md)
  - [CLI Reference](./docs/cli.md)
- Reference
  - [Language Reference](./docs/language.md)
  - [Configuration](./docs/configuration.md)
  - [R Interop](./docs/r-interop.md)
  - [Compatibility and Limits](./docs/compatibility.md)
- Compiler Dev Docs
  - [Overview](./docs/compiler/index.md)
  - [Contributing Audit Checklist](./docs/compiler/contributing-audit.md)

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

`run` resolves `.` or a directory to `src/main.rr` for managed projects, then
falls back to root `main.rr` for legacy projects. The entry must define
`fn main()`. If the source does not already call `main()` at top level, RR
appends that call automatically for `run`.

### Build a project

```bash
cargo run -- build . -O2
```

If the target directory contains `src/main.rr` or `main.rr`, `build` compiles
only that project entry into `Build/debug/` without executing it.

### Watch and recompile

```bash
cargo run -- watch . -O2
```

## Minimal Example

```r
let main <- function() {
  let x <- 1 + 2
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
- `--compiler-parallel-mode off|auto|on`
- `--compiler-parallel-max-jobs <N>`
- `--incremental[=auto|off|1|1,2|1,2,3|all]`
- `--no-incremental`

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
- generated invalid-program diagnostic validation
- optimizer regressions
- incremental auto/disk/session cache regressions
- promoted golden semantics cases from `tests/golden/`
- simulation catalog coverage in `example/data_science/` and `example/physics/`
- benchmark workload smoke coverage in `example/benchmarks/`
- RR vs R differential tests
- generated `-O0/-O1/-O2` differential tests against reference R
- per-pass MIR verification smoke with `RR_VERIFY_EACH_PASS=1`
- generator-driven valid-program fuzzing for optimizer/codegen stability
- compile-time performance gates
- fuzz targets for parser, pipeline, type solver, incremental compilation, generated valid-program pipelines, and error diagnostics

If `Rscript` is unavailable, R-dependent integration tests skip automatically.

Tiered local validation:

- `tier0`: fast compiler-only gates and unit/integration smoke
- `tier1`: library/package interop closure, direct surface, and type-precision regressions
- `optimizer-suite`: pass/vectorization/codegen legality and heavier optimizer equivalence checks
- `tier2-main`: heavier runtime/equivalence/example/perf-style suites
- `tier2-differential`: random differential plus per-pass verify

```bash
bash scripts/test_tier.sh tier0
bash scripts/test_tier.sh tier1
bash scripts/optimizer_suite.sh legality
bash scripts/optimizer_suite.sh heavy
bash scripts/test_tier.sh tier2-main
RR_RANDOM_DIFFERENTIAL_COUNT=12 RR_RANDOM_DIFFERENTIAL_SEED=0xA11CE5EED55AA11C bash scripts/test_tier.sh tier2-differential
```

Equivalent Make targets:

```bash
make test-tier0
make test-tier1
make optimizer-suite-legality
make optimizer-suite-heavy
make test-tier2-main
make test-tier2-differential
```

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

Library package regression suite:

```bash
make library-package-suite
```

To run only part of the package suite:

```bash
RR_PACKAGE_SUITE_FILTER=stats make library-package-suite
RR_PACKAGE_SUITE_FILTER=base make library-package-suite
```

Performance gate:

```bash
make perf-gate
RR_PERF_GATE_FILTER=perf_regression_gate bash scripts/perf_gate.sh
```

Recommended package coverage report:

```bash
make recommended-package-coverage
RR_RECOMMENDED_PACKAGES=MASS,Matrix,survival bash scripts/recommended_package_coverage.sh
```

Triage helpers for failing differential / pass-verify bundles:

```bash
scripts/triage_driver.sh triage differential
scripts/triage_driver.sh triage pass-verify
scripts/triage_driver.sh reduce differential .artifacts/differential-triage/<case-dir>
scripts/triage_driver.sh reduce pass-verify .artifacts/pass-verify-triage/<case-dir>
```

## Fuzzing

```bash
cargo install cargo-fuzz --locked
cargo +nightly fuzz run parser fuzz/corpus/parser -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
cargo +nightly fuzz run pipeline fuzz/corpus/pipeline -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
cargo +nightly fuzz run type_solver fuzz/corpus/type_solver -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
cargo +nightly fuzz run incremental_compile fuzz/corpus/incremental_compile -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
cargo +nightly fuzz run generated_pipeline fuzz/corpus/generated_pipeline -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
cargo +nightly fuzz run error_diagnostics fuzz/corpus/error_diagnostics -- -dict=fuzz/dictionaries/rr.dict -max_total_time=60
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
- compiler contributor docs: [docs/compiler/index.md](./docs/compiler/index.md)

## Contributing

Contributor rules and code style are in [CONTRIBUTING.md](./CONTRIBUTING.md).

## License

[MIT](LICENSE)
