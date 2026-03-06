# Configuration

This page lists environment variables recognized by RR codebase.

## CLI and Output

- `RR_FORCE_COLOR`
  - Force ANSI colors in CLI/errors when `NO_COLOR` is not set.
- `NO_COLOR`
  - Disable ANSI colors.
- `RR_VERBOSE_LOG`
  - Force detailed compile progress traces in CLI logger.
- `RR_SLOW_STEP_MS` (default `3000`)
  - When a compile stage runs longer than this threshold (ms), print an automatic slow-stage progress line.
  - During Tachyon optimization, this also enables tier progress traces (`always`/`heavy`/`de-ssa`) once the slow threshold is reached.
- `RR_SLOW_STEP_REPEAT_MS` (default `6000`)
  - Repeat interval (ms) for slow-stage progress lines after the first threshold is reached.

## Language Strictness

- `RR_STRICT_LET`
  - Forbid implicit declaration through assignment (`<-` / `=` to undeclared name).
- `RR_STRICT_ASSIGN`
  - Alias trigger for strict-let behavior.

## Type and Native Backend

- `RR_TYPE_MODE` (`strict` | `gradual`, default `strict`)
  - Controls strict type-checking policy.
- `RR_NATIVE_BACKEND` (`off` | `optional` | `required`, default `off`)
  - Controls intrinsic backend strategy.
- `RR_NATIVE_LIB`
  - Optional shared library path used by native intrinsic dispatch.
- `RR_NATIVE_AUTOBUILD` (default `1`)
  - If `RR_NATIVE_LIB` is unset, runtime tries to auto-discover `rr_native` library near the generated script and, if missing, attempts `R CMD SHLIB native/rr_native.c` into `target/native`.
  - Set `0` to disable auto-build and force fallback/required-fail behavior.

## Parallel Backend

- `RR_PARALLEL_MODE` (`off` | `optional` | `required`, default `off`)
  - Controls whether parallel paths may be used and whether fallback is allowed.
- `RR_PARALLEL_BACKEND` (`auto` | `r` | `openmp`, default `auto`)
  - Selects backend preference (`auto` tries OpenMP native first, then R backend).
- `RR_PARALLEL_THREADS` (default `0`)
  - Parallel worker count (`0` means auto-detect).
- `RR_PARALLEL_MIN_TRIP` (default `4096`)
  - Minimum vector length before attempting parallel dispatch.

## Optimizer Control

- `RR_VERIFY_EACH_PASS` (default `false`)
  - Run MIR verifier after each pass.
- `RR_OPT_MAX_ITERS` (default `24`)
  - Max per-function optimization iterations.
- `RR_MAX_FN_OPT_MS` (default `250`)
  - Soft per-function optimization time budget (milliseconds).
- `RR_ALWAYS_TIER_ITERS` (default `2`)
  - Max iterations for always-on low-cost Tier-A passes (`simplify + light SCCP + DCE`).
- `RR_MAX_FULL_OPT_IR` (default `2500`)
  - Program-level full-optimization IR-size threshold.
- `RR_MAX_FULL_OPT_FN_IR` (default `900`)
  - Function-level full-optimization IR-size threshold.
- `RR_HEAVY_PASS_FN_IR` (default `650`)
  - Function IR-size threshold above which heavy structural passes are budgeted.
- `RR_ALWAYS_BCE_FN_IR` (default `RR_HEAVY_PASS_FN_IR`)
  - Upper IR-size limit for the bounded always-tier BCE sweep.
  - Increase this to allow guard elimination on larger functions (may increase compile time significantly).
- `RR_BCE_VISIT_LIMIT` (default `200000`)
  - Maximum recursive node visits per function in BCE nested-index safety traversal.
  - Lower this value to bound compile time on very large functions at the cost of fewer guard eliminations.
- `RR_SELECTIVE_OPT_BUDGET` (default `true`)
  - Enable selective optimization under budget pressure (optimize scored subset of functions instead of all-or-nothing fallback).
- `RR_ADAPTIVE_IR_BUDGET` (default `false`)
  - Enable code-analysis-driven dynamic IR budget estimation.
- `RR_PROFILE_USE` (default unset)
  - Optional profile hints file for hot-function prioritization in selective budget mode.
  - Format: one entry per line, `function=count` (also accepts `function:count` or `function count`).
- `RR_INLINE_MAX_ROUNDS` (default `3`)
  - Max inter-procedural inline rounds.
- `RR_VECTORIZE_TRACE` (default `false`)
  - Emit per-loop vectorization trace logs from `v_opt`.
  - Intended for compiler development and regression debugging, not normal end-user use.
  - Shows loop headers, IV origin, skip reasons, and matcher/materialization reject details.

## Inlining Policy

- `RR_DISABLE_INLINE` (default `false`)
- `RR_INLINE_MAX_BLOCKS` (default `24`)
- `RR_INLINE_MAX_INSTRS` (default `160`)
- `RR_INLINE_MAX_COST` (default `220`)
- `RR_INLINE_MAX_CALLER_INSTRS` (default `480`)
- `RR_INLINE_MAX_TOTAL_INSTRS` (default `900`)
- `RR_INLINE_ALLOW_LOOPS` (default `false`)
- `RR_INLINE_MAX_CALLSITE_COST` (default `240`)
- `RR_INLINE_MAX_UNIT_GROWTH_PCT` (default `25`)
- `RR_INLINE_MAX_FN_GROWTH_PCT` (default `35`)

## Test and CI Performance Gates

- `RR_PERF_GATE_MS` (default `12000`)
  - O2 compile-time budget for perf gate test.
- `RR_PERF_O2_O1_RATIO` (default `12`)
  - Allowed O2/O1 slowdown ratio in perf gate test.

## Rscript Override Notes

Integration tests use `RRSCRIPT` in test harnesses to override the executable used for direct R calls.
Main RR CLI runtime path currently invokes `Rscript` directly.
