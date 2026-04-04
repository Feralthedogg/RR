#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

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

BASE_SHA="$(read_base_sha)"
if [[ -n "$BASE_SHA" ]]; then
  exec perl "$ROOT/scripts/contributing_audit.pl" --scan-only --base "$BASE_SHA"
fi

if git -C "$ROOT" rev-parse --verify HEAD^ >/dev/null 2>&1; then
  exec perl "$ROOT/scripts/contributing_audit.pl" --scan-only --base \
    "$(git -C "$ROOT" rev-parse --verify HEAD^)"
fi

AUDIT_FILES=()
AUDIT_FILES_RAW="$(
  git -C "$ROOT" diff-tree --root --no-commit-id --name-only -r HEAD -- \
    | awk '
        /^src\// || /^tests\// || /^docs\// || /^scripts\// || /^fuzz\// || /^native\// || /^policy\// || /^\.github\/pull_request_template\.md$/ || /^CONTRIBUTING\.md$/ {
          print
        }
      ' \
    | sort -u
)"

if [[ -n "$AUDIT_FILES_RAW" ]]; then
  while IFS= read -r line; do
    [[ -n "$line" ]] || continue
    AUDIT_FILES+=("$line")
  done <<< "$AUDIT_FILES_RAW"
fi

if [[ ${#AUDIT_FILES[@]} -gt 0 ]]; then
  exec perl "$ROOT/scripts/contributing_audit.pl" --scan-only --files "${AUDIT_FILES[@]}"
fi

exec perl "$ROOT/scripts/contributing_audit.pl" --scan-only
