from __future__ import annotations

import csv
import json
import os
import pathlib
import re
import subprocess
from typing import Iterable


def write_results_csv(path: pathlib.Path, rows: list[dict[str, object]], preferred: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fieldnames = list(preferred)
    seen = set(fieldnames)
    for row in rows:
        for key in row:
            if key not in seen:
                fieldnames.append(key)
                seen.add(key)
    with path.open("w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        for row in rows:
            writer.writerow(row)


def compile_rr_variant(
    root: pathlib.Path,
    rr_bin: pathlib.Path,
    src: pathlib.Path,
    out_path: pathlib.Path,
    opt_flag: str,
    env: dict[str, str],
    *,
    extra_args: Iterable[str] = (),
    pulse_json_path: pathlib.Path | None = None,
) -> pathlib.Path | None:
    compile_env = env.copy()
    if pulse_json_path is not None:
        pulse_json_path.parent.mkdir(parents=True, exist_ok=True)
        compile_env["RR_PULSE_JSON_PATH"] = str(pulse_json_path)
    subprocess.run(
        [str(rr_bin), str(src), "-o", str(out_path), opt_flag, "--no-incremental", *extra_args],
        cwd=root,
        env=compile_env,
        check=True,
        text=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return pulse_json_path


def rr_artifact_diagnostics(
    rr_path: pathlib.Path,
    pulse_json_path: pathlib.Path | None = None,
) -> dict[str, object]:
    code = rr_path.read_text()
    diag: dict[str, object] = {
        "emit_lines": len(code.splitlines()),
        "emit_bytes": len(code.encode()),
        "emit_for_loops": code.count("for ("),
        "emit_repeat_loops": code.count("repeat {"),
        "emit_rr_index_reads": code.count("rr_index1_read(")
        + code.count("rr_index1_read_vec(")
        + code.count("rr_index1_read_vec_floor("),
        "emit_rr_index_writes": code.count("rr_index1_write("),
        "emit_rr_call_map_helpers": code.count("rr_call_map_"),
        "emit_assign_slice_helpers": code.count("rr_assign_slice("),
        "emit_sym_helpers": len(re.findall(r"(?m)^Sym_[A-Za-z0-9_]+ <- function", code)),
    }
    if pulse_json_path is not None and pulse_json_path.exists():
        pulse = json.loads(pulse_json_path.read_text())
        for key in [
            "vectorized",
            "reduced",
            "vector_loops_seen",
            "vector_skipped",
            "vector_candidate_total",
            "vector_candidate_reductions",
            "vector_candidate_conditionals",
            "vector_candidate_recurrences",
            "vector_candidate_shifted",
            "vector_candidate_call_maps",
            "vector_candidate_expr_maps",
            "vector_candidate_scatters",
            "vector_candidate_cube_slices",
            "vector_candidate_basic_maps",
            "vector_candidate_multi_exprs",
            "vector_candidate_2d",
            "vector_candidate_3d",
            "vector_candidate_call_map_direct",
            "vector_candidate_call_map_runtime",
            "vector_applied_total",
            "vector_applied_reductions",
            "vector_applied_conditionals",
            "vector_applied_recurrences",
            "vector_applied_shifted",
            "vector_applied_call_maps",
            "vector_applied_expr_maps",
            "vector_applied_scatters",
            "vector_applied_cube_slices",
            "vector_applied_basic_maps",
            "vector_applied_multi_exprs",
            "vector_applied_2d",
            "vector_applied_3d",
            "vector_applied_call_map_direct",
            "vector_applied_call_map_runtime",
            "vector_trip_tier_tiny",
            "vector_trip_tier_small",
            "vector_trip_tier_medium",
            "vector_trip_tier_large",
            "simplified_loops",
            "licm_hits",
            "sccp_hits",
            "gvn_hits",
            "bce_hits",
            "simplify_hits",
            "dce_hits",
        ]:
            if key in pulse:
                diag[f"pulse_{key}"] = pulse[key]
    return diag


def attach_diagnostics(row: dict[str, object], diagnostics: dict[str, object] | None) -> dict[str, object]:
    if diagnostics:
        row.update(diagnostics)
    return row
