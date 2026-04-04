#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCOPE="${RR_SEMANTIC_AUDIT_SCOPE:-all}"

COMMON_ARGS=(
  --skip-fuzz
)

read_base_sha() {
  python3 -c '
import json
import os

event_name = os.environ.get("GITHUB_EVENT_NAME", "")
event_path = os.environ.get("GITHUB_EVENT_PATH", "")
base = ""

if event_path:
    with open(event_path, "r", encoding="utf-8") as fh:
        event = json.load(fh)
    if event_name == "pull_request":
        base = event.get("pull_request", {}).get("base", {}).get("sha", "")
    elif event_name == "push":
        base = event.get("before", "")

if base and set(base) == {"0"}:
    base = ""

print(base)
'
}

case "$SCOPE" in
  all)
    exec perl "$ROOT/scripts/contributing_audit.pl" "${COMMON_ARGS[@]}" --all
    ;;
  diff)
    BASE_SHA="$(read_base_sha)"
    if [[ -n "$BASE_SHA" ]]; then
      exec perl "$ROOT/scripts/contributing_audit.pl" "${COMMON_ARGS[@]}" --base "$BASE_SHA"
    fi
    if git -C "$ROOT" rev-parse --verify HEAD^ >/dev/null 2>&1; then
      exec perl "$ROOT/scripts/contributing_audit.pl" "${COMMON_ARGS[@]}" --base \
        "$(git -C "$ROOT" rev-parse --verify HEAD^)"
    fi

    mapfile -t AUDIT_FILES < <(
      git -C "$ROOT" diff-tree --root --no-commit-id --name-only -r HEAD --
      | awk '
          /^src\// || /^tests\// || /^docs\// || /^scripts\// || /^fuzz\// || /^native\// || /^policy\// || /^\.github\/pull_request_template\.md$/ || /^CONTRIBUTING\.md$/ {
            print
          }
        '
      | sort -u
    )

    if [[ ${#AUDIT_FILES[@]} -gt 0 ]]; then
      exec perl "$ROOT/scripts/contributing_audit.pl" "${COMMON_ARGS[@]}" --files "${AUDIT_FILES[@]}"
    fi

    exec perl "$ROOT/scripts/contributing_audit.pl" "${COMMON_ARGS[@]}"
    ;;
  *)
    echo "unknown RR_SEMANTIC_AUDIT_SCOPE: $SCOPE" >&2
    echo "expected one of: all, diff" >&2
    exit 2
    ;;
esac
