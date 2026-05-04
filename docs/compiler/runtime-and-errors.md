# Runtime and Error Model

This page is the runtime contract and diagnostics reference for RR.

## Audience

Read this page when you need to understand:

- what the embedded runtime is responsible for
- when RR fails at compile time versus runtime
- how compile-time policy becomes artifact-local runtime policy

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

Generated artifacts also carry explicit section markers:

- `# --- RR runtime (auto-generated) ---`
- `# --- RR generated code (from user RR source) ---`
- `# --- RR synthesized entrypoints (auto-generated) ---`

Those markers are there to distinguish injected runtime code, lowered RR
functions, and synthetic top-level entry calls in the final `.R` artifact.

Typed parallel wrappers have a narrower contract than the name suggests.

- Vector wrappers may slice by element range.
- Matrix wrappers are only used for straight-line shape-preserving kernels.
- When a matrix wrapper is used, the R fallback splits work by column blocks and
  restores the original `dim` and `dimnames` on the joined result.
- Shape-sensitive matrix kernels such as transpose-like rewrites are not wrapped;
  they stay on the normal single-threaded path.

## Bootstrap Policy

The runtime bootstrap does two distinct things:

1. define env-driven defaults from `Sys.getenv(...)`
2. append compile-time backend/parallel policy defaults chosen by RR for the current artifact

That means the final emitted `.R` keeps the compile-time backend and parallel
policy by default, but an explicit runtime override still wins. Runtime mode and
similar per-run knobs remain env-driven.

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
parallel execution is still R-backend centric. In current artifacts that means:

- the runtime can attempt chunked parallel execution under the configured policy
- some generated kernels still fall back to pure-R sequential or chunked-R paths
- not every typed kernel has a dedicated native/OpenMP implementation

## Indexing and NA Policy

- reads preserve normal R-like NA behavior unless strict read mode is enabled
- writes reject invalid or NA index values
- guard elimination is proof-based only
- `RR_STRICT_INDEX_READ=1` turns NA read-index behavior into a hard runtime error
- obvious matrix bounds errors such as `m[nrow(m) + 1, 1]` or `m[1, ncol(m) + 1]`
  are rejected at compile time when the matrix extent is statically known

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

## Reporting Contract

RR prefers:

- structured compiler diagnostics before execution when proof is available
- source-aware runtime failures when proof is not available until execution
- aggregate reporting when multiple independent failures are provable

RR does not treat these as interchangeable. If a failure can be proven before
execution, the intended behavior is a compile-time diagnostic rather than a
deferred runtime failure.

## Static Trait And Field Diagnostics

Receiver-method trait calls and record/dataframe field access share the same
surface syntax shape: `value.name`. The compiler keeps those diagnostic paths
separate.

- If `value.name(...)` names a known trait method but the receiver has no
  statically known trait-dispatch target, lowering reports a trait-method
  diagnostic. The message asks for a receiver type hint, a matching
  `where T: Trait` bound, or explicit `Trait.method(receiver, ...)` syntax.
- If `value.name` is an actual field access and the record/dataframe type hint
  does not contain `name`, strict type checking reports `unknown field`.
- Field diagnostics must not expose internal solver wording when the user
  action is to fix the field name or type hint.

This keeps missing type hints from surfacing as unrelated record/dataframe field
errors.

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

Runner selection order is:

1. explicit runner path passed by the caller
2. `RRSCRIPT` environment override
3. `Rscript` from `PATH`

If runner startup fails, RR reports it as `RR.RunnerError` and includes recovery
guidance such as:

- install `Rscript` or set `RRSCRIPT`
- rerun with `--keep-r` to inspect the generated `.gen.R` file
- make the source directory writable if RR cannot create the temporary `.gen.R`,
  or rerun `RR build --out-dir <dir>` if you want emitted R somewhere else

When `RR run --keep-r` succeeds, RR also reports the kept generated artifact
path so you can inspect or rerun the exact emitted `.gen.R`.

## Related Manuals

- [Configuration](../configuration.md)
- [Compiler Pipeline](pipeline.md)
- [Testing and Quality Gates](testing.md)
