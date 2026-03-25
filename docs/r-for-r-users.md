# RR for R Users

This guide is for people who already write R and want to use RR without first
thinking like a compiler engineer.

RR is not a replacement runtime for R. It is a source language and compiler
that emits self-contained `.R` artifacts. The target remains the R ecosystem:
GNU R, Renjin, package interop, and familiar vector/matrix workflows.

## What RR Is Good At

RR fits best when you want to keep an R-shaped workflow but make these things
more explicit:

- typed function boundaries
- strict diagnostics
- predictable lowering to generated R
- optimization over loops, slices, maps, reductions, and selected matrix code
- self-contained build artifacts

RR is a poor fit if you need unrestricted metaprogramming, arbitrary search-path
mutation, or code that depends on dynamic name lookup staying opaque.

## Mental Model

Write RR as:

- R-shaped data code
- with stricter declarations
- with clearer function signatures
- with fewer hidden dynamic behaviors

You should expect RR to reward:

- visible shapes
- visible indices
- visible container structure
- visible control flow

You should expect RR to stay conservative around:

- `eval`, `parse`, `get`, `assign`, `do.call`
- package calls RR does not model directly
- schema-erasing dataframe flows
- alias-heavy code where mutation order is unclear

## Syntax Differences from R

### Declarations

RR defaults to strict declaration rules.

Use `let` when a variable is first introduced.

```rr
fn main() {
  let x = 1L
  let y = x + 2L
  y
}
```

Do not rely on implicit creation by assignment.

Bad:

```rr
fn main() {
  x = 1L
  x
}
```

### Functions

RR uses `fn`.

```rr
fn saxpy(a: float, x: vector<float>, y: vector<float>) -> vector<float> {
  a * x + y
}
```

You can still write R-like data code inside the function body, but parameter and
return hints give RR much better type and shape information.

### Integers and Floats

RR keeps the `int` / `float` boundary more explicitly than ordinary R code.

- `/` widens to floating-point
- `%%` stays integer when both sides are integer
- `sum(int-vector)` stays integer when RR can prove the input type

Use:

- `1L`, `2L`, `3L` for integer intent
- `1.0`, `2.5` for floating intent

## Data Structures

### Vectors

RR handles ordinary vector-style R code well when the index space is obvious.

Good:

```rr
fn normalize(x: vector<float>) -> vector<float> {
  let n = length(x)
  let out = rep.int(0.0, n)
  for (i in 1..n) {
    out[i] = x[i] / 100.0
  }
  out
}
```

### Matrices

If something is matrix-shaped, say so explicitly with `matrix<T>`.

```rr
fn center(m: matrix<float>) -> matrix<float> {
  let mu = colSums(m) / nrow(m)
  m - mu
}
```

RR now understands these matrix helpers directly:

- `matrix`
- `dim`
- `dimnames`
- `nrow`
- `ncol`
- `rowSums`
- `colSums`
- `crossprod`
- `tcrossprod`
- `t`
- `diag`
- `rbind`
- `cbind`
- `%*%`

That matters for:

- strict type checking
- shape preservation
- compile-time bounds checks
- typed specialization

### Data Frames

RR can preserve typed dataframe schema information better than before, but you
still get the best results when the schema is explicit and local.

Good:

```rr
fn enrich(df: dataframe{left: vector<int>, right: vector<float>}) {
  let out = rr_field_set(df, "right", rr_field_get(df, "right") + 1.0)
  out
}
```

Named field refinement is strongest when:

- the field name is a literal string
- the schema is visible to RR
- interop stays on RR-known helpers

## Interop Guidance

Prefer namespace-preserving imports and namespaced calls.

Good:

```rr
import r * as stats from "stats"

fn main(x: vector<float>) {
  stats.median(x)
}
```

Less helpful:

```rr
fn main(x) {
  library("stats")
  median(x)
}
```

The second form pushes RR toward hybrid or opaque handling much sooner.

If you need exact package behavior and RR does not model it deeply, keep the
call namespaced and let RR preserve it as interop instead of trying to hide the
boundary.

## Performance Guidance for R Users

### What Usually Optimizes Well

- straight-line elementwise maps
- reductions with clear accumulator flow
- visible loop bounds
- full-slice rewrites
- matrix helpers RR already knows
- typed vector and selected typed matrix kernels

### What Usually Optimizes Poorly

- hidden aliasing
- repeated reconstruction of equivalent indices
- dynamic field names
- dynamic package lookup
- stateful loops with many carried scalars
- code that mixes shape-changing and side-effectful steps in one loop

### Practical Advice

- keep one canonical index variable
- derive bounds from the data you index
- prefer `matrix<T>` over pretending everything is `vector<T>`
- prefer exact field names over computed field names
- keep helper functions small and pure
- use strict mode during development

## Diagnostics You Should Expect

RR is intentionally stricter than typical interactive R code.

Common classes of diagnostics:

- undeclared variable
- argument/return type mismatch
- obvious 2D indexing on non-matrix base
- obvious matrix bounds errors such as `m[nrow(m) + 1, 1]`
- undefined field against an exact visible dataframe schema

This is by design. RR tries to fail early when it can prove a runtime failure.

## Suggested Workflow

1. Start with strict settings.
2. Add type hints at function boundaries first.
3. Add `matrix<T>` and dataframe schema hints where shape matters.
4. Compile with `-O0` when debugging semantics.
5. Compare `-O0` and `-O2` outputs when optimizing.
6. Use `--no-incremental` when auditing emitted R shape.

Example:

```bash
RR_STRICT_LET=1 RR_RUNTIME_MODE=debug \
  cargo run -- example/benchmarks/signal_pipeline_bench.rr -O2 --no-incremental
```

## Read Next

- [Getting Started](./getting-started.md)
- [Writing RR for Performance and Safety](./writing-rr.md)
- [Language Reference](./language.md)
- [R Interop](./r-interop.md)
- [Compatibility and Limits](./compatibility.md)
