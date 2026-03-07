# Getting Started

This page walks through the shortest path to a working RR compile and run.

## Prerequisites

- Rust toolchain with `cargo`
- `Rscript` in `PATH` if you want to execute generated programs

## Build RR

```bash
cargo build
```

Confirm the installed compiler line:

```bash
target/debug/RR --version
```

You can invoke RR either way:

```bash
target/debug/RR --help
```

```bash
cargo run -- --help
```

Current expected output:

```text
RR Tachyon v4.0.0
```

## First Program

Create `main.rr`:

```r
main <- function() {
  x <- 1 + 2
  print(x)
  x
}

print(main())
```

Run it from the current directory:

```bash
cargo run -- run . -O1
```

`run .` resolves the current directory to `main.rr`.

## Compile to R

Compile one file into a standalone `.R` script:

```bash
cargo run -- main.rr -o main.R -O2
```

Compile helper-only output without source/native bootstrap:

```bash
cargo run -- main.rr -o main.R --no-runtime -O2
```

Use `--no-runtime` when you want helper-only emission for inspection or testing. It keeps helper definitions and runtime settings, but omits source/native bootstrap.

## Build a Directory

Compile all `.rr` files under a directory:

```bash
cargo run -- build . --out-dir build -O2
```

Behavior:

- recursively scans for `.rr` files
- skips `build/`, `target/`, and `.git/`
- writes mirrored output paths under the output directory

## Watch Mode

Recompile on changes:

```bash
cargo run -- watch . -O2
```

Useful flags:

- `--once`: run a single watch tick and exit
- `--poll-ms <N>`: control polling interval
- `--incremental=all`: enable all incremental compile phases

## Syntax Snapshot

RR accepts both R-style and native-style surface forms.

- assignment:
  - `x <- 1`
  - `x = 1`
- functions:
  - `name <- function(a, b) { a + b }`
  - `fn add(a, b) = a + b`
- loops:
  - `for (i in 1..n) s <- s + i`
  - `for i in 1..n { s += i }`
- type hints:
  - `fn add(a: float, b: int) -> float = a + b`
  - `x: int = 10L`
  - `vector<float>`, `matrix<float>`, `option<int>`

Recommended user-facing style is the R-like form unless you have a project reason to prefer the native style.

Builtin naming rule:

- use distinct helper names such as `demo_abs` or `my_sqrt` for user-defined math helpers
- only `length`, `floor`, `round`, `ceiling`, and `trunc` are intended to shadow builtin names

## Next Reading

- [CLI Reference](cli.md)
- [Configuration](configuration.md)
- [Language Reference](language.md)
- [Compiler Pipeline](compiler-pipeline.md)
