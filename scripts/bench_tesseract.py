#!/usr/bin/env python3

from __future__ import annotations

import argparse
import csv
import json
import os
import pathlib
import re
import shutil
import statistics
import subprocess
import sys
import time


ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_OUT_DIR = ROOT / "target" / "tesseract_bench"
DEFAULT_RENJIN = ROOT / "target" / "tmp" / "renjin-dist" / "renjin-3.5-beta76" / "bin" / "renjin"


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


def ensure_release_rr(rr_bin: pathlib.Path) -> pathlib.Path:
    if rr_bin.exists():
        return rr_bin
    run(["cargo", "build", "--release", "--bin", "RR"])
    if not rr_bin.exists():
        raise FileNotFoundError(f"missing RR binary after build: {rr_bin}")
    return rr_bin


def strip_param_types(params: str) -> str:
    return re.sub(r":\s*[^,\)]+", "", params)


def translate_tesseract_to_r(rr_text: str) -> str:
    out: list[str] = [
        "rr_assign_slice <- function(dest, start, end, values, ctx = \"slice_assign\") {",
        "  dest[start:end] <- values",
        "  dest",
        "}",
        "",
        "round <- function(x) {",
        "  floor(x + 0.5)",
        "}",
        "",
        "idx_cube <- function(f, x, y, size) {",
        "  ss <- round(size)",
        "  ff <- pmin(pmax(round(f), 1.0), 6.0)",
        "  xx <- pmin(pmax(round(x), 1.0), ss)",
        "  yy <- pmin(pmax(round(y), 1.0), ss)",
        "  ((ff - 1.0) * ss * ss) + ((xx - 1.0) * ss) + yy",
        "}",
        "",
        "idx.cube <- idx_cube",
        "",
    ]
    pending_return_indent: str | None = None
    pending_return_parts: list[str] = []
    pending_return_balance = 0
    skip_function_depth = 0

    for raw in rr_text.splitlines():
        if skip_function_depth > 0:
            skip_function_depth += raw.count("{") - raw.count("}")
            continue

        if raw.lstrip().startswith("//"):
            continue

        if re.match(r"^\s*fn\s+(round|idx_cube|idx\.cube)\(", raw):
            skip_function_depth = raw.count("{") - raw.count("}")
            continue

        s = raw

        inline_expr = re.match(
            r"^(\s*)fn\s+([A-Za-z_][\w\.]*)\((.*?)\)\s*->\s*[^=]+\s*=\s*(.*)$",
            s,
        )
        if not inline_expr:
            inline_expr = re.match(
                r"^(\s*)fn\s+([A-Za-z_][\w\.]*)\((.*?)\)\s*=\s*(.*)$",
                s,
            )
        if inline_expr:
            indent, name, params, expr = inline_expr.groups()
            s = f"{indent}{name} <- function({strip_param_types(params)}) {expr}"
        else:
            inline_block = re.match(
                r"^(\s*)fn\s+([A-Za-z_][\w\.]*)\((.*?)\)\s*(?:->\s*[^\{]+)?\{\s*(.*)\s*\}\s*$",
                s,
            )
            if inline_block:
                indent, name, params, body = inline_block.groups()
                s = f"{indent}{name} <- function({strip_param_types(params)}) {{ {body} }}"
            else:
                block = re.match(
                    r"^(\s*)fn\s+([A-Za-z_][\w\.]*)\((.*?)\)\s*(?:->\s*[^\{]+)?\{\s*$",
                    s,
                )
                if block:
                    indent, name, params = block.groups()
                    s = f"{indent}{name} <- function({strip_param_types(params)}) {{"

        cond_if = re.match(r"^(\s*)if\s+([^\(].*?)\s*\{\s*$", s)
        if cond_if:
            indent, cond = cond_if.groups()
            s = f"{indent}if ({cond}) {{"

        cond_while = re.match(r"^(\s*)while\s+([^\(].*?)\s*\{\s*$", s)
        if cond_while:
            indent, cond = cond_while.groups()
            s = f"{indent}while ({cond}) {{"

        loop_for = re.match(r"^(\s*)for\s+([A-Za-z_][\w]*)\s+in\s+(.+)\.\.(.+?)\s*\{\s*$", s)
        if loop_for:
            indent, var, start, end = loop_for.groups()
            s = f"{indent}for ({var} in seq({start.strip()}, {end.strip()})) {{"

        s = re.sub(r"\blet\s+([A-Za-z_][\w]*)\s*=\s*", r"\1 <- ", s)
        s = re.sub(r"\b([A-Za-z_][\w]*)\s*\+=\s*(.+)$", r"\1 <- \1 + (\2)", s)
        s = re.sub(r"\b([A-Za-z_][\w]*)\s*-=\s*(.+)$", r"\1 <- \1 - (\2)", s)

        if pending_return_indent is not None:
            pending_return_parts.append(s.strip())
            pending_return_balance += s.count("(") - s.count(")")
            if pending_return_balance <= 0:
                expr = " ".join(part for part in pending_return_parts if part)
                out.append(f"{pending_return_indent}return({expr})")
                pending_return_indent = None
                pending_return_parts = []
                pending_return_balance = 0
            continue

        if "return {px: px, py: py, pf: pf}" in s:
            indent = re.match(r"^(\s*)", s).group(1)
            s = f"{indent}return(list(px = px, py = py, pf = pf))"
        else:
            ret = re.match(r"^(\s*)return\s+(.+)$", s)
            if ret:
                indent, expr = ret.groups()
                balance = expr.count("(") - expr.count(")")
                if balance > 0:
                    pending_return_indent = indent
                    pending_return_parts = [expr.strip()]
                    pending_return_balance = balance
                    continue
                s = f"{indent}return({expr})"

        s = s.replace("{name: \"tesseract\", version: 1.0}", 'list(name = "tesseract", version = 1.0)')
        s = s.replace("[1.0, 2.0, 3.0]", "c(1.0, 2.0, 3.0)")
        s = s.replace("meta.version", "meta$version")
        s = s.replace("meta.name", "meta$name")
        s = s.replace("particles.px", "particles$px")
        s = s.replace("particles.py", "particles$py")
        s = s.replace("particles.pf", "particles$pf")
        out.append(s)

    text = "\n".join(out) + "\n"
    text = re.sub(r":\s*(vector<float>|list<vector<float>>|float|int|bool)", "", text)
    text = re.sub(r"(?<![A-Za-z%])%(?![A-Za-z%])", "%%", text)
    text = re.sub(r"(?<=\s)%(?=\s)", "%%", text)
    return text


def capture_stdout(cmd: list[str], *, env: dict[str, str] | None = None) -> str:
    proc = run(cmd, env=env, capture_output=True)
    return proc.stdout


def benchmark(cmd: list[str], *, runs: int, warmup: int, env: dict[str, str] | None = None) -> dict[str, object]:
    for _ in range(warmup):
        run(cmd, env=env)

    timings_ms: list[float] = []
    for _ in range(runs):
        start = time.perf_counter()
        run(cmd, env=env)
        timings_ms.append((time.perf_counter() - start) * 1000.0)

    return {
        "runs_ms": [round(t, 1) for t in timings_ms],
        "mean_ms": round(statistics.mean(timings_ms), 1),
        "stdev_ms": round(statistics.stdev(timings_ms), 1) if len(timings_ms) > 1 else 0.0,
        "min_ms": round(min(timings_ms), 1),
        "max_ms": round(max(timings_ms), 1),
    }


def renjin_env() -> dict[str, str] | None:
    env = os.environ.copy()
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


def write_results_csv(path: pathlib.Path, rows: list[dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="") as f:
        writer = csv.DictWriter(
            f,
            fieldnames=[
                "id",
                "label",
                "kind",
                "engine",
                "artifact",
                "runs_ms",
                "mean_ms",
                "stdev_ms",
                "min_ms",
                "max_ms",
                "notes",
            ],
        )
        writer.writeheader()
        for row in rows:
            writer.writerow(row)


def main() -> int:
    parser = argparse.ArgumentParser(description="Benchmark the tesseract workload.")
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--warmup", type=int, default=1)
    parser.add_argument("--out-dir", type=pathlib.Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--rr-bin", type=pathlib.Path, default=ROOT / "target" / "release" / "RR")
    parser.add_argument("--renjin-bin", type=pathlib.Path, default=DEFAULT_RENJIN)
    parser.add_argument("--skip-renjin", action="store_true")
    args = parser.parse_args()

    out_dir = args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    rr_bin = ensure_release_rr(args.rr_bin)
    rr_src = ROOT / "example" / "tesseract.rr"

    plain_r_path = out_dir / "tesseract_direct_r.R"
    helper_r_path = out_dir / "tesseract_rr_o2_helper_only.R"
    runtime_r_path = out_dir / "tesseract_rr_o2_runtime.R"

    plain_r_path.write_text(translate_tesseract_to_r(rr_src.read_text()))

    run(
        [
            str(rr_bin),
            str(rr_src),
            "-o",
            str(helper_r_path),
            "-O2",
            "--no-incremental",
            "--no-runtime",
        ]
    )
    run(
        [
            str(rr_bin),
            str(rr_src),
            "-o",
            str(runtime_r_path),
            "-O2",
            "--no-incremental",
        ]
    )

    plain_stdout = capture_stdout(["Rscript", "--vanilla", str(plain_r_path)])
    helper_stdout = capture_stdout(["Rscript", "--vanilla", str(helper_r_path)])
    if plain_stdout != helper_stdout:
        raise RuntimeError("direct R transcription stdout did not match RR O2 helper-only output")

    rows: list[dict[str, object]] = []

    direct_stats = benchmark(
        ["Rscript", "--vanilla", str(plain_r_path)],
        runs=args.runs,
        warmup=args.warmup,
    )
    rows.append(
        {
            "id": "direct_r_gnur",
            "label": "Direct base R",
            "kind": "runtime",
            "engine": "GNU R 4.5.2",
            "artifact": "Direct R transcription of example/tesseract.rr",
            "runs_ms": ";".join(f"{x:.1f}" for x in direct_stats["runs_ms"]),
            "mean_ms": direct_stats["mean_ms"],
            "stdev_ms": direct_stats["stdev_ms"],
            "min_ms": direct_stats["min_ms"],
            "max_ms": direct_stats["max_ms"],
            "notes": "stdout matched RR O2 helper-only exactly",
        }
    )

    helper_stats = benchmark(
        ["Rscript", "--vanilla", str(helper_r_path)],
        runs=args.runs,
        warmup=args.warmup,
    )
    rows.append(
        {
            "id": "rr_o2_gnur_helper_only",
            "label": "RR O2 emitted R",
            "kind": "runtime",
            "engine": "GNU R 4.5.2",
            "artifact": "RR O2 helper-only emitted R",
            "runs_ms": ";".join(f"{x:.1f}" for x in helper_stats["runs_ms"]),
            "mean_ms": helper_stats["mean_ms"],
            "stdev_ms": helper_stats["stdev_ms"],
            "min_ms": helper_stats["min_ms"],
            "max_ms": helper_stats["max_ms"],
            "notes": "-O2 --no-incremental --no-runtime",
        }
    )

    runtime_stats = benchmark(
        ["Rscript", "--vanilla", str(runtime_r_path)],
        runs=args.runs,
        warmup=args.warmup,
    )
    rows.append(
        {
            "id": "rr_o2_gnur_runtime",
            "label": "RR O2 emitted R (default runtime)",
            "kind": "runtime",
            "engine": "GNU R 4.5.2",
            "artifact": "RR O2 runtime-injected emitted R",
            "runs_ms": ";".join(f"{x:.1f}" for x in runtime_stats["runs_ms"]),
            "mean_ms": runtime_stats["mean_ms"],
            "stdev_ms": runtime_stats["stdev_ms"],
            "min_ms": runtime_stats["min_ms"],
            "max_ms": runtime_stats["max_ms"],
            "notes": "-O2 --no-incremental",
        }
    )

    compile_stats = benchmark(
        [
            str(rr_bin),
            str(rr_src),
            "-o",
            str(out_dir / "tesseract_rr_compile_probe.R"),
            "-O2",
            "--no-incremental",
            "--no-runtime",
        ],
        runs=args.runs,
        warmup=args.warmup,
    )
    rows.append(
        {
            "id": "rr_compile_o2_release",
            "label": "RR compile only",
            "kind": "compile",
            "engine": "RR 5.0.0 release",
            "artifact": "Compiler front-to-back O2 emit",
            "runs_ms": ";".join(f"{x:.1f}" for x in compile_stats["runs_ms"]),
            "mean_ms": compile_stats["mean_ms"],
            "stdev_ms": compile_stats["stdev_ms"],
            "min_ms": compile_stats["min_ms"],
            "max_ms": compile_stats["max_ms"],
            "notes": "-O2 --no-incremental --no-runtime",
        }
    )

    if not args.skip_renjin and args.renjin_bin.exists():
        env = renjin_env()
        if env is not None:
            renjin_stats = benchmark(
                [str(args.renjin_bin), "-f", str(helper_r_path)],
                runs=args.runs,
                warmup=args.warmup,
                env=env,
            )
            rows.append(
                {
                    "id": "rr_o2_renjin_helper_only",
                    "label": "RR O2 emitted R",
                    "kind": "runtime",
                    "engine": "Renjin 3.5-beta76",
                    "artifact": "RR O2 helper-only emitted R",
                    "runs_ms": ";".join(f"{x:.1f}" for x in renjin_stats["runs_ms"]),
                    "mean_ms": renjin_stats["mean_ms"],
                    "stdev_ms": renjin_stats["stdev_ms"],
                    "min_ms": renjin_stats["min_ms"],
                    "max_ms": renjin_stats["max_ms"],
                    "notes": "OpenJDK 25.0.2",
                }
            )

    json_path = out_dir / "tesseract_bench.json"
    csv_path = out_dir / "tesseract_bench.csv"
    json_path.write_text(json.dumps(rows, indent=2) + "\n")
    write_results_csv(csv_path, rows)

    print(json.dumps(rows, indent=2))
    print(f"\nWrote {json_path}")
    print(f"Wrote {csv_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
