# Getting Started

This page is the shortest path to a working RR compile and run.

## Audience

Read this page if you want to:

- build RR locally
- compile one `.rr` file into `.R`
- run a small RR program end to end
- understand which command to use next

If you already know the driver shape, jump to [CLI Reference](cli.md).

## Requirements

- Rust toolchain with `cargo`
- `Rscript` in `PATH` if you want to execute generated `.R`

## Fast Path

Build the compiler:

```bash
cargo build
```

Check the compiler line:

```bash
target/debug/RR --version
```

Compile one file:

```bash
target/debug/RR main.rr -o main.R -O2
```

Run the generated artifact:

```bash
Rscript --vanilla main.R
```

## First Program

Create `main.rr`:

```rr
let main <- function() {
  let x <- 1 + 2
  print(x)
  x
}

print(main())
```

Compile and run:

```bash
target/debug/RR main.rr -o main.R -O1
Rscript --vanilla main.R
```

Expected output:

```text
[1] 3
[1] 3
```

## Driver Forms

RR supports three common workflows:

### Compile One File

```bash
RR input.rr -o out.R -O2
```

Use this when you want one emitted artifact.

### Run a Project Entry

```bash
RR run . -O2
```

`run .` resolves the current directory to `main.rr`.

### Build a Tree

```bash
RR build . --out-dir build -O2
```

Use this when you want every `.rr` file under a directory compiled into a
mirrored output tree.

## What RR Emits

RR emits ordinary `.R` files.

Normal output:

- includes the runtime helper subset actually referenced by the emitted code
- includes runtime bootstrap and compile-time policy assignments
- is intended to run directly under `Rscript`

Helper-only output:

```bash
RR main.rr -o main.R --no-runtime -O2
```

Helper-only output:

- omits source/native bootstrap
- still injects the helper subset needed by the emitted program
- is useful for inspection, tests, and backend debugging

## Watch and Incremental Compile

Watch mode:

```bash
RR watch . -O2
```

Useful flags:

- `--once`
- `--poll-ms <N>`
- `--no-incremental`
- `--incremental=auto|off|1|1,2|1,2,3|all`

RR defaults to incremental `auto`, so normal compiles already reuse phase 1 and
phase 2 caches when possible.

## Recommended Reading Order

1. [CLI Reference](cli.md)
2. [Language Reference](language.md)
3. [Writing RR for Performance and Safety](writing-rr.md)
4. [Configuration](configuration.md)

If you want to understand emission and optimization next:

1. [Compiler Pipeline](compiler-pipeline.md)
2. [Tachyon Engine](optimization.md)
3. [Runtime and Error Model](runtime-and-errors.md)
