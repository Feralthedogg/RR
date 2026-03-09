#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/paper}"
CSV_PATH="${CSV_PATH:-$OUT_DIR/perf_snapshot.csv}"
RAW_PATH="${RAW_PATH:-$OUT_DIR/perf_snapshot_raw.txt}"
PLOT_PATH="${PLOT_PATH:-$OUT_DIR/perf_snapshot_bars.tex}"

mkdir -p "$OUT_DIR"

tmp_output="$(mktemp)"
trap 'rm -f "$tmp_output"' EXIT

RR_EXAMPLE_PERF_REPEATS="${RR_EXAMPLE_PERF_REPEATS:-5}" \
  cargo test -q --test example_perf_smoke -- --ignored --nocapture | tee "$tmp_output"
cp "$tmp_output" "$RAW_PATH"

python3 - "$tmp_output" "$CSV_PATH" "$PLOT_PATH" <<'PY'
import csv
import re
import sys

src_path, csv_path, plot_path = sys.argv[1], sys.argv[2], sys.argv[3]
pattern = re.compile(
    r"^\s*(?P<label>[A-Za-z0-9_]+)\s+\|\s+"
    r"cO1\s+(?P<compile_o1_median>\d+)/(?P<compile_o1_iqr>\d+)\s+ms\s+\|\s+"
    r"cO2\s+(?P<compile_o2_median>\d+)/(?P<compile_o2_iqr>\d+)\s+ms\s+\|\s+"
    r"rO0\s+(?P<runtime_o0_median>\d+)/(?P<runtime_o0_iqr>\d+)\s+ms\s+\|\s+"
    r"rO1\s+(?P<runtime_o1_median>\d+)/(?P<runtime_o1_iqr>\d+)\s+ms\s+\|\s+"
    r"rO2\s+(?P<runtime_o2_median>\d+)/(?P<runtime_o2_iqr>\d+)\s+ms\s*$"
)

rows = []
with open(src_path, "r", encoding="utf-8") as fh:
    for line in fh:
        match = pattern.match(line.rstrip("\n"))
        if match:
            rows.append(
                {
                    "label": match.group("label"),
                    "compile_o1_median_ms": match.group("compile_o1_median"),
                    "compile_o1_iqr_ms": match.group("compile_o1_iqr"),
                    "compile_o2_median_ms": match.group("compile_o2_median"),
                    "compile_o2_iqr_ms": match.group("compile_o2_iqr"),
                    "runtime_o0_median_ms": match.group("runtime_o0_median"),
                    "runtime_o0_iqr_ms": match.group("runtime_o0_iqr"),
                    "runtime_o1_median_ms": match.group("runtime_o1_median"),
                    "runtime_o1_iqr_ms": match.group("runtime_o1_iqr"),
                    "runtime_o2_median_ms": match.group("runtime_o2_median"),
                    "runtime_o2_iqr_ms": match.group("runtime_o2_iqr"),
                }
            )

if not rows:
    raise SystemExit("no perf rows found in example_perf_smoke output")

for row in rows:
    runtime_o0 = int(row["runtime_o0_median_ms"])
    runtime_o1 = int(row["runtime_o1_median_ms"])
    runtime_o2 = int(row["runtime_o2_median_ms"])
    row["runtime_o1_speedup_vs_o0"] = f"{runtime_o0 / runtime_o1:.2f}" if runtime_o1 else "0.00"
    row["runtime_o2_speedup_vs_o0"] = f"{runtime_o0 / runtime_o2:.2f}" if runtime_o2 else "0.00"

with open(csv_path, "w", newline="", encoding="utf-8") as fh:
    writer = csv.DictWriter(
        fh,
        fieldnames=[
            "label",
            "compile_o1_median_ms",
            "compile_o1_iqr_ms",
            "compile_o2_median_ms",
            "compile_o2_iqr_ms",
            "runtime_o0_median_ms",
            "runtime_o0_iqr_ms",
            "runtime_o1_median_ms",
            "runtime_o1_iqr_ms",
            "runtime_o2_median_ms",
            "runtime_o2_iqr_ms",
            "runtime_o1_speedup_vs_o0",
            "runtime_o2_speedup_vs_o0",
        ],
    )
    writer.writeheader()
    writer.writerows(rows)

max_value = max(
    max(float(row["runtime_o1_speedup_vs_o0"]), float(row["runtime_o2_speedup_vs_o0"]))
    for row in rows
)
scale_cm = 2.0
min_cm = 0.06

def esc(label: str) -> str:
    return label.replace("_", r"\_")

display_labels = {
    "bootstrap_resample_bench": "bootstrap",
    "heat_diffusion_bench": "heat",
    "orbital_sweep_bench": "orbital",
    "reaction_diffusion_bench": "reaction",
    "vector_fusion_bench": "vector",
    "tesseract": "tesseract",
    "TOTAL": "total",
}

def bar(value: str, color: str) -> str:
    width = max((float(value) / max_value) * scale_cm, min_cm)
    return rf"\textcolor{{{color}}}{{\rule{{{width:.2f}cm}}{{1.3ex}}}}"

with open(plot_path, "w", encoding="utf-8") as fh:
    fh.write(r"\begin{tabular}{@{}ll@{}}" + "\n")
    fh.write(r"\toprule" + "\n")
    fh.write(r"Workload & Runtime speedup bars vs. O0 \\" + "\n")
    fh.write(r"\midrule" + "\n")
    for row in rows:
        if row["label"] == "TOTAL":
            continue
        bars = " ".join(
            [
                bar(row["runtime_o1_speedup_vs_o0"], "blue!65!black"),
                bar(row["runtime_o2_speedup_vs_o0"], "green!45!black"),
            ]
        )
        fh.write(f"{esc(display_labels.get(row['label'], row['label']))} & {bars} \\\\\n")
    fh.write(r"\midrule" + "\n")
    total = next(row for row in rows if row["label"] == "TOTAL")
    bars = " ".join(
        [
            bar(total["runtime_o1_speedup_vs_o0"], "blue!65!black"),
            bar(total["runtime_o2_speedup_vs_o0"], "green!45!black"),
        ]
    )
    fh.write(r"\textbf{" + esc(display_labels["TOTAL"]) + r"} & " + bars + r" \\" + "\n")
    fh.write(r"\bottomrule" + "\n")
    fh.write(r"\multicolumn{2}{@{}l@{}}{\footnotesize Legend: blue = O1 runtime speedup vs. O0, green = O2 runtime speedup vs. O0.}" + "\n")
    fh.write(r"\end{tabular}" + "\n")
PY

printf 'wrote %s, %s, and %s\n' "$CSV_PATH" "$RAW_PATH" "$PLOT_PATH"
