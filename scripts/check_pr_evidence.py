#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
POLICY_PATH = ROOT / "policy" / "contributing_rules.toml"


def load_policy() -> dict:
    with POLICY_PATH.open("rb") as fh:
        return tomllib.load(fh)


def read_event() -> dict:
    event_name = os.environ.get("GITHUB_EVENT_NAME", "")
    event_path_raw = os.environ.get("GITHUB_EVENT_PATH", "")
    if event_name != "pull_request" or not event_path_raw:
        return {}
    path = Path(event_path_raw)
    if not path.exists():
        return {}
    with path.open("r", encoding="utf-8") as fh:
        return json.load(fh)


def git_changed_files(base: str) -> list[str]:
    result = subprocess.run(
        ["git", "-C", str(ROOT), "diff", "--name-only", f"{base}...HEAD", "--"],
        check=True,
        text=True,
        capture_output=True,
    )
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def parse_sections(body: str) -> dict[str, str]:
    sections: dict[str, list[str]] = {}
    current: str | None = None
    for raw_line in body.splitlines():
        line = raw_line.rstrip()
        if line.startswith("## "):
            current = line.strip()
            sections.setdefault(current, [])
            continue
        if current is not None:
            sections[current].append(line)
    return {heading: "\n".join(lines).strip() for heading, lines in sections.items()}


def section_is_placeholder(text: str) -> bool:
    normalized = text.strip().lower()
    if not normalized:
        return True
    placeholders = {
        "n/a",
        "na",
        "none",
        "not applicable",
        "- none",
        "- n/a",
        "_none_",
        "_n/a_",
    }
    return normalized in placeholders


def any_matches(files: list[str], prefixes: list[str]) -> bool:
    return any(any(path == prefix or path.startswith(prefix) for prefix in prefixes) for path in files)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--body-file", help="read PR body from this file instead of the GitHub event")
    parser.add_argument(
        "--changed-files-file",
        help="read changed files from this newline-delimited file instead of git diff",
    )
    parser.add_argument("--base", help="override git diff base sha")
    args = parser.parse_args()

    policy = load_policy()
    automation = policy["automation"]

    if args.body_file:
        body = Path(args.body_file).read_text(encoding="utf-8")
    else:
        event = read_event()
        if not event:
            print("skip: PR evidence checks run only for pull_request events")
            return 0
        body = event.get("pull_request", {}).get("body") or ""

    if args.changed_files_file:
        changed_files = [
            line.strip()
            for line in Path(args.changed_files_file).read_text(encoding="utf-8").splitlines()
            if line.strip()
        ]
    else:
        base = args.base
        if base is None:
            event = read_event()
            base = event.get("pull_request", {}).get("base", {}).get("sha", "") if event else ""
        if not base:
            print("skip: no PR base available for evidence diff scope")
            return 0
        changed_files = git_changed_files(base)

    sections = parse_sections(body)
    required = automation["pr_required_sections"]
    missing = [heading for heading in required if heading not in sections]
    errors: list[str] = []

    if missing:
        errors.append("missing required PR body sections: " + ", ".join(missing))

    verification = sections.get("## Verification", "")
    if section_is_placeholder(verification):
        errors.append("`## Verification` must summarize the checks that were actually run")

    perf_sensitive = any_matches(changed_files, automation["perf_sensitive_prefixes"])
    benchmark = sections.get("## Benchmark Evidence", "")
    if perf_sensitive and section_is_placeholder(benchmark):
        errors.append(
            "performance-sensitive files changed, so `## Benchmark Evidence` must contain real evidence or an explicit measured rationale"
        )

    dependency_sensitive = any(
        any(path == dep or path.endswith(f"/{dep}") for dep in automation["dependency_files"])
        for path in changed_files
    )
    dependency = sections.get("## Dependency Impact", "")
    if dependency_sensitive and section_is_placeholder(dependency):
        errors.append(
            "dependency-related files changed, so `## Dependency Impact` must explain approval, portability, and determinism implications"
        )

    exception_sensitive = any_matches(changed_files, automation["exception_sensitive_prefixes"])
    exceptions = sections.get("## Exceptions", "")
    if exception_sensitive and section_is_placeholder(exceptions):
        errors.append(
            "policy or audit wiring changed, so `## Exceptions` must describe any deliberate rule deviations or state `None` with rationale"
        )

    if errors:
        print("PR evidence check failed:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        print("changed files:", file=sys.stderr)
        for path in changed_files:
            print(f"  - {path}", file=sys.stderr)
        return 1

    print("PR evidence check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
