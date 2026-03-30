#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "usage: $0 <differential|pass-verify> <triage-case-dir> [output-file]" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/scripts/lib/triage_common.sh"

KIND="$1"
CASE_DIR="$2"
OUT_PATH="${3:-$CASE_DIR/reduced.rr}"
MANIFEST="$CASE_DIR/bundle.manifest"
RR_BIN="${RR_BIN:-$ROOT/target/debug/RR}"
RSCRIPT_BIN="${RSCRIPT_BIN:-$(command -v Rscript || true)}"

if [[ ! -d "$CASE_DIR" ]]; then
  echo "missing triage case dir: $CASE_DIR" >&2
  exit 1
fi
if [[ ! -f "$MANIFEST" ]]; then
  echo "missing bundle.manifest: $MANIFEST" >&2
  exit 1
fi

case "$KIND" in
  differential)
    triage_require_manifest_contract "$MANIFEST" differential
    triage_require_manifest_fields \
      "$MANIFEST" \
      case \
      opt \
      reference_status \
      compiled_status
    opt_tag="$(triage_read_manifest_field "$MANIFEST" opt)"
    ref_status="$(triage_read_manifest_field "$MANIFEST" reference_status)"
    compiled_status="$(triage_read_manifest_field "$MANIFEST" compiled_status)"
    python3 "$ROOT/scripts/lib/triage_reduce.py" \
      --kind differential \
      --rr-bin "$RR_BIN" \
      --rscript-bin "$RSCRIPT_BIN" \
      --case "$CASE_DIR/case.rr" \
      --output "$OUT_PATH" \
      --opt="-${opt_tag}" \
      --reference "$CASE_DIR/reference.R" \
      --expected-reference-status "$ref_status" \
      --expected-compiled-status "$compiled_status" \
      --reference-stdout-file "$CASE_DIR/reference.stdout" \
      --reference-stderr-file "$CASE_DIR/reference.stderr" \
      --compiled-stdout-file "$CASE_DIR/compiled.stdout" \
      --compiled-stderr-file "$CASE_DIR/compiled.stderr"
    ;;
  pass-verify)
    triage_require_manifest_contract "$MANIFEST" pass-verify
    triage_require_manifest_fields "$MANIFEST" case status
    python3 "$ROOT/scripts/lib/triage_reduce.py" \
      --kind pass-verify \
      --rr-bin "$RR_BIN" \
      --case "$CASE_DIR/case.rr" \
      --output "$OUT_PATH" \
      --opt=-O2 \
      --stderr-anchor-file "$CASE_DIR/compiler.stderr"
    ;;
  *)
    echo "unsupported triage reduction kind: $KIND" >&2
    exit 1
    ;;
esac

echo "[ok] reduced case -> $OUT_PATH"
