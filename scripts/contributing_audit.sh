#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

SCAN_ONLY=0
ALL_FILES=0
SKIP_FUZZ=0
REQUIRE_FUZZ=0
BASE_REF=""
declare -a FILES=()

usage() {
  cat <<'EOF'
Usage: scripts/contributing_audit.sh [options]

Options:
  --scan-only       Skip cargo/fuzz commands and run static audit only.
  --all             Scan all repo files covered by CONTRIBUTING.md.
  --base <ref>      Scan files changed from the given git base ref.
  --files <paths>   Scan the explicit file list that follows.
  --skip-fuzz       Skip fuzz smoke even if cargo-fuzz is installed.
  --require-fuzz    Fail if fuzz smoke cannot be executed.
  --help            Show this help.

Default behavior:
  - scan changed files in the current worktree
  - run cargo check / clippy / test
  - run fuzz smoke when cargo-fuzz is available
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scan-only)
      SCAN_ONLY=1
      shift
      ;;
    --all)
      ALL_FILES=1
      shift
      ;;
    --base)
      BASE_REF="${2:-}"
      if [[ -z "$BASE_REF" ]]; then
        echo "missing value for --base" >&2
        exit 2
      fi
      shift 2
      ;;
    --files)
      shift
      while [[ $# -gt 0 && "$1" != --* ]]; do
        FILES+=("$1")
        shift
      done
      ;;
    --skip-fuzz)
      SKIP_FUZZ=1
      shift
      ;;
    --require-fuzz)
      REQUIRE_FUZZ=1
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

if [[ $ALL_FILES -eq 1 && ${#FILES[@]} -gt 0 ]]; then
  echo "--all and --files cannot be used together" >&2
  exit 2
fi

collect_all_files() {
  (
    cd "$ROOT"
    printf 'CONTRIBUTING.md\n'
    rg --files src tests docs scripts
  ) | sort -u
}

collect_changed_files() {
  if ! git -C "$ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    return 0
  fi

  {
    if [[ -n "$BASE_REF" ]]; then
      git -C "$ROOT" diff --name-only "${BASE_REF}..." --
    fi
    git -C "$ROOT" diff --name-only HEAD --
    git -C "$ROOT" diff --cached --name-only --
    git -C "$ROOT" ls-files --others --exclude-standard
  } | awk '
    /^src\// || /^tests\// || /^docs\// || /^scripts\// || /^CONTRIBUTING\.md$/ {
      print
    }
  ' | sort -u
}

declare -a SCAN_FILES=()
if [[ ${#FILES[@]} -gt 0 ]]; then
  SCAN_FILES=("${FILES[@]}")
elif [[ $ALL_FILES -eq 1 ]]; then
  while IFS= read -r line; do
    SCAN_FILES+=("$line")
  done < <(collect_all_files)
else
  while IFS= read -r line; do
    SCAN_FILES+=("$line")
  done < <(collect_changed_files)
fi

TMP_FILE_LIST="$(mktemp "${TMPDIR:-/tmp}/rr-contributing-audit-files.XXXXXX")"
trap 'rm -f "$TMP_FILE_LIST"' EXIT
printf '%s\n' "${SCAN_FILES[@]}" >"$TMP_FILE_LIST"

echo "== RR Contributing Audit =="
if [[ ${#SCAN_FILES[@]} -eq 0 ]]; then
  echo "scope: no changed files detected"
else
  echo "scope: ${#SCAN_FILES[@]} file(s)"
fi

python3 - "$ROOT" "$TMP_FILE_LIST" <<'PY'
import re
import sys
from pathlib import Path

root = Path(sys.argv[1]).resolve()
list_path = Path(sys.argv[2])

panic_re = re.compile(r"\bpanic!\s*\(")
unwrap_re = re.compile(r"\.\s*unwrap(?:_err)?\s*\(")
# Match unwrap-like .expect(...) calls on Result/Option-style values without
# flagging project-local helper methods such as Parser::expect(TokenKind::...).
expect_re = re.compile(r'\.\s*expect\s*\(\s*(?:"|r#*"|format!\s*\(|&format!\s*\()')
unsafe_re = re.compile(r"\bunsafe\b")
inline_always_re = re.compile(r"#\s*\[\s*inline\s*\(\s*always\s*\)\s*\]")
for_each_re = re.compile(r"\.\s*(?:for_each|try_for_each)\s*\(")

interesting_doc_prefixes = (
    "src/codegen/mir_emit.rs",
    "src/mir/opt.rs",
    "src/mir/opt/",
    "src/runtime/",
    "src/compiler/pipeline.rs",
    "src/compiler/incremental.rs",
    "src/main.rs",
)
interesting_test_prefixes = (
    "src/codegen/mir_emit.rs",
    "src/mir/opt.rs",
    "src/mir/opt/",
)
doc_prefixes = ("docs/",)
test_prefixes = ("tests/",)


def display_path(path: Path) -> str:
    try:
        return str(path.resolve().relative_to(root))
    except ValueError:
        return str(path)


def has_path_segment(path: str, segment: str) -> bool:
    normalized = path.replace("\\", "/")
    return normalized.startswith(f"{segment}/") or f"/{segment}/" in normalized


def load_paths() -> list[Path]:
    paths = []
    with list_path.open("r", encoding="utf-8") as fh:
        for raw in fh:
            value = raw.strip()
            if not value:
                continue
            path = Path(value)
            if not path.is_absolute():
                path = root / path
            paths.append(path)
    return paths


def production_lines(lines: list[str]) -> list[str]:
    for idx, line in enumerate(lines):
        if re.match(r"\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", line):
            return lines[:idx]
    return lines


def is_comment(line: str) -> bool:
    return line.lstrip().startswith("//")


def has_allow_marker(line: str) -> bool:
    return "audit: allow" in line


def has_safety_comment(lines: list[str], idx: int) -> bool:
    start = max(0, idx - 3)
    for prev in range(idx - 1, start - 1, -1):
        text = lines[prev].strip()
        if not text:
            continue
        if text.startswith("// SAFETY:"):
            return True
        if not text.startswith("//"):
            return False
    return False


paths = load_paths()
display_paths = [display_path(path) for path in paths]

errors: list[tuple[str, str, int, str]] = []
warnings: list[tuple[str, str, int, str]] = []
manual: list[str] = []

docs_touched = any(path.startswith(doc_prefixes) or has_path_segment(path, "docs") for path in display_paths)
tests_touched = any(path.startswith(test_prefixes) or has_path_segment(path, "tests") for path in display_paths)
interesting_docs_changed = any(path.startswith(interesting_doc_prefixes) for path in display_paths)
interesting_tests_changed = any(path.startswith(interesting_test_prefixes) for path in display_paths)

for path, shown in zip(paths, display_paths):
    if not path.exists() or path.is_dir():
        continue

    try:
        raw_text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        warnings.append(("binary-skip", shown, 0, "skipped non-UTF-8 file during static audit"))
        continue

    all_lines = raw_text.splitlines()
    prod_lines = production_lines(all_lines)
    is_production_src = has_path_segment(shown, "src") and "/src/legacy/" not in shown.replace("\\", "/")
    is_rust_file = shown.endswith(".rs")

    if is_production_src:
        for line_no, line in enumerate(prod_lines, 1):
            stripped = line.strip()
            if not stripped or is_comment(line):
                continue
            if has_allow_marker(line):
                continue
            if panic_re.search(line):
                errors.append(
                    (
                        "production-panic",
                        shown,
                        line_no,
                        "production compiler path contains panic!; use RRException/ICE flow instead",
                    )
                )
            if unwrap_re.search(line) or expect_re.search(line):
                errors.append(
                    (
                        "production-unwrap",
                        shown,
                        line_no,
                        "production compiler path contains unwrap()/expect()",
                    )
                )
            if unsafe_re.search(line):
                if not has_safety_comment(prod_lines, line_no - 1):
                    errors.append(
                        (
                            "unsafe-missing-safety",
                            shown,
                            line_no,
                            "unsafe usage missing adjacent // SAFETY: rationale",
                        )
                    )

    for line_no, line in enumerate(all_lines, 1):
        stripped = line.strip()
        if not stripped or is_comment(line):
            continue
        if has_allow_marker(line):
            continue
        if is_rust_file and inline_always_re.search(line):
            errors.append(
                (
                    "inline-always",
                    shown,
                    line_no,
                    "#[inline(always)] requires benchmark-backed justification",
                )
            )
        if is_production_src and for_each_re.search(line):
            warnings.append(
                (
                    "for-each-review",
                    shown,
                    line_no,
                    "review chained for_each/try_for_each for hidden side effects",
                )
            )

if interesting_docs_changed and not docs_touched:
    warnings.append(
        (
            "docs-review",
            "(scope)",
            0,
            "runtime/optimizer/codegen surface changed without docs/** updates in the audit scope",
        )
    )

if interesting_tests_changed and not tests_touched:
    warnings.append(
        (
            "tests-review",
            "(scope)",
            0,
            "optimizer/codegen files changed without tests/** updates in the audit scope",
        )
    )

manual.extend(
    [
        "review deterministic traversal/output when iterating hash-based collections",
        "review hot loops for avoidable allocation, clone(), or heavyweight formatting",
        "confirm targeted tests cover emitted R shape/runtime behavior for touched subsystems",
    ]
)

print("== static rule scan ==")
print(f"files scanned: {len(paths)}")
for code, shown, line_no, message in errors:
    location = shown if line_no == 0 else f"{shown}:{line_no}"
    print(f"error[{code}] {location}: {message}")
for code, shown, line_no, message in warnings:
    location = shown if line_no == 0 else f"{shown}:{line_no}"
    print(f"warn[{code}] {location}: {message}")
if not errors and not warnings:
    print("no static findings")
print("manual follow-up:")
for item in manual:
    print(f"  - {item}")

if errors:
    sys.exit(1)
PY

if [[ $SCAN_ONLY -eq 1 ]]; then
  echo "result: PASS (scan-only)"
  exit 0
fi

run_cmd() {
  local label="$1"
  shift
  echo "== $label =="
  "$@"
}

run_cmd "cargo check" cargo check
run_cmd "cargo clippy --all-targets -- -D warnings" cargo clippy --all-targets -- -D warnings
run_cmd "cargo test -q" cargo test -q

if [[ $SKIP_FUZZ -eq 1 ]]; then
  echo "== fuzz smoke =="
  echo "skip: requested by --skip-fuzz"
else
  TOOLCHAIN="${RUSTUP_TOOLCHAIN:-nightly}"
  if cargo +"$TOOLCHAIN" fuzz --help >/dev/null 2>&1; then
    run_cmd "FUZZ_SECONDS=${FUZZ_SECONDS:-1} ./scripts/fuzz_smoke.sh" env "FUZZ_SECONDS=${FUZZ_SECONDS:-1}" ./scripts/fuzz_smoke.sh
  elif [[ $REQUIRE_FUZZ -eq 1 ]]; then
    echo "== fuzz smoke ==" >&2
    echo "fail: cargo-fuzz unavailable for toolchain '$TOOLCHAIN'" >&2
    exit 1
  else
    echo "== fuzz smoke =="
    echo "skip: cargo-fuzz unavailable for toolchain '$TOOLCHAIN' (use --require-fuzz to fail)"
  fi
fi

echo "result: PASS"
