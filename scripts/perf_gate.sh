#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_BIN="${CARGO:-cargo}"
FILTER="${RR_PERF_GATE_FILTER:-}"
RUN_COUNT=0

TESTS=(
  perf_regression_gate
  benchmark_vectorization
  commercial_determinism
)

echo "== RR Perf Gate =="
echo "root: $ROOT"
echo "cargo: $CARGO_BIN"
echo "filter: ${FILTER:-<none>}"

run_test() {
  local test_name="$1"
  shift || true
  if [[ -n "$FILTER" && "$test_name" != *"$FILTER"* ]]; then
    return
  fi
  echo
  echo "== $test_name =="
  RUN_COUNT=$((RUN_COUNT + 1))
  (
    cd "$ROOT"
    CARGO_INCREMENTAL=0 "$CARGO_BIN" test --test "$test_name" --quiet "$@"
  )
}

for test_name in "${TESTS[@]}"; do
  run_test "$test_name"
done

if [[ -z "$FILTER" || "example_perf_smoke" == *"$FILTER"* ]]; then
  echo
  echo "== example_perf_smoke =="
  RUN_COUNT=$((RUN_COUNT + 1))
  (
    cd "$ROOT"
    CARGO_INCREMENTAL=0 "$CARGO_BIN" test --test example_perf_smoke -- --ignored --nocapture
  )
fi

if (( RUN_COUNT == 0 )); then
  echo "no perf-gate tests matched filter '${FILTER:-<none>}'" >&2
  exit 2
fi

echo
echo "[ok] perf gate passed"
