#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "usage: $0 <triage|promote|smoke> <fuzz|differential|pass-verify> [args...]" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ACTION="$1"
RAW_KIND="$2"
shift 2

normalize_kind() {
  case "$1" in
    fuzz)
      printf 'fuzz'
      ;;
    differential)
      printf 'differential'
      ;;
    pass-verify | pass_verify | passverify)
      printf 'pass-verify'
      ;;
    *)
      return 1
      ;;
  esac
}

KIND="$(normalize_kind "$RAW_KIND" || true)"
if [[ -z "$KIND" ]]; then
  echo "unsupported triage kind: $RAW_KIND" >&2
  exit 1
fi

case "$ACTION:$KIND" in
  triage:fuzz)
    exec "$ROOT/scripts/fuzz_triage.sh" "$@"
    ;;
  triage:differential)
    exec "$ROOT/scripts/differential_triage.sh" "$@"
    ;;
  triage:pass-verify)
    exec "$ROOT/scripts/pass_verify_triage.sh" "$@"
    ;;
  promote:fuzz)
    exec "$ROOT/scripts/promote_fuzz_regression.sh" "$@"
    ;;
  promote:differential)
    exec "$ROOT/scripts/promote_differential_regression.sh" "$@"
    ;;
  promote:pass-verify)
    exec "$ROOT/scripts/promote_pass_verify_regression.sh" "$@"
    ;;
  smoke:fuzz | smoke:differential | smoke:pass-verify)
    exec "$ROOT/scripts/smoke_triage_regressions.sh" "$KIND" "$@"
    ;;
  *)
    echo "unsupported action/kind combination: $ACTION $KIND" >&2
    exit 1
    ;;
esac
