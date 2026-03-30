#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/scripts/lib/triage_common.sh"
FAIL_ROOT="${RR_DIFFERENTIAL_FAILURE_ROOT:-$ROOT/target/tests/random_differential_failures}"
OUT_DIR="${RR_DIFFERENTIAL_TRIAGE_OUT_DIR:-$ROOT/.artifacts/differential-triage}"
RR_BIN_DEFAULT="${RR_BIN:-$ROOT/target/debug/RR}"
RSCRIPT_BIN_DEFAULT="${RSCRIPT_BIN:-$(command -v Rscript || true)}"
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
    "Differential Triage Summary" \
    "No differential failure root: \`$FAIL_ROOT\`."
  triage_write_empty_json_report \
    "$SUMMARY_JSON" \
    "differential" \
    "No differential failure root: $FAIL_ROOT."
  exit 0
fi

BUNDLES_LIST="$OUT_DIR/bundles.list"
find "$FAIL_ROOT" -mindepth 1 -maxdepth 1 -type d | sort > "$BUNDLES_LIST"
if [[ ! -s "$BUNDLES_LIST" ]]; then
  triage_write_empty_reports \
    "$SUMMARY" \
    "$JOB_SUMMARY" \
    "Differential Triage Summary" \
    "No differential failure bundles found under \`$FAIL_ROOT\`."
  triage_write_empty_json_report \
    "$SUMMARY_JSON" \
    "differential" \
    "No differential failure bundles found under $FAIL_ROOT."
  exit 0
fi

generate_regression_rs() {
  local case_name="$1"
  local out="$2"
  local test_name
  test_name="$(triage_rust_test_name "differential_regression_${case_name}")"
  cat > "$out" <<RS
mod common;

use common::{compile_rr_env, normalize, rscript_available, rscript_path, run_rscript, unique_dir};
use std::fs;
use std::path::PathBuf;

#[test]
fn ${test_name}() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping differential regression: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("differential_regressions");
    fs::create_dir_all(&sandbox_root).expect("failed to create differential regression root");
    let proj_dir = unique_dir(&sandbox_root, "${test_name}");
    fs::create_dir_all(&proj_dir).expect("failed to create differential regression dir");

    let rr_src = include_str!("differential_regressions/${case_name}/case.rr");
    let ref_src = include_str!("differential_regressions/${case_name}/reference.R");
    let rr_path = proj_dir.join("case.rr");
    let ref_path = proj_dir.join("reference.R");
    fs::write(&rr_path, rr_src).expect("failed to write RR case");
    fs::write(&ref_path, ref_src).expect("failed to write reference R case");

    let reference = run_rscript(&rscript, &ref_path);
    assert_eq!(
        reference.status, 0,
        "reference R failed\nstdout:\n{}\nstderr:\n{}",
        reference.stdout, reference.stderr
    );

    for (flag, tag) in [("-O0", "o0"), ("-O1", "o1"), ("-O2", "o2")] {
        let out_path = proj_dir.join(format!("case_{tag}.R"));
        compile_rr_env(
            &rr_bin,
            &rr_path,
            &out_path,
            flag,
            &[("RR_VERIFY_EACH_PASS", "1"), ("RR_QUIET_LOG", "1")],
        );
        let compiled = run_rscript(&rscript, &out_path);
        assert_eq!(
            reference.status,
            compiled.status,
            "status mismatch for {flag}\nreference stdout:\n{}\ncompiled stdout:\n{}\nreference stderr:\n{}\ncompiled stderr:\n{}",
            reference.stdout,
            compiled.stdout,
            reference.stderr,
            compiled.stderr,
        );
        assert_eq!(
            normalize(&reference.stdout),
            normalize(&compiled.stdout),
            "stdout mismatch for {flag}\nreference:\n{}\ncompiled:\n{}",
            reference.stdout,
            compiled.stdout,
        );
        assert_eq!(
            normalize(&reference.stderr),
            normalize(&compiled.stderr),
            "stderr mismatch for {flag}\nreference:\n{}\ncompiled:\n{}",
            reference.stderr,
            compiled.stderr,
        );
    }
}
RS
}

cat > "$SUMMARY" <<'MD'
# Differential Triage Summary

MD

TOTAL=0
TEXT_SKELETONS=0
INVALID=0

while IFS= read -r bundle; do
  if [[ ! -f "$bundle/case.rr" || ! -f "$bundle/reference.R" || ! -f "$bundle/compiled.R" ]]; then
    continue
  fi
  base="$(basename "$bundle")"
  case_dir="$OUT_DIR/$(triage_sanitize_name "$base")"
  mkdir -p "$case_dir"

  manifest="$bundle/bundle.manifest"
  if [[ ! -f "$manifest" ]]; then
    manifest="$bundle/README.txt"
  fi
  case_name="$base"
  opt_tag="unknown"
  ref_status="unknown"
  compiled_status="unknown"
  if [[ -f "$bundle/bundle.manifest" ]]; then
    if ! triage_require_manifest_contract "$bundle/bundle.manifest" differential || \
      ! triage_require_manifest_fields \
        "$bundle/bundle.manifest" \
        case \
        opt \
        reference_status \
        compiled_status; then
      cat >> "$SUMMARY" <<MD
## Invalid bundle: $base

- bundle: \`$bundle\`
- status: skipped
- reason: invalid \`bundle.manifest\`

MD
      INVALID=$((INVALID + 1))
      continue
    fi
  fi

  if [[ -f "$manifest" ]]; then
    case_name="$(triage_read_manifest_field "$manifest" "case")"
    opt_tag="$(triage_read_manifest_field "$manifest" "opt")"
    ref_status="$(triage_read_manifest_field "$manifest" "reference_status")"
    compiled_status="$(triage_read_manifest_field "$manifest" "compiled_status")"
  fi
  if [[ -z "$case_name" ]]; then
    case_name="$base"
  fi
  if [[ -z "$opt_tag" ]]; then
    opt_tag="unknown"
  fi
  if [[ -z "$ref_status" ]]; then
    ref_status="unknown"
  fi
  if [[ -z "$compiled_status" ]]; then
    compiled_status="unknown"
  fi

  cp "$bundle/case.rr" "$case_dir/case.rr"
  cp "$bundle/reference.R" "$case_dir/reference.R"
  cp "$bundle/compiled.R" "$case_dir/compiled.R"
  cp "$bundle/reference.stdout" "$case_dir/reference.stdout"
  cp "$bundle/reference.stderr" "$case_dir/reference.stderr"
  cp "$bundle/compiled.stdout" "$case_dir/compiled.stdout"
  cp "$bundle/compiled.stderr" "$case_dir/compiled.stderr"
  [[ -f "$bundle/bundle.manifest" ]] && cp "$bundle/bundle.manifest" "$case_dir/bundle.manifest"
  [[ -f "$manifest" ]] && cp "$manifest" "$case_dir/README.txt"

  regression_name="$(triage_rust_test_name "$case_name")"
  generate_regression_rs "$regression_name" "$case_dir/regression.rs"
  cat > "$case_dir/replay.sh" <<SH
#!/usr/bin/env bash
set -euo pipefail
RR_BIN="\${RR_BIN:-$RR_BIN_DEFAULT}"
RSCRIPT_BIN="\${RSCRIPT_BIN:-$RSCRIPT_BIN_DEFAULT}"
WORK_DIR="\${WORK_DIR:-\$(mktemp -d)}"
echo "[replay] case: $case_name ($opt_tag)"
echo "[replay] work: \$WORK_DIR"
set +e
"\$RSCRIPT_BIN" --vanilla "$case_dir/reference.R" >"\$WORK_DIR/reference.stdout" 2>"\$WORK_DIR/reference.stderr"
ref_status=\$?
"\$RR_BIN" "$case_dir/case.rr" -o "\$WORK_DIR/compiled.R" "-$opt_tag" >"\$WORK_DIR/compiler.stdout" 2>"\$WORK_DIR/compiler.stderr"
compile_status=\$?
if [[ \$compile_status -eq 0 ]]; then
  "\$RSCRIPT_BIN" --vanilla "\$WORK_DIR/compiled.R" >"\$WORK_DIR/compiled.stdout" 2>"\$WORK_DIR/compiled.stderr"
  compiled_status=\$?
else
  compiled_status=\$compile_status
fi
set -e
echo "[replay] reference status: \$ref_status"
echo "[replay] compiled status: \$compiled_status"
echo "[replay] original reference status: $ref_status"
echo "[replay] original compiled status: $compiled_status"
echo "[replay] outputs captured in \$WORK_DIR"
SH
  chmod +x "$case_dir/replay.sh"
  cat > "$case_dir/reduce.sh" <<SH
#!/usr/bin/env bash
set -euo pipefail
RR_BIN="\${RR_BIN:-$RR_BIN_DEFAULT}" RSCRIPT_BIN="\${RSCRIPT_BIN:-$RSCRIPT_BIN_DEFAULT}" \
  "$ROOT/scripts/triage_driver.sh" reduce differential "$case_dir" "\${1:-$case_dir/reduced.rr}"
SH
  chmod +x "$case_dir/reduce.sh"
  cat > "$case_dir/meta.json" <<JSON
{
  "schema": "rr-triage-case",
  "version": 1,
  "kind": "differential",
  "case": "$case_name",
  "opt": "$opt_tag",
  "reference_status": "$ref_status",
  "compiled_status": "$compiled_status",
  "case_dir": "$case_dir",
  "replay_script": "$case_dir/replay.sh",
  "reduce_script": "$case_dir/reduce.sh",
  "regression": "$case_dir/regression.rs"
}
JSON
  TEXT_SKELETONS=$((TEXT_SKELETONS + 1))

  printf '%s\t%s\t%s\t%s\t%s\n' "$case_name" "$opt_tag" "$ref_status" "$compiled_status" "$case_dir" >> "$INDEX"

  cat >> "$SUMMARY" <<MD
## $case_name ($opt_tag)

- bundle: \`$bundle\`
- reference status: $ref_status
- compiled status: $compiled_status
- copied files:
  - \`$case_dir/case.rr\`
  - \`$case_dir/reference.R\`
  - \`$case_dir/compiled.R\`
  - \`$case_dir/reference.stdout\`
  - \`$case_dir/reference.stderr\`
  - \`$case_dir/compiled.stdout\`
  - \`$case_dir/compiled.stderr\`
  - \`$case_dir/regression.rs\`
  - \`$case_dir/replay.sh\`
  - \`$case_dir/reduce.sh\`
  - \`$case_dir/meta.json\`

MD

  TOTAL=$((TOTAL + 1))
done < "$BUNDLES_LIST"

cat > "$JOB_SUMMARY" <<MD
# Nightly Differential Triage

- failure bundles: $TOTAL
- rust regression skeletons: $TEXT_SKELETONS
- invalid bundles skipped: $INVALID

| case | opt | reference status | compiled status |
| --- | --- | --- | --- |
MD

while IFS=$'\t' read -r case_name opt_tag ref_status compiled_status case_dir; do
  printf '| `%s` | `%s` | `%s` | `%s` |\n' \
    "$case_name" "$opt_tag" "$ref_status" "$compiled_status" >> "$JOB_SUMMARY"
done < "$INDEX"

python3 - <<'PY' "$SUMMARY_JSON" "$INDEX" "$TOTAL" "$TEXT_SKELETONS" "$INVALID"
import csv
import json
import sys
from pathlib import Path

summary_path = Path(sys.argv[1])
index_path = Path(sys.argv[2])
total = int(sys.argv[3])
text_skeletons = int(sys.argv[4])
invalid = int(sys.argv[5])
cases = []
if index_path.exists():
    with index_path.open(newline="") as fh:
        reader = csv.reader(fh, delimiter="\t")
        for row in reader:
            if len(row) != 5:
                continue
            case_name, opt_tag, ref_status, compiled_status, case_dir = row
            cases.append(
                {
                    "case": case_name,
                    "opt": opt_tag,
                    "reference_status": ref_status,
                    "compiled_status": compiled_status,
                    "case_dir": case_dir,
                }
            )
summary_path.write_text(
    json.dumps(
        {
            "schema": "rr-triage-report",
            "version": 1,
            "kind": "differential",
            "failure_bundles": total,
            "rust_regression_skeletons": text_skeletons,
            "invalid_bundles": invalid,
            "cases": cases,
        },
        indent=2,
    )
    + "\n"
)
PY
