# Runtime and Error Model

This page is the runtime contract and diagnostics reference for RR.

## Runtime Structure

RR emits self-contained `.R` scripts by prepending a selected runtime helper
subset.

Runtime source is split across:

- `src/runtime/runtime_prelude.R`
- `src/runtime/source.rs`
- `src/runtime/subset.rs`
- `src/runtime/mod.rs`

## Runtime Contract

The embedded runtime is responsible for:

- source tracking
  - `rr_mark`
- typed runtime checks
  - `rr_bool`
  - `rr_truthy1`
  - `rr_index1_read`
  - `rr_index1_write`
- helper operations
  - `rr_assign_slice`
  - `rr_index1_read_vec`
  - `rr_gather`
  - `rr_wrap_index_vec_i`
  - `rr_idx_cube_vec_i`
- intrinsic/backend dispatch
  - `rr_intrinsic_vec_*`
  - `rr_native_call`
  - `rr_parallel_*`
  - `rr_parallel_typed_vec_call`

Only referenced helpers plus their transitive dependencies are injected.

## Bootstrap Policy

The runtime bootstrap does two distinct things:

1. define env-driven defaults from `Sys.getenv(...)`
2. append compile-time policy assignments chosen by RR for the current artifact

That means the final emitted `.R` usually preserves compile-time backend/mode
selection unless you edit the artifact manually.

## Runtime Modes

- `RR_RUNTIME_MODE=debug`
  - default
  - full checks enabled
  - marks enabled by default
- `RR_RUNTIME_MODE=release`
  - lighter fast paths
  - marks disabled by default unless explicitly re-enabled
- `RR_FAST_RUNTIME=1`
  - force fast runtime paths
- `RR_ENABLE_MARKS=0|1`
  - explicitly disable or enable marks

## Native and Parallel Policy

### Native

- `off`
  - always use pure-R fallback
- `optional`
  - try native call, fallback on failure
- `required`
  - native failure is a runtime error

### Parallel

- `off`
  - always execute sequentially
- `optional`
  - try configured backend, fallback to sequential
- `required`
  - backend failure is a runtime error

Typed vector wrappers currently rely on the same parallel knobs, but practical
parallel execution is still R-backend centric.

## Indexing and NA Policy

- reads preserve normal R-like NA behavior unless strict read mode is enabled
- writes reject invalid or NA index values
- guard elimination is proof-based only
- `RR_STRICT_INDEX_READ=1` turns NA read-index behavior into a hard runtime error

## Error Model

Compiler diagnostics use `RRException` from `src/error.rs`.

Important fields:

- module / kind
  - `RR.ParseError`
  - `RR.TypeError`
  - `RR.RuntimeError`
  - `RR.InternalCompilerError`
- code
  - for example `E0001`, `E1002`, `E2001`, `E2007`, `ICE9001`
- stage
  - `Lex`, `Parse`, `Lower`, `MIR`, `Opt`, `Codegen`, `Runtime`, `Runner`, `ICE`

The compiler core returns structured diagnostics. The CLI decides final
formatting and exit behavior.

## Multi-Error Reporting

RR may aggregate multiple findings into a single report:

- summary header
- child diagnostics
- snippets, notes, and help text

This is intentional. RR does not force fail-fast-only reporting.

## Runtime Execution

`src/runtime/runner.rs` executes generated `.gen.R` through `Rscript --vanilla`.
RR uses source maps to map runtime line information back to RR spans when
reporting failures.

## Related Manuals

- [Configuration](configuration.md)
- [Compiler Pipeline](compiler-pipeline.md)
- [Testing and Quality Gates](testing.md)
