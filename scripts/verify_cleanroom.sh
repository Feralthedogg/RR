#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

KEEP_WORKTREE=0
SKIP_FUZZ=0
SKIP_DOCS=0
FAST=0
WORKTREE_DIR=""
declare -a PNPM_CMD=()
declare -a FILES=()

usage() {
  cat <<'EOF'
Usage: scripts/verify_cleanroom.sh [options]

Run the strict verification stack in a temporary clean git worktree and overlay
only the selected current-tree files before running checks.

Options:
  --files <paths>   Overlay only the explicit file list that follows.
  --worktree-dir    Use the given directory instead of a temporary worktree.
  --keep-worktree   Keep the clean worktree on success/failure for inspection.
  --fast            Run fmt/check/clippy/audit only.
  --skip-fuzz       Skip FUZZ_SECONDS=1 ./scripts/fuzz_smoke.sh.
  --skip-docs       Skip docs dependency bootstrap and VitePress build.
  --help            Show this help.

Default behavior:
  - create a detached clean worktree at HEAD
  - overlay current changed/untracked files from the source tree
  - run:
      cargo fmt --all --check
      cargo check
      cargo clippy --all-targets -- -D warnings
      python3 scripts/render_contributing_docs.py --check
      cargo test -q --no-fail-fast
      RR_VERIFY_EACH_PASS=1 cargo test -q --test pass_verify_examples
      perl scripts/contributing_audit.pl --all --scan-only
      FUZZ_SECONDS=1 ./scripts/fuzz_smoke.sh
      cd docs && pnpm install --frozen-lockfile && pnpm docs:build

Use --files to verify only the patch you intend to review when the source
worktree already contains unrelated dirty changes.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --files)
      shift
      while [[ $# -gt 0 && "$1" != --* ]]; do
        FILES+=("$1")
        shift
      done
      ;;
    --worktree-dir)
      WORKTREE_DIR="${2:-}"
      if [[ -z "$WORKTREE_DIR" ]]; then
        echo "missing value for --worktree-dir" >&2
        exit 2
      fi
      shift 2
      ;;
    --keep-worktree)
      KEEP_WORKTREE=1
      shift
      ;;
    --fast)
      FAST=1
      shift
      ;;
    --skip-fuzz)
      SKIP_FUZZ=1
      shift
      ;;
    --skip-docs)
      SKIP_DOCS=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if ! git -C "$ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "verify_cleanroom.sh must run inside a git worktree" >&2
  exit 1
fi

collect_scope_paths() {
  if [[ ${#FILES[@]} -gt 0 ]]; then
    printf '%s\n' "${FILES[@]}"
    return
  fi

  python3 - "$ROOT" <<'PY'
import subprocess
import sys

root = sys.argv[1]
status = subprocess.check_output(
    ["git", "-C", root, "status", "--porcelain=v1", "-z", "--untracked-files=all"],
)
entries = [entry for entry in status.split(b"\0") if entry]
paths = set()
i = 0
while i < len(entries):
    entry = entries[i]
    if len(entry) < 3:
        i += 1
        continue
    xy = entry[:2].decode("utf-8", "replace")
    path = entry[3:].decode("utf-8", "surrogateescape")
    if "R" in xy or "C" in xy:
        paths.add(path)
        if i + 1 < len(entries):
            paths.add(entries[i + 1].decode("utf-8", "surrogateescape"))
        i += 2
        continue
    paths.add(path)
    i += 1

for path in sorted(paths):
    if path.startswith(".git/"):
        continue
    if path.startswith("target/"):
        continue
    if path.startswith(".artifacts/"):
        continue
    if path.startswith("docs/node_modules/"):
        continue
    print(path)
PY
}

ensure_cleanroom_docs_deps() {
  if [[ $SKIP_DOCS -eq 1 || $FAST -eq 1 ]]; then
    return
  fi
  if [[ -x "$CLEANROOM/docs/node_modules/.bin/vitepress" ]]; then
    return
  fi
  if command -v pnpm >/dev/null 2>&1; then
    PNPM_CMD=(pnpm)
  elif command -v corepack >/dev/null 2>&1; then
    PNPM_CMD=(corepack pnpm)
  else
    echo "pnpm is required to bootstrap docs dependencies in the clean worktree" >&2
    echo "install pnpm, or provide corepack, or rerun with --skip-docs" >&2
    exit 1
  fi
  (
    cd "$CLEANROOM/docs"
    "${PNPM_CMD[@]}" install --frozen-lockfile
  )
}

run_step() {
  local label="$1"
  shift
  echo
  echo "== $label =="
  (
    cd "$CLEANROOM"
    "$@"
  )
}

ensure_runtime_support_files() {
  local rel src dst
  local -a required=(
    "scripts/contributing_audit.pl"
  )
  if [[ $SKIP_FUZZ -eq 0 && $FAST -eq 0 ]]; then
    required+=("scripts/fuzz_smoke.sh")
  fi
  for rel in "${required[@]}"; do
    src="$ROOT/$rel"
    dst="$CLEANROOM/$rel"
    if [[ ! -e "$src" || -e "$dst" ]]; then
      continue
    fi
    mkdir -p "$(dirname "$dst")"
    cp -p "$src" "$dst"
  done
}

declare -a SCOPE=()
while IFS= read -r line; do
  [[ -n "$line" ]] || continue
  SCOPE+=("$line")
done < <(collect_scope_paths)

if [[ ${#SCOPE[@]} -eq 0 ]]; then
  echo "no changed files detected; nothing to verify in cleanroom" >&2
  exit 1
fi

if [[ -n "$WORKTREE_DIR" ]]; then
  CLEANROOM="$WORKTREE_DIR"
  CLEANROOM_CREATED=0
else
  CLEANROOM="$(mktemp -d "${TMPDIR:-/tmp}/rr-cleanroom.XXXXXX")"
  CLEANROOM_CREATED=1
fi

cleanup() {
  if [[ $KEEP_WORKTREE -eq 1 ]]; then
    echo
    echo "clean worktree kept at: $CLEANROOM"
    return
  fi
  if [[ ${CLEANROOM_CREATED:-0} -eq 1 ]]; then
    git -C "$ROOT" worktree remove --force "$CLEANROOM" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

if [[ $CLEANROOM_CREATED -eq 1 ]]; then
  git -C "$ROOT" worktree add --detach "$CLEANROOM" HEAD >/dev/null
else
  if [[ -e "$CLEANROOM" ]]; then
    echo "--worktree-dir must point to a path that does not already exist: $CLEANROOM" >&2
    exit 2
  fi
  git -C "$ROOT" worktree add --detach "$CLEANROOM" HEAD >/dev/null
fi

echo "== RR Cleanroom Verify =="
echo "source: $ROOT"
echo "clean:  $CLEANROOM"
echo "scope:  ${#SCOPE[@]} file(s)"

for rel in "${SCOPE[@]}"; do
  src="$ROOT/$rel"
  dst="$CLEANROOM/$rel"
  if [[ -e "$src" ]]; then
    mkdir -p "$(dirname "$dst")"
    cp -p "$src" "$dst"
  else
    rm -rf "$dst"
  fi
done

ensure_cleanroom_docs_deps

run_step "Format Check" cargo fmt --all --check
run_step "Cargo Check" cargo check
run_step "Clippy" cargo clippy --all-targets -- -D warnings
run_step "Generated Contributing Docs" python3 scripts/render_contributing_docs.py --check
if [[ $FAST -eq 0 ]]; then
  run_step "Tests" cargo test -q --no-fail-fast
  run_step "Pass Verify" env RR_VERIFY_EACH_PASS=1 cargo test -q --test pass_verify_examples
fi
ensure_runtime_support_files
run_step "Contributing Audit" perl scripts/contributing_audit.pl --all --scan-only

if [[ $SKIP_FUZZ -eq 0 && $FAST -eq 0 ]]; then
  run_step "Fuzz Smoke" env FUZZ_SECONDS=1 ./scripts/fuzz_smoke.sh
fi

if [[ $SKIP_DOCS -eq 0 && $FAST -eq 0 ]]; then
  if [[ ${#PNPM_CMD[@]} -eq 0 ]]; then
    if command -v pnpm >/dev/null 2>&1; then
      PNPM_CMD=(pnpm)
    elif command -v corepack >/dev/null 2>&1; then
      PNPM_CMD=(corepack pnpm)
    fi
  fi
  run_step "Docs Build" bash -lc "cd docs && ${PNPM_CMD[*]} docs:build"
fi

echo
echo "cleanroom verification passed"
