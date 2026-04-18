# Getting Started

This page is the shortest path from a fresh checkout to a running RR program.

## What This Page Covers

Use this page when you want the user-facing path first:

- build RR locally
- compile one `.rr` file into `.R`
- run a small RR program end to end
- learn the next three commands most people use: `run`, `build`, and `watch`

This page intentionally keeps compiler structure out of the fast path. If you
already know the driver shape, jump to [CLI Reference](cli.md).

## Requirements

- Rust toolchain with `cargo`
- `Rscript` in `PATH` if you want to execute generated `.R`
- `corepack pnpm` only if you also want to build the docs site

## Five-Minute Quick Start

Build the compiler:

```bash
cargo build
```

Check the compiler line:

```bash
target/debug/RR --version
```

Create `main.rr`:

```rr
fn main() {
  let x = 1 + 2
  print(x)
  x
}
```

Compile it:

```bash
target/debug/RR main.rr -o main.R -O2
```

Run the emitted R:

```bash
Rscript --vanilla main.R
```

Expected output:

```text
[1] 3
[1] 3
```

## Common Commands

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
For project entry files, RR expects `fn main()` and automatically appends a
top-level `main()` call during `run` if the file does not already contain one.

### Build a Project

```bash
RR build . -O2
```

Use this when you want the project entry compiled into `Build/debug/` without
executing it.

## What RR Emits

RR emits ordinary `.R` files.

- includes the runtime helper subset actually referenced by the emitted code
- includes runtime bootstrap and compile-time policy defaults
- is intended to run directly under `Rscript`

For inspection or backend debugging, you can omit the normal runtime bootstrap:

```bash
RR main.rr -o main.R --no-runtime -O2
```

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
- `--cold`
- `--incremental=auto|off|1|1,2|1,2,3|all`

RR defaults to incremental `auto`, so normal compiles already reuse phase 1 and
phase 2 caches when possible.

If you want one compile to bypass the warm caches without clearing them, use
`--cold`.

`RR watch` also fingerprints the full imported module tree now, so unchanged
poll ticks do not rebuild repeatedly, and edits in imported `*.rr` modules
trigger rebuilds even when `main.rr` itself did not change.

## Where To Go Next

If you are learning RR as a language:

1. [RR for R Users](r-for-r-users.md)
2. [Language Reference](language.md)
3. [Writing RR for Performance and Safety](writing-rr.md)
4. [Configuration](configuration.md)

If you need exact command behavior:

1. [CLI Reference](cli.md)
2. [Configuration](configuration.md)
