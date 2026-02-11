# Runtime and Error Model

RR emits self-contained R scripts by prepending runtime helpers from `src/runtime/mod.rs`.

## Runtime Responsibilities

- source location tracking: `rr_mark`, `rr_set_source`
- typed runtime checks:
  - truthiness: `rr_bool`, `rr_truthy1`
  - indexing: `rr_index1_read`, `rr_index1_write`, `rr_i0`, `rr_i1`
  - vector length contracts: `rr_same_len`, `rr_same_or_scalar`
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
