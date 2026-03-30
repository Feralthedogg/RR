#!/usr/bin/env python3
from __future__ import annotations

import argparse
import math
import os
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path


def normalize_output(text: str) -> str:
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    lines = [line.rstrip() for line in text.split("\n")]
    while lines and lines[-1] == "":
        lines.pop()
    return "\n".join(lines)


def run_command(args: list[str], *, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    merged_env = os.environ.copy()
    if env:
        merged_env.update(env)
    return subprocess.run(
        args,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=merged_env,
        check=False,
    )


def compile_rr(rr_bin: Path, case_path: Path, out_path: Path, opt_flag: str, *, verify_each_pass: bool) -> subprocess.CompletedProcess[str]:
    env = {"RR_QUIET_LOG": "1"}
    if verify_each_pass:
        env["RR_VERIFY_EACH_PASS"] = "1"
    return run_command([str(rr_bin), str(case_path), "-o", str(out_path), opt_flag], env=env)


@dataclass
class DifferentialContext:
    rr_bin: Path
    rscript_bin: str
    opt_flag: str
    reference_path: Path
    expected_reference_status: int
    expected_compiled_status: int
    base_reference_stdout: str
    base_reference_stderr: str
    base_compiled_stdout: str
    base_compiled_stderr: str

    def predicate(self, candidate_text: str) -> bool:
        with tempfile.TemporaryDirectory(prefix="rr-triage-diff-") as tmp:
            tmpdir = Path(tmp)
            case_path = tmpdir / "case.rr"
            compiled_r = tmpdir / "compiled.R"
            case_path.write_text(candidate_text)

            ref = run_command([self.rscript_bin, "--vanilla", str(self.reference_path)])
            if ref.returncode != self.expected_reference_status:
                return False
            if normalize_output(ref.stdout) != self.base_reference_stdout:
                return False
            if normalize_output(ref.stderr) != self.base_reference_stderr:
                return False

            compiled_build = compile_rr(self.rr_bin, case_path, compiled_r, self.opt_flag, verify_each_pass=False)
            if compiled_build.returncode != 0:
                return False

            compiled = run_command([self.rscript_bin, "--vanilla", str(compiled_r)])
            if compiled.returncode != self.expected_compiled_status:
                return False
            if normalize_output(compiled.stdout) != self.base_compiled_stdout:
                return False
            if normalize_output(compiled.stderr) != self.base_compiled_stderr:
                return False
            return True


@dataclass
class PassVerifyContext:
    rr_bin: Path
    opt_flag: str
    stderr_anchors: list[str]

    def predicate(self, candidate_text: str) -> bool:
        with tempfile.TemporaryDirectory(prefix="rr-triage-verify-") as tmp:
            tmpdir = Path(tmp)
            case_path = tmpdir / "case.rr"
            out_path = tmpdir / "out.R"
            case_path.write_text(candidate_text)
            compiled = compile_rr(self.rr_bin, case_path, out_path, self.opt_flag, verify_each_pass=True)
            if compiled.returncode == 0:
                return False
            if not self.stderr_anchors:
                return True
            stderr = compiled.stderr
            return any(anchor in stderr for anchor in self.stderr_anchors)


def derive_stderr_anchors(stderr_text: str) -> list[str]:
    anchors: list[str] = []
    for raw in stderr_text.splitlines():
        line = raw.strip()
        if not line:
            continue
        if line.startswith("[ok]") or line.startswith("=>") or line.startswith("[+]"):
            continue
        anchors.append(line)
        if len(anchors) >= 3:
            break
    return anchors


def ddmin_lines(original_text: str, predicate) -> str:
    lines = original_text.splitlines(keepends=True)
    if not lines:
        return original_text

    granularity = 2
    while len(lines) >= 2:
        chunk_size = math.ceil(len(lines) / granularity)
        changed = False
        for start in range(0, len(lines), chunk_size):
            candidate = lines[:start] + lines[start + chunk_size :]
            if not candidate:
                continue
            candidate_text = "".join(candidate)
            if predicate(candidate_text):
                lines = candidate
                granularity = max(2, granularity - 1)
                changed = True
                break
        if changed:
            continue
        if granularity >= len(lines):
            break
        granularity = min(len(lines), granularity * 2)
    return "".join(lines)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Reduce RR triage cases while preserving failure behavior.")
    parser.add_argument("--kind", required=True, choices=["differential", "pass-verify"])
    parser.add_argument("--rr-bin", required=True)
    parser.add_argument("--case", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--opt", default="-O2")
    parser.add_argument("--reference")
    parser.add_argument("--rscript-bin", default=shutil.which("Rscript") or "Rscript")
    parser.add_argument("--expected-reference-status", type=int)
    parser.add_argument("--expected-compiled-status", type=int)
    parser.add_argument("--reference-stdout-file")
    parser.add_argument("--reference-stderr-file")
    parser.add_argument("--compiled-stdout-file")
    parser.add_argument("--compiled-stderr-file")
    parser.add_argument("--stderr-anchor-file")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    rr_bin = Path(args.rr_bin)
    case_path = Path(args.case)
    output_path = Path(args.output)
    original_text = case_path.read_text()

    if args.kind == "differential":
        if not all(
            [
                args.reference,
                args.expected_reference_status is not None,
                args.expected_compiled_status is not None,
                args.reference_stdout_file,
                args.reference_stderr_file,
                args.compiled_stdout_file,
                args.compiled_stderr_file,
            ]
        ):
            raise SystemExit("differential reduction requires reference path, statuses, and output snapshots")
        ctx = DifferentialContext(
            rr_bin=rr_bin,
            rscript_bin=args.rscript_bin,
            opt_flag=args.opt,
            reference_path=Path(args.reference),
            expected_reference_status=args.expected_reference_status,
            expected_compiled_status=args.expected_compiled_status,
            base_reference_stdout=normalize_output(Path(args.reference_stdout_file).read_text()),
            base_reference_stderr=normalize_output(Path(args.reference_stderr_file).read_text()),
            base_compiled_stdout=normalize_output(Path(args.compiled_stdout_file).read_text()),
            base_compiled_stderr=normalize_output(Path(args.compiled_stderr_file).read_text()),
        )
        predicate = ctx.predicate
    else:
        anchors = derive_stderr_anchors(Path(args.stderr_anchor_file).read_text()) if args.stderr_anchor_file else []
        ctx = PassVerifyContext(
            rr_bin=rr_bin,
            opt_flag=args.opt,
            stderr_anchors=anchors,
        )
        predicate = ctx.predicate

    if not predicate(original_text):
        raise SystemExit("original case does not satisfy reduction predicate")

    reduced = ddmin_lines(original_text, predicate)
    output_path.write_text(reduced)
    print(f"original_bytes={len(original_text.encode())}")
    print(f"reduced_bytes={len(reduced.encode())}")
    print(f"output={output_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
