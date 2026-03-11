# CLI Reference

This page is the driver manual for RR.

Current compiler line: `RR Tachyon v5.0.0`.

## Synopsis

```bash
RR --version
RR version
RR <input.rr> [options]
RR run [main.rr|dir|.] [options]
RR build [dir|file.rr] [options]
RR watch [main.rr|dir|.] [options]
```

During development, `cargo run -- ...` is equivalent to invoking `RR ...`.

## Command Summary

| Command | Purpose | Typical use |
| --- | --- | --- |
| `RR file.rr` | compile one file | emit one `.R` artifact |
| `RR run .` | compile and execute entry | local project runs |
| `RR build . --out-dir build` | compile a tree | batch builds |
| `RR watch .` | rebuild on changes | edit/compile loops |
| `RR --version` | print compiler line | scripts and CI |

## Command Forms

### `version`

```bash
RR --version
RR version
```

Print the compiler line and exit.

### Direct Compile

```bash
RR input.rr -o out.R -O2
```

Use direct compile when:

- one `.rr` file should become one `.R` file
- you want to inspect generated output directly
- you want exact control over `-O0/-O1/-O2`

### `run`

```bash
RR run .
```

Input may be:

- `.`
- a directory
- a `.rr` file

If input is `.` or a directory, RR resolves `main.rr`.

### `build`

```bash
RR build . --out-dir build -O2
```

Use `build` when:

- you want every `.rr` file under a tree compiled
- you want output paths mirrored under a build directory

RR skips `target/`, `.git/`, and the selected output directory during tree walks.

### `watch`

```bash
RR watch . -O2
```

Use `watch` when:

- you want repeated rebuilds from one live session
- you want phase 3 in-memory incremental reuse

## Option Classes

### Optimization and Output

- `-O0`
- `-O1`
- `-O2`
- `-o <file>`
- `--out-dir <dir>`
- `--no-runtime`
- `--keep-r`

### Type and Backend Policy

- `--type-mode strict|gradual`
- `--native-backend off|optional|required`
- `--parallel-mode off|optional|required`
- `--parallel-backend auto|r|openmp`
- `--parallel-threads <N>`
- `--parallel-min-trip <N>`

### Incremental and Watch

- `--incremental[=auto|off|1|1,2|1,2,3|all]`
- `--incremental-phases <auto|off|1|1,2|1,2,3|all>`
- `--no-incremental`
- `--strict-incremental-verify`
- `--poll-ms <N>`
- `--once`

## Semantics Notes

### `--no-runtime`

`--no-runtime` does not mean “emit raw source only”.

It means:

- omit source/native bootstrap
- still emit the runtime helper subset actually used by generated code

Use it for inspection and backend debugging, not for normal end-user execution.

### Builtin Naming

Most math and aggregation names are reserved for builtin/intrinsic lowering.

User shadowing is intentionally narrow:

- allowed scalar-index helpers:
  - `length`
  - `floor`
  - `round`
  - `ceiling`
  - `trunc`

Everything else should use distinct user names.

### R Interop

- `import r "pkg"` gives namespace-style access
- `pkg.fn(...)` lowers to `pkg::fn(...)`
- `import r { fn as local } from "pkg"` binds one local alias
- `import r * as pkg from "pkg"` binds namespace-style access

See [R Interop](r-interop.md) for package coverage and fallback tiers.

## Incremental Compile Policy

Default CLI behavior is `--incremental=auto`.

`auto` means:

- phase 1 enabled
- phase 2 enabled
- phase 3 enabled only when a live session exists, such as `watch`

Use:

- `--no-incremental` when you want a fresh compile for inspection
- `--strict-incremental-verify` when you want cache reuse checked against a rebuild

The incremental artifact model is documented in [Compiler Pipeline](compiler-pipeline.md).

## Exit Status

- `0`
  - compile or run succeeded
- non-zero
  - parse, semantic, type, compiler, or runtime failure

The compiler core returns structured diagnostics. The CLI owns final process
exit behavior and formatting.
