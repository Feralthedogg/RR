#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATE_TAG="${DATE_TAG:-$(date +%F)}"
RUNS="${RUNS:-3}"
WARMUP="${WARMUP:-0}"
SKIP_RENJIN=0

usage() {
  cat <<EOF
Usage: scripts/refresh_benchmark_assets.sh [options]

Options:
  --date <YYYY-MM-DD>   Override the docs asset date tag. Default: today.
  --runs <n>            Timed runs per benchmark script. Default: ${RUNS}
  --warmup <n>          Warmup runs per benchmark script. Default: ${WARMUP}
  --skip-renjin         Skip Renjin rows when refreshing benchmark assets.
  --help                Show this help.

Environment overrides:
  DATE_TAG, RUNS, WARMUP
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --date)
      DATE_TAG="${2:-}"
      [[ -n "$DATE_TAG" ]] || { echo "missing value for --date" >&2; exit 2; }
      shift 2
      ;;
    --runs)
      RUNS="${2:-}"
      [[ -n "$RUNS" ]] || { echo "missing value for --runs" >&2; exit 2; }
      shift 2
      ;;
    --warmup)
      WARMUP="${2:-}"
      [[ -n "$WARMUP" ]] || { echo "missing value for --warmup" >&2; exit 2; }
      shift 2
      ;;
    --skip-renjin)
      SKIP_RENJIN=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

copy_signal_assets() {
  mkdir -p "$ROOT/docs/assets"
  cp "$ROOT/target/signal_pipeline_bench/signal_pipeline_bench.csv" \
    "$ROOT/docs/assets/signal-pipeline-runtime-${DATE_TAG}.csv"
  cp "$ROOT/target/signal_pipeline_bench/signal_pipeline_bench.svg" \
    "$ROOT/docs/assets/signal-pipeline-runtime-${DATE_TAG}.svg"
}

copy_diffusion_assets() {
  mkdir -p "$ROOT/docs/assets"
  cp "$ROOT/target/diffusion_backend_bench/diffusion_backend_bench.csv" \
    "$ROOT/docs/assets/diffusion-backend-runtime-${DATE_TAG}.csv"
  cp "$ROOT/target/diffusion_backend_bench/diffusion_backend_bench.svg" \
    "$ROOT/docs/assets/diffusion-backend-runtime-${DATE_TAG}.svg"
}

copy_backend_candidate_assets() {
  mkdir -p "$ROOT/docs/assets"
  cp "$ROOT/target/backend_candidate_bench/backend_candidate_bench.csv" \
    "$ROOT/docs/assets/backend-candidate-runtime-${DATE_TAG}.csv"
  cp "$ROOT/target/backend_candidate_bench/backend_candidate_bench.svg" \
    "$ROOT/docs/assets/backend-candidate-runtime-${DATE_TAG}.svg"
}

update_docs_testing_links() {
  python3 - "$ROOT/docs/testing.md" "$DATE_TAG" "$SKIP_RENJIN" <<'PY'
import re
import sys
from pathlib import Path

path = Path(sys.argv[1])
date_tag = sys.argv[2]
skip_renjin = sys.argv[3] == "1"
text = path.read_text()
text = re.sub(
    r"signal-pipeline-runtime-\d{4}-\d{2}-\d{2}\.csv",
    f"signal-pipeline-runtime-{date_tag}.csv",
    text,
)
text = re.sub(
    r"signal-pipeline-runtime-\d{4}-\d{2}-\d{2}\.svg",
    f"signal-pipeline-runtime-{date_tag}.svg",
    text,
)
text = re.sub(
    r"diffusion-backend-runtime-\d{4}-\d{2}-\d{2}\.csv",
    f"diffusion-backend-runtime-{date_tag}.csv",
    text,
)
text = re.sub(
    r"diffusion-backend-runtime-\d{4}-\d{2}-\d{2}\.svg",
    f"diffusion-backend-runtime-{date_tag}.svg",
    text,
)
text = re.sub(
    r"backend-candidate-runtime-\d{4}-\d{2}-\d{2}\.csv",
    f"backend-candidate-runtime-{date_tag}.csv",
    text,
)
text = re.sub(
    r"backend-candidate-runtime-\d{4}-\d{2}-\d{2}\.svg",
    f"backend-candidate-runtime-{date_tag}.svg",
    text,
)
text = re.sub(
    r"scripts/refresh_benchmark_assets\.sh --date \d{4}-\d{2}-\d{2}( --skip-renjin)?",
    (
        f"scripts/refresh_benchmark_assets.sh --date {date_tag} --skip-renjin"
        if skip_renjin
        else f"scripts/refresh_benchmark_assets.sh --date {date_tag}"
    ),
    text,
)
text = re.sub(
    r"This refresh used the local GNU R, Julia, NumPy, and Renjin environments, with\s+Renjin `3\.5-beta76` running on Homebrew OpenJDK `25\.0\.2`\.",
    (
        "This refresh used the local GNU R, Julia, and NumPy environments. "
        "Renjin rows were skipped for this snapshot."
        if skip_renjin
        else "This refresh used the local GNU R, Julia, NumPy, and Renjin environments, "
        "with Renjin `3.5-beta76` running on Homebrew OpenJDK `25.0.2`."
    ),
    text,
)
path.write_text(text)
PY
}

update_docs_testing_signal_table() {
  python3 - "$ROOT/docs/testing.md" "$ROOT/docs/assets/signal-pipeline-runtime-${DATE_TAG}.csv" <<'PY'
import csv
import sys
from pathlib import Path

doc_path = Path(sys.argv[1])
csv_path = Path(sys.argv[2])
text = doc_path.read_text()

with csv_path.open() as f:
    rows = list(csv.DictReader(f))

label_map = {
    "direct_r_scalar": "direct base R (scalar) on GNU R",
    "direct_r_vector": "direct base R (vectorized) on GNU R",
    "direct_r_vector_warm": "direct base R (vectorized, warm avg x10) on GNU R",
    "rr_o2_gnur": "RR O2 emitted R on GNU R",
    "rr_o2_gnur_warm": "RR O2 emitted R (warm avg x10) on GNU R",
    "rr_o2_native_gnur": "RR O2 native on GNU R",
    "rr_o2_native_gnur_warm": "RR O2 native (warm avg x10) on GNU R",
    "rr_o2_parallel_r_gnur": "RR O2 parallel R on GNU R",
    "rr_o2_parallel_r_gnur_warm": "RR O2 parallel R (warm avg x10) on GNU R",
    "rr_o2_native_openmp_gnur": "RR O2 native + OpenMP on GNU R",
    "rr_o2_native_openmp_gnur_warm": "RR O2 native + OpenMP (warm avg x10) on GNU R",
    "c_o3": "C O3 native",
    "numpy": "NumPy",
    "julia": "Julia",
    "direct_r_renjin": "direct base R (vectorized) on Renjin",
    "rr_o2_renjin": "RR O2 on Renjin",
}

note_map = {
    "direct_r_scalar": "loop-based scalar base-R baseline",
    "direct_r_vector": "idiomatic base-R vector baseline",
    "direct_r_vector_warm": "same vector kernel averaged across repeated warm calls in one R process",
    "rr_o2_gnur": "same workload compiled from RR",
    "rr_o2_gnur_warm": "RR-emitted kernel averaged across repeated warm calls in one R process",
    "rr_o2_native_gnur": "same workload compiled from RR with required native backend",
    "rr_o2_native_gnur_warm": "RR native kernel averaged across repeated warm calls in one R process",
    "rr_o2_parallel_r_gnur": "same workload compiled from RR with required R parallel backend",
    "rr_o2_parallel_r_gnur_warm": "RR parallel-R kernel averaged across repeated warm calls in one R process",
    "rr_o2_native_openmp_gnur": "same workload compiled from RR with required native backend plus OpenMP parallel backend",
    "rr_o2_native_openmp_gnur_warm": "RR native+OpenMP kernel averaged across repeated warm calls in one R process",
    "c_o3": "`clang -O3`, single-threaded",
    "numpy": "vectorized array math on CPython + NumPy `2.4.3`",
    "julia": "Julia `1.12.5`, base loops with `@inbounds`",
    "direct_r_renjin": "same idiomatic base-R script on Renjin `3.5-beta76`",
    "rr_o2_renjin": "RR-emitted R on Renjin `3.5-beta76`",
}

ordered = []
for row in rows:
    row_id = row["id"]
    if row_id not in label_map:
        continue
    ordered.append(
        f"| {label_map[row_id]} | `{row['mean_ms']}` | `{row['stdev_ms']}` | {note_map[row_id]} |"
    )

table = "\n".join(
    [
        "| Slice | Mean ms | Std ms | Notes |",
        "| --- | ---: | ---: | --- |",
        *ordered,
    ]
)

start = text.index("| Slice | Mean ms | Std ms | Notes |")
end = text.index("\n\nNotes:\n", start)
updated = text[:start] + table + text[end:]
doc_path.write_text(updated)
PY
}

update_docs_testing_diffusion_tables() {
  python3 - "$ROOT/docs/testing.md" "$ROOT/docs/assets/diffusion-backend-runtime-${DATE_TAG}.csv" <<'PY'
import csv
import sys
from pathlib import Path

doc_path = Path(sys.argv[1])
csv_path = Path(sys.argv[2])
text = doc_path.read_text()
rows = list(csv.DictReader(csv_path.open()))

section_start = text.index("### Diffusion Backend Slice")
cold_label = "| Workload | RR O2 | Native | Parallel R | Native + OpenMP |"
warm_label = "| Workload | RR O2 warm | Native warm | Parallel R warm | Native + OpenMP warm |"

row_map = {row["id"]: row for row in rows}
workloads = [
    ("heat diffusion", "heat"),
    ("reaction diffusion", "reaction"),
]

cold_table = "\n".join([
    cold_label,
    "| --- | ---: | ---: | ---: | ---: |",
    *[
        f"| {label} | `{row_map[f'{key}_rr_o2']['mean_ms']}` | `{row_map[f'{key}_native']['mean_ms']}` | `{row_map[f'{key}_parallel_r']['mean_ms']}` | `{row_map[f'{key}_native_openmp']['mean_ms']}` |"
        for label, key in workloads
    ],
])

warm_table = "\n".join([
    warm_label,
    "| --- | ---: | ---: | ---: | ---: |",
    *[
        f"| {label} | `{row_map[f'{key}_rr_o2_warm']['mean_ms']}` | `{row_map[f'{key}_native_warm']['mean_ms']}` | `{row_map[f'{key}_parallel_r_warm']['mean_ms']}` | `{row_map[f'{key}_native_openmp_warm']['mean_ms']}` |"
        for label, key in workloads
    ],
])

cold_start = text.index(cold_label, section_start)
cold_end = text.index("\n\nWarm runtime means:\n", cold_start)
text = text[:cold_start] + cold_table + text[cold_end:]

warm_start = text.index(warm_label, section_start)
warm_end = text.index("\n\nNotes:\n", warm_start)
text = text[:warm_start] + warm_table + text[warm_end:]

doc_path.write_text(text)
PY
}

update_docs_testing_backend_candidate_tables() {
  python3 - "$ROOT/docs/testing.md" "$ROOT/docs/assets/backend-candidate-runtime-${DATE_TAG}.csv" <<'PY'
import csv
import sys
from pathlib import Path

doc_path = Path(sys.argv[1])
csv_path = Path(sys.argv[2])
text = doc_path.read_text()
rows = list(csv.DictReader(csv_path.open()))

section_start = text.index("### Backend Candidate Slice")
cold_label = "| Workload | RR O2 | Native | Parallel R | Native + OpenMP |"
warm_label = "| Workload | RR O2 warm | Native warm | Parallel R warm | Native + OpenMP warm |"

row_map = {row["id"]: row for row in rows}
workloads = [
    ("vector fusion", "vector"),
    ("orbital sweep", "orbital"),
    ("bootstrap resample", "bootstrap"),
]

cold_table = "\n".join([
    cold_label,
    "| --- | ---: | ---: | ---: | ---: |",
    *[
        f"| {label} | `{row_map[f'{key}_rr_o2']['mean_ms']}` | `{row_map[f'{key}_native']['mean_ms']}` | `{row_map[f'{key}_parallel_r']['mean_ms']}` | `{row_map[f'{key}_native_openmp']['mean_ms']}` |"
        for label, key in workloads
    ],
])

warm_table = "\n".join([
    warm_label,
    "| --- | ---: | ---: | ---: | ---: |",
    *[
        f"| {label} | `{row_map[f'{key}_rr_o2_warm']['mean_ms']}` | `{row_map[f'{key}_native_warm']['mean_ms']}` | `{row_map[f'{key}_parallel_r_warm']['mean_ms']}` | `{row_map[f'{key}_native_openmp_warm']['mean_ms']}` |"
        for label, key in workloads
    ],
])

cold_start = text.index(cold_label, section_start)
cold_end = text.index("\n\nWarm runtime means:\n", cold_start)
text = text[:cold_start] + cold_table + text[cold_end:]

warm_start = text.index(warm_label, section_start)
warm_end = text.index("\n\nNotes:\n", warm_start)
text = text[:warm_start] + warm_table + text[warm_end:]

doc_path.write_text(text)
PY
}

echo "== RR Benchmark Asset Refresh =="
echo "date tag: ${DATE_TAG}"
echo "runs: ${RUNS}"
echo "warmup: ${WARMUP}"
if [[ $SKIP_RENJIN -eq 1 ]]; then
  echo "renjin: skipped"
else
  echo "renjin: included when available"
fi

echo "-- refreshing signal pipeline assets"
signal_cmd=(python3 "$ROOT/scripts/bench_signal_pipeline.py" --runs "$RUNS" --warmup "$WARMUP")
if [[ $SKIP_RENJIN -eq 1 ]]; then
  signal_cmd+=(--skip-renjin)
fi
"${signal_cmd[@]}"
copy_signal_assets

echo "-- refreshing diffusion backend assets"
python3 "$ROOT/scripts/bench_diffusion_backends.py" --runs "$RUNS" --warmup "$WARMUP"
copy_diffusion_assets

echo "-- refreshing backend candidate assets"
python3 "$ROOT/scripts/bench_backend_candidates.py" --runs "$RUNS" --warmup "$WARMUP"
copy_backend_candidate_assets

update_docs_testing_links

echo "updated:"
echo "  docs/assets/signal-pipeline-runtime-${DATE_TAG}.csv"
echo "  docs/assets/signal-pipeline-runtime-${DATE_TAG}.svg"
echo "  docs/assets/diffusion-backend-runtime-${DATE_TAG}.csv"
echo "  docs/assets/diffusion-backend-runtime-${DATE_TAG}.svg"
echo "  docs/assets/backend-candidate-runtime-${DATE_TAG}.csv"
echo "  docs/assets/backend-candidate-runtime-${DATE_TAG}.svg"
echo "  docs/testing.md"
