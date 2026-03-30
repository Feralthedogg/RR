#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/scripts/lib/triage_common.sh"
FAIL_ROOT="${RR_PASS_VERIFY_FAILURE_ROOT:-$ROOT/target/tests/pass_verify_failures}"
OUT_DIR="${RR_PASS_VERIFY_TRIAGE_OUT_DIR:-$ROOT/.artifacts/pass-verify-triage}"
RR_BIN_DEFAULT="${RR_BIN:-$ROOT/target/debug/RR}"
mkdir -p "$OUT_DIR"
SUMMARY="$OUT_DIR/summary.md"
JOB_SUMMARY="$OUT_DIR/job-summary.md"
SUMMARY_JSON="$OUT_DIR/summary.json"
INDEX="$OUT_DIR/index.tsv"
: > "$INDEX"

if [[ ! -d "$FAIL_ROOT" ]]; then
  triage_write_empty_reports \
    "$SUMMARY" \
    "$JOB_SUMMARY" \
    "Pass-Verify Triage Summary" \
    "No pass-verify failure root: \`$FAIL_ROOT\`."
  triage_write_empty_json_report \
    "$SUMMARY_JSON" \
    "pass-verify" \
    "No pass-verify failure root: $FAIL_ROOT."
  exit 0
fi

BUNDLES_LIST="$OUT_DIR/bundles.list"
find "$FAIL_ROOT" -mindepth 1 -maxdepth 1 -type d | sort > "$BUNDLES_LIST"
if [[ ! -s "$BUNDLES_LIST" ]]; then
  triage_write_empty_reports \
    "$SUMMARY" \
    "$JOB_SUMMARY" \
    "Pass-Verify Triage Summary" \
    "No pass-verify failure bundles found under \`$FAIL_ROOT\`."
  triage_write_empty_json_report \
    "$SUMMARY_JSON" \
    "pass-verify" \
    "No pass-verify failure bundles found under $FAIL_ROOT."
  exit 0
fi

generate_regression_rs() {
  local case_name="$1"
  local out="$2"
  local test_name
  test_name="$(triage_rust_test_name "pass_verify_regression_${case_name}")"
  cat > "$out" <<RS
mod common;

use common::compile_rr_env;
use std::fs;
use std::path::PathBuf;

#[test]
fn ${test_name}() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("pass_verify_regressions");
    fs::create_dir_all(&sandbox_root).expect("failed to create pass verify regression root");
    let rr_src = root
        .join("tests")
        .join("pass_verify_regressions")
        .join("${case_name}")
        .join("case.rr");
    let out_path = sandbox_root.join("${test_name}.R");
    compile_rr_env(
        &rr_bin,
        &rr_src,
        &out_path,
        "-O2",
        &[("RR_VERIFY_EACH_PASS", "1"), ("RR_QUIET_LOG", "1")],
    );
    let code = fs::read_to_string(&out_path).expect("failed to read emitted R");
    assert!(
        code.contains("<- function(") || code.contains("print("),
        "unexpected empty emitted R for {}",
        rr_src.display()
    );
}
RS
}

cat > "$SUMMARY" <<'MD'
# Pass-Verify Triage Summary

MD

TOTAL=0
INVALID=0
while IFS= read -r bundle; do
  if [[ ! -f "$bundle/case.rr" || ! -f "$bundle/compiler.stdout" || ! -f "$bundle/compiler.stderr" ]]; then
    continue
  fi
  base="$(basename "$bundle")"
  if [[ ! -f "$bundle/bundle.manifest" ]]; then
    cat >> "$SUMMARY" <<MD
## Invalid bundle: $base

- bundle: \`$bundle\`
- status: skipped
- reason: missing \`bundle.manifest\`

MD
    INVALID=$((INVALID + 1))
    continue
  fi
  if ! triage_require_manifest_contract "$bundle/bundle.manifest" pass-verify || \
    ! triage_require_manifest_fields "$bundle/bundle.manifest" case status; then
    cat >> "$SUMMARY" <<MD
## Invalid bundle: $base

- bundle: \`$bundle\`
- status: skipped
- reason: invalid \`bundle.manifest\`

MD
    INVALID=$((INVALID + 1))
    continue
  fi
  case_dir="$OUT_DIR/$(triage_sanitize_name "$base")"
  mkdir -p "$case_dir"
  cp "$bundle/case.rr" "$case_dir/case.rr"
  cp "$bundle/compiler.stdout" "$case_dir/compiler.stdout"
  cp "$bundle/compiler.stderr" "$case_dir/compiler.stderr"
  [[ -f "$bundle/compiled.R" ]] && cp "$bundle/compiled.R" "$case_dir/compiled.R"
  [[ -f "$bundle/bundle.manifest" ]] && cp "$bundle/bundle.manifest" "$case_dir/bundle.manifest"
  [[ -f "$bundle/README.txt" ]] && cp "$bundle/README.txt" "$case_dir/README.txt"
  if [[ -d "$bundle/verify-dumps" ]]; then
    mkdir -p "$case_dir/verify-dumps"
    cp -R "$bundle/verify-dumps/." "$case_dir/verify-dumps/"
  fi

  regression_name="$(triage_rust_test_name "$base")"
  generate_regression_rs "$regression_name" "$case_dir/regression.rs"
  cat > "$case_dir/replay.sh" <<SH
#!/usr/bin/env bash
set -euo pipefail
RR_BIN="\${RR_BIN:-$RR_BIN_DEFAULT}"
OUT_FILE="\${1:-$case_dir/replayed.R}"
RR_VERIFY_EACH_PASS=1 RR_QUIET_LOG=1 "\$RR_BIN" "$case_dir/case.rr" -o "\$OUT_FILE" -O2
SH
  chmod +x "$case_dir/replay.sh"
  cat > "$case_dir/reduce.sh" <<SH
#!/usr/bin/env bash
set -euo pipefail
RR_BIN="\${RR_BIN:-$RR_BIN_DEFAULT}" \
  "$ROOT/scripts/triage_driver.sh" reduce pass-verify "$case_dir" "\${1:-$case_dir/reduced.rr}"
SH
  chmod +x "$case_dir/reduce.sh"
  cat > "$case_dir/meta.json" <<JSON
{
  "schema": "rr-triage-case",
  "version": 1,
  "kind": "pass-verify",
  "case": "$base",
  "status": "$(triage_read_manifest_field "$bundle/bundle.manifest" "status")",
  "case_dir": "$case_dir",
  "replay_script": "$case_dir/replay.sh",
  "reduce_script": "$case_dir/reduce.sh",
  "regression": "$case_dir/regression.rs"
}
JSON
  printf '%s\t%s\n' "$base" "$case_dir" >> "$INDEX"

  cat >> "$SUMMARY" <<MD
## $base

- bundle: \`$bundle\`
- copied files:
  - \`$case_dir/case.rr\`
  - \`$case_dir/compiler.stdout\`
  - \`$case_dir/compiler.stderr\`
  - \`$case_dir/regression.rs\`
  - \`$case_dir/replay.sh\`
  - \`$case_dir/reduce.sh\`
  - \`$case_dir/meta.json\`

MD
  TOTAL=$((TOTAL + 1))
done < "$BUNDLES_LIST"

cat > "$JOB_SUMMARY" <<MD
# Nightly Pass-Verify Triage

- failure bundles: $TOTAL
- invalid bundles skipped: $INVALID

| case |
| --- |
MD

while IFS=$'\t' read -r base case_dir; do
  printf '| `%s` |\n' "$base" >> "$JOB_SUMMARY"
done < "$INDEX"

python3 - <<'PY' "$SUMMARY_JSON" "$INDEX" "$TOTAL" "$INVALID"
import csv
import json
import sys
from pathlib import Path

summary_path = Path(sys.argv[1])
index_path = Path(sys.argv[2])
total = int(sys.argv[3])
invalid = int(sys.argv[4])
cases = []
if index_path.exists():
    with index_path.open(newline="") as fh:
        reader = csv.reader(fh, delimiter="\t")
        for row in reader:
            if len(row) != 2:
                continue
            case_name, case_dir = row
            cases.append({"case": case_name, "case_dir": case_dir})
summary_path.write_text(
    json.dumps(
        {
            "schema": "rr-triage-report",
            "version": 1,
            "kind": "pass-verify",
            "failure_bundles": total,
            "invalid_bundles": invalid,
            "cases": cases,
        },
        indent=2,
    )
    + "\n"
)
PY
