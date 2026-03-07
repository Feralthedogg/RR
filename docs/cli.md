# CLI Reference

RR supports direct compile, run, build, and watch workflows.

## Invocation Forms

```bash
RR --version
RR version
RR <input.rr> [options]
RR run [main.rr|dir|.] [options]
RR build [dir|file.rr] [options]
RR watch [main.rr|dir|.] [options]
```

During development, `cargo run -- ...` is equivalent to invoking `RR ...`.

Current compiler line: `RR Tachyon v4.0.0`.

## Command Behavior

### `version`

```bash
RR --version
RR version
```

- prints the compiler banner version and exits
- intended for scripts, CI, and local environment checks

### Direct Compile

```bash
RR input.rr -o out.R -O2
```

- compiles one `.rr` file
- writes one `.R` file
- accepts `--no-runtime` to omit source/native bootstrap while keeping helper definitions and runtime settings

### `run`

```bash
RR run .
```

Input may be:

- `.`
- a directory
- a `.rr` file

If input is `.` or a directory, RR resolves `main.rr` in that directory.

### `build`

```bash
RR build . --out-dir build -O2
```

Input may be:

- a directory: recursively compile all `.rr` files
- a single `.rr` file: compile one file into the output directory

### `watch`

```bash
RR watch . -O2
```

- resolves targets like `run`
- polls for changes
- keeps an in-memory incremental session across ticks

## Common Options

- `-O0`, `-O1`, `-O2`
- `-o0`, `-o1`, `-o2`
  - accepted optimization aliases
- `-o <file>`
  - direct compile: output file
  - `build`: alias for `--out-dir`
- `--out-dir <dir>`
  - output directory for `build`
- `--no-runtime`
  - omit source/native bootstrap while keeping helper definitions and runtime settings
- `--keep-r`
  - keep generated `.gen.R` after `run`
- `--type-mode strict|gradual`
- `--native-backend off|optional|required`
- `--parallel-mode off|optional|required`
- `--parallel-backend auto|r|openmp`
- `--parallel-threads <N>`
- `--parallel-min-trip <N>`

## Incremental and Watch Options

- `--incremental[=off|1|1,2|1,2,3|all]`
- `--incremental-phases <off|1|1,2|1,2,3|all>`
- `--strict-incremental-verify`
  - rebuild and compare against any available incremental cache artifact
  - compares both emitted R and source maps
  - first compile after a cache miss is populated but not yet "verified"
- `--poll-ms <N>`
  - watch polling interval in milliseconds
- `--once`
  - run one watch tick and exit

Incremental phases are described in [Compiler Pipeline](compiler-pipeline.md).

## Examples

Compile one file:

```bash
RR path/to/input.rr -o out.R -O2
```

Run a directory project:

```bash
RR run path/to/project -O2
```

Build all `.rr` files under a tree:

```bash
RR build path/to/project --out-dir build -O2
```

Watch a project once with full incremental phases:

```bash
RR watch . --incremental=all --once -O2
```

## Exit Behavior

- `0`: success
- non-zero: parse, semantic, compiler, or runtime failure

RR returns structured diagnostics from the compiler core and lets the CLI choose the final process exit code.
Colored output is enabled on supported terminals unless disabled by `NO_COLOR`.
