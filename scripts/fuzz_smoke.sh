#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DICT="${FUZZ_DICT:-$ROOT/fuzz/dictionaries/rr.dict}"
SECS="${FUZZ_SECONDS:-20}"
TOOLCHAIN="${RUSTUP_TOOLCHAIN:-nightly}"
CORPUS_ROOT="${FUZZ_CORPUS_ROOT:-$ROOT/fuzz/corpus_smoke}"
TARGETS=(parser pipeline type_solver incremental_compile)

if ! cargo +"$TOOLCHAIN" fuzz --help >/dev/null 2>&1; then
  echo "cargo-fuzz is not installed for toolchain '$TOOLCHAIN'." >&2
  echo "Install with: cargo install cargo-fuzz --locked" >&2
  exit 1
fi

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/rr-fuzz-smoke.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

for target in "${TARGETS[@]}"; do
  corpus="$CORPUS_ROOT/$target"
  if [[ ! -d "$corpus" ]]; then
    echo "missing corpus directory: $corpus" >&2
    exit 1
  fi
  scratch="$TMP_ROOT/$target"
  mkdir -p "$scratch"
  cp -R "$corpus/." "$scratch/"
  echo "== fuzz smoke: $target =="
  RR_QUIET_LOG=1 cargo +"$TOOLCHAIN" fuzz run "$target" "$scratch" -- \
    -dict="$DICT" \
    -max_total_time="$SECS" \
    -rss_limit_mb=2048 \
    -verbosity=0 \
    -print_final_stats=1
  echo
done
