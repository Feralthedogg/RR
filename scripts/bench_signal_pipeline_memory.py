#!/usr/bin/env python3

from __future__ import annotations

import argparse
import csv
import json
import pathlib
import subprocess
import textwrap

from bench_signal_pipeline import (
    DEFAULT_OUT_DIR,
    DEFAULT_RR_BIN,
    DEFAULT_RR_SRC,
    WARM_KERNEL_ITERS,
    common_env,
    compile_rr_variant,
    ensure_release_rr,
    render_rr_warm_driver,
    write_text,
)
from bench_utils import write_results_csv

ROOT = pathlib.Path(__file__).resolve().parents[1]


def render_direct_kernel_defs() -> str:
    return textwrap.dedent(
        """\
        signal_pipeline_kernel <- function() {
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

          invisible(clean[n] + mean(clean))
        }
        """
    )


def render_rr_kernel_loader(rr_r: pathlib.Path) -> str:
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
          rr_signal_pipeline_kernel <- eval(
            call("function", pairlist(), get(body_name, envir = bench_env, inherits = FALSE)),
            envir = bench_env
          )
        }} else {{
          rr_signal_pipeline_kernel <- get(kernel_name, envir = bench_env, inherits = FALSE)
        }}
        """
    )


def render_memory_profile_script(
    direct_defs: pathlib.Path,
    rr_r: pathlib.Path,
    csv_path: pathlib.Path,
    warm_iters: int,
) -> str:
    direct_path = str(direct_defs).replace("\\", "\\\\")
    csv_out = str(csv_path).replace("\\", "\\\\")
    return textwrap.dedent(
        f"""\
        source("{direct_path}", local = TRUE)

        {render_rr_kernel_loader(rr_r)}

        profile_kernel <- function(id, label, engine, kernel, warm_iters, notes) {{
          profile_path <- tempfile("rr-rprofmem-")
          invisible(gc())
          invisible(gc(reset = TRUE))
          Rprofmem(profile_path)
          start <- proc.time()[["elapsed"]]
          for (.rr_iter in seq_len(warm_iters)) {{
            kernel()
          }}
          elapsed_ms <- (proc.time()[["elapsed"]] - start) * 1000.0 / warm_iters
          Rprofmem(NULL)

          mem_lines <- readLines(profile_path, warn = FALSE)
          unlink(profile_path)
          byte_lines <- grep("^[0-9]+", mem_lines, value = TRUE)
          bytes <- suppressWarnings(as.numeric(sub(" .*", "", byte_lines)))
          bytes <- bytes[!is.na(bytes)]
          gc_after <- gc()

          data.frame(
            id = id,
            label = label,
            engine = engine,
            warm_iters = warm_iters,
            elapsed_ms = round(elapsed_ms, 3),
            alloc_events = length(bytes),
            alloc_bytes = if (length(bytes) == 0L) 0 else sum(bytes),
            alloc_mb = round(if (length(bytes) == 0L) 0 else sum(bytes) / 1024.0 / 1024.0, 3),
            ncells_used = gc_after["Ncells", "used"],
            ncells_trigger = gc_after["Ncells", "gc trigger"],
            vcells_used = gc_after["Vcells", "used"],
            vcells_trigger = gc_after["Vcells", "gc trigger"],
            notes = notes,
            check.names = FALSE
          )
        }}

        rows <- list(
          profile_kernel(
            "direct_r_vector_warm",
            "Direct base R vectorized warm",
            "GNU R",
            signal_pipeline_kernel,
            {warm_iters}L,
            "Rprofmem allocation pressure for the idiomatic vectorized base-R kernel"
          ),
          profile_kernel(
            "rr_o2_gnur_warm",
            "RR O2 GNU R warm",
            "GNU R",
            rr_signal_pipeline_kernel,
            {warm_iters}L,
            "Rprofmem allocation pressure for the RR O2 emitted kernel"
          )
        )

        write.csv(do.call(rbind, rows), "{csv_out}", row.names = FALSE)
        """
    )


def write_svg_chart(path: pathlib.Path, rows: list[dict[str, str]]) -> None:
    width = 920
    left = 260
    right = 160
    top = 80
    row_gap = 76
    bar_h = 32
    height = top + len(rows) * row_gap + 90
    max_alloc = max(float(row["alloc_mb"]) for row in rows) if rows else 1.0
    usable = width - left - right
    colors = ["#2a9d8f", "#e76f51"]

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title desc">',
        '  <title id="title">Signal Pipeline Memory Profile</title>',
        '  <desc id="desc">Rprofmem allocation pressure for direct vectorized GNU R and RR O2 warm kernels.</desc>',
        f'  <rect width="{width}" height="{height}" fill="#fcfcf8"/>',
        '  <text x="24" y="34" font-family="Helvetica, Arial, sans-serif" font-size="22" fill="#16212b">Signal Pipeline Memory Profile</text>',
        f'  <text x="24" y="56" font-family="Helvetica, Arial, sans-serif" font-size="12" fill="#4b5b67">Warm kernel allocation pressure, averaged over {rows[0]["warm_iters"] if rows else WARM_KERNEL_ITERS} calls in one GNU R process</text>',
        f'  <line x1="{left}" y1="{height - 58}" x2="{width - right}" y2="{height - 58}" stroke="#9aa8b2" stroke-width="1"/>',
    ]

    for idx, row in enumerate(rows):
        y = top + idx * row_gap
        alloc_mb = float(row["alloc_mb"])
        bar_w = (alloc_mb / max_alloc) * usable if max_alloc else 0.0
        color = colors[idx % len(colors)]
        lines.append(
            f'  <text x="24" y="{y + 22}" font-family="Helvetica, Arial, sans-serif" font-size="14" fill="#16212b">{row["label"]}</text>'
        )
        lines.append(
            f'  <rect x="{left}" y="{y}" width="{bar_w:.1f}" height="{bar_h}" fill="{color}" rx="4"/>'
        )
        lines.append(
            f'  <text x="{width - 18}" y="{y + 21}" text-anchor="end" font-family="Helvetica, Arial, sans-serif" font-size="13" fill="#16212b">{alloc_mb:.1f} MB, {row["alloc_events"]} alloc events</text>'
        )
    lines.extend(
        [
            '  <g font-family="Helvetica, Arial, sans-serif" font-size="11" fill="#4b5b67">',
            f'    <text x="24" y="{height - 24}">Source data: {path.with_suffix(".csv").name}</text>',
            "  </g>",
            "</svg>",
        ]
    )
    path.write_text("\n".join(lines) + "\n")


def copy_asset(src: pathlib.Path, dst: pathlib.Path | None) -> pathlib.Path | None:
    if dst is None:
        return None
    dst.parent.mkdir(parents=True, exist_ok=True)
    dst.write_text(src.read_text())
    return dst


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Profile signal_pipeline allocation pressure with Rprofmem."
    )
    parser.add_argument("--out-dir", type=pathlib.Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--rr-src", type=pathlib.Path, default=DEFAULT_RR_SRC)
    parser.add_argument("--rr-bin", type=pathlib.Path, default=DEFAULT_RR_BIN)
    parser.add_argument("--rscript-bin", default="Rscript")
    parser.add_argument("--warm-iters", type=int, default=WARM_KERNEL_ITERS)
    parser.add_argument("--csv-out", type=pathlib.Path)
    parser.add_argument("--svg-out", type=pathlib.Path)
    args = parser.parse_args()

    out_dir = args.out_dir.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    env = common_env()
    rr_bin = ensure_release_rr(args.rr_bin.resolve())

    direct_defs = out_dir / "signal_pipeline_direct_r_vector_kernel.R"
    rr_o2 = out_dir / "signal_pipeline_rr_o2.R"
    rr_o2_pulse = out_dir / "signal_pipeline_rr_o2.pulse.json"
    profile_script = out_dir / "signal_pipeline_memory_profile.R"
    csv_path = out_dir / "signal_pipeline_memory_profile.csv"
    svg_path = out_dir / "signal_pipeline_memory_profile.svg"
    json_path = out_dir / "signal_pipeline_memory_profile.json"

    write_text(direct_defs, render_direct_kernel_defs())
    compile_rr_variant(
        ROOT,
        rr_bin,
        args.rr_src.resolve(),
        rr_o2,
        "-O2",
        env,
        pulse_json_path=rr_o2_pulse,
    )
    rr_o2.write_text("options(digits = 15)\n" + rr_o2.read_text())
    write_text(out_dir / "signal_pipeline_rr_o2_warm.R", render_rr_warm_driver(rr_o2))
    write_text(profile_script, render_memory_profile_script(direct_defs, rr_o2, csv_path, args.warm_iters))

    subprocess.run(
        [args.rscript_bin, "--vanilla", str(profile_script)],
        cwd=ROOT,
        env=env,
        check=True,
        text=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    rows = list(csv.DictReader(csv_path.open()))
    write_svg_chart(svg_path, rows)
    write_results_csv(
        csv_path,
        rows,
        [
            "id",
            "label",
            "engine",
            "warm_iters",
            "elapsed_ms",
            "alloc_events",
            "alloc_bytes",
            "alloc_mb",
            "ncells_used",
            "ncells_trigger",
            "vcells_used",
            "vcells_trigger",
            "notes",
        ],
    )
    json_path.write_text(json.dumps({"rows": rows}, indent=2) + "\n")

    copied_csv = copy_asset(csv_path, args.csv_out.resolve() if args.csv_out else None)
    copied_svg = copy_asset(svg_path, args.svg_out.resolve() if args.svg_out else None)
    if copied_svg is not None:
        source_path = copied_csv or csv_path
        source_name = source_path.name
        svg_text = copied_svg.read_text().replace(csv_path.name, source_name)
        copied_svg.write_text(svg_text)

    print(
        json.dumps(
            {
                "csv": str(copied_csv or csv_path),
                "svg": str(copied_svg or svg_path),
                "rows": rows,
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
