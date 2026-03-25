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
import shutil

from bench_utils import (
    attach_diagnostics,
    compile_rr_variant,
    rr_artifact_diagnostics,
    write_results_csv as write_results_csv_dynamic,
)

ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_OUT_DIR = ROOT / "target" / "signal_pipeline_bench"
DEFAULT_RR_SRC = ROOT / "example" / "benchmarks" / "signal_pipeline_bench.rr"
DEFAULT_RR_BIN = ROOT / "target" / "release" / "RR"
DEFAULT_PYTHON = ROOT / "target" / "tmp" / "bench-python" / "bin" / "python3"
DEFAULT_RENJIN = ROOT / "target" / "tmp" / "renjin-dist" / "renjin-3.5-beta76" / "bin" / "renjin"
METRIC_NAMES = ["signal_pipeline_tail", "signal_pipeline_mean"]
SAMPLE_COUNT = 250_000
PASS_COUNT = 16
WARM_KERNEL_ITERS = 10
OPENMP_THREADS = 4


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


def renjin_env() -> dict[str, str] | None:
    env = common_env()
    brew = shutil.which("brew")
    if brew:
        prefix = subprocess.check_output([brew, "--prefix", "openjdk"], text=True).strip()
        java_home = pathlib.Path(prefix) / "libexec" / "openjdk.jdk" / "Contents" / "Home"
        java_bin = pathlib.Path(prefix) / "bin"
        if java_home.exists():
            env["JAVA_HOME"] = str(java_home)
            env["PATH"] = f"{java_bin}:{env['PATH']}"
            return env

    java = shutil.which("java")
    if not java:
        return None

    probe = subprocess.run(
        [java, "-version"],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return env if probe.returncode == 0 else None


def parse_metrics(text: str) -> dict[str, float]:
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    metrics: dict[str, float] = {}
    kv_lines = [line for line in lines if "=" in line]
    if kv_lines:
        for line in kv_lines:
            name, value = line.split("=", 1)
            if name.strip() in METRIC_NAMES:
                metrics[name.strip()] = float(value.strip())
        if set(metrics) >= set(METRIC_NAMES):
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
        if name not in METRIC_NAMES:
            idx += 1
            continue
        metrics[name] = float(normalized[idx + 1])
        idx += 2

    if set(metrics) >= set(METRIC_NAMES):
        return metrics

    raise ValueError(f"unexpected metric output: {text}")


def assert_metrics_close(
    reference: dict[str, float],
    candidate: dict[str, float],
    *,
    label: str,
    rel_tol: float = 1e-8,
    abs_tol: float = 1e-8,
) -> None:
    for name in METRIC_NAMES:
        lhs = reference[name]
        rhs = candidate[name]
        if not math.isclose(lhs, rhs, rel_tol=rel_tol, abs_tol=abs_tol):
            raise AssertionError(
                f"{label} metric mismatch for {name}: ref={lhs:.12f} got={rhs:.12f}"
            )


def write_text(path: pathlib.Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def render_direct_r_scalar_script() -> str:
    return textwrap.dedent(
        """\
        n <- 250000L
        passes <- 16L

        idx <- seq_len(n)
        x <- ((idx * 13L) %% 1000L) / 1000.0 - 0.5
        y <- (((idx * 17L) + 7L) %% 1000L) / 1000.0 - 0.5
        score <- numeric(n)
        clean <- numeric(n)

        for (pass in seq_len(passes)) {
          for (i in seq_len(n)) {
            score[i] <- max(abs(x[i] * 0.65 + y[i] * 0.35 - 0.08), 0.05)
          }
          for (i in seq_len(n)) {
            if (score[i] > 0.40) {
              clean[i] <- sqrt(score[i] + 0.10)
            } else {
              clean[i] <- score[i] * 0.55 + 0.03
            }
          }
          for (i in seq_len(n)) {
            x[i] <- clean[i] + y[i] * 0.15
          }
          for (i in seq_len(n)) {
            y[i] <- score[i] * 0.80 + clean[i] * 0.20
          }
        }

        cat(sprintf("signal_pipeline_tail=%.12f\\n", clean[n]))
        cat(sprintf("signal_pipeline_mean=%.12f\\n", mean(clean)))
        """
    )


def render_direct_r_vector_script() -> str:
    return textwrap.dedent(
        """\
        n <- 250000L
        passes <- 16L

        idx <- seq_len(n)
        x <- ((idx * 13L) %% 1000L) / 1000.0 - 0.5
        y <- (((idx * 17L) + 7L) %% 1000L) / 1000.0 - 0.5
        score <- numeric(n)
        clean <- numeric(n)

        for (pass in seq_len(passes)) {
          score <- pmax(abs(x * 0.65 + y * 0.35 - 0.08), 0.05)
          clean <- ifelse(score > 0.40, sqrt(score + 0.10), score * 0.55 + 0.03)
          x <- clean + y * 0.15
          y <- score * 0.80 + clean * 0.20
        }

        cat(sprintf("signal_pipeline_tail=%.12f\\n", clean[n]))
        cat(sprintf("signal_pipeline_mean=%.12f\\n", mean(clean)))
        """
    )


def render_direct_r_vector_warm_script() -> str:
    return textwrap.dedent(
        f"""\
        signal_pipeline_kernel <- function() {{
          n <- 250000L
          passes <- 16L

          idx <- seq_len(n)
          x <- ((idx * 13L) %% 1000L) / 1000.0 - 0.5
          y <- (((idx * 17L) + 7L) %% 1000L) / 1000.0 - 0.5
          score <- numeric(n)
          clean <- numeric(n)

          for (pass in seq_len(passes)) {{
            score <- pmax(abs(x * 0.65 + y * 0.35 - 0.08), 0.05)
            clean <- ifelse(score > 0.40, sqrt(score + 0.10), score * 0.55 + 0.03)
            x <- clean + y * 0.15
            y <- score * 0.80 + clean * 0.20
          }}

          invisible(clean[n] + mean(clean))
        }}

        for (.rr_iter in seq_len({WARM_KERNEL_ITERS}L)) {{
          signal_pipeline_kernel()
        }}
        """
    )


def render_rr_warm_driver(rr_r: pathlib.Path) -> str:
    rr_path = str(rr_r).replace("\\", "\\\\")
    return textwrap.dedent(
        f"""\
        bench_env <- new.env(parent = baseenv())
        bench_env$print <- function(...) invisible(NULL)
        bench_env$rr_mark <- function(...) invisible(NULL)
        source("{rr_path}", local = bench_env, chdir = TRUE)

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


def render_numpy_script() -> str:
    return textwrap.dedent(
        """\
        import numpy as np

        n = 250_000
        passes = 16

        idx = np.arange(1, n + 1, dtype=np.float64)
        x = np.mod(idx * 13.0, 1000.0) / 1000.0 - 0.5
        y = np.mod(idx * 17.0 + 7.0, 1000.0) / 1000.0 - 0.5
        score = np.empty(n, dtype=np.float64)
        clean = np.empty(n, dtype=np.float64)

        for _ in range(passes):
            np.maximum(np.abs(x * 0.65 + y * 0.35 - 0.08), 0.05, out=score)
            np.copyto(clean, np.where(score > 0.40, np.sqrt(score + 0.10), score * 0.55 + 0.03))
            x[:] = clean + y * 0.15
            y[:] = score * 0.80 + clean * 0.20

        print(f"signal_pipeline_tail={clean[-1]:.12f}")
        print(f"signal_pipeline_mean={clean.mean():.12f}")
        """
    )


def render_julia_script() -> str:
    return textwrap.dedent(
        """\
        function main()
            n = 250_000
            passes = 16

            idx = collect(1.0:n)
            x = mod.(idx .* 13.0, 1000.0) ./ 1000.0 .- 0.5
            y = mod.(idx .* 17.0 .+ 7.0, 1000.0) ./ 1000.0 .- 0.5
            score = Vector{Float64}(undef, n)
            clean = Vector{Float64}(undef, n)

            @inbounds for _ in 1:passes
                for i in eachindex(x)
                    score[i] = max(abs(x[i] * 0.65 + y[i] * 0.35 - 0.08), 0.05)
                end
                for i in eachindex(x)
                    if score[i] > 0.40
                        clean[i] = sqrt(score[i] + 0.10)
                    else
                        clean[i] = score[i] * 0.55 + 0.03
                    end
                end
                for i in eachindex(x)
                    x[i] = clean[i] + y[i] * 0.15
                end
                for i in eachindex(x)
                    y[i] = score[i] * 0.80 + clean[i] * 0.20
                end
            end

            println("signal_pipeline_tail=$(round(clean[end]; digits=12))")
            println("signal_pipeline_mean=$(round(sum(clean) / length(clean); digits=12))")
        end

        main()
        """
    )


def render_c_source() -> str:
    return textwrap.dedent(
        """\
        #include <math.h>
        #include <stdio.h>
        #include <stdlib.h>

        int main(void) {
          const int n = 250000;
          const int passes = 16;
          double* x = (double*)malloc(sizeof(double) * (size_t)n);
          double* y = (double*)malloc(sizeof(double) * (size_t)n);
          double* score = (double*)malloc(sizeof(double) * (size_t)n);
          double* clean = (double*)malloc(sizeof(double) * (size_t)n);
          if (x == NULL || y == NULL || score == NULL || clean == NULL) {
            fprintf(stderr, "allocation failed\\n");
            free(x);
            free(y);
            free(score);
            free(clean);
            return 1;
          }

          for (int i = 0; i < n; ++i) {
            const double idx = (double)(i + 1);
            x[i] = fmod(idx * 13.0, 1000.0) / 1000.0 - 0.5;
            y[i] = fmod(idx * 17.0 + 7.0, 1000.0) / 1000.0 - 0.5;
          }

          for (int pass = 0; pass < passes; ++pass) {
            for (int i = 0; i < n; ++i) {
              const double v = fabs(x[i] * 0.65 + y[i] * 0.35 - 0.08);
              score[i] = v > 0.05 ? v : 0.05;
            }
            for (int i = 0; i < n; ++i) {
              clean[i] = score[i] > 0.40 ? sqrt(score[i] + 0.10) : score[i] * 0.55 + 0.03;
            }
            for (int i = 0; i < n; ++i) {
              x[i] = clean[i] + y[i] * 0.15;
            }
            for (int i = 0; i < n; ++i) {
              y[i] = score[i] * 0.80 + clean[i] * 0.20;
            }
          }

          double sum = 0.0;
          for (int i = 0; i < n; ++i) {
            sum += clean[i];
          }

          printf("signal_pipeline_tail=%.12f\\n", clean[n - 1]);
          printf("signal_pipeline_mean=%.12f\\n", sum / (double)n);
          free(x);
          free(y);
          free(score);
          free(clean);
          return 0;
        }
        """
    )


def write_results_csv(path: pathlib.Path, rows: list[dict[str, object]]) -> None:
    write_results_csv_dynamic(
        path,
        rows,
        [
            "id",
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


def wrap_chart_label(label: str, max_chars: int = 34) -> list[str]:
    if len(label) <= max_chars:
        return [label]
    split_at = label.rfind(" ", 0, max_chars + 1)
    if split_at <= 0:
        split_at = max_chars
    first = label[:split_at].strip()
    second = label[split_at:].strip()
    if not second:
        return [first]
    return [first, second]


def write_svg_chart(path: pathlib.Path, rows: list[dict[str, object]]) -> None:
    width = 1120
    left = 320
    right = 220
    top = 86
    row_gap = 68
    bar_h = 28
    height = top + len(rows) * row_gap + 94
    max_mean = max(float(row["mean_ms"]) for row in rows)
    usable = width - left - right
    ticks = 5
    tick_values = [max_mean * idx / ticks for idx in range(ticks + 1)]
    colors = ["#2a9d8f", "#e76f51", "#264653", "#6d597a", "#457b9d"]

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title desc">',
        '  <title id="title">Signal Pipeline Runtime Comparison</title>',
        '  <desc id="desc">Mean wall-clock runtime in milliseconds for a cross-language signal preprocessing pipeline benchmark.</desc>',
        f'  <rect width="{width}" height="{height}" fill="#fcfcf8"/>',
        '  <text x="24" y="34" font-family="Helvetica, Arial, sans-serif" font-size="22" fill="#16212b">Signal Pipeline Runtime Comparison</text>',
        f'  <text x="24" y="56" font-family="Helvetica, Arial, sans-serif" font-size="12" fill="#4b5b67">{SAMPLE_COUNT:,} samples, {PASS_COUNT} passes, Apple M4, optimizer tiers O0/O1/O2 plus direct baselines</text>',
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
        y = top + idx * row_gap + 18
        label_lines = wrap_chart_label(str(row["label"]))
        if len(label_lines) == 1:
            lines.append(f'    <text x="24" y="{y}">{label_lines[0]}</text>')
        else:
            lines.append(f'    <text x="24" y="{y}">')
            lines.append(f'      <tspan x="24" dy="0">{label_lines[0]}</tspan>')
            lines.append(f'      <tspan x="24" dy="16">{label_lines[1]}</tspan>')
            lines.append('    </text>')
    lines.append("  </g>")
    for idx, row in enumerate(rows):
        bar_y = top + idx * row_gap + 8
        bar_w = (float(row["mean_ms"]) / max_mean) * usable if max_mean else 0.0
        color = colors[idx % len(colors)]
        lines.append(f'  <rect x="{left}" y="{bar_y}" width="{bar_w:.1f}" height="{bar_h}" fill="{color}" rx="4"/>')
        label_x = width - 18
        mean_ms = float(row["mean_ms"])
        stdev_ms = float(row["stdev_ms"])
        lines.append(
            f'  <text x="{label_x:.1f}" y="{bar_y + 19}" text-anchor="end" font-family="Helvetica, Arial, sans-serif" font-size="13" fill="#16212b">{mean_ms:.1f} ms ± {stdev_ms:.1f}</text>'
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
    parser = argparse.ArgumentParser(description="Benchmark a cross-language signal preprocessing pipeline.")
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--warmup", type=int, default=1)
    parser.add_argument("--out-dir", type=pathlib.Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--rr-src", type=pathlib.Path, default=DEFAULT_RR_SRC)
    parser.add_argument("--rr-bin", type=pathlib.Path, default=DEFAULT_RR_BIN)
    parser.add_argument("--rscript-bin", default="Rscript")
    parser.add_argument("--python-bin", default=str(DEFAULT_PYTHON))
    parser.add_argument("--julia-bin", default="julia")
    parser.add_argument("--renjin-bin", type=pathlib.Path, default=DEFAULT_RENJIN)
    parser.add_argument("--skip-renjin", action="store_true")
    parser.add_argument("--renjin-runs", type=int)
    parser.add_argument("--renjin-warmup", type=int)
    args = parser.parse_args()

    out_dir = args.out_dir.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    env = common_env()
    renjin_runs = args.renjin_runs if args.renjin_runs is not None else args.runs
    renjin_warmup = args.renjin_warmup if args.renjin_warmup is not None else args.warmup

    rr_bin = ensure_release_rr(args.rr_bin.resolve())
    rr_src = args.rr_src.resolve()
    direct_r_scalar = out_dir / "signal_pipeline_direct_r_scalar.R"
    direct_r_vector = out_dir / "signal_pipeline_direct_r_vector.R"
    direct_r_vector_warm = out_dir / "signal_pipeline_direct_r_vector_warm.R"
    numpy_py = out_dir / "signal_pipeline_numpy.py"
    julia_jl = out_dir / "signal_pipeline.jl"
    c_src = out_dir / "signal_pipeline.c"
    c_bin = out_dir / "signal_pipeline_c"
    rr_o0 = out_dir / "signal_pipeline_rr_o0.R"
    rr_o0_warm = out_dir / "signal_pipeline_rr_o0_warm.R"
    rr_o1 = out_dir / "signal_pipeline_rr_o1.R"
    rr_o1_warm = out_dir / "signal_pipeline_rr_o1_warm.R"
    rr_o2 = out_dir / "signal_pipeline_rr_o2.R"
    rr_o2_warm = out_dir / "signal_pipeline_rr_o2_warm.R"
    rr_o0_pulse = out_dir / "signal_pipeline_rr_o0.pulse.json"
    rr_o1_pulse = out_dir / "signal_pipeline_rr_o1.pulse.json"
    rr_o2_pulse = out_dir / "signal_pipeline_rr_o2.pulse.json"

    write_text(direct_r_scalar, render_direct_r_scalar_script())
    write_text(direct_r_vector, render_direct_r_vector_script())
    write_text(direct_r_vector_warm, render_direct_r_vector_warm_script())
    write_text(numpy_py, render_numpy_script())
    write_text(julia_jl, render_julia_script())
    write_text(c_src, render_c_source())

    compile_rr_variant(ROOT, rr_bin, rr_src, rr_o0, "-O0", env, pulse_json_path=rr_o0_pulse)
    compile_rr_variant(ROOT, rr_bin, rr_src, rr_o1, "-O1", env, pulse_json_path=rr_o1_pulse)
    compile_rr_variant(ROOT, rr_bin, rr_src, rr_o2, "-O2", env, pulse_json_path=rr_o2_pulse)
    rr_o0.write_text("options(digits = 15)\n" + rr_o0.read_text())
    rr_o1.write_text("options(digits = 15)\n" + rr_o1.read_text())
    rr_o2.write_text("options(digits = 15)\n" + rr_o2.read_text())
    write_text(rr_o0_warm, render_rr_warm_driver(rr_o0))
    write_text(rr_o1_warm, render_rr_warm_driver(rr_o1))
    write_text(rr_o2_warm, render_rr_warm_driver(rr_o2))
    run(["clang", "-O3", "-std=c11", str(c_src), "-lm", "-o", str(c_bin)], env=env)

    rr_o0_metrics = parse_metrics(
        run([args.rscript_bin, "--vanilla", str(rr_o0)], env=env, capture_output=True).stdout
    )
    rr_o1_metrics = parse_metrics(
        run([args.rscript_bin, "--vanilla", str(rr_o1)], env=env, capture_output=True).stdout
    )
    rr_o2_metrics = parse_metrics(
        run([args.rscript_bin, "--vanilla", str(rr_o2)], env=env, capture_output=True).stdout
    )
    direct_r_scalar_metrics = parse_metrics(
        run(
            [args.rscript_bin, "--vanilla", str(direct_r_scalar)],
            env=env,
            capture_output=True,
        ).stdout
    )
    direct_r_vector_metrics = parse_metrics(
        run(
            [args.rscript_bin, "--vanilla", str(direct_r_vector)],
            env=env,
            capture_output=True,
        ).stdout
    )
    c_metrics = parse_metrics(run([str(c_bin)], env=env, capture_output=True).stdout)
    numpy_metrics = parse_metrics(
        run([args.python_bin, str(numpy_py)], env=env, capture_output=True).stdout
    )
    julia_metrics = parse_metrics(
        run([args.julia_bin, "--startup-file=no", str(julia_jl)], env=env, capture_output=True).stdout
    )
    renjin_runtime_env = None if args.skip_renjin else renjin_env()
    direct_renjin_metrics: dict[str, float] | None = None
    rr_renjin_metrics: dict[str, float] | None = None
    if not args.skip_renjin and args.renjin_bin.exists() and renjin_runtime_env is not None:
        direct_renjin_metrics = parse_metrics(
            run(
                [str(args.renjin_bin), "-f", str(direct_r_vector)],
                env=renjin_runtime_env,
                capture_output=True,
            ).stdout
        )
        rr_renjin_metrics = parse_metrics(
            run([str(args.renjin_bin), "-f", str(rr_o2)], env=renjin_runtime_env, capture_output=True).stdout
        )

    reference = c_metrics
    assert_metrics_close(reference, rr_o0_metrics, label="RR O0 on GNU R", rel_tol=1e-7, abs_tol=1e-7)
    assert_metrics_close(reference, rr_o1_metrics, label="RR O1 on GNU R", rel_tol=1e-7, abs_tol=1e-7)
    assert_metrics_close(reference, rr_o2_metrics, label="RR O2 on GNU R", rel_tol=1e-7, abs_tol=1e-7)
    assert_metrics_close(
        reference,
        direct_r_scalar_metrics,
        label="Direct base R (scalar)",
        rel_tol=1e-7,
        abs_tol=1e-7,
    )
    assert_metrics_close(
        reference,
        direct_r_vector_metrics,
        label="Direct base R (vectorized)",
        rel_tol=1e-7,
        abs_tol=1e-7,
    )
    assert_metrics_close(reference, numpy_metrics, label="NumPy", rel_tol=1e-7, abs_tol=1e-7)
    assert_metrics_close(reference, julia_metrics, label="Julia", rel_tol=1e-7, abs_tol=1e-7)
    if direct_renjin_metrics is not None:
        assert_metrics_close(reference, direct_renjin_metrics, label="Direct base R (vectorized) on Renjin", rel_tol=1e-7, abs_tol=1e-7)
    if rr_renjin_metrics is not None:
        assert_metrics_close(reference, rr_renjin_metrics, label="RR O2 on Renjin", rel_tol=1e-7, abs_tol=1e-7)

    rr_diagnostics = {
        "rr_o0": rr_artifact_diagnostics(rr_o0, rr_o0_pulse),
        "rr_o1": rr_artifact_diagnostics(rr_o1, rr_o1_pulse),
        "rr_o2": rr_artifact_diagnostics(rr_o2, rr_o2_pulse),
    }

    cases = [
        (
            "direct_r_scalar",
            "Direct base R (scalar)",
            "Rscript",
            [args.rscript_bin, "--vanilla", str(direct_r_scalar)],
            "loop-based scalar base-R baseline",
            1.0,
            None,
        ),
        (
            "direct_r_vector",
            "Direct base R (vectorized)",
            "Rscript",
            [args.rscript_bin, "--vanilla", str(direct_r_vector)],
            "idiomatic base-R vector baseline",
            1.0,
            None,
        ),
        (
            "direct_r_vector_warm",
            f"Direct base R (vectorized, warm avg x{WARM_KERNEL_ITERS})",
            "Rscript",
            [args.rscript_bin, "--vanilla", str(direct_r_vector_warm)],
            "same vector kernel averaged across repeated warm calls in one R process",
            float(WARM_KERNEL_ITERS),
            None,
        ),
        (
            "rr_o0_gnur",
            "RR O0 on GNU R",
            "Rscript",
            [args.rscript_bin, "--vanilla", str(rr_o0)],
            "RR-emitted R from example/benchmarks/signal_pipeline_bench.rr at -O0",
            1.0,
            rr_diagnostics["rr_o0"],
        ),
        (
            "rr_o0_gnur_warm",
            f"RR O0 on GNU R (warm avg x{WARM_KERNEL_ITERS})",
            "Rscript",
            [args.rscript_bin, "--vanilla", str(rr_o0_warm)],
            "RR-emitted O0 kernel averaged across repeated warm calls in one R process",
            float(WARM_KERNEL_ITERS),
            rr_diagnostics["rr_o0"],
        ),
        (
            "rr_o1_gnur",
            "RR O1 on GNU R",
            "Rscript",
            [args.rscript_bin, "--vanilla", str(rr_o1)],
            "RR-emitted R from example/benchmarks/signal_pipeline_bench.rr at -O1",
            1.0,
            rr_diagnostics["rr_o1"],
        ),
        (
            "rr_o1_gnur_warm",
            f"RR O1 on GNU R (warm avg x{WARM_KERNEL_ITERS})",
            "Rscript",
            [args.rscript_bin, "--vanilla", str(rr_o1_warm)],
            "RR-emitted O1 kernel averaged across repeated warm calls in one R process",
            float(WARM_KERNEL_ITERS),
            rr_diagnostics["rr_o1"],
        ),
        (
            "rr_o2_gnur",
            "RR O2 on GNU R",
            "Rscript",
            [args.rscript_bin, "--vanilla", str(rr_o2)],
            "RR-emitted R from example/benchmarks/signal_pipeline_bench.rr at -O2",
            1.0,
            rr_diagnostics["rr_o2"],
        ),
        (
            "rr_o2_gnur_warm",
            f"RR O2 on GNU R (warm avg x{WARM_KERNEL_ITERS})",
            "Rscript",
            [args.rscript_bin, "--vanilla", str(rr_o2_warm)],
            "RR-emitted O2 kernel averaged across repeated warm calls in one R process",
            float(WARM_KERNEL_ITERS),
            rr_diagnostics["rr_o2"],
        ),
        (
            "c_o3",
            "C O3 native",
            "clang 17",
            [str(c_bin)],
            "single-threaded clang -O3 build",
            1.0,
            None,
        ),
        (
            "numpy",
            "NumPy",
            pathlib.Path(args.python_bin).name,
            [args.python_bin, str(numpy_py)],
            "vectorized array math on CPython",
            1.0,
            None,
        ),
        (
            "julia",
            "Julia",
            pathlib.Path(args.julia_bin).name,
            [args.julia_bin, "--startup-file=no", str(julia_jl)],
            "Base Julia loops with @inbounds",
            1.0,
            None,
        ),
    ]
    if not args.skip_renjin and args.renjin_bin.exists() and renjin_runtime_env is not None:
        cases.extend(
            [
                (
                    "direct_r_renjin",
                    "Direct base R (vectorized) on Renjin",
                    "Renjin 3.5-beta76",
                    [str(args.renjin_bin), "-f", str(direct_r_vector)],
                    "same idiomatic base-R script on Renjin",
                    1.0,
                    None,
                ),
                (
                    "rr_o2_renjin",
                    "RR O2 on Renjin",
                    "Renjin 3.5-beta76",
                    [str(args.renjin_bin), "-f", str(rr_o2)],
                    "RR-emitted R on Renjin",
                    1.0,
                    rr_diagnostics["rr_o2"],
                ),
            ]
        )

    rows: list[dict[str, object]] = []
    for row_id, label, engine, cmd, notes, scale, diagnostics in cases:
        case_env = renjin_runtime_env if engine == "Renjin 3.5-beta76" else env
        runs = renjin_runs if engine == "Renjin 3.5-beta76" else args.runs
        warmup = renjin_warmup if engine == "Renjin 3.5-beta76" else args.warmup
        stats = benchmark(cmd, runs=runs, warmup=warmup, env=case_env, scale=scale)
        rows.append(
            attach_diagnostics(
                {
                    "id": row_id,
                    "label": label,
                    "engine": engine,
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

    json_path = out_dir / "signal_pipeline_bench.json"
    csv_path = out_dir / "signal_pipeline_bench.csv"
    svg_path = out_dir / "signal_pipeline_bench.svg"
    json_path.write_text(json.dumps({"rows": rows, "metrics": reference}, indent=2) + "\n")
    write_results_csv(csv_path, rows)
    write_svg_chart(svg_path, rows)

    print(json.dumps({"csv": str(csv_path), "svg": str(svg_path), "rows": rows}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
