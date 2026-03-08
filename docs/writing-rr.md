# Writing RR for Performance and Safety

This guide is for RR users writing `.rr` programs.

RR's optimizer is strongest when code stays close to explicit numeric kernels,
canonical loops, and predictable dataflow. It is intentionally conservative when
proofs are missing, so "easy to reason about" code is usually both faster and safer.

## Why This Style Optimizes Well

Tachyon does not try to guess arbitrary intent from dynamic code. It recognizes
specific safe source shapes and rewrites them aggressively when the proof is clear.

Think of that source shape as an optimization fingerprint:

| Source fingerprint | Tachyon can usually prove |
| --- | --- |
| one induction variable + one obvious trip count | loop canonicalization, bounds-check elimination, vector-style lowering |
| direct `x[i]` / `y[i]` reads and one direct `out[i]` write | map/reduction recognition |
| one scalar accumulator updated once per iteration | reduction rewrites |
| pure, small helper calls | inlining, SCCP, GVN/CSE, vectorized call-map |
| loop-invariant shape/scalar facts computed once | LICM and simpler safety proofs |

There is not yet a standalone end-user "fingerprint checker" command. In practice,
use the optimizer's own feedback:

- normal `-O1` / `-O2` compile output for `Vectorized`, `Reduced`, `Simplified`, and `VecSkip`
- `RR_VECTORIZE_TRACE=1` when you need per-loop reject reasons

Canonical loops matter mainly because they are the main entry point to RR's
vectorization pipeline. Today that usually means rewriting scalar loops into
bulk vector helpers or builtin vector operations, which is RR's current path to
SIMD-like execution and lower interpreter overhead.

## The Short Version

- Prefer straight-line numeric code, canonical loops, and pure helpers.
- Keep indexing, lengths, and reduction state obvious.
- Avoid dynamic runtime features in hot paths.
- Develop with strict/runtime safety knobs enabled, then measure with optimized builds.
- Prefer one statement per line; semicolons are not supported.

## Write So Tachyon Can Optimize It

### Keep loops canonical

RR currently recognizes map, reduction, gather/scatter, and selected matrix-style
patterns best when the loop shape is simple:

- one induction variable
- one clear trip count
- reads and writes indexed directly from that induction variable
- no hidden control flow or alias-heavy writes

Good:

```rr
fn saxpy(x, y, a) {
  let n = length(x)
  let out = y
  for i in 1..n {
    out[i] = (a * x[i]) + y[i]
  }
  out
}
```

Harder to optimize:

```rr
fn saxpy_dynamic(x, y, a) {
  let n = length(x)
  let out = y
  let idx = seq_len(n)
  for i in idx {
    let j = idx[i]
    out[j] = (a * x[j]) + y[j]
  }
  out
}
```

Why the first version is better:

- the induction variable is **directly** the index used for reads and writes
- the write target is **one obvious destination**
- the loop body looks like an elementwise map, which is the pattern Tachyon
  vectorizes most reliably

`for i in 1..length(x)` is valid RR and may still optimize well. But prefer:

```rr
let n = length(x)
for i in 1..n {
  ...
}
```

`length(x)` is loop-invariant, and LICM may hoist some invariants automatically.
Still, spelling out `n` is better because it makes the trip count explicit for
both the reviewer and the optimizer instead of relying on inference.

### Keep reductions simple

Scalar reductions are easier to prove and rewrite when there is one obvious
accumulator and one update per iteration.

Good:

```rr
fn total_energy(x) {
  let n = length(x)
  let acc = 0.0
  for i in 1..n {
    acc = acc + (x[i] * x[i])
  }
  acc
}
```

Less helpful:

- carrying multiple coupled accumulators when one would do
- mixing the accumulator with unrelated side effects
- rebuilding the loop bound or index from intermediate containers

Rule of thumb:

- one loop
- one accumulator
- one obvious recurrence update

### Prefer pure, small helpers

Inlining, GVN/CSE, SCCP, and vectorization all benefit when helper functions:

- depend only on their arguments
- do not mutate hidden global state
- do not call dynamic runtime features
- stay structurally small

Good:

```rr
fn clamp01(x) {
  if x < 0.0 {
    return 0.0
  }
  if x > 1.0 {
    return 1.0
  }
  x
}
```

Use helpers like that freely inside loops. RR can reason about them much more
easily than helpers built around `get`, `assign`, `eval`, or `do.call`.

Good helper fingerprint:

- input-only
- deterministic
- no hidden environment access
- no mutation outside its local result

### Keep indexing and shapes explicit

Bounds-check elimination and vector rewrites work best when shape facts are easy
to see in source:

- derive loop ranges from the data being indexed
- keep paired vectors the same length
- use one canonical index instead of rebuilding equivalent indices in several forms
- prefer row/column access patterns that stay visibly tied to matrix dimensions

Good:

```rr
fn pairwise_add(x, y) {
  let n = length(x)
  let out = x
  for i in 1..n {
    out[i] = x[i] + y[i]
  }
  out
}
```

Before code like this, validate the length relation at the API boundary if it is
not already guaranteed by construction.

The main goal is to keep dataflow reviewable:

- the compiler can see where `n` came from
- the loop bound and the indexing expression stay in sync
- BCE and vectorization do not need to rediscover equivalent facts from several forms

### Use direct builtins when they express the math

RR already knows about many intrinsic/numeric forms such as:

- `sum`
- `mean`
- `abs`
- `min`
- `max`
- `sqrt`
- `log10`
- `atan2`

Prefer the direct builtin over hiding the same computation behind dynamic dispatch
or metaprogramming.

### Avoid dynamic fallback in hot code

These builtins force conservative handling and reduce optimization opportunities:

- `eval`, `parse`, `get`, `assign`, `exists`, `mget`, `rm`, `ls`
- `parent.frame`, `environment`, `sys.frame`, `sys.call`, `do.call`

Opaque or hybrid R interop also narrows what the optimizer can prove. Keep those
features at the edges of the program rather than inside numeric kernels.

### Hoist invariants when code stops being obvious

LICM can move some loop-invariant work automatically, but you still get better
results when expensive or shape-related values are computed once up front.

Good:

```rr
fn centered_sum(x) {
  let n = length(x)
  let mu = mean(x)
  let acc = 0.0
  for i in 1..n {
    acc = acc + (x[i] - mu)
  }
  acc
}
```

If a value is cheap but semantically important, still hoist it. RR is proof-based,
so explicit invariants are often worth more than trusting a future cleanup pass.

## Write RR Safely

### Use strict development settings

These modes catch common mistakes early:

| Setting | Use it for |
| --- | --- |
| `RR_STRICT_LET=1` | assignment to an undeclared name becomes a compile error |
| `RR_WARN_IMPLICIT_DECL=1` | warns when assignment would implicitly declare a variable |
| `RR_RUNTIME_MODE=debug` | enables the fuller runtime safety path |
| `RR_STRICT_INDEX_READ=1` | turns NA read-index behavior into a hard runtime error |

Recommended workflow:

1. Develop and test with `RR_STRICT_LET=1 RR_RUNTIME_MODE=debug`.
2. Add `RR_STRICT_INDEX_READ=1` when indexing bugs are plausible.
3. Measure with `RR_RUNTIME_MODE=release` only after correctness is stable.

> Recommended default during development:
> `RR_STRICT_LET=1 RR_RUNTIME_MODE=debug`

### Validate lengths and indices at boundaries

Inside a tight loop, RR prefers code that assumes clean shape facts.
At module or function boundaries:

- reject mismatched lengths before entering the hot loop
- avoid NA indices on write paths
- normalize optional/null inputs before the numeric kernel starts

### Keep side effects separate from numeric kernels

A loop that updates one output buffer is easier to optimize and easier to trust
than a loop that also:

- prints or logs
- mutates global state
- conditionally calls dynamic package helpers
- rewrites several aliasing containers at once

Split effectful orchestration from the compute kernel when possible.

### Check optimized and unoptimized behavior

For numerically important code, compare `-O0`, `-O1`, and `-O2` outputs on the
same inputs. RR is designed to preserve semantics, and mismatches should be
treated as a bug report or a signal that the code path relies on unsupported
dynamic behavior.

### Prefer explicit, newline-separated style

RR statements are newline-delimited:

- write one statement per line
- do not use trailing semicolons
- start the next statement on a new line instead of packing it onto the same line
- break long expressions across lines rather than chaining several statements together

This style is required by the parser and keeps control flow easier to review.

If you accidentally write a semicolon, RR now rejects it directly. For example:

```rr
let x = 1;
let y = 2
```

Typical error:

```text
semicolons are not supported; end the statement with a newline or '}'
```

## A Good Default Pattern

If you are writing performance-sensitive RR, start here:

1. Validate inputs once.
2. Compute lengths and invariant scalars once.
3. Run one clear loop or one direct builtin pipeline.
4. Keep helper calls pure.
5. Return the result without hidden state changes.

That structure matches what RR can currently optimize well and what humans can
review safely.

## Optimization Checklist

Before blaming the optimizer, check these first:

- Is the hot loop using one clear induction variable?
- Are reads and writes indexed directly from that variable?
- Did you hoist `length(...)`, `mean(...)`, and other invariant scalars once?
- Is the helper call pure enough to inline or reason about?
- Are dynamic features kept outside the numeric kernel?
- Does `-O2` output report vectorization or an obvious `VecSkip` reason?

## Related Docs

- [Language Reference](language.md)
- [Tachyon Engine](optimization.md)
- [Runtime and Errors](runtime-and-errors.md)
- [Compatibility and Limits](compatibility.md)
- [Configuration](configuration.md)
