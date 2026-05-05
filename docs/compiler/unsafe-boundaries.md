# Unsafe Boundaries

This page records RR's current unsafe boundary. It covers two different things:

- Rust `unsafe` inside the compiler implementation
- RR source `unsafe r { ... }`, which emits raw R as an escape hatch

They are related only by policy: both must make the trust boundary explicit.
User-authored `unsafe r` does not permit unsound Rust compiler behavior, and
Rust `unsafe` in the compiler does not make RR source code implicitly unsafe.

## Compiler Rust Unsafe

RR does not aim for a zero-`unsafe` compiler today. The current claim is narrower:
Rust `unsafe` is allowed only when it is necessary, locally justified, and
covered by audit or targeted tests.

Current implementation categories:

- `src/main/compile/cache.rs`
  - Uses scoped process-environment mutation for the `--cold` compile cache path.
  - The mutation is limited to one CLI compile invocation and restored by
    `ScopedCompileCacheOverride`.
  - The reason this remains `unsafe` is that Rust has no safe process-local
    scoped environment override; process environment is global and may be read by
    other threads.
- `tests/common/mod.rs`
  - Provides test-only environment mutation helpers.
  - Callers must hold `env_lock()`, which serializes test-process environment
    mutation.
- `src/mir/opt/poly/isl.rs`
  - Owns ISL/libc FFI calls for polyhedral scheduling.
  - Raw pointers, C strings, ownership transfer, and explicit frees are kept in
    small wrappers; null or failed ISL states become optimizer misses rather than
    correctness-affecting behavior.
  - Conditional-validity candidates are recorded but not passed directly to ISL
    in-process today. Some linked libisl builds can fail below Rust's error
    boundary for those maps, so RR keeps scheduling hermetic by using the normal
    constraints and recording a fallback hint.
  - This is a conditional-validity fallback policy, not a semantic dependency:
    the in-process optimizer may skip the conditional ISL path, but it must not
    change program meaning based on whether that optional path is available.

## Required Comment Shape

Every Rust `unsafe` block or unsafe FFI declaration must have an adjacent
`// SAFETY:` comment. The comment must state:

- the local invariant that makes the operation valid
- why safe alternatives are insufficient or unavailable
- how the scope is kept narrow
- what restores or releases any global or external state

Preferred shape:

```rust
// SAFETY: No safe alternatives exist for scoped process-env overrides;
// this path mutates the process environment only until the guard restores
// the previous value.
unsafe {
    std::env::set_var(key, value);
}
```

Avoid separating the `// SAFETY:` line from the `unsafe` block with unrelated
comments or code. The audit expects the safety rationale to be directly visible
at the block.

## Audit Commands

Run the diff-scoped check before landing unsafe-related changes:

```bash
mkdir -p .artifacts/ci
RR_SEMANTIC_AUDIT_SCOPE=diff RR_SEMANTIC_AUDIT_PROFILE=scan-only \
  bash scripts/ci_contributing_semantic_audit.sh | tee .artifacts/ci/semantic-audit.log >/dev/null
perl scripts/check_new_warnings.pl \
  --baseline policy/warning_baseline.txt \
  --log .artifacts/ci/semantic-audit.log
```

For a repository-wide check:

```bash
mkdir -p .artifacts/ci
RR_SEMANTIC_AUDIT_SCOPE=all RR_SEMANTIC_AUDIT_PROFILE=scan-only \
  bash scripts/ci_contributing_semantic_audit.sh | tee .artifacts/ci/semantic-audit-all.log >/dev/null
```

## RR Source `unsafe r`

`unsafe r { ... }` is a user-facing raw R escape hatch. It is not a compiler
implementation `unsafe` block.

The compiler contract for `unsafe r` is:

- the block is statement-only and has no RR return value
- RR preserves the raw R body verbatim at the statement position
- RR parameters and locals that are emitted as R bindings are visible to the raw
  R body through the generated function's normal R lexical scope
- the containing MIR function is marked opaque interop
- post-emission raw text cleanup is skipped for that function
- optimization is conservative around values that may be affected by the raw R
  body
- the R code is not sandboxed and may mutate R-visible state
- RR reloads all visible generated-function-frame locals after a read/write raw
  R block, because the block may assign through ordinary R lexical scope
- `unsafe r(read) { ... }` is available for raw R that reads RR-visible bindings
  but does not assign to them; it keeps the containing function out of the
  opaque-interop conservative tier

`unsafe r` does not use template substitution today. Write the RR local or
parameter name directly in the raw R body, for example `energy <- sum(values *
values)`. Compiler-generated temporaries and mangled trait/generic helper names
are not part of the stable capture surface.

`unsafe r(read)` is a promise to the optimizer. RR still emits the raw R block
in statement order and treats the block as a raw-text rewrite barrier, but it
does not reload RR locals after the block and does not mark the containing MIR
function opaque. Assigning to RR locals from inside `unsafe r(read)` is outside
the stable language contract; use plain `unsafe r { ... }` when raw R writes must
be observed by later RR statements.

Closure/capture caveat: lambda capture discovery conservatively scans raw R
identifiers in unsafe blocks so currently lifted closures capture RR locals that
the raw R body names. This is still scoped to generated function-frame bindings.
If RR later adds deeper nested-environment lowering, the `unsafe r` invalidation
and capture rules must be audited again so captured variables are handled
explicitly.

Use `unsafe r` only when direct or opaque R interop cannot express the operation.
Normal package calls should stay on the documented R interop path.

## Remaining Technical Debt

The cold compile cache override is policy-compliant but still not ideal because
process-environment mutation is global. The preferred long-term direction is to
thread the temporary cache root through compile configuration/context instead of
using an environment override.

The ISL boundary is also intentionally isolated. New ISL calls should stay in
the same wrapper area instead of spreading raw pointer logic through optimizer
passes.
