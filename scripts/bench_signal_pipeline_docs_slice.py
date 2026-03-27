#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import pathlib
import re

import bench_signal_pipeline as base


PUBLIC_SLICE_IDS = [
    "direct_r_scalar",
    "direct_r_vector",
    "direct_r_vector_warm",
    "rr_o2_gnur",
    "rr_o2_gnur_warm",
    "c_o3",
    "numpy",
    "julia",
]


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Benchmark the published signal-pipeline cross-language docs slice."
    )
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--warmup", type=int, default=0)
    parser.add_argument("--out-dir", type=pathlib.Path, default=base.DEFAULT_OUT_DIR)
    parser.add_argument("--rr-src", type=pathlib.Path, default=base.DEFAULT_RR_SRC)
    parser.add_argument("--rr-bin", type=pathlib.Path, default=base.DEFAULT_RR_BIN)
    parser.add_argument("--rscript-bin", default="Rscript")
    parser.add_argument("--python-bin", default=str(base.DEFAULT_PYTHON))
    parser.add_argument("--julia-bin", default="julia")
    parser.add_argument("--renjin-bin", type=pathlib.Path, default=base.DEFAULT_RENJIN)
    parser.add_argument("--skip-renjin", action="store_true")
    args = parser.parse_args()

    out_dir = args.out_dir.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    env = base.common_env()
    rr_bin = base.ensure_release_rr(args.rr_bin.resolve())
    rr_src = args.rr_src.resolve()

    direct_r_scalar = out_dir / "signal_pipeline_direct_r_scalar.R"
    direct_r_vector = out_dir / "signal_pipeline_direct_r_vector.R"
    direct_r_vector_warm = out_dir / "signal_pipeline_direct_r_vector_warm.R"
    numpy_py = out_dir / "signal_pipeline_numpy.py"
    julia_jl = out_dir / "signal_pipeline.jl"
    c_src = out_dir / "signal_pipeline.c"
    c_bin = out_dir / "signal_pipeline_c"
    rr_o2 = out_dir / "signal_pipeline_rr_o2.R"
    rr_o2_warm = out_dir / "signal_pipeline_rr_o2_warm.R"
    rr_o2_pulse = out_dir / "signal_pipeline_rr_o2.pulse.json"

    base.write_text(direct_r_scalar, base.render_direct_r_scalar_script())
    base.write_text(direct_r_vector, base.render_direct_r_vector_script())
    base.write_text(direct_r_vector_warm, base.render_direct_r_vector_warm_script())
    base.write_text(numpy_py, base.render_numpy_script())
    base.write_text(julia_jl, base.render_julia_script())
    base.write_text(c_src, base.render_c_source())

    base.compile_rr_variant(
        base.ROOT, rr_bin, rr_src, rr_o2, "-O2", env, pulse_json_path=rr_o2_pulse
    )
    rr_o2.write_text("options(digits = 15)\n" + rr_o2.read_text())
    base.write_text(rr_o2_warm, base.render_rr_warm_driver(rr_o2))
    base.run(["clang", "-O3", "-std=c11", str(c_src), "-lm", "-o", str(c_bin)], env=env)

    reference = base.parse_metrics(
        base.run([str(c_bin)], env=env, capture_output=True).stdout
    )

    for label, cmd in [
        ("Direct base R (scalar)", [args.rscript_bin, "--vanilla", str(direct_r_scalar)]),
        ("Direct base R (vectorized)", [args.rscript_bin, "--vanilla", str(direct_r_vector)]),
        ("RR O2 on GNU R", [args.rscript_bin, "--vanilla", str(rr_o2)]),
        ("NumPy", [args.python_bin, str(numpy_py)]),
        ("Julia", [args.julia_bin, "--startup-file=no", str(julia_jl)]),
    ]:
        base.assert_metrics_close(
            reference,
            base.parse_metrics(base.run(cmd, env=env, capture_output=True).stdout),
            label=label,
            rel_tol=1e-7,
            abs_tol=1e-7,
        )

    rr_diag = base.rr_artifact_diagnostics(rr_o2, rr_o2_pulse)

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
            f"Direct base R (vectorized, warm avg x{base.WARM_KERNEL_ITERS})",
            "Rscript",
            [args.rscript_bin, "--vanilla", str(direct_r_vector_warm)],
            "same vector kernel averaged across repeated warm calls in one R process",
            float(base.WARM_KERNEL_ITERS),
            None,
        ),
        (
            "rr_o2_gnur",
            "RR O2 on GNU R",
            "Rscript",
            [args.rscript_bin, "--vanilla", str(rr_o2)],
            "RR-emitted R from example/benchmarks/signal_pipeline_bench.rr at -O2",
            1.0,
            rr_diag,
        ),
        (
            "rr_o2_gnur_warm",
            f"RR O2 on GNU R (warm avg x{base.WARM_KERNEL_ITERS})",
            "Rscript",
            [args.rscript_bin, "--vanilla", str(rr_o2_warm)],
            "RR-emitted O2 kernel averaged across repeated warm calls in one R process",
            float(base.WARM_KERNEL_ITERS),
            rr_diag,
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

    renjin_runtime_env = None if args.skip_renjin else base.renjin_env()
    if not args.skip_renjin and args.renjin_bin.exists() and renjin_runtime_env is not None:
        direct_renjin = (
            "direct_r_renjin",
            "Direct base R (vectorized) on Renjin",
            "Renjin 3.5-beta76",
            [str(args.renjin_bin), "-f", str(direct_r_vector)],
            "same idiomatic base-R script on Renjin",
            1.0,
            None,
        )
        rr_renjin = (
            "rr_o2_renjin",
            "RR O2 on Renjin",
            "Renjin 3.5-beta76",
            [str(args.renjin_bin), "-f", str(rr_o2)],
            "RR-emitted R on Renjin",
            1.0,
            rr_diag,
        )
        for label, cmd in [
            (direct_renjin[1], direct_renjin[3]),
            (rr_renjin[1], rr_renjin[3]),
        ]:
            base.assert_metrics_close(
                reference,
                base.parse_metrics(
                    base.run(cmd, env=renjin_runtime_env, capture_output=True).stdout
                ),
                label=label,
                rel_tol=1e-7,
                abs_tol=1e-7,
            )
        cases.extend([direct_renjin, rr_renjin])

    rows: list[dict[str, object]] = []
    for row_id, label, engine, cmd, notes, scale, diagnostics in cases:
        case_env = renjin_runtime_env if engine == "Renjin 3.5-beta76" else env
        stats = base.benchmark(cmd, runs=args.runs, warmup=args.warmup, env=case_env, scale=scale)
        rows.append(
            base.attach_diagnostics(
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
    base.write_results_csv(csv_path, rows)
    base.write_svg_chart(svg_path, rows)

    svg_text = svg_path.read_text()
    svg_text = re.sub(
        r"250,000 samples, 16 passes, Apple M4, optimizer tiers O0/O1/O2 plus direct baselines",
        "250,000 samples, 16 passes, Apple M4, cross-language public slice with RR O2 cold/warm rows",
        svg_text,
    )
    svg_path.write_text(svg_text)

    print(json.dumps({"csv": str(csv_path), "svg": str(svg_path), "rows": rows}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
