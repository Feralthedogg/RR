#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

SCAN_ONLY=0
ALL_FILES=0
SKIP_FUZZ=0
REQUIRE_FUZZ=0
SKIP_PASS_VERIFY=0
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
  --skip-pass-verify
                    Skip pass-verify smoke even when pass-sensitive files are in scope.
  --skip-fuzz       Skip fuzz smoke even if cargo-fuzz is installed.
  --require-fuzz    Fail if fuzz smoke cannot be executed.
  --help            Show this help.

Default behavior:
  - scan changed files in the current worktree
  - run cargo check / clippy / test
  - run RR_VERIFY_EACH_PASS smoke when pass-sensitive compiler files are in scope
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
    --skip-pass-verify)
      SKIP_PASS_VERIFY=1
      shift
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
  if git -C "$ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    (
      cd "$ROOT"
      git ls-files -- CONTRIBUTING.md src tests docs scripts fuzz native
    ) | sort -u
    return
  fi

  (
    cd "$ROOT"
    printf 'CONTRIBUTING.md\n'
    rg --files src tests docs scripts fuzz native
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
    /^src\// || /^tests\// || /^docs\// || /^scripts\// || /^fuzz\// || /^native\// || /^CONTRIBUTING\.md$/ {
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

scope_needs_pass_verify() {
  local rel
  for rel in "${SCAN_FILES[@]}"; do
    case "$rel" in
      src/hir/*|src/mir/*|src/legacy/ir/*|src/codegen/mir_emit.rs|src/compiler/pipeline.rs|src/compiler/incremental.rs|tests/pass_verify_examples.rs)
        return 0
        ;;
    esac
  done
  return 1
}

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
dbg_re = re.compile(r"\bdbg!\s*\(")
inline_always_re = re.compile(r"#\s*\[\s*inline\s*\(\s*always\s*\)\s*\]")
for_each_re = re.compile(r"\.\s*(?:for_each|try_for_each)\s*\(")
heading_re = re.compile(r"^(#{2,3})\s+(.+?)\s*$")
structured_todo_re = re.compile(r"^TODO\([^)]+\):\s+\S")
structured_fixme_re = re.compile(r"^FIXME:\s+\S")
structured_note_re = re.compile(r"^NOTE:\s+\S")

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
    "src/hir/",
    "src/mir/",
    "src/legacy/ir/",
    "src/codegen/mir_emit.rs",
    "src/mir/opt.rs",
    "src/mir/opt/",
    "src/compiler/pipeline.rs",
    "src/compiler/incremental.rs",
)
pass_sensitive_prefixes = (
    "src/hir/",
    "src/mir/",
    "src/legacy/ir/",
    "src/codegen/mir_emit.rs",
    "src/compiler/pipeline.rs",
    "src/compiler/incremental.rs",
)
cache_sensitive_prefixes = (
    "src/compiler/incremental.rs",
    "src/compiler/pipeline.rs",
    "docs/compiler/pipeline.md",
    "docs/cli.md",
)
doc_prefixes = ("docs/",)
test_prefixes = ("tests/",)
known_roots = ("src", "tests", "docs", "scripts", "fuzz", "native")
safe_alt_markers = (
    "safe alternative",
    "safe alternatives",
    "safe rust",
    "cannot be expressed safely",
    "cannot express safely",
    "cannot do this safely",
    "cannot use safe",
    "ffi",
    "raw pointer",
    "aliasing",
    "layout",
    "provenance",
)
required_contributing_sections = (
    "Scope",
    "Core Principles",
    "Rule Levels",
    "Rules",
    "Exception Process",
    "PR Checklist",
)
required_contributing_rule_topics = (
    "Deterministic Output and Traversal",
    "Error Model (User Error vs Compiler Fault)",
    "Testing and Validation Requirements",
    "Unsafe Code Policy",
    "Commenting Rules",
)
required_testing_phrases = (
    "`cargo check`",
    "`cargo clippy --all-targets -- -D warnings`",
    "at least one minimal targeted test",
)
required_unsafe_phrases = (
    "// SAFETY:",
    "safe alternatives",
)
required_pr_checklist_phrases = (
    "Behavior is deterministic",
    "Relevant tests and checks were executed",
    "Docs updated if CLI/runtime/error semantics changed.",
)


def display_path(path: Path) -> str:
    try:
        return str(path.resolve().relative_to(root))
    except ValueError:
        return str(path)


def has_path_segment(path: str, segment: str) -> bool:
    normalized = path.replace("\\", "/")
    return normalized.startswith(f"{segment}/") or f"/{segment}/" in normalized


def path_matches_prefix(path: str, prefix: str) -> bool:
    normalized = path.replace("\\", "/").lstrip("./").rstrip("/")
    prefix = prefix.replace("\\", "/").rstrip("/")
    return (
        normalized == prefix
        or normalized.startswith(prefix)
        or normalized.endswith("/" + prefix)
        or f"/{prefix}/" in normalized
    )


def audit_key(path: str) -> str:
    normalized = path.replace("\\", "/").rstrip("/")
    parts = [part for part in normalized.split("/") if part]
    if parts and parts[-1] == "CONTRIBUTING.md":
        return "CONTRIBUTING.md"
    for idx in range(len(parts) - 2, -1, -1):
        if parts[idx] in known_roots:
            return "/".join(parts[idx:])
    return normalized.lstrip("./")


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


def has_safe_alt_comment(lines: list[str], idx: int) -> bool:
    start = max(0, idx - 5)
    for prev in range(idx - 1, start - 1, -1):
        text = lines[prev].strip()
        if not text:
            continue
        if text.startswith("//"):
            lower = text.lower()
            if any(marker in lower for marker in safe_alt_markers):
                return True
            continue
        return False
    return False


def actionable_comment_error(line: str) -> tuple[str, str] | None:
    stripped = line.lstrip()
    if not stripped.startswith("//"):
        return None
    comment = stripped[2:].lstrip()
    if comment.startswith("TODO") and not structured_todo_re.match(comment):
        return (
            "comment-prefix",
            "TODO comments must use // TODO(name/issue): rationale",
        )
    if comment.startswith("FIXME") and not structured_fixme_re.match(comment):
        return (
            "comment-prefix",
            "FIXME comments must use // FIXME: rationale",
        )
    if comment.startswith("NOTE") and not structured_note_re.match(comment):
        return (
            "comment-prefix",
            "NOTE comments must use // NOTE: rationale",
        )
    if comment.startswith("XXX"):
        return (
            "comment-prefix",
            "replace // XXX with a structured // FIXME: or // NOTE: comment",
        )
    return None


def collect_headings(lines: list[str]) -> list[tuple[int, int, str]]:
    headings: list[tuple[int, int, str]] = []
    for line_no, line in enumerate(lines, 1):
        match = heading_re.match(line.strip())
        if not match:
            continue
        headings.append((line_no, len(match.group(1)), match.group(2).strip()))
    return headings


def extract_section_body(
    lines: list[str],
    headings: list[tuple[int, int, str]],
    wanted_title: str,
) -> tuple[int, str] | None:
    for idx, (line_no, level, title) in enumerate(headings):
        if title != wanted_title:
            continue
        end_line = len(lines) + 1
        for next_line_no, next_level, _ in headings[idx + 1 :]:
            if next_level <= level:
                end_line = next_line_no
                break
        body = "\n".join(lines[line_no:end_line - 1]).strip()
        return (line_no, body)
    return None


def audit_contributing_doc(
    shown: str,
    lines: list[str],
    errors: list[tuple[str, str, int, str]],
    warnings: list[tuple[str, str, int, str]],
) -> None:
    text = "\n".join(lines)
    headings = collect_headings(lines)
    top_sections = {title: line_no for line_no, level, title in headings if level == 2}
    rule_sections: list[tuple[int, int, str]] = []

    for title in required_contributing_sections:
        if title not in top_sections:
            errors.append(
                (
                    "contributing-section",
                    shown,
                    0,
                    f"CONTRIBUTING.md missing required section '## {title}'",
                )
            )

    for line_no, level, title in headings:
        if level != 3:
            continue
        match = re.match(r"^(\d+)\)\s+(.+)$", title)
        if not match:
            continue
        rule_sections.append((line_no, int(match.group(1)), match.group(2).strip()))

    if not rule_sections:
        errors.append(
            (
                "contributing-rules",
                shown,
                top_sections.get("Rules", 0),
                "CONTRIBUTING.md must define numbered rule sections under '## Rules'",
            )
        )
    else:
        rule_numbers = [number for _, number, _ in rule_sections]
        expected = list(range(1, max(rule_numbers) + 1))
        if rule_numbers != expected:
            errors.append(
                (
                    "contributing-rule-order",
                    shown,
                    rule_sections[0][0],
                    f"numbered rule headings must be contiguous starting at 1; found {rule_numbers}",
                )
            )

        rule_titles = [title for _, _, title in rule_sections]
        for topic in required_contributing_rule_topics:
            if topic not in rule_titles:
                errors.append(
                    (
                        "contributing-rule-topic",
                        shown,
                        top_sections.get("Rules", 0),
                        f"CONTRIBUTING.md missing rule topic '{topic}'",
                    )
                )
        for line_no, number, title in rule_sections:
            section = extract_section_body(lines, headings, f"{number}) {title}")
            if section is None:
                continue
            _, body = section
            if not re.search(r"\b(MUST|SHOULD|MAY)\b", body):
                warnings.append(
                    (
                        "contributing-rule-strength",
                        shown,
                        line_no,
                        f"rule section '{title}' has no MUST/SHOULD/MAY guidance",
                    )
                )

    testing = next((item for item in rule_sections if item[2] == "Testing and Validation Requirements"), None)
    if testing is not None:
        testing_title = f"{testing[1]}) {testing[2]}"
        testing_section = extract_section_body(lines, headings, testing_title)
        if testing_section is not None:
            testing_line, testing_body = testing_section
            for phrase in required_testing_phrases:
                if phrase not in testing_body:
                    errors.append(
                        (
                            "contributing-testing",
                            shown,
                            testing_line,
                            f"testing requirements should mention {phrase}",
                        )
                    )

    unsafe_policy = next((item for item in rule_sections if item[2] == "Unsafe Code Policy"), None)
    if unsafe_policy is not None:
        unsafe_title = f"{unsafe_policy[1]}) {unsafe_policy[2]}"
        unsafe_section = extract_section_body(lines, headings, unsafe_title)
        if unsafe_section is not None:
            unsafe_line, unsafe_body = unsafe_section
            for phrase in required_unsafe_phrases:
                if phrase not in unsafe_body:
                    errors.append(
                        (
                            "contributing-unsafe",
                            shown,
                            unsafe_line,
                            f"unsafe policy should mention {phrase}",
                        )
                    )

    checklist = extract_section_body(lines, headings, "PR Checklist")
    if checklist is not None:
        checklist_line, checklist_body = checklist
        for phrase in required_pr_checklist_phrases:
            if phrase not in checklist_body:
                errors.append(
                    (
                        "contributing-pr-checklist",
                        shown,
                        checklist_line,
                        f"PR checklist should mention '{phrase}'",
                    )
                )

    if "docs/compiler/contributing-audit.md" not in text:
        warnings.append(
            (
                "contributing-doc-link",
                shown,
                0,
                "CONTRIBUTING.md should link to docs/compiler/contributing-audit.md for the concrete verification pass",
            )
        )


paths = load_paths()
display_paths = [display_path(path) for path in paths]
audit_paths = [audit_key(path) for path in display_paths]

errors: list[tuple[str, str, int, str]] = []
warnings: list[tuple[str, str, int, str]] = []
manual: list[str] = []

docs_touched = any(path.startswith(doc_prefixes) for path in audit_paths)
tests_touched = any(path.startswith(test_prefixes) for path in audit_paths)
contributing_changed = any(path == "CONTRIBUTING.md" for path in audit_paths)
contributing_audit_doc_changed = any(
    path_matches_prefix(path, "docs/compiler/contributing-audit.md")
    for path in audit_paths
)
interesting_docs_changed = any(
    any(path_matches_prefix(path, prefix) for prefix in interesting_doc_prefixes)
    for path in audit_paths
)
interesting_tests_changed = any(
    any(path_matches_prefix(path, prefix) for prefix in interesting_test_prefixes)
    for path in audit_paths
)
pass_sensitive_changed = any(
    any(path_matches_prefix(path, prefix) for prefix in pass_sensitive_prefixes)
    for path in audit_paths
)
cache_sensitive_changed = any(
    any(path_matches_prefix(path, prefix) for prefix in cache_sensitive_prefixes)
    for path in audit_paths
)


def add_manual(item: str) -> None:
    if item not in manual:
        manual.append(item)

for path, shown, audit in zip(paths, display_paths, audit_paths):
    if not path.exists() or path.is_dir():
        continue

    try:
        raw_text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        warnings.append(("binary-skip", shown, 0, "skipped non-UTF-8 file during static audit"))
        continue

    all_lines = raw_text.splitlines()
    prod_lines = production_lines(all_lines)
    is_production_src = (
        audit.startswith("src/")
        and not audit.startswith("src/legacy/")
        and not audit.endswith("/tests.rs")
    )
    is_rust_file = shown.endswith(".rs")

    if audit == "CONTRIBUTING.md":
        audit_contributing_doc(shown, all_lines, errors, warnings)

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
            if dbg_re.search(line):
                errors.append(
                    (
                        "production-dbg",
                        shown,
                        line_no,
                        "production compiler path contains dbg!(); use structured logging or diagnostics instead",
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
                elif not has_safe_alt_comment(prod_lines, line_no - 1):
                    warnings.append(
                        (
                            "unsafe-safe-alt-review",
                            shown,
                            line_no,
                            "unsafe block should explain why safe alternatives were insufficient",
                        )
                    )

    for line_no, line in enumerate(all_lines, 1):
        stripped = line.strip()
        if not stripped:
            continue
        if has_allow_marker(line):
            continue
        if is_rust_file and is_comment(line):
            comment_error = actionable_comment_error(line)
            if comment_error is not None:
                code, message = comment_error
                errors.append((code, shown, line_no, message))
            continue
        if is_comment(line):
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
            "compiler implementation files changed without tests/** updates in the audit scope",
        )
    )

if cache_sensitive_changed and not tests_touched:
    warnings.append(
        (
            "cache-tests-review",
            "(scope)",
            0,
            "cache/incremental logic changed without tests/** updates in the audit scope",
        )
    )

if contributing_changed and not contributing_audit_doc_changed:
    warnings.append(
        (
            "contributing-doc-sync",
            "(scope)",
            0,
            "CONTRIBUTING.md changed without docs/compiler/contributing-audit.md update in the audit scope",
        )
    )

add_manual("review deterministic traversal/output when iterating hash-based collections")
add_manual("review hot loops for avoidable allocation, clone(), or heavyweight formatting")
add_manual("confirm at least one minimal targeted test isolates the changed behavior or invariant")
if pass_sensitive_changed:
    add_manual("confirm pass ownership, verifier timing, and IR growth bounds for touched rewrites")
    add_manual("rerun pass-verify smoke if touched changes affect HIR/MIR/pipeline/codegen behavior")
if cache_sensitive_changed:
    add_manual("confirm cache keys capture all correctness-affecting inputs, invalidation assumptions, and an easy cache-bypass debug path")
if contributing_changed:
    add_manual("confirm docs/compiler/contributing-audit.md still matches the current CONTRIBUTING.md contract")
if any("unsafe" in code for code, *_ in errors + warnings):
    add_manual("confirm nearby unsafe comments explain both the safety contract and why safe alternatives were not sufficient")

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

if [[ $SKIP_PASS_VERIFY -eq 1 ]]; then
  echo "== pass verify =="
  echo "skip: requested by --skip-pass-verify"
elif scope_needs_pass_verify; then
  run_cmd "RR_VERIFY_EACH_PASS=1 cargo test -q --test pass_verify_examples" env RR_VERIFY_EACH_PASS=1 cargo test -q --test pass_verify_examples
else
  echo "== pass verify =="
  echo "skip: scope does not touch pass-sensitive compiler files"
fi

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
