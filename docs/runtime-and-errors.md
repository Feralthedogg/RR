# Runtime and Error Model

RR emits self-contained R scripts by prepending runtime helpers from `src/runtime/mod.rs`.

## Runtime Responsibilities

The embedded runtime is responsible for:

- source location tracking:
  - `rr_mark`
  - `rr_set_source`
- typed runtime checks:
  - `rr_bool`, `rr_truthy1`
  - `rr_index1_read`, `rr_index1_write`
  - `rr_i0`, `rr_i1`
  - `rr_same_len`, `rr_same_or_scalar`
- vectorized/helper operations:
  - `rr_assign_slice`
  - `rr_assign_index_vec`
  - `rr_index1_read_vec`
  - `rr_wrap_index_vec_i`
  - `rr_idx_cube_vec_i`
- intrinsic and backend dispatch:
  - `rr_intrinsic_vec_*`
  - `rr_native_call(...)`
  - `rr_parallel_*`
- data helpers:
  - record/list helpers
  - closure helpers
  - matrix row/column helpers

## Runtime Modes

- `RR_RUNTIME_MODE=debug`
  - default
  - full checks enabled
  - marks enabled by default
- `RR_RUNTIME_MODE=release`
  - enables lighter fast paths
  - disables marks by default unless explicitly re-enabled
- `RR_FAST_RUNTIME=1`
  - force fast-path rebinding regardless of mode
- `RR_ENABLE_MARKS=0|1`
  - explicitly disable or enable `rr_mark`

Use `debug` when diagnosing correctness problems.
Use `release` when measuring runtime performance.

## NA and Indexing Policy

- read paths preserve R-like NA behavior unless strict read mode is enabled
- write paths reject NA index values
- BCE and proof-based guard removal may eliminate wrappers only when safety is proven
- `RR_STRICT_INDEX_READ=1` converts NA read-index behavior into a hard runtime error

## Native Backend

Compile/runtime glue carries these settings:

- `RR_NATIVE_BACKEND=off|optional|required`
- `RR_NATIVE_LIB=/path/to/librr_native.{so,dylib,dll}`
- `RR_NATIVE_AUTOBUILD=0|1`

Policy:

- `off`
  - always use pure-R fallback helpers
- `optional`
  - attempt native `.Call`, then fallback to pure-R if load/call fails
- `required`
  - native load/call failure becomes a runtime error

If `RR_NATIVE_LIB` is unset and autobuild is enabled, the runtime searches project-relative paths
and may attempt `R CMD SHLIB native/rr_native.c` into `target/native`.

## Parallel Backend

- `RR_PARALLEL_MODE=off|optional|required`
- `RR_PARALLEL_BACKEND=auto|r|openmp`
- `RR_PARALLEL_THREADS=<N>`
- `RR_PARALLEL_MIN_TRIP=<N>`

Policy:

- `off`
  - always execute sequentially
- `optional`
  - try the configured backend, then fallback to sequential
- `required`
  - backend failure is a runtime error (`E1031`)

## Error Object

Compiler diagnostics use `RRException` from `src/error.rs`.

Fields include:

- module/kind:
  - `RR.ParseError`
  - `RR.TypeError`
  - `RR.RuntimeError`
  - `RR.InternalCompilerError`
- code:
  - for example `E0001`, `E1002`, `E2001`, `E2007`, `ICE9001`
- stage:
  - `Lex`, `Parse`, `Lower`, `MIR`, `Opt`, `Codegen`, `Runtime`, `Runner`, `ICE`
- optional span, notes, related diagnostics, and stack frames

The compiler core returns structured diagnostics to callers; the CLI decides final process exit behavior.

## Multi-Error Reporting

Parser and semantic/runtime validators can aggregate multiple findings into a single report:

- summary header
- child diagnostics
- per-error snippets and notes

This is intentionally not fail-fast only.

## Strict-Type and Backend Diagnostics

Examples of important codes:

- `E1010`
  - type hint conflict
- `E1011`
  - call signature type mismatch
- `E1012`
  - unresolved strict-required type
- `E1030`
  - required parallel safety proof failed
- `E1031`
  - required parallel backend load/call failure
- `E1032`
  - non-deterministic parallel reduction rejected

## Colored Diagnostics

Diagnostics are colorized by category when terminal output supports ANSI or `RR_FORCE_COLOR` is set.
Set `NO_COLOR` to disable color.

## Runtime Execution and Source Mapping

`src/runtime/runner.rs` executes generated `.gen.R` through `Rscript --vanilla`.
RR uses generated source maps to map R/runtime line information back to RR spans when reporting failures.
