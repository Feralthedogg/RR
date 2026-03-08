#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_ROOT="${1:-$ROOT/.artifacts}"
OUT_DIR="${2:-$ARTIFACT_ROOT/nightly-soak}"

DIFF_JSON="$ARTIFACT_ROOT/differential-triage/summary.json"
PASS_JSON="$ARTIFACT_ROOT/pass-verify-triage/summary.json"
FUZZ_JSON="$ARTIFACT_ROOT/fuzz-triage/summary.json"
DIFF_DIR="$ARTIFACT_ROOT/differential-triage"
PASS_DIR="$ARTIFACT_ROOT/pass-verify-triage"
FUZZ_DIR="$ARTIFACT_ROOT/fuzz-triage"

RUN_ID="${GITHUB_RUN_ID:-}"
RUN_ATTEMPT="${GITHUB_RUN_ATTEMPT:-}"
WORKFLOW="${GITHUB_WORKFLOW:-}"
JOB_NAME="${GITHUB_JOB:-}"
REF_NAME="${GITHUB_REF_NAME:-}"
SHA="${GITHUB_SHA:-}"

mkdir -p "$OUT_DIR"

python3 - "$DIFF_JSON" "$PASS_JSON" "$FUZZ_JSON" "$DIFF_DIR" "$PASS_DIR" "$FUZZ_DIR" "$OUT_DIR" "$RUN_ID" "$RUN_ATTEMPT" "$WORKFLOW" "$JOB_NAME" "$REF_NAME" "$SHA" <<'PY'
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

diff_path = Path(sys.argv[1])
pass_path = Path(sys.argv[2])
fuzz_path = Path(sys.argv[3])
diff_dir = Path(sys.argv[4])
pass_dir = Path(sys.argv[5])
fuzz_dir = Path(sys.argv[6])
out_dir = Path(sys.argv[7])
run_id = sys.argv[8]
run_attempt = sys.argv[9]
workflow = sys.argv[10]
job_name = sys.argv[11]
ref_name = sys.argv[12]
sha = sys.argv[13]


def empty_report(kind: str, message: str):
    return {
        "schema": "rr-triage-report",
        "version": 1,
        "kind": kind,
        "message": message,
        "cases": [],
    }


def load_report(path: Path, kind: str):
    if not path.exists():
        return empty_report(kind, f"missing summary at {path}")
    with path.open("r", encoding="utf-8") as fh:
        data = json.load(fh)
    if data.get("schema") != "rr-triage-report":
        raise SystemExit(f"invalid triage schema in {path}: {data.get('schema')!r}")
    if data.get("version") != 1:
        raise SystemExit(f"invalid triage version in {path}: {data.get('version')!r}")
    if data.get("kind") != kind:
        raise SystemExit(f"invalid triage kind in {path}: expected {kind!r}, got {data.get('kind')!r}")
    return data


def collect_promotable_cases(kind: str, root: Path):
    if not root.exists():
        return []

    def parse_manifest(manifest: Path):
        data = {}
        if not manifest.exists():
            return data
        with manifest.open("r", encoding="utf-8") as fh:
            for raw in fh:
                line = raw.strip()
                if not line or ": " not in line:
                    continue
                key, value = line.split(": ", 1)
                data[key] = value
        return data

    def score_candidate(kind: str, manifest_data: dict):
        if kind == "pass-verify":
            return ("critical", 300, "verifier failed after a compiler pass")
        if kind == "differential":
            ref = manifest_data.get("reference_status", "")
            compiled = manifest_data.get("compiled_status", "")
            if ref == "0" and compiled != "0":
                return ("critical", 260, "optimized output changed runtime exit behavior")
            return ("high", 220, "optimized output diverged from reference behavior")
        if kind == "fuzz":
            repro = manifest_data.get("repro_status", "")
            tmin = manifest_data.get("tmin_status", "")
            if repro == "0" and tmin == "0":
                return ("high", 180, "fuzz crash reproduces and minimizes cleanly")
            if repro == "0":
                return ("medium", 150, "fuzz crash reproduces but did not minimize")
            return ("medium", 120, "fuzz artifact needs manual replay")
        return ("medium", 100, "generic promotion candidate")

    candidates = []
    for case_dir in sorted(p for p in root.iterdir() if p.is_dir()):
        regression = case_dir / "regression.rs"
        manifest = case_dir / "bundle.manifest"
        if not regression.exists():
            continue
        manifest_data = parse_manifest(manifest)
        if kind == "fuzz":
            if manifest_data.get("skeleton_kind") != "rust":
                continue
        severity, priority, rationale = score_candidate(kind, manifest_data)
        candidates.append(
            {
                "kind": kind,
                "case_dir": str(case_dir),
                "bundle": case_dir.name,
                "command": f"scripts/triage_driver.sh promote {kind} {case_dir}",
                "severity": severity,
                "priority": priority,
                "rationale": rationale,
            }
        )
    return sorted(
        candidates,
        key=lambda candidate: (-candidate["priority"], candidate["kind"], candidate["bundle"]),
    )


def count_by(items, key_fn):
    counts = {}
    for item in items:
        key = key_fn(item)
        counts[key] = counts.get(key, 0) + 1
    return dict(sorted(counts.items()))


diff = load_report(diff_path, "differential")
pass_verify = load_report(pass_path, "pass-verify")
fuzz = load_report(fuzz_path, "fuzz")
promotion_candidates = (
    collect_promotable_cases("differential", diff_dir)
    + collect_promotable_cases("pass-verify", pass_dir)
    + collect_promotable_cases("fuzz", fuzz_dir)
)
promotion_candidates = sorted(
    promotion_candidates,
    key=lambda candidate: (-candidate["priority"], candidate["kind"], candidate["bundle"]),
)

summary = {
    "schema": "rr-verification-summary",
    "version": 1,
    "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
    "run_context": {
        "github_run_id": run_id or None,
        "github_run_attempt": run_attempt or None,
        "github_workflow": workflow or None,
        "github_job": job_name or None,
        "github_ref_name": ref_name or None,
        "github_sha": sha or None,
    },
    "sources": {
        "differential": str(diff_path),
        "pass_verify": str(pass_path),
        "fuzz": str(fuzz_path),
    },
    "differential": diff,
    "pass_verify": pass_verify,
    "fuzz": fuzz,
    "promotion_candidates": promotion_candidates,
    "breakdown": {
        "differential_by_opt": count_by(diff.get("cases", []), lambda case: str(case.get("opt", "unknown"))),
        "fuzz_by_target": count_by(fuzz.get("cases", []), lambda case: str(case.get("target", "unknown"))),
        "promotion_candidates_by_kind": count_by(promotion_candidates, lambda case: str(case.get("kind", "unknown"))),
        "promotion_candidates_by_severity": count_by(
            promotion_candidates, lambda case: str(case.get("severity", "unknown"))
        ),
    },
    "totals": {
        "differential_failure_bundles": int(diff.get("failure_bundles", 0)),
        "differential_invalid_bundles": int(diff.get("invalid_bundles", 0)),
        "differential_rust_regression_skeletons": int(diff.get("rust_regression_skeletons", 0)),
        "pass_verify_failure_bundles": int(pass_verify.get("failure_bundles", 0)),
        "pass_verify_invalid_bundles": int(pass_verify.get("invalid_bundles", 0)),
        "fuzz_artifacts": int(fuzz.get("artifacts", 0)),
        "fuzz_reproduced": int(fuzz.get("reproduced", 0)),
        "fuzz_minimized": int(fuzz.get("minimized", 0)),
        "fuzz_rust_regression_skeletons": int(fuzz.get("rust_regression_skeletons", 0)),
        "fuzz_manual_replay_notes": int(fuzz.get("manual_replay_notes", 0)),
        "total_cases": len(diff.get("cases", [])) + len(pass_verify.get("cases", [])) + len(fuzz.get("cases", [])),
        "promotion_candidate_count": len(promotion_candidates),
    },
}

summary_json_path = out_dir / "verification-summary.json"
summary_md_path = out_dir / "verification-summary.md"
top_candidates_json_path = out_dir / "top-promotion-candidates.json"
top_candidates_md_path = out_dir / "top-promotion-candidates.md"
top_candidates = promotion_candidates[:5]

with summary_json_path.open("w", encoding="utf-8") as fh:
    json.dump(summary, fh, indent=2, sort_keys=True)
    fh.write("\n")

with top_candidates_json_path.open("w", encoding="utf-8") as fh:
    json.dump(
        {
            "schema": "rr-promotion-candidates",
            "version": 1,
            "generated_at_utc": summary["generated_at_utc"],
            "run_context": summary["run_context"],
            "candidates": top_candidates,
        },
        fh,
        indent=2,
        sort_keys=True,
    )
    fh.write("\n")

with summary_md_path.open("w", encoding="utf-8") as fh:
    fh.write("# Nightly Verification Summary\n\n")
    fh.write(f"- generated at: `{summary['generated_at_utc']}`\n")
    if run_id:
        fh.write(f"- github run id: `{run_id}`\n")
    if run_attempt:
        fh.write(f"- github run attempt: `{run_attempt}`\n")
    if workflow:
        fh.write(f"- github workflow: `{workflow}`\n")
    if job_name:
        fh.write(f"- github job: `{job_name}`\n")
    if ref_name:
        fh.write(f"- github ref: `{ref_name}`\n")
    if sha:
        fh.write(f"- github sha: `{sha}`\n")
    fh.write(f"- differential failure bundles: `{summary['totals']['differential_failure_bundles']}`\n")
    fh.write(f"- differential invalid bundles: `{summary['totals']['differential_invalid_bundles']}`\n")
    fh.write(f"- differential regression skeletons: `{summary['totals']['differential_rust_regression_skeletons']}`\n")
    fh.write(f"- pass-verify failure bundles: `{summary['totals']['pass_verify_failure_bundles']}`\n")
    fh.write(f"- pass-verify invalid bundles: `{summary['totals']['pass_verify_invalid_bundles']}`\n")
    fh.write(f"- fuzz artifacts: `{summary['totals']['fuzz_artifacts']}`\n")
    fh.write(f"- fuzz reproduced: `{summary['totals']['fuzz_reproduced']}`\n")
    fh.write(f"- fuzz minimized: `{summary['totals']['fuzz_minimized']}`\n")
    fh.write(f"- fuzz regression skeletons: `{summary['totals']['fuzz_rust_regression_skeletons']}`\n")
    fh.write(f"- fuzz manual replay notes: `{summary['totals']['fuzz_manual_replay_notes']}`\n")
    fh.write(f"- total triaged cases: `{summary['totals']['total_cases']}`\n")
    fh.write(f"- promotion candidates: `{summary['totals']['promotion_candidate_count']}`\n")
    fh.write("\n## Breakdown\n\n")
    fh.write(f"- differential by opt: `{summary['breakdown']['differential_by_opt']}`\n")
    fh.write(f"- fuzz by target: `{summary['breakdown']['fuzz_by_target']}`\n")
    fh.write(f"- promotion candidates by kind: `{summary['breakdown']['promotion_candidates_by_kind']}`\n")
    fh.write(
        f"- promotion candidates by severity: `{summary['breakdown']['promotion_candidates_by_severity']}`\n"
    )
    fh.write("\n## Top Promotion Candidates\n\n")
    if top_candidates:
        for candidate in top_candidates:
            fh.write(
                f"- `{candidate['severity']}` / `{candidate['kind']}` / priority `{candidate['priority']}`: `{candidate['bundle']}`\n"
            )
            fh.write(f"  why: {candidate['rationale']}\n")
            fh.write(f"  promote: `{candidate['command']}`\n")
    else:
        fh.write("- none\n")
    fh.write("\n## Promotion Candidates\n\n")
    if promotion_candidates:
        for candidate in promotion_candidates:
            fh.write(
                f"- `{candidate['severity']}` / `{candidate['kind']}` / priority `{candidate['priority']}`: `{candidate['bundle']}`\n"
            )
            fh.write(f"  why: {candidate['rationale']}\n")
            fh.write(f"  promote: `{candidate['command']}`\n")
    else:
        fh.write("- none\n")

with top_candidates_md_path.open("w", encoding="utf-8") as fh:
    fh.write("# Top Promotion Candidates\n\n")
    fh.write(f"- generated at: `{summary['generated_at_utc']}`\n")
    if run_id:
        fh.write(f"- github run id: `{run_id}`\n")
    fh.write(f"- candidate count: `{len(top_candidates)}`\n")
    if top_candidates:
        fh.write("\n")
        for candidate in top_candidates:
            fh.write(
                f"- `{candidate['severity']}` / `{candidate['kind']}` / priority `{candidate['priority']}`: `{candidate['bundle']}`\n"
            )
            fh.write(f"  why: {candidate['rationale']}\n")
            fh.write(f"  promote: `{candidate['command']}`\n")
    else:
        fh.write("\n- none\n")
PY
