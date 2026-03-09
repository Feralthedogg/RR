#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/paper}"
SUMMARY_CSV="${SUMMARY_CSV:-$OUT_DIR/optimizer_firing_stats.csv}"
EXAMPLES_CSV="${EXAMPLES_CSV:-$OUT_DIR/optimizer_firing_stats_examples.csv}"
RAW_PATH="${RAW_PATH:-$OUT_DIR/optimizer_firing_stats_raw.txt}"
RR_BIN="${RR_BIN:-$ROOT/target/debug/RR}"

mkdir -p "$OUT_DIR"

if [[ ! -x "$RR_BIN" ]]; then
  cargo build -q
fi

python3 - "$ROOT" "$RR_BIN" "$SUMMARY_CSV" "$EXAMPLES_CSV" "$RAW_PATH" <<'PY'
import csv
import re
import subprocess
import sys
import tempfile
from pathlib import Path

root = Path(sys.argv[1])
rr_bin = Path(sys.argv[2])
summary_csv = Path(sys.argv[3])
examples_csv = Path(sys.argv[4])
raw_path = Path(sys.argv[5])

files = [
    p for p in sorted((root / "example").rglob("*.rr"))
    if "example/common/" not in p.as_posix()
]

vec_re = re.compile(r"Vectorized: (\d+) \| Reduced: (\d+) \| Simplified: (\d+) loops")
skip_re = re.compile(
    r"VecSkip: (\d+)/(\d+) "
    r"\(no-iv (\d+) \| bound (\d+) \| cfg (\d+) \| indirect (\d+) \| store (\d+) \| no-pattern (\d+)\)"
)
pass_re = re.compile(r"Passes: SCCP (\d+) \| GVN (\d+) \| LICM (\d+) \| BCE (\d+) \| TCO (\d+) \| DCE (\d+)")

per_example = []
aggregate = {
    "examples": 0,
    "vectorized": 0,
    "reduced": 0,
    "simplified": 0,
    "vecskip": 0,
    "vecseen": 0,
    "no_iv": 0,
    "bound": 0,
    "cfg": 0,
    "indirect": 0,
    "store": 0,
    "no_pattern": 0,
    "sccp": 0,
    "gvn": 0,
    "licm": 0,
    "bce": 0,
    "tco": 0,
    "dce": 0,
    "examples_with_vectorized": 0,
    "examples_with_reduced": 0,
    "examples_with_bce": 0,
}

with tempfile.TemporaryDirectory(prefix="rr-paper-opt-stats-") as tmpdir:
    tmpdir = Path(tmpdir)
    with raw_path.open("w", encoding="utf-8") as raw:
        for src in files:
            out = tmpdir / f"{src.stem}.R"
            proc = subprocess.run(
                [str(rr_bin), str(src), "-o", str(out), "-O2"],
                cwd=root,
                text=True,
                capture_output=True,
            )
            raw.write(f"=== {src.as_posix()} ===\n")
            raw.write(proc.stdout)
            if proc.stderr:
                raw.write(proc.stderr)
            raw.write("\n")
            if proc.returncode != 0:
                raise SystemExit(f"optimizer-stats compile failed for {src}")

            text = proc.stdout + "\n" + proc.stderr
            row = {"file": src.relative_to(root).as_posix()}
            m = vec_re.search(text)
            s = skip_re.search(text)
            p = pass_re.search(text)

            row["vectorized"] = int(m.group(1)) if m else 0
            row["reduced"] = int(m.group(2)) if m else 0
            row["simplified"] = int(m.group(3)) if m else 0

            if s:
                row["vecskip"] = int(s.group(1))
                row["vecseen"] = int(s.group(2))
                row["no_iv"] = int(s.group(3))
                row["bound"] = int(s.group(4))
                row["cfg"] = int(s.group(5))
                row["indirect"] = int(s.group(6))
                row["store"] = int(s.group(7))
                row["no_pattern"] = int(s.group(8))
            else:
                row["vecskip"] = 0
                row["vecseen"] = 0
                row["no_iv"] = 0
                row["bound"] = 0
                row["cfg"] = 0
                row["indirect"] = 0
                row["store"] = 0
                row["no_pattern"] = 0

            if p:
                row["sccp"] = int(p.group(1))
                row["gvn"] = int(p.group(2))
                row["licm"] = int(p.group(3))
                row["bce"] = int(p.group(4))
                row["tco"] = int(p.group(5))
                row["dce"] = int(p.group(6))
            else:
                row["sccp"] = 0
                row["gvn"] = 0
                row["licm"] = 0
                row["bce"] = 0
                row["tco"] = 0
                row["dce"] = 0

            per_example.append(row)
            aggregate["examples"] += 1
            for key in [
                "vectorized",
                "reduced",
                "simplified",
                "vecskip",
                "vecseen",
                "no_iv",
                "bound",
                "cfg",
                "indirect",
                "store",
                "no_pattern",
                "sccp",
                "gvn",
                "licm",
                "bce",
                "tco",
                "dce",
            ]:
                aggregate[key] += row[key]
            if row["vectorized"] > 0:
                aggregate["examples_with_vectorized"] += 1
            if row["reduced"] > 0:
                aggregate["examples_with_reduced"] += 1
            if row["bce"] > 0:
                aggregate["examples_with_bce"] += 1

summary_rows = [
    ("executable_examples", aggregate["examples"]),
    ("candidate_vector_loops_seen", aggregate["vecseen"]),
    ("vectorized_loops", aggregate["vectorized"]),
    ("reduction_rewrites", aggregate["reduced"]),
    ("simplified_loops", aggregate["simplified"]),
    ("examples_with_vectorization", aggregate["examples_with_vectorized"]),
    ("examples_with_reduction", aggregate["examples_with_reduced"]),
    ("examples_with_bce", aggregate["examples_with_bce"]),
    ("bce_hits", aggregate["bce"]),
    ("vecskip_total", aggregate["vecskip"]),
    ("vecskip_indirect", aggregate["indirect"]),
    ("vecskip_no_pattern", aggregate["no_pattern"]),
    ("vecskip_bound", aggregate["bound"]),
    ("vecskip_store", aggregate["store"]),
    ("vecskip_no_iv", aggregate["no_iv"]),
    ("vecskip_cfg", aggregate["cfg"]),
    ("sccp_hits", aggregate["sccp"]),
    ("gvn_hits", aggregate["gvn"]),
    ("licm_hits", aggregate["licm"]),
    ("dce_hits", aggregate["dce"]),
]

with summary_csv.open("w", newline="", encoding="utf-8") as fh:
    writer = csv.writer(fh)
    writer.writerow(["metric", "count"])
    writer.writerows(summary_rows)

with examples_csv.open("w", newline="", encoding="utf-8") as fh:
    fieldnames = [
        "file",
        "vectorized",
        "reduced",
        "simplified",
        "vecseen",
        "vecskip",
        "no_iv",
        "bound",
        "cfg",
        "indirect",
        "store",
        "no_pattern",
        "bce",
    ]
    writer = csv.DictWriter(fh, fieldnames=fieldnames)
    writer.writeheader()
    for row in per_example:
        writer.writerow({name: row[name] for name in fieldnames})

print(f"wrote {summary_csv}, {examples_csv}, and {raw_path}")
PY
