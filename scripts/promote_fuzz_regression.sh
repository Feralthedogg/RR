#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "usage: $0 <triage-case-dir> [test-name]" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/scripts/lib/triage_common.sh"
CASE_DIR="$1"
if [[ ! -d "$CASE_DIR" ]]; then
  echo "missing triage case dir: $CASE_DIR" >&2
  exit 1
fi

DEFAULT_NAME="$(triage_rust_test_name "$(basename "$CASE_DIR")")"
TEST_NAME="${2:-$DEFAULT_NAME}"
TEST_NAME="$(triage_rust_test_name "$TEST_NAME")"

MANIFEST="$CASE_DIR/bundle.manifest"
if [[ ! -f "$MANIFEST" ]]; then
  echo "missing bundle.manifest in fuzz triage case: $CASE_DIR" >&2
  exit 1
fi
triage_require_manifest_contract "$MANIFEST" fuzz
triage_require_manifest_fields \
  "$MANIFEST" \
  target \
  artifact \
  repro_status \
  tmin_status \
  skeleton_kind

MINIMIZED="$CASE_DIR/minimized-input"
REGRESSION_RS="$CASE_DIR/regression.rs"
REGRESSION_MD="$CASE_DIR/regression.md"

if [[ -f "$REGRESSION_RS" ]]; then
  mkdir -p "$ROOT/tests/fuzz_regressions"
  INPUT_DEST="$ROOT/tests/fuzz_regressions/${TEST_NAME}.rr"
  TEST_DEST="$ROOT/tests/${TEST_NAME}.rs"
  cp "$MINIMIZED" "$INPUT_DEST"
  python3 - <<PY
from pathlib import Path
src = Path(r'''$REGRESSION_RS''').read_text()
src = src.replace('include_str!("minimized-input")', 'include_str!("fuzz_regressions/$TEST_NAME.rr")')
Path(r'''$TEST_DEST''').write_text(src)
PY
  cp "$MANIFEST" "$ROOT/tests/fuzz_regressions/${TEST_NAME}.manifest"
  echo "promoted text regression:"
  echo "  input -> $INPUT_DEST"
  echo "  test  -> $TEST_DEST"
  exit 0
fi

if [[ -f "$REGRESSION_MD" ]]; then
  DEST_DIR="$ROOT/tests/fuzz_regressions/manual_${TEST_NAME}"
  mkdir -p "$DEST_DIR"
  cp "$MINIMIZED" "$DEST_DIR/minimized-input"
  cp "$REGRESSION_MD" "$DEST_DIR/README.md"
  cp "$MANIFEST" "$DEST_DIR/bundle.manifest"
  echo "promoted manual regression note:"
  echo "  dir -> $DEST_DIR"
  exit 0
fi

echo "triage case does not contain regression.rs or regression.md: $CASE_DIR" >&2
exit 1
