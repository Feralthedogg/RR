#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${1:-$ROOT/.artifacts/recommended-package-coverage}"
mkdir -p "$OUT_DIR"

PACKAGE_LIST="${RR_RECOMMENDED_PACKAGES:-boot,class,cluster,codetools,foreign,KernSmooth,lattice,MASS,Matrix,mgcv,nlme,nnet,rpart,spatial,survival}"
JSON_OUT="$OUT_DIR/recommended-package-coverage.json"
MD_OUT="$OUT_DIR/recommended-package-coverage.md"

python3 - "$ROOT" "$PACKAGE_LIST" "$JSON_OUT" "$MD_OUT" <<'PY'
import json
import re
import subprocess
import sys
from pathlib import Path

root = Path(sys.argv[1])
packages = [pkg.strip() for pkg in sys.argv[2].split(",") if pkg.strip()]
json_out = Path(sys.argv[3])
md_out = Path(sys.argv[4])

quoted_re = re.compile(r'"([A-Za-z0-9_.]+::[A-Za-z0-9_.]+)"')
surface_file = root / "src" / "mir" / "semantics" / "call_model_package_surface.rs"
surface_tree = root / "src" / "mir" / "semantics" / "call_model_package_surface"
surface_body = surface_file.read_text()

quoted_calls = []
for path in sorted(surface_tree.rglob("*.rs")):
    quoted_calls.extend(m.group(1) for m in quoted_re.finditer(path.read_text()))

results = []
for package in packages:
    check = subprocess.run(
        ["R", "--slave", "-e", f"quit(status = if (requireNamespace('{package}', quietly=TRUE)) 0 else 1)"],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    installed = check.returncode == 0
    regex_safe_exports = []
    if installed:
        exports = subprocess.run(
            ["R", "--slave", "-e", f"cat(getNamespaceExports('{package}'), sep='\\n')"],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        if exports.returncode == 0:
            regex_safe_exports = sorted(
                f"{package}::{line.strip()}"
                for line in exports.stdout.splitlines()
                if re.fullmatch(r"[A-Za-z0-9_.]+", line.strip())
            )
    direct_calls = sorted(name for name in quoted_calls if name.startswith(f"{package}::"))
    direct_set = set(direct_calls)
    export_set = set(regex_safe_exports)
    missing = sorted(export_set - direct_set)
    has_prefix_fallback = f'name.starts_with("{package}::")' in surface_body
    if not installed:
        status = "unavailable"
    elif has_prefix_fallback:
        status = "prefix-fallback"
    elif not regex_safe_exports:
        status = "no-regex-safe-exports"
    elif len(missing) == 0:
        status = "closed"
    elif direct_calls:
        status = "partial"
    else:
        status = "none"
    results.append(
        {
            "package": package,
            "installed": installed,
            "status": status,
            "regex_safe_export_count": len(regex_safe_exports),
            "direct_surface_count": len(direct_calls),
            "has_prefix_fallback": has_prefix_fallback,
            "missing_count": len(missing),
            "sample_missing": missing[:12],
        }
    )

summary = {
    "schema": "rr-recommended-package-coverage",
    "version": 1,
    "packages": results,
    "totals": {
        "package_count": len(results),
        "installed_count": sum(1 for row in results if row["installed"]),
        "closed_count": sum(1 for row in results if row["status"] == "closed"),
        "partial_count": sum(1 for row in results if row["status"] == "partial"),
        "none_count": sum(1 for row in results if row["status"] == "none"),
    },
}

json_out.write_text(json.dumps(summary, indent=2) + "\n")

lines = [
    "# Recommended Package Coverage",
    "",
    f"- package count: `{summary['totals']['package_count']}`",
    f"- installed: `{summary['totals']['installed_count']}`",
    f"- closed: `{summary['totals']['closed_count']}`",
    f"- partial: `{summary['totals']['partial_count']}`",
    f"- none: `{summary['totals']['none_count']}`",
    "",
    "| package | installed | status | regex-safe exports | direct surface | missing |",
    "| --- | --- | --- | ---: | ---: | ---: |",
]
for row in results:
    lines.append(
        f"| `{row['package']}` | `{str(row['installed']).lower()}` | `{row['status']}` | `{row['regex_safe_export_count']}` | `{row['direct_surface_count']}` | `{row['missing_count']}` |"
    )
    if row["sample_missing"]:
        lines.append(f"|  |  | sample missing |  |  | `{', '.join(row['sample_missing'])}` |")

md_out.write_text("\n".join(lines) + "\n")
PY

echo "[ok] wrote $JSON_OUT"
echo "[ok] wrote $MD_OUT"
