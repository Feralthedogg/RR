# CLI Reference

This page is the driver manual for RR.

Current compiler line: `RR Tachyon v7.0.0`.

## Audience

Read this page when you need exact driver behavior:

- accepted command forms
- flag classes
- precedence and defaults
- output and exit behavior

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
- you want imported `*.rr` changes to trigger rebuilds without restarting the session

Current watch behavior:

- unchanged poll ticks do not rebuild repeatedly
- imported module edits are tracked as part of the watched module tree
- `--once` still runs exactly one watch tick and exits

### R Runner Selection

`RR run` executes emitted `.gen.R` through:

1. explicit runner path passed by internal callers
2. `RRSCRIPT` if set
3. plain `Rscript` from `PATH`

If RR cannot start the selected R runner, it prints a recovery hint and points
at `--keep-r` so you can inspect the generated artifact.

## Option Classes

### Optimization and Output

- `-O0`
- `-O1`
- `-O2`
- `-o <file>`
- `--out-dir <dir>`

### Type and Backend Policy

- `--type-mode strict|gradual`
- `--native-backend off|optional|required`
- `--parallel-mode off|optional|required`
- `--parallel-backend auto|r|openmp`
- `--parallel-threads <N>`
- `--parallel-min-trip <N>`

### Language and Declaration Policy

- `--strict-let on|off`
- `--warn-implicit-decl on|off`

### Incremental and Watch

- `--incremental[=auto|off|1|1,2|1,2,3|all]`
- `--incremental-phases <auto|off|1|1,2|1,2,3|all>`
- `--no-incremental`
- `--strict-incremental-verify`
- `--poll-ms <N>`
- `--once`

### Command-Specific Options

- `--keep-r`
  - accepted on the direct legacy compile/run path and on `RR run`
  - not accepted on `build` or `watch`
- `--no-runtime`
  - accepted only on the direct compile path `RR file.rr ...`
  - not accepted on `run`, `build`, or `watch`
- `--preserve-all-defs`
  - accepted on direct compile, `run`, `build`, and `watch`
  - keeps unreachable top-level `Sym_*` definitions in emitted R
- `--preserve-all-def`
  - alias for `--preserve-all-defs`

## Exit Status

RR follows normal compiler-driver conventions:

- `0`
  - compile or run request completed successfully
- non-zero
  - a structured diagnostic or runner failure occurred

The CLI owns final process exit behavior. Internal compiler layers return
structured diagnostics instead of calling `std::process::exit(...)` directly.

## Artifact Policy

The direct compile path emits `.R` artifacts with:

- selected runtime helper subset
- compile-time runtime policy defaults for backend/parallel settings
- source map side data when requested by internal flows

By default RR treats emitted R as a whole-program artifact:

- reachable top-level definitions are kept
- unreachable `Sym_*` helpers may be stripped

If you need a more source-preserving artifact, pass `--preserve-all-defs` or
`--preserve-all-def`.

If you pass `--no-runtime`, RR still emits helper-only output, not raw MIR or an
intermediate dump.

## Related Manuals

- [Getting Started](getting-started.md)
- [Configuration](configuration.md)
- [Runtime and Error Model](runtime-and-errors.md)
- [Compiler Pipeline](compiler-pipeline.md)

## Semantics Notes

### `--no-runtime`

`--no-runtime` does not mean “emit raw source only”.

It means:

- omit compile-time source bootstrap and compile-time runtime policy defaults
- still emit the helper subset required by the generated program
- still emit ordinary `.R` code, not an internal IR dump

Use it for inspection and backend debugging, not for normal end-user execution.

### `--preserve-all-defs`

`--preserve-all-defs` keeps otherwise unreachable top-level RR definitions in
the emitted artifact.

`--preserve-all-def` is a supported alias.

Use it when:

- you want a closer source-to-source transpilation view
- you plan to inspect or call helper definitions from generated R
- you do not want whole-program dead-definition stripping

Without this flag, RR is free to drop unused top-level `Sym_*` definitions as
part of normal emitted-R cleanup.

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
