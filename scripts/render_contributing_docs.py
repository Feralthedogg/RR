#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
POLICY_PATH = ROOT / "policy" / "contributing_rules.toml"
SUBSYSTEM_POLICY_PATH = ROOT / "policy" / "subsystems.toml"


def load_policy() -> dict:
    with POLICY_PATH.open("rb") as fh:
        return tomllib.load(fh)


def load_subsystem_policy() -> dict:
    with SUBSYSTEM_POLICY_PATH.open("rb") as fh:
        return tomllib.load(fh)


def render_list(items: list[str]) -> str:
    return "\n".join(f"- {item}" for item in items)


def render_numbered(items: list[str]) -> str:
    return "\n".join(f"{idx}. {item}" for idx, item in enumerate(items, 1))


def render_code_block(items: list[str]) -> str:
    body = "\n".join(items)
    return f"```bash\n{body}\n```"


def render_generated_banner(policy: dict) -> str:
    subtitle = policy["meta"]["subtitle"]
    return "\n".join(
        [
            "<!-- GENERATED FILE: DO NOT EDIT DIRECTLY -->",
            f"<!-- Source: {POLICY_PATH.relative_to(ROOT)} -->",
            "",
            subtitle,
            "",
        ]
    )


def render_contributing(policy: dict) -> str:
    meta = policy["meta"]
    scope = policy["scope"]
    summary = policy["contributing_summary"]

    parts = [render_generated_banner(policy)]
    parts.append(f"# {meta['title']}")
    parts.append(scope["intro"])
    parts.append("The target style is:")
    parts.append(render_list(scope["target_style"]))
    parts.append(scope["tagline"])
    parts.append("## Scope")
    parts.append(render_list(summary["scope_lines"]))
    parts.append("## Core Principles")
    parts.append(render_numbered(scope["core_principles"]))
    parts.append("## Rule Levels")
    parts.append("\n".join(f"- {line}" for line in scope["rule_levels"][:3]))
    parts.append("## Rules")
    for idx, (title, body) in enumerate(
        zip(summary["rule_titles"], summary["rule_summaries"], strict=True), 1
    ):
        parts.append(f"### {idx}) {title}")
        parts.append(f"- {body}")

    parts.append("## Exception Process")
    parts.append(summary["exception_summary"])
    parts.append("## PR Checklist")
    parts.append(render_list(summary["checklist_items"]))
    parts.append("For a concrete post-change verification pass, use")
    parts.append("[`docs/compiler/contributing-audit.md`](docs/compiler/contributing-audit.md).")
    return "\n".join(parts)


def render_contributing_audit(policy: dict) -> str:
    meta = policy["meta"]
    audit = policy["audit"]
    parts = [render_generated_banner(policy)]
    parts.append("# Contributing Audit Checklist")
    parts.append("")
    parts.append(f"Current compiler line: `{meta['current_line']}`.")
    parts.append("")
    parts.append(
        "Use this checklist after meaningful compiler changes to verify that the code still matches "
        "[`CONTRIBUTING.md`](https://github.com/Feralthedogg/RR/blob/main/CONTRIBUTING.md)."
    )
    parts.append("")
    parts.append("## Scope")
    parts.append("")
    parts.append(
        "This page is the post-change verification contract. It complements `CONTRIBUTING.md`; it does not replace it."
    )
    parts.append("")
    parts.append("## Current Status")
    parts.append("")
    parts.append(f"Automation baseline as of `{meta['manual_audit_status_date']}`:")
    parts.append("")
    parts.append(render_list(audit["status_items"]))
    parts.append("")
    parts.append("## Fast Audit")
    parts.append("")
    parts.append("Run these commands from the repository root:")
    parts.append("")
    parts.append(render_code_block(audit["fast_audit_commands"]))
    parts.append("")
    parts.append(
        "`perl scripts/contributing_audit.pl` also runs `RR_VERIFY_EACH_PASS=1 cargo test -q --test pass_verify_examples` when the scanned scope touches pass-sensitive compiler files."
    )
    parts.append("")
    parts.append("For a quick heuristic-only pass without running cargo or fuzz:")
    parts.append("")
    parts.append(render_code_block(audit["scan_only_commands"]))
    parts.append("")
    parts.append("For a strict clean-checkout-style pass:")
    parts.append("")
    parts.append(render_code_block(audit["cleanroom_commands"]))
    parts.append("")
    parts.append("Semantic smoke lanes triggered by non-scan audits:")
    parts.append("")
    parts.append(render_list(audit["semantic_lanes"]))
    parts.append("")
    parts.append(
        "Use `--skip-semantic-smoke` only when wiring the audit itself and you want to avoid re-running meaning-preservation suites."
    )
    parts.append("")
    parts.append("## When To Do More")
    parts.append("")
    parts.append("Recommended extended checks:")
    parts.append("")
    parts.append(render_code_block(audit["extended_checks"]))
    parts.append("")
    parts.append("## Manual Review Checklist")
    parts.append("")
    parts.append(render_list(audit["manual_review"]))
    parts.append("")
    parts.append("## Ongoing Watch Items")
    parts.append("")
    parts.append(render_list(audit["watch_items"]))
    parts.append("")
    parts.append("## Review Focus Areas")
    parts.append("")
    parts.append(render_list([f"`{path}`" for path in audit["focus_paths"]]))
    parts.append("")
    parts.append("## Notes")
    parts.append("")
    parts.append(render_list(audit["notes"]))
    parts.append("")
    return "\n".join(parts)


def render_testing(policy: dict) -> str:
    testing = policy["testing"]
    parts = [render_generated_banner(policy)]
    parts.append("# Testing and Quality Gates")
    parts.append("")
    parts.append("This page is the verification manual for RR.")
    parts.append("")
    parts.append("## Audience")
    parts.append("")
    parts.append("Read this page when you need to choose:")
    parts.append("")
    parts.append(render_list(testing["audience"]))
    parts.append("")
    parts.append("The goal is not just “did it compile?” but:")
    parts.append("")
    parts.append(render_list(testing["verification_goals"]))
    parts.append("")
    parts.append("## Prerequisites")
    parts.append("")
    parts.append(render_list(testing["prerequisites"]))
    parts.append("")
    parts.append("## Primary Commands")
    parts.append("")
    parts.append("Run the standard local verification stack:")
    parts.append("")
    parts.append(render_code_block(testing["primary_commands"]))
    parts.append("")
    parts.append("Run one focused suite:")
    parts.append("")
    parts.append(render_code_block(testing["focused_commands"]))
    parts.append("")
    parts.append("Audit helper:")
    parts.append("")
    parts.append(render_code_block(testing["audit_helper"]))
    parts.append("")
    parts.append(
        "On non-scan runs, `perl scripts/contributing_audit.pl` also escalates into scope-driven semantic smoke for cache correctness, determinism, numeric semantics, and fallback/runtime behavior. Use `--skip-semantic-smoke` only when changing the audit wiring itself."
    )
    parts.append("")
    parts.append("Cleanroom strict verification helper:")
    parts.append("")
    parts.append(render_code_block(testing["cleanroom_helper"]))
    parts.append("")
    parts.append("## Local vs CI")
    parts.append("")
    parts.append("RR CI does not replace local verification. The intended model is:")
    parts.append("")
    parts.append(render_list(testing["ci_model"]))
    parts.append("")
    parts.append("## Test Families")
    parts.append("")
    for family in testing["families"]:
        parts.append(f"### {family['title']}")
        parts.append("")
        parts.append(render_list([f"`{item}`" for item in family["items"]]))
        parts.append("")
        parts.append(family["summary"])
        parts.append("")
    parts.append("For the normative contributor rule set, see")
    parts.append("[`CONTRIBUTING.md`](https://github.com/Feralthedogg/RR/blob/main/CONTRIBUTING.md).")
    parts.append("")
    return "\n".join(parts)


def render_pr_template(policy: dict, subsystem_policy: dict) -> str:
    template = policy["pr_template"]
    required = policy["automation"]["pr_required_sections"]
    metadata_sections = subsystem_policy["meta"]["required_pr_sections"]

    section_prompts = {
        "## Verification": template["verification_prompts"],
        "## Benchmark Evidence": template["benchmark_prompts"],
        "## Dependency Impact": template["dependency_prompts"],
        "## Exceptions": template["exception_prompts"],
    }

    parts = [render_generated_banner(policy)]
    parts.append("# PR Evidence Template")
    parts.append("")
    parts.extend(template["intro"])
    parts.append("")
    for heading in metadata_sections:
        parts.append(heading)
        parts.append("")
        parts.append("- Fill this section with concise, concrete information.")
        parts.append("")
    for heading in required:
        parts.append(heading)
        parts.append("")
        for prompt in section_prompts[heading]:
            parts.append(f"- {prompt}")
        parts.append("")
    return "\n".join(parts)


def file_digest(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail if generated files differ")
    args = parser.parse_args()

    policy = load_policy()
    subsystem_policy = load_subsystem_policy()
    targets = {
        ROOT / "CONTRIBUTING.md": lambda: render_contributing(policy),
        ROOT / "docs" / "compiler" / "contributing-audit.md": lambda: render_contributing_audit(policy),
        ROOT / "docs" / "compiler" / "testing.md": lambda: render_testing(policy),
        ROOT / ".github" / "pull_request_template.md": lambda: render_pr_template(policy, subsystem_policy),
    }
    failures = []
    for path, renderer in targets.items():
        rendered = renderer()
        if args.check:
            existing = path.read_text(encoding="utf-8")
            if existing != rendered:
                failures.append(
                    f"{path.relative_to(ROOT)} drifted from policy "
                    f"(expected {file_digest(rendered)[:12]}, got {file_digest(existing)[:12]})"
                )
        else:
            path.write_text(rendered, encoding="utf-8")

    if failures:
        print("generated documentation is out of date:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
