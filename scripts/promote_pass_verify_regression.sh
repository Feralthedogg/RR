#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "usage: $0 <pass-verify-triage-case-dir> [test-name]" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/scripts/lib/triage_common.sh"
CASE_DIR="$1"
if [[ ! -d "$CASE_DIR" ]]; then
  echo "missing pass-verify triage case dir: $CASE_DIR" >&2
  exit 1
fi

basename_safe() {
  local s="$1"
  s="${s##*/}"
  s="${s// /_}"
  s="${s//-/_}"
  s="${s//\//_}"
  s="$(printf '%s' "$s" | tr -cd '[:alnum:]_')"
  printf '%s' "$s"
}

DEFAULT_NAME="$(basename_safe "$CASE_DIR")"
TEST_NAME="${2:-$DEFAULT_NAME}"
TEST_NAME="$(basename_safe "$TEST_NAME")"

MANIFEST="$CASE_DIR/bundle.manifest"
if [[ ! -f "$MANIFEST" ]]; then
  echo "missing bundle.manifest in pass-verify triage case: $CASE_DIR" >&2
  exit 1
fi
triage_require_manifest_contract "$MANIFEST" pass-verify
triage_require_manifest_fields "$MANIFEST" case status

REGRESSION_RS="$CASE_DIR/regression.rs"
if [[ ! -f "$REGRESSION_RS" ]]; then
  echo "missing regression.rs in pass-verify triage case: $CASE_DIR" >&2
  exit 1
fi

DEST_DIR="$ROOT/tests/pass_verify_regressions/$TEST_NAME"
mkdir -p "$DEST_DIR"
cp "$CASE_DIR/case.rr" "$DEST_DIR/case.rr"
[[ -f "$CASE_DIR/compiler.stdout" ]] && cp "$CASE_DIR/compiler.stdout" "$DEST_DIR/compiler.stdout"
[[ -f "$CASE_DIR/compiler.stderr" ]] && cp "$CASE_DIR/compiler.stderr" "$DEST_DIR/compiler.stderr"
[[ -f "$CASE_DIR/compiled.R" ]] && cp "$CASE_DIR/compiled.R" "$DEST_DIR/compiled.R"
cp "$MANIFEST" "$DEST_DIR/bundle.manifest"
[[ -f "$CASE_DIR/README.txt" ]] && cp "$CASE_DIR/README.txt" "$DEST_DIR/README.txt"
if [[ -d "$CASE_DIR/verify-dumps" ]]; then
  mkdir -p "$DEST_DIR/verify-dumps"
  cp -R "$CASE_DIR/verify-dumps/." "$DEST_DIR/verify-dumps/"
fi

TEST_DEST="$ROOT/tests/${TEST_NAME}.rs"
python3 - <<PY
from pathlib import Path
import re

src = Path(r'''$REGRESSION_RS''').read_text()
src = re.sub(
    r'join\("tests"\)\n\s+\.join\("pass_verify_regressions"\)\n\s+\.join\("[^"]+"\)',
    'join("tests")\n        .join("pass_verify_regressions")\n        .join("$TEST_NAME")',
    src,
    count=1,
)
Path(r'''$TEST_DEST''').write_text(src)
PY

echo "promoted pass-verify regression:"
echo "  bundle -> $DEST_DIR"
echo "  test   -> $TEST_DEST"
