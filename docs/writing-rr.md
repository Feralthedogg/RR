# Writing RR for Performance and Safety

This page is the performance and safety guide for RR authors.

RR's optimizer is strongest when code stays close to explicit numeric kernels,
canonical loops, and predictable dataflow. It is intentionally conservative when
proofs are missing, so "easy to reason about" code is usually both faster and safer.

## Audience

Read this page when:

- generated R is slower or stranger than expected
- a loop did not vectorize
- you want code that is both optimization-friendly and review-friendly

This is not a syntax reference. Use [Language Reference](language.md) for that.

## Contract

Tachyon is proof-first, with a pattern-driven vectorization/reduction layer on
top of more general MIR simplification passes.

That means:

- some optimization comes from general scalar/dataflow passes such as SCCP, GVN,
  simplification, DCE, BCE, LICM, inlining, and de-SSA cleanup
- the most aggressive loop rewrites still depend on recognizable source shape
  once those earlier passes have simplified the MIR enough
- missing proof usually means a deliberate skip, not a near miss
- the best source style is the one that makes dataflow obvious

The practical consequence is that “compiler-friendly” RR usually also reads more
like a numerical kernel specification and less like dynamic metaprogramming,
but that is not because Tachyon only does pattern matching. It is because the
general optimizer and the pattern-based loop optimizer both benefit from
explicit, stable dataflow.

## Why This Style Optimizes Well

Tachyon does not try to guess arbitrary intent from dynamic code. In practice it
works in two layers:

- general MIR cleanup and scalar/dataflow optimization
- pattern-based loop and container rewrites when the remaining structure is
  simple enough to certify

That is why source shape still matters so much: the early passes can simplify
and sharpen facts, but the stronger loop rewrites still need a safe,
recognizable shape at the end of that pipeline.

Think of that source shape as an optimization fingerprint:

| Source fingerprint | Tachyon can usually prove |
| --- | --- |
| one induction variable + one obvious trip count | loop canonicalization, bounds-check elimination, vector-style lowering |
| direct `x[i]` / `y[i]` reads and one direct `out[i]` write | map/reduction recognition |
| one scalar accumulator updated once per iteration | reduction rewrites |
| pure, small helper calls | inlining, SCCP, GVN/CSE, vectorized call-map |
| loop-invariant shape/scalar facts computed once | LICM and simpler safety proofs |

There is not yet a per-loop standalone "fingerprint checker" command for RR
authors. In practice, use the optimizer's own feedback:

- normal `-O1` / `-O2` compile output for `Vectorized`, `Reduced`, `Simplified`, and `VecSkip`
- `RR_VECTORIZE_TRACE=1` when you need per-loop reject reasons
- `--no-incremental` when you are debugging optimizer output on a stable input path;
  the normal CLI default is incremental `auto`, so unchanged inputs may reuse a
  cached artifact instead of rebuilding the hot loop you are inspecting

If you are validating RR itself rather than one RR program, the nearest
project-level checks are:

- `bash scripts/optimizer_suite.sh legality`
- `bash scripts/optimizer_suite.sh heavy`
- `bash scripts/test_tier.sh tier1`
- `make library-package-suite`

Canonical loops matter mainly because they are the main entry point to RR's
vectorization pipeline. Today that usually means rewriting scalar loops into
bulk vector helpers or builtin vector operations, which is RR's current path to
SIMD-like execution and lower interpreter overhead.

## The Short Version

- Prefer straight-line numeric code, canonical loops, and pure helpers.
- Keep indexing, lengths, and reduction state obvious.
- Avoid dynamic runtime features in hot paths.
- Remember that helper-heavy vector loops may intentionally keep a scalar fallback at runtime.
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

RR can also optimize selected partial-range kernels when the slice boundaries are
explicit and all reads/writes stay tied to the same induction variable.

Good:

```rr
fn tail_abs(x) {
  let n = length(x)
  let out = x
  for i in 2..n {
    out[i] = abs(x[i])
  }
  out
}
```

Also good:

```rr
fn interior_step(x) {
  let y = seq_len(length(x))
  let i = 1L
  while (i < length(x)) {
    y[i] = x[i] + 10L
    i = i + 1L
  }
  y
}
```

These shapes typically lower through slice-assignment helpers instead of a
scalar `repeat` loop. They still benefit from the same discipline:

- keep the loop bound obvious
- keep the index direct
- avoid rebuilding `i` through aliases or indirect lookup tables

### Simple fills and multi-output loops can still optimize

RR is not limited to a single `out[i] = f(x[i])` map. The optimizer can also
handle a few nearby shapes when the writes remain direct and non-aliasing:

- invariant fills such as `y[i] = 0`
- multiple direct destinations such as `y[i] = x[i] + 1` and `z[i] = x[i] * 2`
- a loop-carried scalar that is only tracking the last value written to one
  destination, for example `last = z[i]`

Good:

```rr
fn pair_kernel(x) {
  let n = length(x)
  let y = seq_len(n)
  let z = seq_len(n)
  let last = 0

  for i in 1..n {
    y[i] = x[i] + 1
    z[i] = x[i] * 2
    last = z[i]
  }

  print(last)
  z
}
```

This is still proof-friendly because:

- every destination is written at one obvious index
- the scalar state is just a shadow of one destination element
- there are no unrelated side effects inside the loop

Once the loop starts mixing logging, package calls, alias-heavy writes, or
indirect indices, RR becomes conservative again.

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

### Make numeric intent obvious

RR now preserves the integer / floating-point boundary more aggressively, so it is
worth writing the operation you actually mean instead of relying on later cleanup.

- use `/` when you want floating semantics
- use `%` when you want integer modulo semantics; generated R emits this as `%%`
- expect `sum(int-vector)` to stay integer when RR can prove the input is integer
- expect `mean`, `log`, `log10`, `log2`, `sqrt`, `atan2`, and similar math builtins to widen to floating-point
- `abs`, `pmax`, and `pmin` keep integer element type when all proven numeric inputs are integer

This matters because the strict type solver now feeds more precise facts into:

- branch-condition checking
- call-signature checking
- backend intrinsic selection
- vector shape and length preservation
- matrix/container shape preservation for typed code

Good:

```rr
fn score(counts: vector<int>, base: int) {
  let clipped = pmax(counts, base)
  let total = sum(clipped)
  let ratio = total / 3
  ratio
}
```

Here RR can keep `clipped` and `total` on the integer side, and only widen at
the division.

Less helpful:

```rr
fn score(counts, base) {
  let clipped = pmax(counts, base)
  let total = sum(clipped)
  total / 3
}
```

This still compiles, but the solver has to recover more facts from use sites and
will fall back to unknown/dynamic behavior sooner if other hints are missing.

If you have matrix-shaped code, say so explicitly with `matrix<T>` instead of
only `vector<T>`. RR now preserves that distinction internally, which improves:

- matrix builtin inference
- downstream strict checking
- length/shape preservation through typed helper calls

That also applies to matrix summary helpers. When RR can see calls such as
`rowSums`, `colSums`, `crossprod`, or `tcrossprod`, it now keeps matrix/vector
intent in the type layer instead of dropping back to unknown immediately.

Matrix shape algebra is also more precise than before. Calls and operators such
as `t`, `diag`, `rbind`, `cbind`, and `%*%` now preserve matrix-shaped terms
more accurately, so RR can carry row/column intent further into strict checks
and specialization instead of collapsing back to a generic matrix immediately.

Typed matrix parallel wrappers are available, but the contract is intentionally
narrow. RR only wraps straight-line shape-preserving matrix kernels there. In
the R fallback runtime, those kernels are split by column blocks and rejoined
with the original `dim` and `dimnames`. Shape-sensitive matrix transforms such
as transpose-like behavior stay on the ordinary non-wrapper path.

Likewise, nested container hints such as `list<box<float>>` and typed dataframe
schemas are worth keeping when you have them. RR still handles dataframe logic
conservatively at optimization time, but the type layer no longer has to discard
that structure immediately. In particular, named dataframe field access can now
refine to the matching column type when the schema is visible to RR.

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
- 3D scalar indexing is supported
- selected 3D array maps, expr-maps, conditional maps, call-maps, scatter-maps, and sum/prod/min/max reductions can optimize when one axis is the loop induction variable and the remaining index expressions stay loop-shaped or loop-invariant in a way Tachyon can prove
- aligned 3D slice reads such as `a[i, j0, k0]` work best when `j0` and `k0` stay visibly loop-invariant
- 3D expr-map RHS can also use gather-style reads such as `a[idx_i[i], idx_j[i], k0]` as long as the loop still writes one obvious single-axis slice
- the same applies to 3D branch kernels: `if (a[idx_i[i], idx_j[i], k0] > t) out[i, j0, k0] = ...` can optimize if the destination slice is still obvious
- vector-safe 3D call kernels such as `out[i, j0, k0] = pmax(a[idx_i[i], idx_j[i], k0], b[idx_i[i], idx_j[i], k0])` can optimize under the same constraint
- multiple 3D slice destinations in the same loop can optimize too when each write stays direct and independent, for example `y[i, j0, k0] = ...` and `z[i, j1, k1] = ...`
- 3D scatter writes such as `out[idx_i[i], idx_j[i], k0] = value[i]` can also optimize when the write still comes from one obvious loop and the RHS does not read back from the destination
- 3D reductions such as `acc = acc + a[idx_i[i], idx_j[i], k0]` can optimize too when the accumulator is the only loop-carried state
- single-axis 3D shift/recur kernels such as `out[i, j0, k0] = src[i + 1, j0, k0]` and `out[i, j0, k0] = out[i - 1, j0, k0] + c` can also optimize when the axis and fixed coordinates stay obvious
- arbitrary 3D traversal is still much less optimization-friendly than canonical 1D/2D shapes

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

Use the language/runtime checks for day-to-day development, and add the
compiler-side verification knobs when you are debugging optimization or
incremental behavior.

Under the default strict declaration rules, the first assignment to a local or
top-level name should use `let`, including traditional forms such as
`let main <- function() { ... }`.

Language and runtime safety settings:

| Setting | Use it for |
| --- | --- |
| `--type-mode strict` | keep static typing in the stricter mode during development; this is the normal default |
| `--strict-let on` | explicit spelling of the default strict-let behavior: assignment to an undeclared name is a compile error |
| `RR_ALLOW_LEGACY_IMPLICIT_DECL=1 --strict-let off --warn-implicit-decl on` | temporary RR 1.x migration mode for code that still relies on implicit declaration |
| `RR_RUNTIME_MODE=debug` | enables the fuller runtime safety path |
| `RR_STRICT_INDEX_READ=1` | turns NA read-index behavior into a hard runtime error |

Compiler verification settings:

| Setting | Use it for |
| --- | --- |
| `RR_VERIFY_EACH_PASS=1` | run the MIR verifier after each optimization pass; use this when `-O1` or `-O2` behavior looks suspicious |
| `--strict-incremental-verify` | rebuild and compare against any reused incremental artifact instead of trusting the cache blindly |
| `--no-incremental` | force a fresh compile when you want to inspect the current optimizer output rather than a cached result |
| `--cold` | bypass warm compile caches for one run without clearing the normal cache root |

Recommended workflow:

1. Develop and test in `--type-mode strict` with `RR_RUNTIME_MODE=debug`.
2. Add `RR_STRICT_INDEX_READ=1` when indexing bugs are plausible.
3. Add `RR_VERIFY_EACH_PASS=1` when investigating optimizer regressions.
4. Use `--strict-incremental-verify` when you are validating incremental reuse.
5. Use `--no-incremental` when you care about the exact emitted R from the current source tree.
6. Use `--cold` when you want a one-off cold-path measurement without blowing away the warm cache.
7. Measure with `RR_RUNTIME_MODE=release` only after correctness is stable.
8. When benchmarking helper-heavy vector loops, pin the runtime profitability knobs so you know which path you are measuring.

> Recommended default during development:
> `RR_RUNTIME_MODE=debug`

Examples:

```bash
RR_RUNTIME_MODE=debug cargo run -- example/tesseract.rr -O0 --type-mode strict
```

```bash
RR_RUNTIME_MODE=debug RR_VERIFY_EACH_PASS=1 \
  cargo run -- example/tesseract.rr -O2 --type-mode strict --no-incremental
```

```bash
cargo run -- example/tesseract.rr -O2 --type-mode strict --strict-incremental-verify
```

```bash
RR_VECTOR_FALLBACK_BASE_TRIP=0 RR_VECTOR_FALLBACK_HELPER_SCALE=0 \
  cargo run -- example/tesseract.rr -O2 --type-mode strict --no-incremental
```

If you are porting older RR code that still relies on implicit declaration, use:

```bash
RR_ALLOW_LEGACY_IMPLICIT_DECL=1 \
  cargo run -- example/tesseract.rr -O0 --type-mode strict --strict-let off --warn-implicit-decl on
```

The last two commands serve different purposes:

- `--no-incremental` is for "show me what the optimizer emits right now"
- `--strict-incremental-verify` is for "prove the incremental cache matches a fresh rebuild"

`--cold` is different from both:

- it keeps the normal cache on disk
- it runs one compile against an empty temporary cache root
- it is the right flag when you want a cold-path timing or behavior check

Helper-heavy vector call kernels have an extra runtime decision point:

- RR may emit `rr_call_map_whole_auto(...)` or `rr_call_map_slice_auto(...)` instead of a direct helper-heavy vector call.
- That guard keeps the vector form for large trip counts, but can use a scalar R loop when the helper overhead would dominate on small inputs.
- Use `RR_VECTOR_FALLBACK_BASE_TRIP` and `RR_VECTOR_FALLBACK_HELPER_SCALE` to pin that choice while benchmarking.

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

A small loop-carried scalar shadow is usually fine if it is obviously derived
from the current destination element. Arbitrary side effects are not.

Fully-typed vector helpers are also a good boundary for RR's parallel wrapper
path. If RR can prove the sliced vector inputs from explicit hints or from a
straight-line typed binding chain, and the helper stays as a slice-stable
expression kernel, RR can emit an implementation helper plus a parallel wrapper
automatically. In practice, small numeric helpers such as:

```rr
fn fused(a: vector<float>, b: vector<float>) -> vector<float> {
  return (a + b) * 0.5
}
```

are a better fit than reduction-style helpers such as `mean(abs(x))`, because
the wrapper can safely split and reassemble the former.

When a workload is really a fixed whole-vector pipeline, try to keep the stages
explicit and shape-stable so RR can fuse them into a backend-aware helper later.
The current `signal_pipeline` benchmark is the reference example:

- one stable input shape
- one straight-line sequence of elementwise stages
- no side effects between stages
- no alias-heavy writes inside the pipeline

In practice, code shaped like this:

```rr
score = pmax(abs(x * 0.65 + y * 0.35 - 0.08), 0.05)
clean = ifelse(score > 0.4, sqrt(score + 0.1), score * 0.55 + 0.03)
x = clean + y * 0.15
y = score * 0.8 + clean * 0.2
```

is a much better fused-native candidate than code that spreads the same work
across dynamic helper dispatch, logging, or process-level orchestration.

Also treat R-process parallelism as a separate tool, not the default answer.
For this kind of dense numeric pipeline, fused native/OpenMP lowering can win,
while `parallel::mclapply` fan-out may be slower than even plain emitted R.

But do not overgeneralize that rule to every hot loop. The current benchmark
set now shows three distinct outcomes:

- `signal_pipeline` and `orbital_sweep` benefit a lot from fused backend helpers
- `vector_fusion` is already so compact in emitted R that backend fusion loses
- `bootstrap_resample` is gather-heavy enough that a fused helper only breaks even or regresses

So the best authoring rule is not “always force backend fusion,” but “write the
kernel so RR has the option to fuse it when the shape and cost model both say
that is worthwhile.”

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
- If the loop intentionally skips a prefix/suffix, is that slice range explicit?
- Did you hoist `length(...)`, `mean(...)`, and other invariant scalars once?
- Is the helper call pure enough to inline or reason about?
- Are dynamic features kept outside the numeric kernel?
- Does `-O2` output report vectorization or an obvious `VecSkip` reason?

## Related Docs

- [Language Reference](language.md)
- [Compatibility and Limits](compatibility.md)
- [Configuration](configuration.md)
