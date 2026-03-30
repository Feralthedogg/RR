#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_BIN="${CARGO:-cargo}"

TESTS=(
  compiler_parallel_defaults
  compiler_parallel_equivalence
  parallel_cli_flags
  cli_option_errors
  cli_commands
  cli_watch_once
  incremental_phase2
  incremental_strict_verify
  docs_surface_sync
)

echo "== Compiler Parallel Smoke =="
echo "root: $ROOT"
echo "cargo: $CARGO_BIN"
echo "tests: ${#TESTS[@]}"

for test_name in "${TESTS[@]}"; do
  echo
  echo "== $test_name =="
  (
    cd "$ROOT"
    "$CARGO_BIN" test --test "$test_name" -- --nocapture
  )
done

echo
echo "[ok] compiler-parallel smoke passed"
