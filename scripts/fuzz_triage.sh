#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/scripts/lib/triage_common.sh"
FUZZ_DIR="${FUZZ_DIR:-$ROOT/fuzz}"
ARTIFACT_ROOT="${FUZZ_ARTIFACT_ROOT:-$FUZZ_DIR/artifacts}"
OUT_DIR="${FUZZ_TRIAGE_OUT_DIR:-$ROOT/.artifacts/fuzz-triage}"
TOOLCHAIN="${RUSTUP_TOOLCHAIN:-nightly}"
DICT="${FUZZ_DICT:-$FUZZ_DIR/dictionaries/rr.dict}"
TMIN_RUNS="${FUZZ_TMIN_RUNS:-64}"
RSS_LIMIT_MB="${FUZZ_RSS_LIMIT_MB:-2048}"
SKIP_EXEC="${FUZZ_TRIAGE_SKIP_EXEC:-0}"

mkdir -p "$OUT_DIR"
REGRESSION_DIR="$OUT_DIR/regressions"
mkdir -p "$REGRESSION_DIR"
SUMMARY="$OUT_DIR/summary.md"
JOB_SUMMARY="$OUT_DIR/job-summary.md"
SUMMARY_JSON="$OUT_DIR/summary.json"
INDEX="$OUT_DIR/index.tsv"
ARTIFACT_LIST="$OUT_DIR/artifacts.list"
: > "$INDEX"

if [[ ! -d "$ARTIFACT_ROOT" ]]; then
  triage_write_empty_reports \
    "$SUMMARY" \
    "$JOB_SUMMARY" \
    "Fuzz Triage Summary" \
    "No artifact root: \`$ARTIFACT_ROOT\`."
  triage_write_empty_json_report \
    "$SUMMARY_JSON" \
    "fuzz" \
    "No artifact root: $ARTIFACT_ROOT."
  exit 0
fi

find "$ARTIFACT_ROOT" -type f \( -name 'crash-*' -o -name 'leak-*' -o -name 'oom-*' -o -name 'timeout-*' \) | sort > "$ARTIFACT_LIST"
if [[ ! -s "$ARTIFACT_LIST" ]]; then
  triage_write_empty_reports \
    "$SUMMARY" \
    "$JOB_SUMMARY" \
    "Fuzz Triage Summary" \
    "No crash artifacts found under \`$ARTIFACT_ROOT\`."
  triage_write_empty_json_report \
    "$SUMMARY_JSON" \
    "fuzz" \
    "No crash artifacts found under $ARTIFACT_ROOT."
  exit 0
fi

if [[ "$SKIP_EXEC" != "1" ]] && ! cargo +"$TOOLCHAIN" fuzz --help >/dev/null 2>&1; then
  echo "cargo-fuzz is not installed for toolchain '$TOOLCHAIN'." >&2
  exit 1
fi

is_probably_text() {
  local path="$1"
  local non_printable
  non_printable="$(LC_ALL=C tr -d '\11\12\15\40-\176' < "$path" | wc -c | tr -d ' ')"
  [[ "$non_printable" == "0" ]]
}

generate_text_regression() {
  local target="$1"
  local base="$2"
  local case_dir="$3"
  local test_name
  test_name="$(triage_rust_test_name "fuzz_regression_${target}_${base}")"
  cat > "$case_dir/regression.rs" <<RS
mod common;

#[test]
fn ${test_name}() {
    let source = include_str!("minimized-input");
    let (ok_o2, stdout_o2, stderr_o2) = common::run_compile_case(
        "${test_name}",
        source,
        "case.rr",
        "-O2",
        &[("RR_QUIET_LOG", "1")],
    );
    assert!(
        ok_o2,
        "fuzz regression compile failed\\nstdout:\\n{stdout_o2}\\nstderr:\\n{stderr_o2}"
    );
}
RS
}

generate_binary_note() {
  local target="$1"
  local base="$2"
  local case_dir="$3"
  cat > "$case_dir/regression.md" <<MD
# Manual Regression Candidate

- target: \`$target\`
- artifact: \`$base\`
- reason: minimized input is not plain RR source text

Replay commands:

\
\`RR_QUIET_LOG=1 cargo +$TOOLCHAIN fuzz run $target $case_dir/minimized-input --fuzz-dir fuzz -- -runs=1 -rss_limit_mb=$RSS_LIMIT_MB -dict=fuzz/dictionaries/rr.dict\`
\
\`RR_QUIET_LOG=1 cargo +$TOOLCHAIN fuzz run $target $case_dir/original-input --fuzz-dir fuzz -- -runs=1 -rss_limit_mb=$RSS_LIMIT_MB -dict=fuzz/dictionaries/rr.dict\`
MD
}

cat > "$SUMMARY" <<'MD'
# Fuzz Triage Summary

MD

TOTAL=0
REPRO_OK=0
TMIN_OK=0
TEXT_SKELETONS=0
MANUAL_NOTES=0

while IFS= read -r artifact; do
  target="$(basename "$(dirname "$artifact")")"
  base="$(basename "$artifact")"
  case_dir="$OUT_DIR/${target}_$(triage_sanitize_name "$base")"
  mkdir -p "$case_dir"
  cp "$artifact" "$case_dir/original-input"

  repro_log="$case_dir/repro.log"
  tmin_log="$case_dir/tmin.log"
  minimized="$case_dir/minimized-input"
  cp "$artifact" "$minimized"

  repro_status="skipped"
  tmin_status="skipped"
  if [[ "$SKIP_EXEC" != "1" ]]; then
    repro_status="ok"
    RR_QUIET_LOG=1 cargo +"$TOOLCHAIN" fuzz run "$target" "$artifact" --fuzz-dir "$FUZZ_DIR" -- \
      -runs=1 \
      -rss_limit_mb="$RSS_LIMIT_MB" \
      -dict="$DICT" >"$repro_log" 2>&1 || repro_status="fail:$?"

    tmin_status="ok"
    RR_QUIET_LOG=1 cargo +"$TOOLCHAIN" fuzz tmin "$target" "$minimized" --fuzz-dir "$FUZZ_DIR" -O -r "$TMIN_RUNS" -- \
      -rss_limit_mb="$RSS_LIMIT_MB" \
      -dict="$DICT" >"$tmin_log" 2>&1 || tmin_status="fail:$?"
  else
    printf 'triage execution skipped (FUZZ_TRIAGE_SKIP_EXEC=1)\n' >"$repro_log"
    printf 'tmin skipped (FUZZ_TRIAGE_SKIP_EXEC=1)\n' >"$tmin_log"
  fi

  if [[ "$repro_status" == "ok" ]]; then
    REPRO_OK=$((REPRO_OK + 1))
  fi
  if [[ "$tmin_status" == "ok" ]]; then
    TMIN_OK=$((TMIN_OK + 1))
  fi

  if is_probably_text "$minimized"; then
    generate_text_regression "$target" "$base" "$case_dir"
    skeleton_kind="rust-test"
    TEXT_SKELETONS=$((TEXT_SKELETONS + 1))
  else
    generate_binary_note "$target" "$base" "$case_dir"
    skeleton_kind="manual-note"
    MANUAL_NOTES=$((MANUAL_NOTES + 1))
  fi

  cat > "$case_dir/bundle.manifest" <<MD
schema: rr-triage-bundle
version: 1
kind: fuzz
target: $target
artifact: $base
repro_status: $repro_status
tmin_status: $tmin_status
skeleton_kind: $skeleton_kind
MD

  cat > "$case_dir/replay.sh" <<SH
#!/usr/bin/env bash
set -euo pipefail
RR_QUIET_LOG=1 cargo +"$TOOLCHAIN" fuzz run "$target" "$case_dir/original-input" --fuzz-dir "$FUZZ_DIR" -- -runs=1 -rss_limit_mb="$RSS_LIMIT_MB" -dict="$DICT"
SH
  chmod +x "$case_dir/replay.sh"
  cat > "$case_dir/reduce.sh" <<SH
#!/usr/bin/env bash
set -euo pipefail
RR_QUIET_LOG=1 cargo +"$TOOLCHAIN" fuzz tmin "$target" "$case_dir/minimized-input" --fuzz-dir "$FUZZ_DIR" -O -r "$TMIN_RUNS" -- -rss_limit_mb="$RSS_LIMIT_MB" -dict="$DICT"
SH
  chmod +x "$case_dir/reduce.sh"
  cat > "$case_dir/meta.json" <<JSON
{
  "schema": "rr-triage-case",
  "version": 1,
  "kind": "fuzz",
  "target": "$target",
  "artifact": "$base",
  "repro_status": "$repro_status",
  "tmin_status": "$tmin_status",
  "skeleton_kind": "$skeleton_kind",
  "case_dir": "$case_dir",
  "replay_script": "$case_dir/replay.sh",
  "reduce_script": "$case_dir/reduce.sh"
}
JSON

  orig_size=$(wc -c < "$artifact" | tr -d ' ')
  min_size=$(wc -c < "$minimized" | tr -d ' ')
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$target" "$base" "$repro_status" "$tmin_status" "$skeleton_kind" "$case_dir" >> "$INDEX"

  cat >> "$SUMMARY" <<MD
## $target / $base

- original: \`$artifact\`
- original bytes: $orig_size
- minimized bytes: $min_size
- repro status: $repro_status
- tmin status: $tmin_status
- regression skeleton: $skeleton_kind
- replay:
  - \`RR_QUIET_LOG=1 cargo +$TOOLCHAIN fuzz run $target $artifact --fuzz-dir fuzz -- -runs=1 -rss_limit_mb=$RSS_LIMIT_MB -dict=fuzz/dictionaries/rr.dict\`
  - \`RR_QUIET_LOG=1 cargo +$TOOLCHAIN fuzz run $target $case_dir/minimized-input --fuzz-dir fuzz -- -runs=1 -rss_limit_mb=$RSS_LIMIT_MB -dict=fuzz/dictionaries/rr.dict\`
- logs:
  - \`$case_dir/repro.log\`
  - \`$case_dir/tmin.log\`
- generated files:
  - \`$case_dir/minimized-input\`
  - \`$case_dir/original-input\`
  - \`$case_dir/replay.sh\`
  - \`$case_dir/reduce.sh\`
  - \`$case_dir/meta.json\`
MD

  if [[ "$skeleton_kind" == "rust-test" ]]; then
    cat >> "$SUMMARY" <<MD
  - \`$case_dir/regression.rs\`

MD
  else
    cat >> "$SUMMARY" <<MD
  - \`$case_dir/regression.md\`

MD
  fi

  TOTAL=$((TOTAL + 1))
done < "$ARTIFACT_LIST"

cat > "$JOB_SUMMARY" <<MD
# Nightly Fuzz Triage

- artifacts: $TOTAL
- reproduced: $REPRO_OK
- minimized: $TMIN_OK
- rust regression skeletons: $TEXT_SKELETONS
- manual replay notes: $MANUAL_NOTES

| target | artifact | repro | tmin | skeleton |
| --- | --- | --- | --- | --- |
MD

while IFS=$'\t' read -r target base repro_status tmin_status skeleton_kind case_dir; do
  printf '| `%s` | `%s` | `%s` | `%s` | `%s` |\n' \
    "$target" "$base" "$repro_status" "$tmin_status" "$skeleton_kind" >> "$JOB_SUMMARY"
done < "$INDEX"

python3 - <<'PY' "$SUMMARY_JSON" "$INDEX" "$TOTAL" "$REPRO_OK" "$TMIN_OK" "$TEXT_SKELETONS" "$MANUAL_NOTES"
import csv
import json
import sys
from pathlib import Path

summary_path = Path(sys.argv[1])
index_path = Path(sys.argv[2])
total = int(sys.argv[3])
repro_ok = int(sys.argv[4])
tmin_ok = int(sys.argv[5])
text_skeletons = int(sys.argv[6])
manual_notes = int(sys.argv[7])
cases = []
if index_path.exists():
    with index_path.open(newline="") as fh:
        reader = csv.reader(fh, delimiter="\t")
        for row in reader:
            if len(row) != 6:
                continue
            target, artifact, repro_status, tmin_status, skeleton_kind, case_dir = row
            cases.append(
                {
                    "target": target,
                    "artifact": artifact,
                    "repro_status": repro_status,
                    "tmin_status": tmin_status,
                    "skeleton_kind": skeleton_kind,
                    "case_dir": case_dir,
                }
            )
summary_path.write_text(
    json.dumps(
        {
            "schema": "rr-triage-report",
            "version": 1,
            "kind": "fuzz",
            "artifacts": total,
            "reproduced": repro_ok,
            "minimized": tmin_ok,
            "rust_regression_skeletons": text_skeletons,
            "manual_replay_notes": manual_notes,
            "cases": cases,
        },
        indent=2,
    )
    + "\n"
)
PY
