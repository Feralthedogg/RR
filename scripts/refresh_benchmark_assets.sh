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
  python3 - "$ROOT" "$DATE_TAG" <<'PY'
import csv
import importlib.util
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
date_tag = sys.argv[2]
scripts = root / "scripts"
sys.path.insert(0, str(scripts))

spec = importlib.util.spec_from_file_location(
    "bench_signal_pipeline", scripts / "bench_signal_pipeline.py"
)
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

src_csv = root / "target" / "signal_pipeline_bench" / "signal_pipeline_bench.csv"
dst_csv = root / "docs" / "assets" / f"signal-pipeline-runtime-{date_tag}.csv"
dst_svg = root / "docs" / "assets" / f"signal-pipeline-runtime-{date_tag}.svg"

rows = list(csv.DictReader(src_csv.open()))
preferred_ids = [
    "direct_r_scalar",
    "direct_r_vector",
    "direct_r_vector_warm",
    "rr_o2_gnur",
    "rr_o2_gnur_warm",
    "c_o3",
    "numpy",
    "julia",
    "direct_r_renjin",
    "rr_o2_renjin",
]
filtered = []
for row_id in preferred_ids:
    row = next((entry for entry in rows if entry["id"] == row_id), None)
    if row is not None:
        filtered.append(row)

mod.write_results_csv(dst_csv, filtered)
mod.write_svg_chart(dst_svg, filtered)
svg_text = dst_svg.read_text()
svg_text = re.sub(
    r"250,000 samples, 16 passes, Apple M4, optimizer tiers O0/O1/O2 plus direct baselines",
    "250,000 samples, 16 passes, Apple M4, cross-language public slice with RR O2 cold/warm rows",
    svg_text,
)
dst_svg.write_text(svg_text)
PY
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
  python3 - "$ROOT/docs/compiler/testing.md" "$DATE_TAG" "$SKIP_RENJIN" <<'PY'
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

update_docs_testing_diffusion_summary() {
  python3 - "$ROOT/docs/compiler/testing.md" "$ROOT/docs/assets/diffusion-backend-runtime-${DATE_TAG}.csv" <<'PY'
import csv
import sys
from pathlib import Path

doc_path = Path(sys.argv[1])
csv_path = Path(sys.argv[2])
text = doc_path.read_text()
rows = list(csv.DictReader(csv_path.open()))

row_map = {row["id"]: row for row in rows}
summary = (
    f"- On the current signoff snapshot, the useful `-O2` reference points are\n"
    f"  roughly `{row_map['heat_rr_o2']['mean_ms']} ms` / `{row_map['heat_rr_o2_warm']['mean_ms']} ms` "
    f"for `heat_diffusion` cold/warm and\n"
    f"  `{row_map['reaction_rr_o2']['mean_ms']} ms` / `{row_map['reaction_rr_o2_warm']['mean_ms']} ms` "
    f"for `reaction_diffusion` cold/warm."
)
start = text.index("- On the current signoff snapshot, the useful `-O2` reference points are")
end = text.index("\n\n### Optimizer Candidate Slice", start)
text = text[:start] + summary + text[end:]
doc_path.write_text(text)
PY
}

update_docs_testing_optimizer_delta_table() {
  python3 - "$ROOT/docs/compiler/testing.md" "$ROOT/docs/assets/diffusion-backend-runtime-${DATE_TAG}.csv" "$ROOT/docs/assets/backend-candidate-runtime-${DATE_TAG}.csv" <<'PY'
import csv
import sys
from pathlib import Path

doc_path = Path(sys.argv[1])
diffusion_csv = Path(sys.argv[2])
backend_csv = Path(sys.argv[3])
text = doc_path.read_text()
row_map = {}
for csv_path in [diffusion_csv, backend_csv]:
    for row in csv.DictReader(csv_path.open()):
        row_map[row["id"]] = row

def format_cell(row_id: str) -> str:
    row = row_map[row_id]
    return f"`{row['mean_ms']} ({row['stdev_ms'].rstrip('0').rstrip('.') if '.' in row['stdev_ms'] else row['stdev_ms']})`"

def ratio(o0: str, other: str) -> str:
    base = float(row_map[o0]["mean_ms"])
    target = float(row_map[other]["mean_ms"])
    return f"`{base / target:.2f}x`"

table = "\n".join(
    [
        "| Workload | O0 ms | O1 ms | O2 ms | O1/O0 | O2/O0 |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
        f"| `bootstrap` | {format_cell('bootstrap_rr_o0')} | {format_cell('bootstrap_rr_o1')} | {format_cell('bootstrap_rr_o2')} | {ratio('bootstrap_rr_o0', 'bootstrap_rr_o1')} | {ratio('bootstrap_rr_o0', 'bootstrap_rr_o2')} |",
        f"| `heat` | {format_cell('heat_rr_o0')} | {format_cell('heat_rr_o1')} | {format_cell('heat_rr_o2')} | {ratio('heat_rr_o0', 'heat_rr_o1')} | {ratio('heat_rr_o0', 'heat_rr_o2')} |",
        f"| `orbital` | {format_cell('orbital_rr_o0')} | {format_cell('orbital_rr_o1')} | {format_cell('orbital_rr_o2')} | {ratio('orbital_rr_o0', 'orbital_rr_o1')} | {ratio('orbital_rr_o0', 'orbital_rr_o2')} |",
        f"| `reaction` | {format_cell('reaction_rr_o0')} | {format_cell('reaction_rr_o1')} | {format_cell('reaction_rr_o2')} | {ratio('reaction_rr_o0', 'reaction_rr_o1')} | {ratio('reaction_rr_o0', 'reaction_rr_o2')} |",
        f"| `vector` | {format_cell('vector_rr_o0')} | {format_cell('vector_rr_o1')} | {format_cell('vector_rr_o2')} | {ratio('vector_rr_o0', 'vector_rr_o1')} | {ratio('vector_rr_o0', 'vector_rr_o2')} |",
    ]
)

start = text.index("| Workload | O0 ms | O1 ms | O2 ms | O1/O0 | O2/O0 |")
end = text.index("\n\nNotes:\n", start)
text = text[:start] + table + text[end:]

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
signal_cmd=(python3 "$ROOT/scripts/bench_signal_pipeline_docs_slice.py" --runs "$RUNS" --warmup "$WARMUP")
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
update_docs_testing_diffusion_summary
update_docs_testing_optimizer_delta_table

echo "updated:"
echo "  docs/assets/signal-pipeline-runtime-${DATE_TAG}.csv"
echo "  docs/assets/signal-pipeline-runtime-${DATE_TAG}.svg"
echo "  docs/assets/diffusion-backend-runtime-${DATE_TAG}.csv"
echo "  docs/assets/diffusion-backend-runtime-${DATE_TAG}.svg"
echo "  docs/assets/backend-candidate-runtime-${DATE_TAG}.csv"
echo "  docs/assets/backend-candidate-runtime-${DATE_TAG}.svg"
echo "  docs/compiler/testing.md"
