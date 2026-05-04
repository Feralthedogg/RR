#!/usr/bin/env python3

from __future__ import annotations

import argparse
import csv
import json
import math
import os
import pathlib
import re
import statistics
import subprocess
import textwrap
import time

from bench_utils import (
    attach_diagnostics,
    compile_rr_variant,
    rr_artifact_diagnostics,
    write_results_csv as write_results_csv_dynamic,
)

ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_OUT_DIR = ROOT / "target" / "backend_candidate_bench"
DEFAULT_RR_BIN = ROOT / "target" / "release" / "RR"
WARM_KERNEL_ITERS = 10
PARALLEL_THREADS = 4

WORKLOADS = {
    "vector": {
        "src": ROOT / "example" / "benchmarks" / "vector_fusion_bench.rr",
        "metrics": ["vector_fusion_tail", "vector_fusion_mean"],
        "title": "Vector Fusion",
        "parallel_min_trip": 64,
    },
    "orbital": {
        "src": ROOT / "example" / "benchmarks" / "orbital_sweep_bench.rr",
        "metrics": ["orbit_bench_radius", "orbit_bench_speed"],
        "title": "Orbital Sweep",
        "parallel_min_trip": 1,
    },
    "bootstrap": {
        "src": ROOT / "example" / "benchmarks" / "bootstrap_resample_bench.rr",
        "metrics": ["bootstrap_bench_mean", "bootstrap_bench_acc"],
        "title": "Bootstrap Resample",
        "parallel_min_trip": 64,
    },
}


def run(
    cmd: list[str],
    *,
    env: dict[str, str] | None = None,
    capture_output: bool = False,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        cwd=ROOT,
        env=env,
        check=True,
        text=True,
        stdout=subprocess.PIPE if capture_output else subprocess.DEVNULL,
        stderr=subprocess.PIPE if capture_output else subprocess.DEVNULL,
    )


def benchmark(
    cmd: list[str],
    *,
    runs: int,
    warmup: int,
    env: dict[str, str],
    scale: float = 1.0,
) -> dict[str, object]:
    for _ in range(warmup):
        run(cmd, env=env)

    timings_ms: list[float] = []
    for _ in range(runs):
        start = time.perf_counter()
        run(cmd, env=env)
        timings_ms.append(((time.perf_counter() - start) * 1000.0) / scale)

    return {
        "runs_ms": [round(t, 1) for t in timings_ms],
        "mean_ms": round(statistics.mean(timings_ms), 1),
        "median_ms": round(statistics.median(timings_ms), 1),
        "stdev_ms": round(statistics.stdev(timings_ms), 1) if len(timings_ms) > 1 else 0.0,
        "min_ms": round(min(timings_ms), 1),
        "max_ms": round(max(timings_ms), 1),
    }


def ensure_release_rr(rr_bin: pathlib.Path) -> pathlib.Path:
    if not rr_bin.exists():
        run(["cargo", "build", "--release", "--bin", "RR"])
    if not rr_bin.exists():
        raise FileNotFoundError(f"missing RR binary after build: {rr_bin}")
    return rr_bin


def purge_native_artifacts() -> None:
    native_dir = ROOT / "target" / "native"
    if not native_dir.exists():
        return
    for path in native_dir.glob("rr_native*"):
        if path.is_file():
            path.unlink()


def common_env() -> dict[str, str]:
    env = os.environ.copy()
    env["OMP_NUM_THREADS"] = "1"
    env["OPENBLAS_NUM_THREADS"] = "1"
    env["MKL_NUM_THREADS"] = "1"
    env["VECLIB_MAXIMUM_THREADS"] = "1"
    env["NUMEXPR_NUM_THREADS"] = "1"
    env["JULIA_NUM_THREADS"] = "1"
    return env


def parse_metrics(text: str, metric_names: list[str]) -> dict[str, float]:
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    metrics: dict[str, float] = {}

    kv_lines = [line for line in lines if "=" in line]
    if kv_lines:
        for line in kv_lines:
            name, value = line.split("=", 1)
            if name.strip() in metric_names:
                metrics[name.strip()] = float(value.strip())
        if set(metrics) >= set(metric_names):
            return metrics

    normalized: list[str] = []
    for line in lines:
        match = re.match(r'^\[\d+\]\s+"(.*)"$', line)
        if match:
            normalized.append(match.group(1))
            continue
        match = re.match(r"^\[\d+\]\s+(.+)$", line)
        if match:
            normalized.append(match.group(1))
            continue
        normalized.append(line)

    idx = 0
    while idx + 1 < len(normalized):
        name = normalized[idx]
        if name not in metric_names:
            idx += 1
            continue
        metrics[name] = float(normalized[idx + 1])
        idx += 2

    if set(metrics) >= set(metric_names):
        return metrics

    raise ValueError(f"unexpected metric output: {text}")


def assert_metrics_close(
    reference: dict[str, float],
    candidate: dict[str, float],
    metric_names: list[str],
    *,
    label: str,
    rel_tol: float = 1e-8,
    abs_tol: float = 1e-8,
) -> None:
    for name in metric_names:
        lhs = reference[name]
        rhs = candidate[name]
        if not math.isclose(lhs, rhs, rel_tol=rel_tol, abs_tol=abs_tol):
            raise AssertionError(
                f"{label} metric mismatch for {name}: ref={lhs:.12f} got={rhs:.12f}"
            )


def write_text(path: pathlib.Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def render_rr_warm_driver(rr_r: pathlib.Path) -> str:
    rr_path = str(rr_r).replace("\\", "\\\\")
    return textwrap.dedent(
        f"""\
        bench_env <- new.env(parent = baseenv())
        bench_env$print <- function(...) invisible(NULL)
        bench_env$rr_mark <- function(...) invisible(NULL)
        rr_code <- readLines("{rr_path}", warn = FALSE)
        entry_marker <- "# --- RR synthesized entrypoints (auto-generated) ---"
        entry_idx <- match(entry_marker, rr_code)
        if (!is.na(entry_idx)) {{
          rr_code <- rr_code[seq_len(entry_idx - 1L)]
        }}
        rr_conn <- textConnection(rr_code)
        on.exit(close(rr_conn), add = TRUE)
        source(rr_conn, local = bench_env)

        kernel_name <- if (exists("Sym_1", envir = bench_env, inherits = FALSE)) "Sym_1" else "Sym_top_0"
        body_name <- paste0(".__rr_body_", kernel_name)
        if (exists(body_name, envir = bench_env, inherits = FALSE)) {{
          kernel <- eval(call("function", pairlist(), get(body_name, envir = bench_env, inherits = FALSE)), envir = bench_env)
        }} else {{
          kernel <- get(kernel_name, envir = bench_env, inherits = FALSE)
        }}
        for (.rr_iter in seq_len({WARM_KERNEL_ITERS}L)) {{
          kernel()
        }}
        """
    )


def write_results_csv(path: pathlib.Path, rows: list[dict[str, object]]) -> None:
    write_results_csv_dynamic(
        path,
        rows,
        [
            "id",
            "workload",
            "label",
            "engine",
            "artifact",
            "runs_ms",
            "mean_ms",
            "median_ms",
            "stdev_ms",
            "min_ms",
            "max_ms",
            "notes",
        ],
    )


def write_svg_chart(path: pathlib.Path, rows: list[dict[str, object]]) -> None:
    width = 980
    left = 390
    right = 70
    top = 86
    row_gap = 40
    bar_h = 22
    height = top + len(rows) * row_gap + 94
    max_mean = max(float(row["mean_ms"]) for row in rows)
    usable = width - left - right
    ticks = 5
    tick_values = [max_mean * idx / ticks for idx in range(ticks + 1)]
    colors = {
        "rr_o0": "#6d597a",
        "rr_o0_warm": "#b07aa1",
        "rr_o1": "#457b9d",
        "rr_o1_warm": "#74a7d4",
        "rr_o2": "#2a9d8f",
        "rr_o2_warm": "#8fd0c4",
    }

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title desc">',
        '  <title id="title">Optimizer Candidate Workload Comparison</title>',
        '  <desc id="desc">Mean wall-clock runtime in milliseconds for RR candidate workloads across optimization tiers.</desc>',
        f'  <rect width="{width}" height="{height}" fill="#fcfcf8"/>',
        '  <text x="24" y="34" font-family="Helvetica, Arial, sans-serif" font-size="22" fill="#16212b">Optimizer Candidate Workload Comparison</text>',
        f'  <text x="24" y="56" font-family="Helvetica, Arial, sans-serif" font-size="12" fill="#4b5b67">Vector fusion, orbital sweep, bootstrap resample; Apple M4; RR O0/O1/O2 cold and warm timing</text>',
        f'  <line x1="{left}" y1="{top}" x2="{left}" y2="{height - 60}" stroke="#9aa8b2" stroke-width="1"/>',
        f'  <line x1="{left}" y1="{height - 60}" x2="{width - right}" y2="{height - 60}" stroke="#9aa8b2" stroke-width="1"/>',
        '  <g font-family="Helvetica, Arial, sans-serif" font-size="11" fill="#5d6b75">',
    ]
    for value in tick_values:
        x = left + (value / max_mean) * usable if max_mean else left
        lines.append(f'    <text x="{x:.0f}" y="{height - 42}" text-anchor="middle">{value:.0f}</text>')
    lines.append("  </g>")
    lines.append('  <g font-family="Helvetica, Arial, sans-serif" font-size="14" fill="#16212b">')
    for idx, row in enumerate(rows):
        y = top + idx * row_gap + 16
        lines.append(f'    <text x="24" y="{y}">{row["label"]}</text>')
    lines.append("  </g>")
    for idx, row in enumerate(rows):
        bar_y = top + idx * row_gap
        bar_w = (float(row["mean_ms"]) / max_mean) * usable if max_mean else 0.0
        variant = str(row["id"]).split("_", 1)[1]
        color = colors.get(variant, "#6d597a")
        lines.append(f'  <rect x="{left}" y="{bar_y}" width="{bar_w:.1f}" height="{bar_h}" fill="{color}" rx="4"/>')
        lines.append(
            f'  <text x="{left + bar_w + 10:.1f}" y="{bar_y + 16}" font-family="Helvetica, Arial, sans-serif" font-size="12" fill="#16212b">{row["mean_ms"]:.1f} ms ± {row["stdev_ms"]:.1f}</text>'
        )
    lines.extend(
        [
            '  <g font-family="Helvetica, Arial, sans-serif" font-size="11" fill="#4b5b67">',
            f'    <text x="24" y="{height - 22}">Source data: {path.with_suffix(".csv").name}</text>',
            "  </g>",
            "</svg>",
        ]
    )
    path.write_text("\n".join(lines) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description="Benchmark RR optimizer tiers on candidate workloads.")
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--warmup", type=int, default=1)
    parser.add_argument("--out-dir", type=pathlib.Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--rr-bin", type=pathlib.Path, default=DEFAULT_RR_BIN)
    parser.add_argument("--rscript-bin", default="Rscript")
    args = parser.parse_args()

    out_dir = args.out_dir.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    env = common_env()
    rr_bin = ensure_release_rr(args.rr_bin.resolve())

    rows: list[dict[str, object]] = []

    for workload_id, info in WORKLOADS.items():
        src = pathlib.Path(info["src"]).resolve()
        metric_names = list(info["metrics"])
        title = str(info["title"])
        parallel_min_trip = int(info["parallel_min_trip"])
        workload_dir = out_dir / workload_id
        workload_dir.mkdir(parents=True, exist_ok=True)

        rr_o0 = workload_dir / f"{workload_id}_rr_o0.R"
        rr_o0_warm = workload_dir / f"{workload_id}_rr_o0_warm.R"
        rr_o1 = workload_dir / f"{workload_id}_rr_o1.R"
        rr_o1_warm = workload_dir / f"{workload_id}_rr_o1_warm.R"
        rr_o2 = workload_dir / f"{workload_id}_rr_o2.R"
        rr_o2_warm = workload_dir / f"{workload_id}_rr_o2_warm.R"
        rr_o0_pulse = workload_dir / f"{workload_id}_rr_o0.pulse.json"
        rr_o1_pulse = workload_dir / f"{workload_id}_rr_o1.pulse.json"
        rr_o2_pulse = workload_dir / f"{workload_id}_rr_o2.pulse.json"

        compile_rr_variant(ROOT, rr_bin, src, rr_o0, "-O0", env, pulse_json_path=rr_o0_pulse)
        compile_rr_variant(ROOT, rr_bin, src, rr_o1, "-O1", env, pulse_json_path=rr_o1_pulse)
        compile_rr_variant(ROOT, rr_bin, src, rr_o2, "-O2", env, pulse_json_path=rr_o2_pulse)

        for rr_file in (rr_o0, rr_o1, rr_o2):
            rr_file.write_text("options(digits = 15)\n" + rr_file.read_text())
        write_text(rr_o0_warm, render_rr_warm_driver(rr_o0))
        write_text(rr_o1_warm, render_rr_warm_driver(rr_o1))
        write_text(rr_o2_warm, render_rr_warm_driver(rr_o2))

        reference = parse_metrics(
            run([args.rscript_bin, "--vanilla", str(rr_o0)], env=env, capture_output=True).stdout,
            metric_names,
        )
        rr_o1_metrics = parse_metrics(
            run([args.rscript_bin, "--vanilla", str(rr_o1)], env=env, capture_output=True).stdout,
            metric_names,
        )
        rr_o2_metrics = parse_metrics(
            run([args.rscript_bin, "--vanilla", str(rr_o2)], env=env, capture_output=True).stdout,
            metric_names,
        )

        assert_metrics_close(reference, rr_o1_metrics, metric_names, label=f"{title} O1")
        assert_metrics_close(reference, rr_o2_metrics, metric_names, label=f"{title} O2")

        rr_diagnostics = {
            "rr_o0": rr_artifact_diagnostics(rr_o0, rr_o0_pulse),
            "rr_o1": rr_artifact_diagnostics(rr_o1, rr_o1_pulse),
            "rr_o2": rr_artifact_diagnostics(rr_o2, rr_o2_pulse),
        }

        cases = [
            (
                f"{workload_id}_rr_o0",
                f"{title} RR O0",
                [args.rscript_bin, "--vanilla", str(rr_o0)],
                "RR-emitted R on GNU R at -O0",
                1.0,
                rr_diagnostics["rr_o0"],
            ),
            (
                f"{workload_id}_rr_o0_warm",
                f"{title} RR O0 warm x{WARM_KERNEL_ITERS}",
                [args.rscript_bin, "--vanilla", str(rr_o0_warm)],
                "RR-emitted O0 kernel averaged across repeated warm calls in one R process",
                float(WARM_KERNEL_ITERS),
                rr_diagnostics["rr_o0"],
            ),
            (
                f"{workload_id}_rr_o1",
                f"{title} RR O1",
                [args.rscript_bin, "--vanilla", str(rr_o1)],
                "RR-emitted R on GNU R at -O1",
                1.0,
                rr_diagnostics["rr_o1"],
            ),
            (
                f"{workload_id}_rr_o1_warm",
                f"{title} RR O1 warm x{WARM_KERNEL_ITERS}",
                [args.rscript_bin, "--vanilla", str(rr_o1_warm)],
                "RR-emitted O1 kernel averaged across repeated warm calls in one R process",
                float(WARM_KERNEL_ITERS),
                rr_diagnostics["rr_o1"],
            ),
            (
                f"{workload_id}_rr_o2",
                f"{title} RR O2",
                [args.rscript_bin, "--vanilla", str(rr_o2)],
                "RR-emitted R on GNU R at -O2",
                1.0,
                rr_diagnostics["rr_o2"],
            ),
            (
                f"{workload_id}_rr_o2_warm",
                f"{title} RR O2 warm x{WARM_KERNEL_ITERS}",
                [args.rscript_bin, "--vanilla", str(rr_o2_warm)],
                "RR-emitted O2 kernel averaged across repeated warm calls in one R process",
                float(WARM_KERNEL_ITERS),
                rr_diagnostics["rr_o2"],
            ),
        ]

        for row_id, label, cmd, notes, scale, diagnostics in cases:
            stats = benchmark(cmd, runs=args.runs, warmup=args.warmup, env=env, scale=scale)
            rows.append(
                attach_diagnostics(
                    {
                        "id": row_id,
                        "workload": workload_id,
                        "label": label,
                        "engine": "Rscript",
                        "artifact": str(cmd[-1]),
                        "runs_ms": ";".join(str(v) for v in stats["runs_ms"]),
                        "mean_ms": stats["mean_ms"],
                        "median_ms": stats["median_ms"],
                        "stdev_ms": stats["stdev_ms"],
                        "min_ms": stats["min_ms"],
                        "max_ms": stats["max_ms"],
                        "notes": notes,
                    },
                    diagnostics,
                )
            )

    json_path = out_dir / "backend_candidate_bench.json"
    csv_path = out_dir / "backend_candidate_bench.csv"
    svg_path = out_dir / "backend_candidate_bench.svg"
    json_path.write_text(json.dumps({"rows": rows}, indent=2) + "\n")
    write_results_csv(csv_path, rows)
    write_svg_chart(svg_path, rows)

    print(json.dumps({"csv": str(csv_path), "svg": str(svg_path), "rows": rows}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
