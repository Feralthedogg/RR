# Compatibility and Limits

This page is the compatibility and current-boundary manual for RR.

It states what RR can do today, where RR deliberately falls back, and where RR
still lowers conservatively.

## What RR Guarantees Today

- RR compiles to plain `.R` and executes through `Rscript`.
- Runtime guard helpers enforce scalar condition/index contracts with source-aware diagnostics.
- Strict declaration is the default language mode.
- R-style source forms are first-class:
  - `<-`
  - `function(...)`
  - dotted identifiers
  - single-line control forms
- closures and lambda lifting are supported
- list and record patterns are supported in core forms

## Conservative Zones

RR intentionally becomes conservative around:

- `eval`
- `parse`
- `get`
- `assign`
- `exists`
- `mget`
- `rm`
- `ls`
- `parent.frame`
- `environment`
- `sys.frame`
- `sys.call`
- `do.call`

These still emit runnable R, but aggressive optimization is restricted.

## Current Structural Limits

- record rest patterns are not supported
- vectorization is pattern-driven, not arbitrary-loop automatic transformation
- matrix/dataframe optimization remains selective
- selected 3D scalar and vector patterns are supported, but arbitrary nested 3D traversal is still conservative
- legacy `src/legacy/ir/*` is not the production compiler pipeline

## Current 3D Status

RR currently supports:

- scalar 3D indexing
- selected 3D map / expr-map / call-map / conditional-map / scatter-map
- selected 3D reduction (`sum/prod/min/max`)
- selected 3D shift / recurrence patterns

RR does not currently promise:

- arbitrary 3D traversal optimization
- general nested 3D scheduling
- broad polyhedral-style transformation

## Practical Expectations

Use `-O1` or `-O2` when you want optimization, but keep runtime parity tests for
any numerically meaningful workload.

If a workload is:

- canonical
- direct-indexed
- low in hidden state

RR is much more likely to optimize it well.

If a workload is:

- dynamic
- state-heavy
- alias-heavy
- metaprogramming-oriented

RR is much more likely to preserve correctness by skipping aggressive rewrites.

## Related Manuals

- [Language Reference](language.md)
- [Writing RR for Performance and Safety](writing-rr.md)
- [R Interop](r-interop.md)
- [Tachyon Engine](optimization.md)
