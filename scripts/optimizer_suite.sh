#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_BIN="${CARGO:-cargo}"
MODE="${1:-}"
FILTER="${RR_OPTIMIZER_SUITE_FILTER:-}"

if [[ -z "$MODE" ]]; then
  echo "usage: $0 <legality|heavy|all>" >&2
  exit 2
fi

source "$ROOT/scripts/lib/test_manifests.sh"
RUN_COUNT=0

run_cargo() {
  local label="$1"
  shift
  echo
  echo "== $label =="
  RUN_COUNT=$((RUN_COUNT + 1))
  (
    cd "$ROOT"
    CARGO_INCREMENTAL=0 "$@"
  )
}

matches_filter() {
  local name="$1"
  [[ -z "$FILTER" || "$name" == *"$FILTER"* ]]
}

run_test_binary() {
  local test_name="$1"
  if ! matches_filter "$test_name"; then
    return
  fi
  run_cargo "$test_name" "$CARGO_BIN" test --test "$test_name" --quiet
}

run_group() {
  local group_name="$1"
  shift
  local tests=("$@")
  echo
  echo "-- group: $group_name (${#tests[@]} tests) --"
  for test_name in "${tests[@]}"; do
    run_test_binary "$test_name"
  done
}

echo "== RR Optimizer Suite =="
echo "root: $ROOT"
echo "cargo: $CARGO_BIN"
echo "mode: $MODE"
echo "filter: ${FILTER:-<none>}"

case "$MODE" in
  legality)
    run_group legality "${RR_OPTIMIZER_LEGALITY_TESTS[@]}"
    ;;
  heavy)
    run_group heavy "${RR_OPTIMIZER_HEAVY_TESTS[@]}"
    ;;
  all)
    run_group legality "${RR_OPTIMIZER_LEGALITY_TESTS[@]}"
    run_group heavy "${RR_OPTIMIZER_HEAVY_TESTS[@]}"
    ;;
  *)
    echo "unknown optimizer suite mode: $MODE" >&2
    exit 2
    ;;
esac

if (( RUN_COUNT == 0 )); then
  echo "no optimizer tests matched filter '${FILTER:-<none>}'" >&2
  exit 2
fi

echo
echo "[ok] optimizer suite ($MODE) passed"
