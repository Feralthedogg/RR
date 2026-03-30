#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_BIN="${CARGO:-cargo}"
TIER="${1:-}"
FILTER="${RR_TEST_TIER_FILTER:-}"

if [[ -z "$TIER" ]]; then
  echo "usage: $0 <tier0|tier1|tier2-main|tier2-differential>" >&2
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
  shift || true
  if ! matches_filter "$test_name"; then
    return
  fi
  run_cargo "$test_name" "$CARGO_BIN" test --test "$test_name" --quiet "$@"
}

run_exact_test() {
  local spec="$1"
  local test_name="${spec%%::*}"
  local exact_name="${spec#*::}"
  if ! matches_filter "$spec" && ! matches_filter "$test_name"; then
    return
  fi
  run_cargo "$test_name :: $exact_name" "$CARGO_BIN" test --test "$test_name" "$exact_name" -- --exact
}

run_lib_tests() {
  if ! matches_filter "lib"; then
    return
  fi
  run_cargo "lib" "$CARGO_BIN" test --lib --quiet
}

list_unassigned_tests() {
  python3 - "$ROOT" "$FILTER" \
    "${RR_TIER0_FAST_TESTS[@]}" \
    "__SPLIT__" \
    "${RR_LIBRARY_PACKAGE_TESTS[@]}" \
    "__SPLIT__" \
    "${RR_LIBRARY_PACKAGE_EXACT_TESTS[@]}" \
    "__SPLIT__" \
    "${RR_TIER2_SPECIAL_TESTS[@]}" \
    "__SPLIT__" \
    "${RR_PERF_GATE_TESTS[@]}" \
    "__SPLIT__" \
    "${RR_OPTIMIZER_LEGALITY_TESTS[@]}" \
    "__SPLIT__" \
    "${RR_OPTIMIZER_HEAVY_TESTS[@]}" \
    "__SPLIT__" \
    "${RR_TIER2_MAIN_TESTS[@]}" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
flt = sys.argv[2]
args = sys.argv[3:]
parts = []
current = []
for item in args:
    if item == "__SPLIT__":
        parts.append(current)
        current = []
    else:
        current.append(item)
parts.append(current)

tier0 = set(parts[0])
tier1 = set(parts[1])
tier1_exact = {item.split("::", 1)[0] for item in parts[2]}
tier2_special = set(parts[3])
perf_gate = set(parts[4])
optimizer_legality = set(parts[5])
optimizer_heavy = set(parts[6])
tier2_main = set(parts[7])
excluded = tier0 | tier1 | tier1_exact | tier2_special | perf_gate | optimizer_legality | optimizer_heavy | tier2_main

for p in sorted((root / "tests").glob("*.rs")):
    stem = p.stem
    if stem in excluded:
        continue
    if flt and flt not in stem:
        continue
    print(stem)
PY
}

validate_manifest_assignments() {
  local unassigned
  unassigned="$(list_unassigned_tests)"
  if [[ -n "$unassigned" ]]; then
    echo "unassigned tests found in tier manifest:" >&2
    printf '%s\n' "$unassigned" >&2
    exit 2
  fi
}

echo "== RR Test Tier Runner =="
echo "root: $ROOT"
echo "cargo: $CARGO_BIN"
echo "tier: $TIER"
echo "filter: ${FILTER:-<none>}"

validate_manifest_assignments

case "$TIER" in
  tier0)
    run_lib_tests
    for test_name in "${RR_TIER0_FAST_TESTS[@]}"; do
      run_test_binary "$test_name"
    done
    ;;
  tier1)
    run_cargo "library-package-suite" env RR_PACKAGE_SUITE_FILTER="$FILTER" bash ./scripts/library_package_suite.sh
    ;;
  tier2-main)
    for test_name in "${RR_TIER2_MAIN_TESTS[@]}"; do
      run_test_binary "$test_name"
    done
    ;;
  tier2-differential)
    if matches_filter "random_differential"; then
      run_cargo "random_differential" env \
        RR_RANDOM_DIFFERENTIAL_COUNT="${RR_RANDOM_DIFFERENTIAL_COUNT:-12}" \
        RR_RANDOM_DIFFERENTIAL_SEED="${RR_RANDOM_DIFFERENTIAL_SEED:-0xA11CE5EED55AA11C}" \
        "$CARGO_BIN" test --test random_differential --quiet
    fi
    if matches_filter "pass_verify_examples"; then
      run_cargo "pass_verify_examples" env \
        RR_VERIFY_EACH_PASS=1 \
        "$CARGO_BIN" test --test pass_verify_examples --quiet
    fi
    ;;
  *)
    echo "unknown tier: $TIER" >&2
    exit 2
    ;;
esac

if (( RUN_COUNT == 0 )); then
  echo "no tests matched tier '$TIER' with filter '${FILTER:-<none>}'" >&2
  exit 2
fi

echo
echo "[ok] $TIER passed"
