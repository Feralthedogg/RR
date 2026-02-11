# Compatibility and Current Limits

This document describes implemented compatibility behavior and known boundaries.

## Implemented Compatibility

- RR compiles to plain R scripts and executes via `Rscript`.
- Runtime guard layer enforces scalar condition/index contracts with source-aware diagnostics.
- NA-sensitive behavior is encoded in runtime helpers and MIR safety checks.
- R-style surface syntax is first-class:
  - `name <- function(...) { ... }`
  - assignment with `<-` / `=`
  - single-line `if/while/for` bodies
- `<-` assignment is accepted (with optional strict mode for undeclared names).
- Closures/lambdas are supported via lambda lifting + runtime closure wrappers.
- Match/list/record patterns are supported in core forms.

## Hybrid (Conservative) Paths

Dynamic runtime features are recognized and marked as non-aggressive optimization zones:

- `eval`, `parse`, `get`, `assign`, `exists`, `mget`, `rm`, `ls`
- `parent.frame`, `environment`, `sys.frame`, `sys.call`, `do.call`

RR still emits runnable R for these, but optimization is restricted for correctness.

## Known Limits

- Pattern record rest (`..` inside record pattern) is not supported.
- MIR/codegen currently targets 1D and 2D indexing forms.
- Vectorization is broad but still pattern-driven, not full automatic transformation for arbitrary loops.
- Matrix/dataframe optimization is partial and based on recognizable loop/value patterns.
- Legacy `src/ir/*` path is not the primary production pipeline.

## Practical Guidance

- Use `-O1`/`-O2` for optimization, but keep tests for semantics parity.
- For strict declaration discipline, set `RR_STRICT_LET=1`.
- Keep dynamic metaprogramming-heavy code in hybrid-friendly style and validate with runtime tests.
