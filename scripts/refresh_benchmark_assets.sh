#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATE_TAG="${DATE_TAG:-$(date +%F)}"
RUNS="${RUNS:-3}"
WARMUP="${WARMUP:-0}"
RR_BIN_OVERRIDE="${RR_BIN:-}"
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
  DATE_TAG, RUNS, WARMUP, RR_BIN
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

update_docs_optimization_snapshot() {
  python3 - \
    "$ROOT/docs/compiler/optimization.md" \
    "$DATE_TAG" \
    "$SKIP_RENJIN" \
    "$ROOT/docs/assets/signal-pipeline-runtime-${DATE_TAG}.csv" \
    "$ROOT/docs/assets/diffusion-backend-runtime-${DATE_TAG}.csv" \
    "$ROOT/docs/assets/backend-candidate-runtime-${DATE_TAG}.csv" <<'PY'
import csv
import sys
from pathlib import Path

doc_path = Path(sys.argv[1])
date_tag = sys.argv[2]
skip_renjin = sys.argv[3] == "1"
signal_csv = Path(sys.argv[4])
diffusion_csv = Path(sys.argv[5])
backend_csv = Path(sys.argv[6])

start_marker = "<!-- BEGIN GENERATED BENCHMARK SNAPSHOT -->"
end_marker = "<!-- END GENERATED BENCHMARK SNAPSHOT -->"
text = doc_path.read_text()
start = text.index(start_marker)
end = text.index(end_marker, start) + len(end_marker)

signal_rows = {row["id"]: row for row in csv.DictReader(signal_csv.open())}
diffusion_rows = {row["id"]: row for row in csv.DictReader(diffusion_csv.open())}
backend_rows = {row["id"]: row for row in csv.DictReader(backend_csv.open())}

def format_stdev(raw: str) -> str:
    return raw.rstrip("0").rstrip(".") if "." in raw else raw

def format_cell(row_map: dict[str, dict[str, str]], row_id: str) -> str:
    row = row_map[row_id]
    return f"`{row['mean_ms']} ({format_stdev(row['stdev_ms'])})`"

def ratio(row_map: dict[str, dict[str, str]], o0_id: str, other_id: str) -> str:
    base = float(row_map[o0_id]["mean_ms"])
    target = float(row_map[other_id]["mean_ms"])
    return f"`{base / target:.2f}x`"

refresh_cmd = f"scripts/refresh_benchmark_assets.sh --date {date_tag}"
if skip_renjin:
    refresh_cmd += " --skip-renjin"

renjin_note = (
    "Renjin rows were skipped for this snapshot."
    if skip_renjin
    else "This refresh used the local GNU R, Julia, NumPy, and Renjin environments."
)

block = f"""<!-- BEGIN GENERATED BENCHMARK SNAPSHOT -->
### Benchmark Snapshots

Refreshed with `{refresh_cmd}`.

#### Signal Pipeline Public Slice

[CSV](../assets/signal-pipeline-runtime-{date_tag}.csv) · [SVG](../assets/signal-pipeline-runtime-{date_tag}.svg)

![Signal Pipeline Runtime Comparison](../assets/signal-pipeline-runtime-{date_tag}.svg)

- RR O2 on GNU R: `{signal_rows['rr_o2_gnur']['mean_ms']} ms` cold / `{signal_rows['rr_o2_gnur_warm']['mean_ms']} ms` warm
- direct vectorized GNU R baseline: `{signal_rows['direct_r_vector']['mean_ms']} ms` cold / `{signal_rows['direct_r_vector_warm']['mean_ms']} ms` warm
- NumPy: `{signal_rows['numpy']['mean_ms']} ms`; C O3 native: `{signal_rows['c_o3']['mean_ms']} ms`
- emitted RR O2 artifact on this snapshot: `{signal_rows['rr_o2_gnur']['emit_lines']}` lines, `{signal_rows['rr_o2_gnur']['emit_repeat_loops']}` repeat loop, `{signal_rows['rr_o2_gnur']['pulse_vectorized']}` vectorized loops

#### Diffusion Optimizer Slice

[CSV](../assets/diffusion-backend-runtime-{date_tag}.csv) · [SVG](../assets/diffusion-backend-runtime-{date_tag}.svg)

![Diffusion Optimizer Tier Comparison](../assets/diffusion-backend-runtime-{date_tag}.svg)

- useful `-O2` reference points: `{diffusion_rows['heat_rr_o2']['mean_ms']} ms` / `{diffusion_rows['heat_rr_o2_warm']['mean_ms']} ms` for `heat_diffusion` cold/warm and `{diffusion_rows['reaction_rr_o2']['mean_ms']} ms` / `{diffusion_rows['reaction_rr_o2_warm']['mean_ms']} ms` for `reaction_diffusion` cold/warm

| Workload | O0 ms | O1 ms | O2 ms | O0/O1 | O0/O2 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `heat` | {format_cell(diffusion_rows, 'heat_rr_o0')} | {format_cell(diffusion_rows, 'heat_rr_o1')} | {format_cell(diffusion_rows, 'heat_rr_o2')} | {ratio(diffusion_rows, 'heat_rr_o0', 'heat_rr_o1')} | {ratio(diffusion_rows, 'heat_rr_o0', 'heat_rr_o2')} |
| `reaction` | {format_cell(diffusion_rows, 'reaction_rr_o0')} | {format_cell(diffusion_rows, 'reaction_rr_o1')} | {format_cell(diffusion_rows, 'reaction_rr_o2')} | {ratio(diffusion_rows, 'reaction_rr_o0', 'reaction_rr_o1')} | {ratio(diffusion_rows, 'reaction_rr_o0', 'reaction_rr_o2')} |

#### Optimizer Candidate Slice

[CSV](../assets/backend-candidate-runtime-{date_tag}.csv) · [SVG](../assets/backend-candidate-runtime-{date_tag}.svg)

![Optimizer Candidate Workload Comparison](../assets/backend-candidate-runtime-{date_tag}.svg)

| Workload | O0 ms | O1 ms | O2 ms | O0/O1 | O0/O2 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `bootstrap` | {format_cell(backend_rows, 'bootstrap_rr_o0')} | {format_cell(backend_rows, 'bootstrap_rr_o1')} | {format_cell(backend_rows, 'bootstrap_rr_o2')} | {ratio(backend_rows, 'bootstrap_rr_o0', 'bootstrap_rr_o1')} | {ratio(backend_rows, 'bootstrap_rr_o0', 'bootstrap_rr_o2')} |
| `orbital` | {format_cell(backend_rows, 'orbital_rr_o0')} | {format_cell(backend_rows, 'orbital_rr_o1')} | {format_cell(backend_rows, 'orbital_rr_o2')} | {ratio(backend_rows, 'orbital_rr_o0', 'orbital_rr_o1')} | {ratio(backend_rows, 'orbital_rr_o0', 'orbital_rr_o2')} |
| `vector` | {format_cell(backend_rows, 'vector_rr_o0')} | {format_cell(backend_rows, 'vector_rr_o1')} | {format_cell(backend_rows, 'vector_rr_o2')} | {ratio(backend_rows, 'vector_rr_o0', 'vector_rr_o1')} | {ratio(backend_rows, 'vector_rr_o0', 'vector_rr_o2')} |

Notes:

- `bootstrap_resample` gets most of its gain at `-O1`; `-O2` is still ahead of `-O0`, but not the best point on this snapshot.
- `orbital_sweep` is effectively flat on this snapshot, with warm `-O1/-O2` at `{backend_rows['orbital_rr_o1_warm']['mean_ms']} ms` and `{backend_rows['orbital_rr_o2_warm']['mean_ms']} ms`.
- `vector_fusion` splits cold and warm leadership: `-O2` is best cold, while `-O1` stays better on the warm path.
- {renjin_note}
<!-- END GENERATED BENCHMARK SNAPSHOT -->"""

doc_path.write_text(text[:start] + block + text[end:])
PY
}

prune_old_assets() {
  local pattern
  for pattern in signal-pipeline-runtime diffusion-backend-runtime backend-candidate-runtime; do
    find "$ROOT/docs/assets" -maxdepth 1 -type f \
      \( -name "${pattern}-*.csv" -o -name "${pattern}-*.svg" \) \
      ! -name "${pattern}-${DATE_TAG}.csv" \
      ! -name "${pattern}-${DATE_TAG}.svg" \
      -delete
  done
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
if [[ -n "$RR_BIN_OVERRIDE" ]]; then
  signal_cmd+=(--rr-bin "$RR_BIN_OVERRIDE")
fi
if [[ $SKIP_RENJIN -eq 1 ]]; then
  signal_cmd+=(--skip-renjin)
fi
"${signal_cmd[@]}"
copy_signal_assets

echo "-- refreshing diffusion backend assets"
diffusion_cmd=(python3 "$ROOT/scripts/bench_diffusion_backends.py" --runs "$RUNS" --warmup "$WARMUP")
if [[ -n "$RR_BIN_OVERRIDE" ]]; then
  diffusion_cmd+=(--rr-bin "$RR_BIN_OVERRIDE")
fi
"${diffusion_cmd[@]}"
copy_diffusion_assets

echo "-- refreshing backend candidate assets"
backend_cmd=(python3 "$ROOT/scripts/bench_backend_candidates.py" --runs "$RUNS" --warmup "$WARMUP")
if [[ -n "$RR_BIN_OVERRIDE" ]]; then
  backend_cmd+=(--rr-bin "$RR_BIN_OVERRIDE")
fi
"${backend_cmd[@]}"
copy_backend_candidate_assets

update_docs_optimization_snapshot
prune_old_assets

echo "updated:"
echo "  docs/assets/signal-pipeline-runtime-${DATE_TAG}.csv"
echo "  docs/assets/signal-pipeline-runtime-${DATE_TAG}.svg"
echo "  docs/assets/diffusion-backend-runtime-${DATE_TAG}.csv"
echo "  docs/assets/diffusion-backend-runtime-${DATE_TAG}.svg"
echo "  docs/assets/backend-candidate-runtime-${DATE_TAG}.csv"
echo "  docs/assets/backend-candidate-runtime-${DATE_TAG}.svg"
echo "  docs/compiler/optimization.md"
