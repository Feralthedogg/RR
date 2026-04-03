# CLI Reference

This page is the driver manual for RR.

Current compiler line: `RR Tachyon v9.0.0`.

## Audience

Read this page when you need exact driver behavior:

- accepted command forms
- flag classes
- precedence and defaults
- output and exit behavior

## Synopsis

```bash
RR -h
RR help
RR --version
RR -V
RR version
RR <input.rr> [options]
RR new [--bin|--lib] <module-path> [dir]
RR init [--bin|--lib] [module-path]
RR install <github-url|module-path>[@version]
RR remove <module-path>
RR outdated
RR update [module-path]
RR publish <version> [--dry-run] [--allow-dirty] [--push-tag] [--remote <name>] [--registry <dir>]
RR search <query> [--registry <dir>]
RR registry keygen [identity] [--out-dir <dir>]
RR registry onboard [identity] [--out-dir <dir>] [--require-signed] [--require-approval] [--auto-approve] [--registry <dir>]
RR registry list [--registry <dir>]
RR registry report [module-path] [--registry <dir>]
RR registry diff <module-path> <from-version> <to-version> [--registry <dir>]
RR registry risk <module-path> <version> [--against <version>] [--registry <dir>]
RR registry channel show <module-path> [--registry <dir>]
RR registry channel set <module-path> <channel> <version> [--registry <dir>]
RR registry channel clear <module-path> <channel> [--registry <dir>]
RR registry queue [--registry <dir>]
RR registry audit [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]
RR registry audit export <file> [--format <tsv|jsonl>] [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]
RR registry policy bootstrap <trusted-public-key> [--signer <identity>] [--auto-approve-signer <identity>] [--require-signed] [--require-approval] [--registry <dir>]
RR registry policy show [--registry <dir>]
RR registry policy lint [--registry <dir>]
RR registry policy rotate-key <old-public-key> <new-public-key> [--registry <dir>]
RR registry policy apply <file> [--registry <dir>]
RR registry info <module-path> [--registry <dir>]
RR registry approve <module-path> <version> [--registry <dir>]
RR registry unapprove <module-path> <version> [--registry <dir>]
RR registry promote <module-path> <version> [--registry <dir>]
RR registry yank <module-path> <version> [--registry <dir>]
RR registry unyank <module-path> <version> [--registry <dir>]
RR registry deprecate <module-path> <message> [--registry <dir>]
RR registry undeprecate <module-path> [--registry <dir>]
RR registry verify [module-path] [--registry <dir>]
RR mod graph
RR mod why <module-path>
RR mod verify
RR mod tidy
RR mod vendor
RR run [entry.rr|dir|.] [options]
RR build [dir|file.rr] [options]
RR watch [entry.rr|dir|.] [options]
```

During development, `cargo run -- ...` is equivalent to invoking `RR ...`.

## Command Summary

| Command | Purpose | Typical use |
| --- | --- | --- |
| `RR file.rr` | compile one file | emit one `.R` artifact |
| `RR new github.com/acme/app` | create a managed project | scaffold `src/main.rr` |
| `RR init --lib github.com/acme/lib` | initialize in place | scaffold `src/lib.rr` |
| `RR install https://github.com/acme/mathlib@latest` | fetch a dependency | update `rr.mod` and `rr.lock` |
| `RR remove github.com/acme/mathlib` | drop a direct dependency | rewrite `rr.mod` and `rr.lock` |
| `RR outdated` | check direct dependency freshness | compare `rr.mod` against remote tags |
| `RR update` | refresh direct dependencies | rewrite `rr.mod` and `rr.lock` to newer versions |
| `RR publish v1.0.0` | create a release archive | write a tarball under `Build/publish/` |
| `RR search math` | search a registry | match module paths and metadata |
| `RR registry keygen release-bot --out-dir keys/` | create a new ed25519 signing keypair | write secret/public/env helper files |
| `RR registry onboard release-bot --out-dir keys/ --registry ./registry` | initialize registry signing and policy in one step | generate keys and bootstrap policy |
| `RR registry list` | enumerate registry modules | print one summary line per module |
| `RR registry report` | summarize registry state | print totals and per-module counts |
| `RR registry diff rr.local/mathlib v1.0.0 v1.1.0` | compare two registry releases | show metadata changes and file-level diff summary |
| `RR registry risk rr.local/mathlib v1.1.0 --against v1.0.0` | estimate release risk | score metadata and file-level changes |
| `RR registry channel set rr.local/mathlib stable v1.0.0` | assign a named channel to one release | make `@stable` resolve to that version |
| `RR registry queue` | show pending approvals | list unapproved registry releases |
| `RR registry audit --limit 20` | inspect registry changes | print recent policy and release events |
| `RR registry audit export audit.jsonl --format jsonl` | export filtered audit entries | write JSONL or TSV logs |
| `RR registry policy bootstrap <pubkey>` | create a starter trust policy | trust one key and optionally require signatures, approval, and auto-approval |
| `RR registry policy show` | print canonical policy contents | inspect the effective policy file |
| `RR registry policy lint` | check a registry trust policy | validate duplicates and conflicting rules |
| `RR registry policy rotate-key <old> <new>` | rotate a trusted public key | trust the new key and revoke the old one |
| `RR registry policy apply policy.toml` | replace a registry policy from file | parse and write canonical policy |
| `RR registry info rr.local/mathlib` | inspect one registry module | print metadata and release list |
| `RR registry approve rr.local/mathlib v1.2.0` | approve one release | make it installable and visible to `@latest` |
| `RR registry unapprove rr.local/mathlib v1.2.0` | put one release back into pending state | hide it from `@latest` until reviewed again |
| `RR registry promote rr.local/mathlib v1.1.0` | make one release the active approved version | approve the target and demote the rest |
| `RR registry yank rr.local/mathlib v1.2.0` | withdraw one registry release | keep the archive but exclude it from `@latest` |
| `RR registry unyank rr.local/mathlib v1.2.0` | restore one registry release | allow `@latest` to consider it again |
| `RR registry deprecate rr.local/mathlib "use rr.local/newmath"` | mark a registry module deprecated | show the replacement message in search/info output |
| `RR registry undeprecate rr.local/mathlib` | clear a registry deprecation | remove the migration warning |
| `RR registry verify` | verify a registry store | check archives, checksums, and embedded `rr.mod` files |
| `RR mod graph` | print the resolved dependency graph | root-to-module edge list |
| `RR mod why github.com/acme/baseutil` | explain why a module is present | dependency chain from the root |
| `RR mod verify` | verify lockfile checksums | compare lock sums to replace/vendor/cache contents |
| `RR mod tidy` | sync imports and direct dependencies | add missing and remove unused direct requirements |
| `RR mod vendor` | vendor resolved dependencies | populate `vendor/` from `rr.lock` |
| `RR run .` | compile and execute entry | local project runs |
| `RR build .` | build a project entry | writes to `Build/debug/` by default |
| `RR watch .` | rebuild on changes | edit/compile loops |
| `RR --version` | print compiler line | scripts and CI |

## Command Forms

### `version`

```bash
RR --version
RR version
```

Print the compiler line and exit.

### `new`

```bash
RR new github.com/acme/app
RR new demo-app
RR new demo-app .
RR new .
RR new --lib github.com/acme/math
```

Creates a managed RR project with Cargo-like source layout:

- `src/main.rr` by default
- `src/lib.rr` with `--lib`
- module paths do not need a GitHub prefix
- `RR new .` scaffolds the current directory and infers the module path from the directory name
- `Build/` reserved for generated artifacts and caches

### `init`

```bash
RR init github.com/acme/app
RR init
RR init --lib github.com/acme/math
```

Initializes the current directory as a managed RR project.
If `[module-path]` is omitted, RR uses the current directory name.

### `install`

```bash
RR install https://github.com/acme/mathlib@latest
RR install github.com/acme/mathlib@v1.2.3
```

Fetches a GitHub-backed RR dependency into the package cache, records the exact
version in `rr.mod`, and rewrites `rr.lock` with the resolved dependency graph.
When `@latest` is used, RR now follows the module-path major-version rule:

- `github.com/acme/mathlib` resolves only `v0` and `v1` tags
- `github.com/acme/mathlib/v2` resolves only `v2` tags

Registry-backed dependencies also accept channel selectors such as
`rr.local/mathlib@stable` or `rr.local/mathlib@canary`.

### `remove`

```bash
RR remove github.com/acme/mathlib
```

Removes a direct dependency from `rr.mod` and refreshes `rr.lock`.

### `outdated`

```bash
RR outdated
```

Checks each direct dependency in `rr.mod` against the latest compatible remote
tag and reports whether it is up to date.
Dependencies pinned through `replace` are reported as `replaced`.

### `update`

```bash
RR update
RR update github.com/acme/mathlib
```

Updates all direct dependencies, or one selected direct dependency, to the
latest compatible remote tag and refreshes `rr.lock`.

### `publish`

```bash
RR publish v1.0.0
RR publish v1.0.0 --dry-run
RR publish v1.0.0 --allow-dirty
RR publish v1.0.0 --push-tag --remote origin
RR publish v1.0.0 --registry /path/to/registry
```

Packages the current project into `Build/publish/<module>@<version>.tar.gz`.

- validates the module-path major-version rule
- requires `src/lib.rr` or `src/main.rr`
- rejects a dirty git worktree by default when `.git/` exists
- `--dry-run` validates and reports the archive path without writing it
- `--push-tag` creates a git tag and pushes it to the selected remote
- `--remote <name>` overrides the default publish remote (`origin`) when pushing a tag
- `--registry <dir>` also copies the publish archive into a local registry store and updates its index

If the project `rr.mod` includes metadata fields, RR also stores them in the
registry index:

- `description = "Catalog utilities"`
- `license = "MIT"`
- `homepage = "https://example.com/catalog"`

Registry signing supports two modes:

1. legacy HMAC
2. ed25519 public-key signatures

Legacy HMAC mode:

- `RR_REGISTRY_SIGNING_KEY` signs published releases
- `RR_REGISTRY_TRUST_KEY` verifies signed releases

Ed25519 mode:

- `RR_REGISTRY_SIGNING_ED25519_SECRET` is a 32-byte hex secret key used during publish
- `RR_REGISTRY_SIGNING_IDENTITY` optionally records a human or bot label in the registry index
- `RR_REGISTRY_TRUST_ED25519_KEYS` is a comma- or newline-separated list of trusted public keys in hex

Trust policy file:

- by default RR loads `policy.toml` from the selected registry root
- `RR_REGISTRY_POLICY` overrides that path
- policy can require signatures, trust extra public keys, revoke keys, and allow only named signers

Example `policy.toml`:

```toml
version = 1
require_signed = true
require_approval = true
trusted_ed25519 = "0123abcd..."
trusted_ed25519 = "89ef4567..."
revoked_ed25519 = "deadbeef..."
allowed_signer = "release-bot"
allowed_signer = "security-team"
auto_approve_signer = "release-bot"
```

If both HMAC and ed25519 signing inputs are present, RR prefers ed25519.
Signed registry releases are verified during install and `registry verify`.

### `search`

```bash
RR search math --registry /path/to/registry
```

Searches the configured registry index by module path and stored metadata.
Output includes:

- latest non-yanked release
- release count
- yanked release count
- license
- deprecation message
- description

### `registry list`

```bash
RR registry list --registry /path/to/registry
```

Prints the same one-line summary format as `RR search`, but for every module in
the selected registry.

### `registry report`

```bash
RR registry report --registry /path/to/registry
RR registry report rr.local/mathlib --registry /path/to/registry
```

Prints a compact summary of:

- total modules
- total releases
- approved releases
- pending releases
- yanked releases
- signed releases
- deprecated module count

For each matching module RR also prints one line with its latest approved
version and per-module counters.

### `registry diff`

```bash
RR registry diff rr.local/mathlib v1.0.0 v1.1.0 --registry /path/to/registry
```

Compares two stored registry release archives for the same module.

- shows approval/yanked/signature metadata changes
- counts added, removed, and changed files
- prints `+`, `-`, and `~` lines for the file paths that changed

### `registry risk`

```bash
RR registry risk rr.local/mathlib v1.1.0 --against v1.0.0 --registry /path/to/registry
```

Computes a simple risk score for one release.

- considers yanked, pending, unsigned, and deprecated state
- optionally compares against a baseline release
- adds points for larger file-level changes
- reports a `low`, `medium`, or `high` level plus the contributing factors

### `registry channel show`

```bash
RR registry channel show rr.local/mathlib --registry /path/to/registry
```

Prints every named channel assignment for one module.

### `registry channel set`

```bash
RR registry channel set rr.local/mathlib stable v1.0.0 --registry /path/to/registry
```

Assigns one named channel to one approved, non-yanked release.

### `registry channel clear`

```bash
RR registry channel clear rr.local/mathlib stable --registry /path/to/registry
```

Removes one named channel assignment from a module.

### `registry queue`

```bash
RR registry queue --registry /path/to/registry
```

Lists every release that is still pending approval.

### `registry audit`

```bash
RR registry audit --registry /path/to/registry
RR registry audit --limit 20 --registry /path/to/registry
RR registry audit --action registry-index --module rr.local/mathlib --contains approve --registry /path/to/registry
RR registry audit export ./audit.jsonl --format jsonl --action publish --registry /path/to/registry
```

Prints the registry audit log as tab-separated lines:

- unix timestamp
- action kind
- detail payload

Optional filters:

- `--action <kind>` only keep one audit action
- `--module <path>` only keep entries mentioning a module
- `--contains <text>` only keep entries containing a substring

`registry audit export` writes the filtered result set to a file.

- `--format tsv` writes the same tab-separated shape as stdout audit output
- `--format jsonl` writes one JSON object per line

### `registry keygen`

```bash
RR registry keygen release-bot --out-dir ./keys
```

Generates a new ed25519 signing keypair for registry publishing.

- prints the public and secret keys to stdout
- writes `registry-ed25519-secret.key` when `--out-dir` is set
- writes `registry-ed25519-public.key` when `--out-dir` is set
- writes `registry-signing.env` with ready-to-use environment variables

### `registry onboard`

```bash
RR registry onboard release-bot --out-dir ./keys --require-signed --require-approval --auto-approve --registry /path/to/registry
```

Combines `registry keygen` and `registry policy bootstrap`.

- generates an ed25519 keypair
- writes helper key/env files when `--out-dir` is set
- bootstraps `policy.toml` in the registry
- `--auto-approve` also registers the generated identity as an auto-approval signer

### `registry policy lint`

```bash
RR registry policy lint --registry /path/to/registry
```

Loads the effective `policy.toml` and reports:

- duplicate trusted, revoked, or signer entries
- invalid ed25519 key material
- contradictory trust and revoke rules
- basic signed-release policy gaps

### `registry policy bootstrap`

```bash
RR registry policy bootstrap <trusted-public-key> --signer release-bot --auto-approve-signer release-bot --require-signed --require-approval --registry /path/to/registry
```

Creates or updates a starter `policy.toml` for a registry.

- trusts the provided ed25519 public key
- optionally records one allowed signer
- optionally records one signer that should auto-approve at publish time
- can enable signed-release enforcement
- can enable release approval before `@latest` resolution

### `registry policy show`

```bash
RR registry policy show --registry /path/to/registry
```

Prints the canonical rendered policy file that RR will enforce.

### `registry policy rotate-key`

```bash
RR registry policy rotate-key <old-public-key> <new-public-key> --registry /path/to/registry
```

Updates `policy.toml` so the new public key becomes trusted and the old public
key becomes revoked. If the policy file does not exist yet, RR creates one.

### `registry policy apply`

```bash
RR registry policy apply ./policy.toml --registry /path/to/registry
```

Loads a local policy file, validates that RR can parse it, and writes it back
to the registry in canonical form.

### `registry info`

```bash
RR registry info rr.local/mathlib --registry /path/to/registry
```

Prints the registry metadata and every recorded release for a single module.
Each release line shows the version, file count, yanked state, archive path,
approval state, signature state, signature scheme, signer identity, archive path, and archive checksum.

### `registry approve`

```bash
RR registry approve rr.local/mathlib v1.2.0 --registry /path/to/registry
```

Marks one release as approved. Approved releases can be selected by `@latest`
and installed from the registry.

### `registry unapprove`

```bash
RR registry unapprove rr.local/mathlib v1.2.0 --registry /path/to/registry
```

Marks one release as pending approval again. Pending releases are skipped by
`@latest` and rejected by registry install.

### `registry promote`

```bash
RR registry promote rr.local/mathlib v1.1.0 --registry /path/to/registry
```

Promotes one release to be the active approved target for its module.

- the selected release becomes `approved=true`
- all other releases for the same module become `approved=false`
- `@latest` resolution now picks the promoted release

### `registry yank`

```bash
RR registry yank rr.local/mathlib v1.2.0 --registry /path/to/registry
```

Marks one registry release as yanked.

- the archive remains available for existing lockfiles and exact installs
- `@latest` no longer resolves to that yanked release
- `search` and `registry info` show the yanked state

### `registry unyank`

```bash
RR registry unyank rr.local/mathlib v1.2.0 --registry /path/to/registry
```

Clears the yanked state for one release so future `@latest` resolution can use
it again.

### `registry deprecate`

```bash
RR registry deprecate rr.local/mathlib "use rr.local/newmath" --registry /path/to/registry
```

Stores a module-level deprecation message in the registry index.
`search` and `registry info` surface that message so consumers can migrate
before the package is removed or superseded.

### `registry undeprecate`

```bash
RR registry undeprecate rr.local/mathlib --registry /path/to/registry
```

Removes a module-level deprecation message from the registry index.

### `registry verify`

```bash
RR registry verify --registry /path/to/registry
RR registry verify rr.local/mathlib --registry /path/to/registry
```

Verifies the registry store by checking:

- every indexed archive exists
- every archive checksum matches the index entry
- every HMAC-signed release matches the configured trust key
- every ed25519-signed release matches one configured trusted public key
- no release uses a revoked ed25519 signer key from policy
- every signed release matches the policy signer allowlist when configured
- every release is signed when policy requires signed artifacts
- every archive contains a valid `rr.mod`
- the embedded `rr.mod` module path matches the indexed module path

If any release fails verification, RR exits non-zero and prints one line per
issue with the module path, version, archive path, and failure reason.

### Private GitHub

Remote install/update/outdated flows accept:

- canonical module paths like `github.com/org/repo`
- HTTPS URLs like `https://github.com/org/repo`
- SSH sources like `git@github.com:org/repo.git`
- SSH URLs like `ssh://git@github.com/org/repo`

For private HTTPS access, RR checks:

1. `RRGITHUB_TOKEN`
2. `GITHUB_TOKEN`

If a token is present, RR passes it to `git` for GitHub HTTPS operations.
SSH sources continue to use the local SSH agent/configuration.

If `RR_REGISTRY_DIR` is set, RR also accepts non-GitHub module paths as
registry-backed dependencies during install/update/outdated flows.

### Workspace

If RR finds an `rr.work` file in a parent directory, package imports that match
workspace member module paths resolve directly to those local members before
falling back to external dependencies.

### `mod graph`

```bash
RR mod graph
```

Prints the resolved dependency graph as edge lines.
Nodes use locked versions when available, for example
`github.com/acme/mathlib@v1.2.3`.

### `mod why`

```bash
RR mod why github.com/acme/baseutil
```

Prints one dependency chain from the project root to the selected module.

### `mod verify`

```bash
RR mod verify
```

Checks every locked module checksum against the currently selected source root:

1. `replace`
2. `vendor/`
3. cache

If a checksum mismatch is found, RR exits non-zero and prints the expected sum,
actual sum, and selected source root for the offending module.

### `mod tidy`

```bash
RR mod tidy
```

Scans project `*.rr` sources, adds missing direct dependencies for package
imports, removes unused direct dependencies, and refreshes `rr.lock`.
If `rr.lock` already contains an exact resolved version, RR prefers that locked
version during builds over a stale manual edit in `rr.mod`.

### `mod vendor`

```bash
RR mod vendor
```

Copies the modules recorded in `rr.lock` into `vendor/`.

Resolver priority is now:

1. `replace` entries from the main `rr.mod`
2. vendored modules under `vendor/`
3. cached modules under `RRPKGHOME` or `~/.rr/`

### Direct Compile

```bash
RR input.rr -o out.R -O2
```

Use direct compile when:

- one `.rr` file should become one `.R` file
- you want to inspect generated output directly
- you want exact control over `-O0/-O1/-O2`

### `run`

```bash
RR run .
```

Input may be:

- `.`
- a directory
- a `.rr` file

If input is `.` or a directory, RR resolves `main.rr`.
Managed projects prefer `src/main.rr`; legacy projects still fall back to root
`main.rr`.

Project `run` expects the entry file to define `fn main()`.
If the source does not already call `main()` at top level, RR appends that call
automatically for `run`.

### `build`

```bash
RR build . -O2
```

Use `build` when:

- you want the project entry built without executing it
- you want output written under a build directory

Default output root for managed projects:

- `Build/debug/`

If the target directory contains `src/main.rr` or `main.rr`, RR treats it as a
project build and compiles only that entry.

If the target directory does not contain a runnable entry, RR falls back to the
older tree-build behavior and compiles every `*.rr` file it finds.

Project `build` also expects the entry file to define `fn main()`.
If the source does not already call `main()` at top level, RR appends that call
automatically in the emitted artifact.

RR skips `Build/`, `target/`, `.git/`, and `vendor/` during tree walks.

### `watch`

```bash
RR watch . -O2
```

Use `watch` when:

- you want repeated rebuilds from one live session
- you want phase 3 in-memory incremental reuse
- you want imported `*.rr` changes to trigger rebuilds without restarting the session

Current watch behavior:

- unchanged poll ticks do not rebuild repeatedly
- imported module edits are tracked as part of the watched module tree
- `--once` still runs exactly one watch tick and exits

### R Runner Selection

`RR run` executes emitted `.gen.R` through:

1. explicit runner path passed by internal callers
2. `RRSCRIPT` if set
3. plain `Rscript` from `PATH`

If RR cannot start the selected R runner, it prints a recovery hint and points
at `--keep-r` so you can inspect the generated artifact.

## Option Classes

### Optimization and Output

- `-O0`
- `-O1`
- `-O2`
- `-o <file>`
- `--out-dir <dir>`
- `--bin`
- `--lib`

Today RR distinguishes `-O0` from optimized mode. `-O1` and `-O2` currently
run the same optimizing pipeline, so the difference is naming/intent rather
than pass selection.

### Type and Backend Policy

- `--type-mode strict|gradual`
- `--native-backend off|optional|required`
- `--parallel-mode off|optional|required`
- `--parallel-backend auto|r|openmp`
- `--parallel-threads <N>`
- `--parallel-min-trip <N>`
- `--compiler-parallel-mode off|auto|on`
- `--compiler-parallel-threads <N>`
- `--compiler-parallel-min-functions <N>`
- `--compiler-parallel-min-fn-ir <N>`
- `--compiler-parallel-max-jobs <N>`

### Language and Declaration Policy

- `--strict-let on|off`
- `--warn-implicit-decl on|off`

### Incremental and Watch

- `--incremental[=auto|off|1|1,2|1,2,3|all]`
- `--incremental-phases <auto|off|1|1,2|1,2,3|all>`
- `--no-incremental`
- `--strict-incremental-verify`
- `--poll-ms <N>`
- `--once`

### Command-Specific Options

- `--keep-r`
  - accepted on the direct legacy compile/run path and on `RR run`
  - not accepted on `build` or `watch`
- `--no-runtime`
  - accepted only on the direct compile path `RR file.rr ...`
  - not accepted on `run`, `build`, or `watch`
- `--preserve-all-defs`
  - accepted on direct compile, `run`, `build`, and `watch`
  - keeps unreachable top-level `Sym_*` definitions in emitted R
- `--preserve-all-def`
  - alias for `--preserve-all-defs`

## Exit Status

RR follows normal compiler-driver conventions:

- `0`
  - compile or run request completed successfully
- non-zero
  - a structured diagnostic or runner failure occurred

The CLI owns final process exit behavior. Internal compiler layers return
structured diagnostics instead of calling `std::process::exit(...)` directly.

## Artifact Policy

The direct compile path emits `.R` artifacts with:

- selected runtime helper subset
- compile-time runtime policy defaults for backend/parallel settings
- source map side data when requested by internal flows

By default RR treats emitted R as a whole-program artifact:

- reachable top-level definitions are kept
- unreachable `Sym_*` helpers may be stripped

If you need a more source-preserving artifact, pass `--preserve-all-defs` or
`--preserve-all-def`.

If you pass `--no-runtime`, RR still emits helper-only output, not raw MIR or an
intermediate dump.

## Related Manuals

- [Getting Started](getting-started.md)
- [Configuration](configuration.md)
- [Language Reference](language.md)

## Semantics Notes

### `--no-runtime`

`--no-runtime` does not mean “emit raw source only”.

It means:

- omit compile-time source bootstrap and compile-time runtime policy defaults
- still emit the helper subset required by the generated program
- still emit ordinary `.R` code, not an internal IR dump

Use it for inspection and backend debugging, not for normal end-user execution.

### `--preserve-all-defs`

`--preserve-all-defs` keeps otherwise unreachable top-level RR definitions in
the emitted artifact.

`--preserve-all-def` is a supported alias.

Use it when:

- you want a closer source-to-source transpilation view
- you plan to inspect or call helper definitions from generated R
- you do not want whole-program dead-definition stripping

Without this flag, RR is free to drop unused top-level `Sym_*` definitions as
part of normal emitted-R cleanup.

### Builtin Naming

Most math and aggregation names are reserved for builtin/intrinsic lowering.

User shadowing is intentionally narrow:

- allowed scalar-index helpers:
  - `length`
  - `floor`
  - `round`
  - `ceiling`
  - `trunc`

Everything else should use distinct user names.

### R Interop

- `import r "pkg"` gives namespace-style access
- `pkg.fn(...)` lowers to `pkg::fn(...)`
- `import r { fn as local } from "pkg"` binds one local alias
- `import r * as pkg from "pkg"` binds namespace-style access

See [R Interop](r-interop.md) for package coverage and fallback tiers.

## Incremental Compile Policy

Default CLI behavior is `--incremental=auto`.

`auto` means:

- phase 1 enabled
- phase 2 enabled
- phase 3 enabled only when a live session exists, such as `watch`

Use:

- `--no-incremental` when you want a fresh compile for inspection
- `--strict-incremental-verify` when you want cache reuse checked against a rebuild

## Compiler Parallelism

Compiler scheduling is separate from generated-runtime parallel policy.

Default CLI behavior is automatic:

- compiler parallel mode defaults to `auto`
- compiler worker count defaults to host parallelism (`available_parallelism()`)
- compiler max jobs defaults to the active worker count unless you override it

Use:

- `--compiler-parallel-mode off|auto|on`
- `--compiler-parallel-threads <N>`
- `--compiler-parallel-min-functions <N>`
- `--compiler-parallel-min-fn-ir <N>`
- `--compiler-parallel-max-jobs <N>`

These flags control host-side compile scheduling for stages such as:

- MIR synthesis
- type analysis
- function-local Tachyon waves
- MIR-to-R emission

They do not change emitted runtime semantics.

`--compiler-parallel-max-jobs` caps the number of simultaneously active
compiler jobs and is intended as the memory-pressure safety valve for large
projects or heavily nested compile workloads.

`RR build` is still file-serial today. If RR grows file-level parallel build
mode later, that outer layer should continue to use the same compiler-side pool
instead of creating a second independent pool.

Runtime parallel behavior is still controlled by:

- `--parallel-mode`
- `--parallel-backend`
- `--parallel-threads`
- `--parallel-min-trip`
