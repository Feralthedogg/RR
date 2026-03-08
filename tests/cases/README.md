# File-Based Regression Cases

Each case lives under:

- `tests/cases/<category>/<case-name>/main.rr`
- `tests/cases/<category>/<case-name>/case.meta`

Supported `case.meta` directives:

- `expect=compile-ok`
- `expect=parse-error`
- `expect=semantic-error`
- `expect=type-error`
- `expect=run-equal-o0-o2`
- `flag=<cli-arg>`
- `env=KEY=VALUE`
- `stdout_contains=<substring>`
- `stdout_not_contains=<substring>`
- `stderr_contains=<substring>`
- `stderr_not_contains=<substring>`
- `emit_contains=<substring>`
- `emit_not_contains=<substring>`

Notes:

- Repeat `flag=...` or `*_contains=...` lines as needed.
- `compile-ok` cases compile once and can assert on compile output and emitted R.
- `run-equal-o0-o2` cases compile twice (`-O0` and `-O2`) and compare `Rscript`
  exit code, stdout, and stderr. Any `stdout_*`, `stderr_*`, or `emit_*`
  assertions are checked against the `-O2` compile.
- Keep cases small and bug-focused. One regression should usually map to one case.
