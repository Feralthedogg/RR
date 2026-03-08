#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <fuzz|differential|pass-verify> <triage-root>" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/scripts/lib/triage_common.sh"
KIND="$1"
TRIAGE_ROOT="$2"

if [[ ! -d "$TRIAGE_ROOT" ]]; then
  echo "triage root not found: $TRIAGE_ROOT" >&2
  exit 1
fi

case "$KIND" in
  fuzz)
    PROMOTE_SCRIPT="$ROOT/scripts/promote_fuzz_regression.sh"
    CLEANUP_ROOT="$ROOT/tests/fuzz_regressions"
    ;;
  differential)
    PROMOTE_SCRIPT="$ROOT/scripts/promote_differential_regression.sh"
    CLEANUP_ROOT="$ROOT/tests/differential_regressions"
    ;;
  pass-verify)
    PROMOTE_SCRIPT="$ROOT/scripts/promote_pass_verify_regression.sh"
    CLEANUP_ROOT="$ROOT/tests/pass_verify_regressions"
    ;;
  *)
    echo "unsupported triage kind: $KIND" >&2
    exit 1
    ;;
esac

cleanup_case() {
  local test_name="$1"
  rm -f "$ROOT/tests/${test_name}.rs"
  case "$KIND" in
    fuzz)
      rm -f "$CLEANUP_ROOT/${test_name}.rr"
      rm -f "$CLEANUP_ROOT/${test_name}.manifest"
      rm -rf "$CLEANUP_ROOT/manual_${test_name}"
      ;;
    *)
      rm -rf "$CLEANUP_ROOT/$test_name"
      ;;
  esac
}

CURRENT_TEST_NAME=""
cleanup_current() {
  if [[ -n "$CURRENT_TEST_NAME" ]]; then
    cleanup_case "$CURRENT_TEST_NAME"
    CURRENT_TEST_NAME=""
  fi
}

trap cleanup_current EXIT

FOUND=0
while IFS= read -r regression; do
  FOUND=1
  case_dir="$(dirname "$regression")"
  case_base="$(triage_rust_test_name "$(basename "$case_dir")")"
  test_name="$(triage_rust_test_name "__triage_smoke_${KIND}_$case_base")"
  cleanup_current
  cleanup_case "$test_name"
  CURRENT_TEST_NAME="$test_name"
  "$PROMOTE_SCRIPT" "$case_dir" "$test_name" >/dev/null
  cargo test -q --test "$test_name"
  cleanup_current
done < <(find "$TRIAGE_ROOT" -mindepth 2 -maxdepth 2 -type f -name 'regression.rs' | sort)

if [[ "$FOUND" -eq 0 ]]; then
  echo "no regression.rs skeletons found under $TRIAGE_ROOT"
fi
