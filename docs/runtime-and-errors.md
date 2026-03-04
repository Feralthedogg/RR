# Runtime and Error Model

RR emits self-contained R scripts by prepending runtime helpers from `src/runtime/mod.rs`.

## Runtime Responsibilities

- source location tracking: `rr_mark`, `rr_set_source`
- typed runtime checks:
  - truthiness: `rr_bool`, `rr_truthy1`
  - indexing: `rr_index1_read`, `rr_index1_write`, `rr_i0`, `rr_i1`
  - vector length contracts: `rr_same_len`, `rr_same_or_scalar`
- intrinsic dispatch wrappers:
  - `rr_intrinsic_vec_*` helpers for typed vectorized MIR paths
  - backend routing through `rr_native_call(...)`
- data helpers:
  - named record/list helpers
  - closure helpers: `rr_closure_make`, `rr_call_closure`
  - matrix row/col helper operations

## NA and Indexing Policy

- read index path keeps R-compatible NA behavior in reads (`x[NA]` style semantics)
- write index path rejects NA index on assignment
- BCE can mark index operations safe and remove runtime wrappers
- optional strict read mode: set `RR_STRICT_INDEX_READ=1` to make NA read-index a hard error

## Runtime Modes

- `RR_RUNTIME_MODE=debug` (default): full checks + marks enabled
- `RR_RUNTIME_MODE=release`: enables runtime fast-paths and disables marks by default
- `RR_FAST_RUNTIME=1`: force fast-paths regardless of mode
- `RR_ENABLE_MARKS=0|1`: explicitly disable/enable `rr_mark`

Type/native/parallel compile settings are also injected into runtime glue:

- `RR_TYPE_MODE=strict|gradual` (`strict` default)
- `RR_NATIVE_BACKEND=off|optional|required` (`off` default)
- `RR_NATIVE_LIB=/path/to/librr_native.{so,dylib}` (native shared object path)
- `RR_PARALLEL_MODE=off|optional|required` (`off` default)
- `RR_PARALLEL_BACKEND=auto|r|openmp` (`auto` default)
- `RR_PARALLEL_THREADS=<N>` (`0` means auto-detect)
- `RR_PARALLEL_MIN_TRIP=<N>` (`4096` default)

Native backend policy:

- `off`: always use pure-R intrinsic fallback implementation
- `optional`: attempt `.Call`, fallback to pure-R on load/call failure
- `required`: `.Call` load/call failure is a hard runtime error

Native dispatch details:

- `.Call` symbols are registered in `native/rr_native.c` and looked up through the runtime bridge.
- `optional` mode must preserve semantic equivalence with `off` when the native library is missing.
- `required` mode is intended for controlled environments where native deployment is guaranteed.

Parallel dispatch policy:

- `off`: always execute sequential path.
- `optional`: attempt selected parallel backend, fallback to sequential if unavailable.
- `required`: parallel backend failure becomes runtime error (`E1031`).

Release mode behavior:

- runtime helpers are rebound to lightweight versions (`rr_mark`, `rr_truthy1`, `rr_index1_read/write`, `rr_i0`)
- this avoids per-call guard branching in hot loops when compiler invariants already guarantee safety
- keep `debug` mode for diagnosis; use `release` for performance runs

## Error Object

Compiler diagnostics use `RRException` (`src/error.rs`) with:

- module (`RR.ParseError`, `RR.TypeError`, `RR.RuntimeError`, ...)
- code (`E0001`, `E1001`, `E1002`, `E2001`, `E2007`, `ICE9001`, ...)
- stage (`Lex`, `Parse`, `Lower`, `MIR`, `Opt`, `Codegen`, `Runtime`, `Runner`, `ICE`)
- optional span, stacktrace frames, notes, related errors

Type-system diagnostics introduced for strict mode:

- `E1010`: type hint conflict
- `E1011`: call signature type mismatch
- `E1012`: unresolved strict-required type
- `E1030`: parallel safety proof failed in required mode
- `E1031`: parallel backend load/call failure in required mode
- `E1032`: non-deterministic parallel reduction rejected

Compiler core API (`src/compiler/pipeline.rs`) returns these diagnostics as `RR<T>`
to callers; process exit behavior is handled by CLI wiring in `src/main.rs`.

## Multi-Error Reporting

Parser and semantic/runtime validators can aggregate multiple findings into one report:

- top-level summary
- child diagnostics list
- per-error snippets and stack frames

This avoids fail-fast-only behavior and improves batch fixing.

## Colored Diagnostics

Color palettes are module-aware:

- parse/lex
- type/semantic
- runtime/bounds/value
- runner
- ICE/internal
- warning palette

Color enablement:

- enabled on TTY by default
- forced with `RR_FORCE_COLOR`
- disabled with `NO_COLOR`

## Runtime Execution and Mapping

`src/runtime/runner.rs` executes generated `.gen.R` via `Rscript --vanilla`.
It maps runtime/R-side line info back to RR spans using generated source maps.
