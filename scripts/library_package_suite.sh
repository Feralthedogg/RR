#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_BIN="${CARGO:-cargo}"
FILTER="${RR_PACKAGE_SUITE_FILTER:-}"
source "$ROOT/scripts/lib/test_manifests.sh"

filtered_exact_tests=()
for entry in "${RR_LIBRARY_PACKAGE_EXACT_TESTS[@]}"; do
  if [[ -z "$FILTER" || "$entry" == *"$FILTER"* ]]; then
    filtered_exact_tests+=("$entry")
  fi
done

filtered_package_tests=()
for test_name in "${RR_LIBRARY_PACKAGE_TESTS[@]}"; do
  if [[ -z "$FILTER" || "$test_name" == *"$FILTER"* ]]; then
    filtered_package_tests+=("$test_name")
  fi
done

total=$(( ${#filtered_exact_tests[@]} + ${#filtered_package_tests[@]} ))

echo "== Library Package Suite =="
echo "root: $ROOT"
echo "cargo: $CARGO_BIN"
echo "filter: ${FILTER:-<none>}"
echo "tests: $total"

if [[ $total -eq 0 ]]; then
  echo "no library package tests matched filter '${FILTER:-<none>}'" >&2
  exit 2
fi

run_exact() {
  local spec="$1"
  local test_name="${spec%%::*}"
  local exact_name="${spec#*::}"
  echo
  echo "== $test_name :: $exact_name =="
  (
    cd "$ROOT"
    CARGO_INCREMENTAL=0 "$CARGO_BIN" test --test "$test_name" "$exact_name" -- --exact
  )
}

run_package_test() {
  local test_name="$1"
  echo
  echo "== $test_name =="
  (
    cd "$ROOT"
    CARGO_INCREMENTAL=0 "$CARGO_BIN" test --test "$test_name" --quiet
  )
}

if (( ${#filtered_exact_tests[@]} > 0 )); then
  for spec in "${filtered_exact_tests[@]}"; do
    run_exact "$spec"
  done
fi

if (( ${#filtered_package_tests[@]} > 0 )); then
  for test_name in "${filtered_package_tests[@]}"; do
    run_package_test "$test_name"
  done
fi

echo
echo "[ok] library package suite passed"
