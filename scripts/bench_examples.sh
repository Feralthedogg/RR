#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RR_BIN="${RR_BIN:-$ROOT/target/release/RR}"
OUT_DIR="${OUT_DIR:-$ROOT/target/bench_examples}"
LEVELS=("-O0" "-O2")
WORKLOADS=(
  "vector_fusion_bench"
  "bootstrap_resample_bench"
  "heat_diffusion_bench"
  "orbital_sweep_bench"
  "reaction_diffusion_bench"
)

mkdir -p "$OUT_DIR"

if [[ ! -x "$RR_BIN" ]]; then
  cargo build --release --bin RR --manifest-path "$ROOT/Cargo.toml"
fi

compile_cmd() {
  local stem="$1"
  local level="$2"
  local src="$ROOT/example/benchmarks/${stem}.rr"
  local tag="${level#-}"
  local out="$OUT_DIR/${stem}_${tag}.R"
  printf '%q ' "$RR_BIN" "$src" -o "$out" --no-runtime "$level"
}

run_cmd() {
  local stem="$1"
  local level="$2"
  local tag="${level#-}"
  local out="$OUT_DIR/${stem}_${tag}.R"
  printf '%q ' Rscript --vanilla "$out"
}

bench_one() {
  local stem="$1"
  echo "== $stem =="
  if command -v hyperfine >/dev/null 2>&1; then
    local cmds=()
    for level in "${LEVELS[@]}"; do
      cmds+=("$(compile_cmd "$stem" "$level")")
    done
    hyperfine --warmup 1 --runs 5 "${cmds[@]}"
    if command -v Rscript >/dev/null 2>&1; then
      for level in "${LEVELS[@]}"; do
        eval "$(compile_cmd "$stem" "$level")" >/dev/null 2>&1
      done
      local run_cmds=()
      for level in "${LEVELS[@]}"; do
        run_cmds+=("$(run_cmd "$stem" "$level")")
      done
      hyperfine --warmup 1 --runs 5 "${run_cmds[@]}"
    fi
  else
    for level in "${LEVELS[@]}"; do
      echo "-- compile $stem $level"
      time bash -lc "$(compile_cmd "$stem" "$level")"
    done
    if command -v Rscript >/dev/null 2>&1; then
      for level in "${LEVELS[@]}"; do
        echo "-- run $stem $level"
        time bash -lc "$(run_cmd "$stem" "$level") >/dev/null"
      done
    fi
  fi
}

for stem in "${WORKLOADS[@]}"; do
  bench_one "$stem"
  echo
done
