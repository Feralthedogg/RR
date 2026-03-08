#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "usage: $0 <differential-triage-case-dir> [test-name]" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/scripts/lib/triage_common.sh"
CASE_DIR="$1"
if [[ ! -d "$CASE_DIR" ]]; then
  echo "missing differential triage case dir: $CASE_DIR" >&2
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
  echo "missing bundle.manifest in differential triage case: $CASE_DIR" >&2
  exit 1
fi
triage_require_manifest_contract "$MANIFEST" differential
triage_require_manifest_fields \
  "$MANIFEST" \
  case \
  opt \
  reference_status \
  compiled_status

REGRESSION_RS="$CASE_DIR/regression.rs"
if [[ ! -f "$REGRESSION_RS" ]]; then
  echo "missing regression.rs in differential triage case: $CASE_DIR" >&2
  exit 1
fi

DEST_DIR="$ROOT/tests/differential_regressions/$TEST_NAME"
mkdir -p "$DEST_DIR"
cp "$CASE_DIR/case.rr" "$DEST_DIR/case.rr"
cp "$CASE_DIR/reference.R" "$DEST_DIR/reference.R"
cp "$CASE_DIR/compiled.R" "$DEST_DIR/compiled.R"
cp "$CASE_DIR/reference.stdout" "$DEST_DIR/reference.stdout"
cp "$CASE_DIR/reference.stderr" "$DEST_DIR/reference.stderr"
cp "$CASE_DIR/compiled.stdout" "$DEST_DIR/compiled.stdout"
cp "$CASE_DIR/compiled.stderr" "$DEST_DIR/compiled.stderr"
cp "$MANIFEST" "$DEST_DIR/bundle.manifest"
[[ -f "$CASE_DIR/README.txt" ]] && cp "$CASE_DIR/README.txt" "$DEST_DIR/README.txt"

TEST_DEST="$ROOT/tests/${TEST_NAME}.rs"
python3 - <<PY
from pathlib import Path
import re

src = Path(r'''$REGRESSION_RS''').read_text()
src = re.sub(
    r'include_str!\("differential_regressions/[^"/]+/case\.rr"\)',
    'include_str!("differential_regressions/$TEST_NAME/case.rr")',
    src,
)
src = re.sub(
    r'include_str!\("differential_regressions/[^"/]+/reference\.R"\)',
    'include_str!("differential_regressions/$TEST_NAME/reference.R")',
    src,
)
Path(r'''$TEST_DEST''').write_text(src)
PY

echo "promoted differential regression:"
echo "  bundle -> $DEST_DIR"
echo "  test   -> $TEST_DEST"
