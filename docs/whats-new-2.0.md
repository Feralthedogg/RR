# What Is New in RR 2.0.0

RR 2.0.0 is the first release line that treats RR as a managed project compiler,
not only a single-file `.rr` to `.R` translator. It also resets several
compiler defaults around strictness, optimizer budgeting, package workflows, and
the public Rust API.

Use this page when upgrading from the 1.4.x line.

## Upgrade Summary

Most existing single-file programs still compile when passed as explicit files:

```bash
RR main.rr -o main.R -O2
```

Managed project commands now resolve project entries through `src/`:

```bash
RR run .
RR build .
RR watch .
```

For those commands, `.` or a directory resolves to `src/main.rr`. A root-level
`main.rr` is no longer used as the directory fallback.

## Breaking and Migration Notes

### Managed Project Entry

RR 2.0 uses a cargo-like project layout:

- `src/main.rr` for runnable binaries
- `src/lib.rr` for libraries
- `rr.mod` for module metadata
- `rr.lock` for resolved dependency versions

Commands affected:

- `RR run .`
- `RR build .`
- `RR watch .`

If a 1.x project relied on a root-level `main.rr`, either move it to
`src/main.rr` or pass the file explicitly:

```bash
RR run main.rr
RR build main.rr
```

### Strict Declarations by Default

RR 2.0 makes explicit declarations the stable default. Assignment to an
undeclared name is rejected unless migration mode is explicitly enabled.

Migration options:

- keep the stable default and add missing `let` bindings
- temporarily use `RR_ALLOW_LEGACY_IMPLICIT_DECL=1 --strict-let off`
- add `--warn-implicit-decl on` while cleaning old code

### Strict Type Mode by Default

The stable CLI defaults to strict type checking. The gradual mode remains as a
temporary migration hatch:

```bash
RR_ALLOW_GRADUAL_TYPE_MODE=1 RR input.rr --type-mode gradual
```

### Public Rust API Boundary

The stable Rust library surface is intentionally smaller:

- `compiler`
- `error`
- `pkg`
- `runtime`
- `Span`

HIR, MIR, syntax, type-checking, and codegen internals are private implementation
details. Downstream code that imported internal modules should move through the
stable compiler/package entrypoints.

## User-Facing Additions

### Cleaner CLI Progress

Build output now favors short, rust-style stage lines. Detailed optimizer
counters are kept out of normal output and remain available through verbose
logging or compile profiles.

### Source Name Preservation

RR 2.0 preserves source-visible function and variable names where possible in
emitted R. Internal helpers may still use generated names such as
`__rr_outline_...`; those names are not stable API.

### Read-Only Raw R Escape

In addition to read/write raw R blocks:

```rr
unsafe r {
  ...
}
```

RR 2.0 supports the narrower read-only form:

```rr
unsafe r(read) {
  ...
}
```

This form is a promise that the raw R block reads RR-visible locals without
writing them. It lets the optimizer avoid treating the containing function as a
fully opaque interop block.

## Type and NA Precision

RR 2.0 improves strict type precision without abandoning conservative behavior:

- Hindley-Milner hints help infer unannotated local and function shapes
- vector-aware `is.na` and `is.finite` hints preserve logical-vector shape
- comparison and logical operations preserve vector result shape when operands
  make that shape visible
- integer and floating-point boundaries are kept more precisely
- generic hints such as `option<T>`, `union`, `result`, and nested list/box
  forms retain more element information
- dataframe and matrix facts are preserved further through the type solver

NA behavior remains conservative:

- `is.na` and `is.finite` can refine branch facts when the tested value is
  visible
- index reads preserve R-like NA behavior unless strict index-read mode is
  enabled
- index writes reject invalid or NA indexes
- `RR_STRICT_INDEX_READ=1` turns NA read indexes into hard runtime errors

## Optimizer and Compile-Time Changes

### Chronos Pass Manager

Chronos now owns the main MIR optimization stage boundaries. It records pass
identity, verification labels, analysis invalidation, pass timing, and
opportunity counters so optimizer behavior is easier to audit.

### Adaptive Phase Ordering

`-O2` now uses adaptive heavy-tier scheduling by default. RR classifies functions
into bounded schedule families such as `Balanced`, `ComputeHeavy`, and
`ControlFlowHeavy`.

### MIR Outlining

O2/O3 can split selected large, safe MIR regions into internal helper functions.
This is primarily a compile-budget and cache-shape control pass, not an
arbitrary hot-path transform.

Key limits:

- skips opaque raw R and conservative functions
- limits live-ins and live-outs
- uses deterministic internal helper names
- keeps source-visible names separate from helper names

### Loop Unrolling

O2/O3 can unroll simple constant-trip counted loops after vectorization and
polyhedral opportunities have already been considered.

Default full-unroll limits:

- `-O2`: up to 8 trips
- `-O3`: up to 16 trips

Larger constant-trip loops may use bounded partial unrolling when enabled and
when the IR growth budget allows it.

### Optimization Fuel

Tachyon now has per-function fuel. Fuel exhaustion skips the remaining heavy or
structural work for that function instead of failing the compile.

Relevant expert env vars:

- `RR_OPT_FUEL`
- `RR_OPT_FUEL_TRACE`
- `RR_OUTLINE_ENABLE`
- `RR_UNROLL_ENABLE`

### Compile Profiles

`--profile-compile` now writes RR 2.0 compile-profile schema version `4`.

The profile includes:

- Tachyon pass decisions
- optimization opportunity counters
- outline/unroll counters
- fuel consumed and exhausted-function counts
- optimized-MIR cache hit/miss counters
- pass timings and emitted-artifact cache behavior

## Incremental and Cache Changes

RR 2.0 keeps the existing phase caches and adds safer optimized-MIR reuse
metadata:

- env-sensitive optimizer knobs participate in cache fingerprints
- changed fuel/outlining/unroll policy prevents stale optimized-MIR hits
- function-level optimized-MIR hit/miss counters are exposed in compile profiles
- `RR watch` fingerprints the imported module tree so imported `*.rr` edits
  trigger rebuilds

The whole-program fallback remains the conservative path when a function-level
hit is not safe.

## Package Manager and Registry

RR 2.0 expands the package workflow around `rr.mod`, `rr.lock`, local module
caches, and registry operations.

Project commands:

- `RR new`
- `RR init`
- `RR mod tidy`
- `RR mod graph`
- `RR mod why`
- `RR mod vendor`
- `RR mod verify`

Registry and release commands:

- `RR publish`
- `RR registry search`
- `RR registry info`
- `RR registry verify`
- `RR registry diff`
- `RR registry risk`
- `RR registry channel`
- `RR registry approve`
- `RR registry unapprove`
- `RR registry promote`
- `RR registry yank`
- `RR registry unyank`
- `RR registry deprecate`
- `RR registry undeprecate`
- `RR registry keygen`
- `RR registry onboard`
- `RR registry policy`
- `RR registry audit`

Registry releases can be signed and verified with HMAC or ed25519 keys. Registry
policy can require signed releases, approval before `@latest` resolution,
trusted public keys, signer allowlists, and revoked signer keys.

## R Interop and Runtime Surface

RR 2.0 keeps expanding direct package interop while preserving conservative
fallbacks around dynamic R metaprogramming.

The base-priority package line now includes direct models for:

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

The `tcltk` surface remains conservative but no longer forces broad opaque
fallback for the common helper family.

## Verification and Docs

RR 2.0 strengthens contributor and docs validation:

- generated contributing docs are checked in CI
- docs CLI option surface is tested against the driver usage text
- docs environment-variable surface is tested against public config sources
- direct R interop docs are tested against the call-model surface
- unsafe boundaries are documented separately from RR `unsafe r`
- docs build through VitePress

## Upgrade Checklist

1. Move managed project entries to `src/main.rr` or pass old root-level files
   explicitly.
2. Add missing `let` bindings, then remove
   `RR_ALLOW_LEGACY_IMPLICIT_DECL`.
3. Try strict type mode first. Use gradual mode only as a temporary migration
   hatch.
4. Rebuild with `-O2` and run parity tests for numeric workloads.
5. If compile time changes unexpectedly, inspect `--profile-compile` before
   changing optimizer env knobs.
6. If a package uses registry workflows, decide whether releases require
   signatures and approval before publishing.
