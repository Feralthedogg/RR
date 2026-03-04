# Configuration

This page lists environment variables recognized by RR codebase.

## CLI and Output

- `RR_FORCE_COLOR`
  - Force ANSI colors in CLI/errors when `NO_COLOR` is not set.
- `NO_COLOR`
  - Disable ANSI colors.
- `RR_VERBOSE_LOG`
  - Force detailed compile progress traces in CLI logger.

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
- `RR_SELECTIVE_OPT_BUDGET` (default `false`)
  - Enable selective optimization under budget pressure (optimize scored subset of functions instead of all-or-nothing fallback).
- `RR_ADAPTIVE_IR_BUDGET` (default `false`)
  - Enable code-analysis-driven dynamic IR budget estimation.
- `RR_PROFILE_USE` (default unset)
  - Optional profile hints file for hot-function prioritization in selective budget mode.
  - Format: one entry per line, `function=count` (also accepts `function:count` or `function count`).
- `RR_INLINE_MAX_ROUNDS` (default `3`)
  - Max inter-procedural inline rounds.

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
