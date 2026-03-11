# Configuration

This page is the environment and driver policy reference for RR.

## Precedence

RR resolves configuration in this order:

1. CLI flags
2. environment variables
3. built-in defaults

Normal runtime-injected artifacts then append compile-time policy assignments so
the emitted `.R` preserves the policy chosen at compile time.

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

- `RR_STRICT_LET`
  - default strict
  - undeclared assignment is a compile error unless explicitly disabled
- `RR_STRICT_ASSIGN`
  - alias control for strict-let behavior
- `RR_WARN_IMPLICIT_DECL`
  - warn when legacy implicit declaration is permitted

## Type and Native Backend

- `RR_TYPE_MODE`
  - `strict` or `gradual`
- `RR_NATIVE_BACKEND`
  - `off`, `optional`, or `required`
- `RR_NATIVE_LIB`
  - explicit shared library path for native helpers
- `RR_NATIVE_AUTOBUILD`
  - enable or disable runtime auto-build of `rr_native`

## Parallel Backend

- `RR_PARALLEL_MODE`
  - `off`, `optional`, or `required`
- `RR_PARALLEL_BACKEND`
  - `auto`, `r`, or `openmp`
- `RR_PARALLEL_THREADS`
  - worker count (`0` means auto)
- `RR_PARALLEL_MIN_TRIP`
  - minimum trip count before parallel dispatch is attempted

Runtime-injected artifacts embed the compile-time-resolved values for these
parallel knobs at the end of bootstrap, so the emitted `.R` keeps the compile
policy unless edited manually.

## Runtime Behavior

- `RR_RUNTIME_MODE`
  - `debug` or `release`
- `RR_STRICT_INDEX_READ`
  - turn NA read-index behavior into a hard runtime error
- `RR_FAST_RUNTIME`
  - force fast runtime path
- `RR_ENABLE_MARKS`
  - explicitly disable or enable `rr_mark`

## Optimizer Control

- `RR_VERIFY_EACH_PASS`
- `RR_VERIFY_DUMP_DIR`
- `RR_OPT_MAX_ITERS`
- `RR_MAX_FN_OPT_MS`
- `RR_ALWAYS_TIER_ITERS`
- `RR_MAX_FULL_OPT_IR`
- `RR_MAX_FULL_OPT_FN_IR`
- `RR_HEAVY_PASS_FN_IR`
- `RR_ALWAYS_BCE_FN_IR`
- `RR_BCE_VISIT_LIMIT`
- `RR_SELECTIVE_OPT_BUDGET`
- `RR_ADAPTIVE_IR_BUDGET`
- `RR_ENABLE_LICM`
- `RR_ENABLE_GVN`
- `RR_PROFILE_USE`
- `RR_INLINE_MAX_ROUNDS`
- `RR_VECTORIZE_TRACE`
- `RR_WRAP_TRACE`

Use these only when:

- reproducing an optimizer bug
- calibrating compile-time budgets
- investigating skipped vectorization

They are not the normal end-user entry surface.

## Inlining Policy

- `RR_DISABLE_INLINE`
- `RR_INLINE_MAX_BLOCKS`
- `RR_INLINE_MAX_INSTRS`
- `RR_INLINE_MAX_COST`
- `RR_INLINE_MAX_CALLSITE_COST`
- `RR_INLINE_MAX_CALLER_INSTRS`
- `RR_INLINE_MAX_TOTAL_INSTRS`
- `RR_INLINE_MAX_UNIT_GROWTH_PCT`
- `RR_INLINE_MAX_FN_GROWTH_PCT`
- `RR_INLINE_ALLOW_LOOPS`

These control inlining eligibility and growth limits.

## Performance Gates

- `RR_PERF_GATE_MS`
- `RR_PERF_O2_O1_RATIO`

These are test-budget knobs, not general optimization controls.

## Test Harness Override

- `RRSCRIPT`
  - override the R executable used by integration tests and local harnesses

## Related Manuals

- [CLI Reference](cli.md)
- [Runtime and Error Model](runtime-and-errors.md)
- [Testing and Quality Gates](testing.md)
