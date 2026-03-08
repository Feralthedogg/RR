#!/usr/bin/env bash

triage_sanitize_name() {
  local s="$1"
  s="${s//\//_}"
  s="${s//:/_}"
  s="${s//-/_}"
  printf '%s' "$s"
}

triage_rust_test_name() {
  local s="$1"
  s="$(triage_sanitize_name "$s")"
  s="$(printf '%s' "$s" | tr -cd '[:alnum:]_')"
  printf '%s' "$s"
}

triage_write_empty_reports() {
  local summary_path="$1"
  local job_summary_path="$2"
  local title="$3"
  local message="$4"
  cat > "$summary_path" <<MD
# $title

$message
MD
  cat > "$job_summary_path" <<MD
# $title

- $message
MD
}

triage_write_empty_json_report() {
  local json_path="$1"
  local kind="$2"
  local message="$3"
  cat > "$json_path" <<JSON
{
  "schema": "rr-triage-report",
  "version": 1,
  "kind": "$kind",
  "message": "$message",
  "cases": []
}
JSON
}

triage_read_manifest_field() {
  local manifest="$1"
  local key="$2"
  awk -F': ' -v key="$key" '$1 == key {print $2; exit}' "$manifest"
}

triage_require_manifest_fields() {
  local manifest="$1"
  shift
  local missing=()
  local key
  for key in "$@"; do
    if [[ -z "$(triage_read_manifest_field "$manifest" "$key")" ]]; then
      missing+=("$key")
    fi
  done
  if [[ "${#missing[@]}" -gt 0 ]]; then
    echo "manifest missing required field(s) in $manifest: ${missing[*]}" >&2
    return 1
  fi
}

triage_require_manifest_kind() {
  local manifest="$1"
  local expected_kind="$2"
  triage_require_manifest_fields "$manifest" kind || return 1
  local actual_kind
  actual_kind="$(triage_read_manifest_field "$manifest" kind)"
  if [[ "$actual_kind" != "$expected_kind" ]]; then
    echo "manifest kind mismatch in $manifest: expected '$expected_kind', got '$actual_kind'" >&2
    return 1
  fi
}

triage_require_manifest_contract() {
  local manifest="$1"
  local expected_kind="$2"
  triage_require_manifest_fields "$manifest" schema version kind || return 1
  local schema version
  schema="$(triage_read_manifest_field "$manifest" schema)"
  version="$(triage_read_manifest_field "$manifest" version)"
  if [[ "$schema" != "rr-triage-bundle" ]]; then
    echo "manifest schema mismatch in $manifest: expected 'rr-triage-bundle', got '$schema'" >&2
    return 1
  fi
  if [[ "$version" != "1" ]]; then
    echo "manifest version mismatch in $manifest: expected '1', got '$version'" >&2
    return 1
  fi
  triage_require_manifest_kind "$manifest" "$expected_kind"
}
