# Compatibility and Limits

This page is the compatibility and current-boundary manual for RR.

It states what RR can do today, where RR deliberately falls back, and where RR
still lowers conservatively.

## Compatibility Classes

Read the bullets on this page in three classes:

- guaranteed
  - expected product behavior today
- conservative
  - accepted surface, but proof/optimization remains intentionally limited
- unsupported
  - not part of the current optimizing contract

## Base-Priority Package Line

On this docs site, the "base-priority package line" means RR's first
direct-interop compatibility tier for the core R package set:

- `base`
- `compiler`
- `datasets`
- `graphics`
- `grDevices`
- `grid`
- `methods`
- `parallel`
- `splines`
- `stats`
- `stats4`
- `tools`
- `utils`

`datasets` is included in that term.

In practice, `datasets` currently shows up more as a direct typed data-object
surface than as an export-for-export function-closure claim. That means RR has
direct typed models for a large built-in subset of namespaced dataset bindings,
while the other packages in this line are the main direct call/interop surface.

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
- the base-priority package line is on RR's direct surface today
- `tcltk` stays on a conservative direct/proxy surface rather than forcing opaque fallback for the common helper family

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

Interpret that conservatism literally:

- RR will usually preserve the emitted R call shape
- RR may disable vectorization, reduction, or inlining around that code
- RR is choosing proof safety over a speculative rewrite

## Current Structural Limits

- record rest patterns are not supported
- vectorization is pattern-driven, not arbitrary-loop automatic transformation
- matrix/dataframe optimization remains selective
- the recommended package line (`MASS`, `Matrix`, `survival`, `nlme`, and similar)
  is not yet under the same "substantially closed" guarantee as the
  [base-priority package line](#base-priority-package-line)
- surface `matrix<T>` hints are accepted, but matrix-specific type precision is still more conservative than plain scalar/vector precision
- selected 3D scalar and vector patterns are supported, but arbitrary nested 3D traversal, schedule changes, and polyhedral-style transforms are not part of the current contract
- legacy `src/legacy/ir/*` is not the production compiler pipeline

## Current Type-System Precision

RR's strict type system is now materially better at a few cases that used to
collapse to `any` too early:

- `option<T>`, `union`, and `result` hints retain element primitive/shape facts instead of immediately degrading to a generic vector
- nested generic hints such as `list<box<float>>` survive strict call checking and index-element inference
- the `int` / `float` boundary is kept more precisely for arithmetic and common builtins
  - `/` widens to floating-point
  - `%%` stays integer when both inputs are integer
  - `sum(int-vector)` stays integer
  - `abs` / `pmax` / `pmin` keep integer element type when the inputs are proven integer
- vector builtins keep symbolic length facts when RR can prove the result length matches the input length

Still conservative today:

- dataframe typing is preserved internally more than before, but dataframe-specific field/schema reasoning is still limited
- matrix hints now stay matrix-typed internally, but matrix shape propagation is still not as strong as scalar/vector propagation in every optimizer pass
- matrix-shaped numeric arithmetic now feeds more of the same intrinsic/type-specialize path used by vector arithmetic, but matrix-aware optimization is still not uniform across every pass
- matrix shape algebra is now more precise at the type-term level:
  `t`, `diag`, `rbind`, `cbind`, and `%*%` preserve matrix-shaped intent and
  can retain exact row/column literals when RR can see them
- selected straight-line typed matrix kernels can now use the same typed parallel wrapper path as typed vectors when RR can preserve matrix shape and dimnames safely
- that contract is intentionally narrow: matrix wrappers are reserved for shape-preserving elementwise kernels; shape-sensitive matrix transforms still fall back to the ordinary non-wrapper path
- unresolved package interop still forces gradual/opaque behavior at the optimizer boundary

Read those bullets as "accepted but not uniformly exploited". They are not parse
errors, but they are also not a guarantee that every pass will use the extra
shape/schema information.

Practical consequence:

- strict mode will now reject obvious `vector<T>` / `matrix<T>` mismatches at 2D index sites
- dataframe structure is retained well enough to improve field-access typing, and named field access can now use visible schema information when RR has it
- in strict mode, an exact visible dataframe schema can also reject obviously missing field reads or field writes that violate the visible column type
- when RR can see an exact named dataframe schema, a field write such as `df.right = value` now updates the resulting schema term instead of leaving the old field type behind
- still, dataframe reasoning remains conservative once opaque package interop or schema-erasing transformations enter the path

## Current 3D Status

RR currently supports:

- scalar 3D indexing
- single-loop 3D map / expr-map / call-map / conditional-map / scatter-map when one obvious axis is the induction axis
- selected 3D reduction (`sum/prod/min/max`) for the same single-axis style loops
- selected 3D shift / recurrence patterns when the carried state remains reconstructible

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
- [Configuration](configuration.md)
