# Configuration

This page is the environment and driver policy reference for RR.

## Reading This Page

Treat this page like a compiler-driver and runtime-policy table:

- CLI flags are the primary user-facing surface
- environment variables are mainly for logging, runtime hooks, cache/debug
  controls, and compatibility shims
- many optimizer knobs are intentionally expert-only

## Precedence

RR resolves compilation policy in this order:

1. explicit CLI flags or explicit API config values
2. built-in defaults

Ambient environment variables do not select compiler type/native/parallel
policy anymore. Runtime-injected artifacts embed the compile-time-resolved
backend and parallel settings directly.

## Driver and Output

- `RR_FORCE_COLOR`
  - force ANSI color output
- `NO_COLOR`
  - disable ANSI color output
- `RR_VERBOSE_LOG`
  - force detailed compile progress traces
- `RR_QUIET_LOG`
  - suppress normal pipeline progress output
- `RR_SLOW_STEP_MS`
  - threshold for slow-stage progress logging
- `RR_SLOW_STEP_REPEAT_MS`
  - repeat interval for slow-stage progress logging

## Language Strictness

Strict-let and implicit-declaration behavior are now explicit compile inputs,
not ambient environment-driven policy.

- `--strict-let on|off`
  - control whether assignment to an undeclared name is rejected
- `--warn-implicit-decl on|off`
  - warn when relaxed implicit declaration is allowed

## Type and Native Backend

Use CLI flags or explicit API config to choose compile-time type/native policy.

- `RR_NATIVE_LIB`
  - explicit shared library path for native helpers at runtime
- `RR_NATIVE_AUTOBUILD`
  - enable or disable runtime auto-build of `rr_native`

## Parallel Backend

Use CLI flags or explicit API config to choose compile-time parallel policy.

Runtime-injected artifacts embed the compile-time-resolved values for these
parallel knobs directly, so emitted `.R` keeps the compile policy unless edited
manually.

## Runtime Behavior

- `RR_RUNTIME_MODE`
  - `debug` or `release`
- `RR_STRICT_INDEX_READ`
  - turn NA read-index behavior into a hard runtime error
- `RR_FAST_RUNTIME`
  - force fast runtime path
- `RR_ENABLE_MARKS`
  - explicitly disable or enable `rr_mark`
- `RR_VECTOR_FALLBACK_BASE_TRIP`
  - runtime threshold for helper-heavy vector-to-scalar fallback decisions
- `RR_VECTOR_FALLBACK_HELPER_SCALE`
  - runtime helper-cost multiplier used by vector fallback heuristics

## Incremental and Cache

- `RR_INCREMENTAL_CACHE_DIR`
  - override the incremental cache root directory

## Optimizer Control

- `RR_VERIFY_EACH_PASS`
- `RR_VERIFY_DUMP_DIR`
- `RR_DEBUG_FNIR`
  - dump selected optimized `FnIR` bodies while debugging optimizer/codegen issues
- `RR_DEBUG_RAW_R_PATH`
  - write the pre-peephole emitted R artifact to a file before final cleanup/remap
- `RR_PULSE_JSON_PATH`
  - write `TachyonPulseStats` JSON diagnostics for a compile to the given path
- `RR_VECTORIZE_TRACE`
- `RR_VOPT_PROOF`
  - enable proof-certified vectorization rewrites explicitly
- `RR_VOPT_PROOF_TRACE`
  - trace proof certification and proof-apply decisions
- `RR_WRAP_TRACE`

Use these only when:

- reproducing an optimizer bug
- investigating skipped vectorization

They are not the normal end-user entry surface.

## Inlining Policy

- `RR_INLINE_MAX_BLOCKS`
- `RR_INLINE_MAX_INSTRS`
- `RR_INLINE_MAX_COST`
- `RR_INLINE_MAX_CALLSITE_COST`
- `RR_INLINE_MAX_CALLER_INSTRS`
- `RR_INLINE_MAX_TOTAL_INSTRS`
- `RR_INLINE_MAX_UNIT_GROWTH_PCT`
- `RR_INLINE_MAX_FN_GROWTH_PCT`
- `RR_INLINE_ALLOW_LOOPS`
- `RR_INLINE_LOCAL_ROUNDS`

These control inlining eligibility and growth limits.

## Related Manuals

- [CLI Reference](cli.md)
- [Runtime and Error Model](runtime-and-errors.md)
- [Testing and Quality Gates](testing.md)

## Performance Gates

- `RR_PERF_GATE_MS`
- `RR_PERF_O2_O1_RATIO`

These are test-budget knobs, not general optimization controls.

## Test Harness Override

- `RRSCRIPT`
  - override the R executable used by integration tests and local harnesses
