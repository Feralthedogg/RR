# RR Package Manager Design

Status: proposed

This document defines a package-manager design for RR that keeps the current
compiler model intact where possible, but adds a Go-like module and install
workflow for real projects.

The design is intentionally biased toward:

- project creation as a first-class CLI flow
- GitHub-backed dependency installation
- Go-style module paths and version selection
- deterministic, cache-friendly builds
- cargo-like project scaffolding for managed projects
- backward compatibility for RR's existing file-import workflows

## Why RR Needs This

Today RR already has two useful pieces:

- project execution centered on a single entry file
- file-based imports such as `import "./helper.rr"`

Those are enough for local code reuse, but not enough for dependency
distribution. A real package manager needs four things RR does not have yet:

1. a project manifest
2. a dependency graph and version solver
3. a shared module cache
4. a package import resolver that can map import strings to installed code

The good news is that RR's current compiler structure is already close to what
we need:

- the CLI already resolves directories to an entry file
- source analysis already walks an import graph
- incremental compilation already fingerprints the loaded module tree

That means the cleanest design is not "build a separate package tool". It is
"add a resolver layer in front of the existing compiler".

## Design Goals

- Keep `RR run .`, `RR build .`, and `RR watch .` as the primary workflows.
- Add `RR new` and `RR init` so users can create projects without manual
  bootstrapping.
- Add `RR install <github-url-or-module-path>` so dependencies can be fetched
  directly from GitHub.
- Use cargo-like project layout for managed RR projects:
  - `src/main.rr` for binaries
  - `src/lib.rr` for libraries
  - `Build/` for generated artifacts and caches
- Use Go-style module paths and exact version selection instead of npm-style
  semver ranges.
- Keep local relative file imports working exactly as they do today.
- Make builds reproducible with a lock file and checksums.
- Make offline rebuilds possible from a local cache or vendored sources.

## Non-Goals For V1

- a public central registry
- package publishing
- GitLab/Bitbucket/source-host abstraction
- SAT-style version ranges
- package-level sandbox execution during install
- automatic code generation hooks in dependencies

V1 should solve the common path: create a project, install from GitHub, import
the package, build deterministically.

## User-Facing CLI

### New Commands

```bash
RR new [--bin|--lib] <module-path> [dir]
RR init [--bin|--lib] [module-path]
RR install <github-url|module-path>[@version]
RR remove <module-path>
RR mod tidy
RR mod vendor
RR mod graph
RR mod why <module-path>
RR cache clean
```

### Command Semantics

#### `RR new`

Creates a new RR project directory with:

- `rr.mod`
- `src/main.rr` by default
- `src/lib.rr` when `--lib` is used
- `.gitignore`
- ignored `Build/`

Example:

```bash
RR new github.com/acme/simapp
```

Creates:

```text
simapp/
  rr.mod
  src/
    main.rr
  .gitignore
```

Binary-vs-library rules should mirror Cargo:

- default is `--bin`
- `RR new --lib github.com/acme/math` creates `src/lib.rr`
- managed projects keep source under `src/`
- generated artifacts and caches go under `Build/`

#### `RR init`

Initializes package metadata inside an existing directory using the same layout
rules as `RR new`.

Example:

```bash
RR init github.com/acme/analytics
```

For managed projects, `RR init` should create:

- `rr.mod`
- `src/main.rr` by default, or `src/lib.rr` with `--lib`
- `.gitignore` that ignores `Build/`

Legacy RR projects with root-level `main.rr` should remain runnable, but new
scaffolding should be cargo-like rather than root-file based.

#### `RR install`

Adds a dependency to the current project, resolves a concrete version, fetches
the module into cache, and updates `rr.mod` and `rr.lock`.

Accepted inputs:

- `RR install github.com/acme/math`
- `RR install github.com/acme/math@v1.2.3`
- `RR install github.com/acme/math@latest`
- `RR install https://github.com/acme/math`
- `RR install https://github.com/acme/math/tree/main/lib/stats`

Normalization rule:

- raw GitHub URLs are accepted for user convenience
- the canonical identity stored in manifests is always a module path

So this:

```bash
RR install https://github.com/acme/math
```

writes this:

```text
require github.com/acme/math v0.4.1
```

#### `RR remove`

Removes a direct dependency from `rr.mod`, then recomputes the graph and lock
file.

#### `RR mod tidy`

Go-style cleanup pass:

- remove unused direct requirements
- add missing direct requirements inferred from imports
- refresh indirect dependencies
- rewrite `rr.lock`

#### `RR mod vendor`

Copies the resolved dependency graph into `vendor/` for air-gapped or
fully-reproducible builds.

#### `RR cache clean`

Removes cached module downloads and unpacked module trees under RR's shared
package home.

## Managed Project Layout

New RR projects should use a cargo-like source layout and a target-like build
layout with the directory name changed to `Build/`.

### Binary Project

```text
simapp/
  rr.mod
  src/
    main.rr
  Build/
  .gitignore
```

### Library Project

```text
mathlib/
  rr.mod
  src/
    lib.rr
  Build/
  .gitignore
```

### Build Directory Policy

Everything generated by RR for a managed project should live under `Build/`.

Recommended structure:

```text
Build/
  debug/
  release/
  incremental/
  watch/
  pkg/
  tmp/
```

Meaning:

- `Build/debug/`
  - default build outputs and emitted R artifacts for non-release builds
- `Build/release/`
  - optimized build outputs
- `Build/incremental/`
  - phase 1/2/3 incremental caches
- `Build/watch/`
  - watch-mode generated artifacts
- `Build/pkg/`
  - synthetic package entry files and resolver working data
- `Build/tmp/`
  - temporary extraction or staging files

This should replace current ad hoc locations such as:

- `build/`
- `target/.rr-cache`
- `.rr-watch/`

## Manifest And Lock Files

### `rr.mod`

RR should use a Go-like manifest, not a generic TOML project file.

Reason:

- the core problem is module resolution, not broad project metadata
- Go's line-oriented model is simple, readable, and proven
- `require` and `replace` fit RR better than a general package.json-like shape

Proposed example:

```text
module github.com/acme/simapp

rr 8.0

require (
    github.com/acme/math v1.2.3
    github.com/acme/plot v0.0.0-20260402-abcdef123456
)

replace github.com/acme/localmath => ../localmath
```

Directives:

- `module`
  - the module's canonical identity
- `rr`
  - the minimum RR toolchain line expected by the module
- `require`
  - direct and indirect module requirements
- `replace`
  - local development overrides and fork overrides

`exclude` can be added later, but it is not required for V1.

### `rr.lock`

RR should keep an explicit lock file even though Go itself relies mainly on
`go.mod` plus checksums.

Reason:

- RR should prefer exact reproducibility from day one
- RR will be integrating resolution into build/run/watch, so a solved graph is
  valuable
- lockfile-driven offline mode is easier to implement than re-solving from
  scratch

Proposed shape:

```toml
version = 1

[[module]]
path = "github.com/acme/math"
version = "v1.2.3"
commit = "3f2d0d9d0e1a4d8f8d73b6768e6f1b1f6b5d8b9c"
sum = "sha256:..."
direct = true

[[module]]
path = "github.com/acme/plot"
version = "v0.0.0-20260402-abcdef123456"
commit = "abcdef1234567890abcdef1234567890abcdef12"
sum = "sha256:..."
direct = false
```

`rr.lock` is the exact graph RR builds against.

## Import Model

RR already uses:

```rr
import "./helper.rr"
```

That must keep working unchanged.

The new rule is:

- local file import:
  - starts with `./`
  - starts with `../`
  - starts with `/`
  - ends with `.rr`
- package import:
  - everything else

Examples:

```rr
import "./helper.rr"                    // local file
import "../common/math.rr"              // local file
import "github.com/acme/math"           // module package
import "github.com/acme/math/vector"    // subpackage
```

This gives RR a simple, stable split:

- file imports stay file imports
- Go-style module paths become package imports

### Why Not Add New Syntax?

Because RR does not need new syntax here. The existing `import "..."` surface is
already enough. The missing feature is resolution, not parsing.

## Package Layout Model

The package manager should be module-based, but packages inside a module should
be directory-based, like Go.

That means:

- a module is rooted by `rr.mod`
- managed source files live under `src/`
- a package is a directory inside that module's `src/` tree
- an import path identifies a package directory

Example:

```text
github.com/acme/math                -> module root library target `src/lib.rr`
github.com/acme/math/vector         -> subpackage directory `src/vector/`
github.com/acme/math/stats/linear   -> nested subpackage directory `src/stats/linear/`
```

### How This Fits RR's Current Compiler

RR today compiles files plus imported files. It does not have first-class
directory packages.

The clean bridge is a synthetic package entry file:

1. resolve an import path to either:
   - `src/lib.rr` for a module root library
   - a package directory under `src/` for a subpackage
2. gather all `*.rr` files in that package directory when directory mode is used
3. generate a synthetic file in `Build/pkg/` such as `__rr_pkg_entry__.rr`
4. make that synthetic file import the package's member files in stable lexical
   order
5. feed the synthetic file into the existing import/source-analysis pipeline

Example generated file:

```rr
import "./a.rr"
import "./b.rr"
import "./c.rr"
```

This avoids a large compiler rewrite in V1 while still giving RR a Go-like
package unit.

## Version Model

RR should copy Go's version model, not npm's.

### Version Rules

- canonical tagged versions are `vX.Y.Z`
- `@latest` resolves to the newest tagged version
- bare install defaults to `@latest`
- if no tag exists, RR generates a pseudo-version:
  - `v0.0.0-YYYYMMDDHHMMSS-<12sha>`
- commit SHA and branch names are accepted install targets

Examples:

```bash
RR install github.com/acme/math@latest
RR install github.com/acme/math@v1.4.2
RR install github.com/acme/math@1f2d3c4
RR install github.com/acme/math@main
```

### Version Selection

RR should use MVS-style resolution similar to Go:

- each module requirement names one minimum version
- the selected version for a module path is the highest required version seen in
  the graph
- there are no semver ranges in `rr.mod`

Why this is the right fit:

- deterministic
- easy to explain
- easy to cache
- matches the user's "copy Go" requirement

## Major Version Policy

RR should adopt Go's major-version suffix rule for v2+ modules:

- `github.com/acme/math` for v0/v1
- `github.com/acme/math/v2` for v2+

This prevents silent major-version conflicts inside one build graph.

## GitHub Install Flow

V1 should support GitHub only.

### Accepted Inputs

- repository root URL
- repository URL plus `/tree/<ref>/<subdir>`
- canonical module path

### Resolution Steps

1. normalize user input into a canonical module path
2. determine requested version:
   - explicit tag
   - explicit branch
   - explicit commit
   - or `latest`
3. resolve the request to an exact commit
4. download the module source
5. validate that `rr.mod` exists at the module root
6. validate that the `module` directive matches the canonical module path
7. compute and store checksum
8. read dependency requirements from the dependency's `rr.mod`
9. continue graph resolution

### Monorepo Support

GitHub URLs with subdirectories should map to module subpaths.

Example:

```text
https://github.com/acme/mono/tree/main/libs/stats
```

normalizes to:

```text
github.com/acme/mono/libs/stats
```

provided `libs/stats/rr.mod` exists and declares the same module path.

## Local Cache Layout

Shared package home:

```text
~/.rr/
  bin/
  pkg/
    mod/
      github.com/acme/math@v1.2.3/
      github.com/acme/plot@v0.0.0-20260402-abcdef123456/
    vcs/
      <repo-cache-hash>/
```

Recommended environment override:

- `RRPKGHOME`

Behavior:

- `pkg/mod/` stores unpacked, content-addressed module trees
- `pkg/vcs/` stores reusable clone or archive state for fetch efficiency
- `bin/` is reserved for future tool installs

This global cache is separate from the per-project `Build/` directory.

- remote source downloads can stay shared under `~/.rr/`
- project-local generated artifacts and incremental state should go under
  `Build/`

## Vendor Layout

Optional vendored tree:

```text
vendor/
  github.com/acme/math/...
  github.com/acme/plot/...
  modules.txt
```

Resolution priority:

1. `replace` local directory
2. `vendor/` when vendoring is enabled
3. shared cache
4. network fetch

## Resolver Integration With Existing RR Commands

This is the key architectural point.

RR should not treat package install as a separate universe. `run`, `build`, and
`watch` should all use the same resolver.

### Project Root Detection

Current RR resolves projects mostly through `main.rr`.

With package management:

1. walk upward looking for `rr.mod`
2. if found, that directory is the project root
3. for managed projects, resolve:
   - `src/main.rr` for runnable binaries
   - `src/lib.rr` for library-only packages
4. if not found, preserve legacy behavior and treat a directory containing
   root-level `main.rr` as a valid RR project anyway

This keeps old projects working while enabling new ones.

### Build Pipeline Placement

The resolver should run before source analysis.

Proposed flow:

1. CLI resolves target and project root
2. package resolver loads `rr.mod` and `rr.lock`
3. imports are rewritten from module paths to synthetic cached package entry
   files
4. existing source analysis loads those synthetic files and their local members
5. the rest of the compiler stays unchanged

### Watch And Incremental Compile

The module/package resolver must feed package files into the existing module
tree fingerprint logic.

Otherwise `RR watch` would miss dependency edits from:

- local `replace` directories
- vendored packages
- generated synthetic package entry files

The incremental cache root should change from:

- nearest `Cargo.toml` or other incidental workspace marker

to:

- nearest `rr.mod`, using `Build/incremental/`
- otherwise current fallback behavior for legacy projects

This matters because package-aware projects should cache relative to the RR
module root, not the Rust workspace root.

Watch output should also move into `Build/watch/` instead of `.rr-watch/`.

## Proposed Internal Modules

Add a new package-management layer under `src/pkg/`:

```text
src/pkg/
  mod.rs
  manifest.rs
  lockfile.rs
  version.rs
  cache.rs
  github.rs
  resolver.rs
  rewrite.rs
  vendor.rs
```

Responsibilities:

- `manifest.rs`
  - parse and write `rr.mod`
- `lockfile.rs`
  - parse and write `rr.lock`
- `version.rs`
  - semver and pseudo-version ordering
- `cache.rs`
  - package-home paths, checksums, unpacking
- `github.rs`
  - GitHub URL normalization and fetch logic
- `resolver.rs`
  - graph construction and MVS resolution
- `rewrite.rs`
  - import-path to cached-file rewriting
- `vendor.rs`
  - vendor tree generation and loading

### Existing Code That Will Need Integration

- `src/main.rs`
  - add new commands, cargo-like scaffolding, and root detection
- `src/compiler/pipeline/phases/source_emit.rs`
  - consume rewritten resolved imports
- `src/compiler/incremental.rs`
  - fingerprint resolved package sources and move cache-root detection toward
    `rr.mod` plus `Build/incremental/`
- `src/hir/lower.rs`
  - local/module import syntax can stay mostly unchanged

## Security Model

V1 should be conservative.

- only `https://github.com/...` and canonical `github.com/...` module paths are
  accepted for remote install
- no install-time hook execution
- dependency contents are checksum-verified against `rr.lock`
- checksum mismatch is a hard error
- offline mode uses only cache, vendor, and local replacements

## Error Cases RR Must Handle Well

- installed module is missing `rr.mod`
- `module` directive does not match requested module path
- imported package directory does not exist in the selected module version
- dependency graph contains a cycle
- checksum mismatch in cache or vendor directory
- `replace` target is missing
- package import string is ambiguous because it is neither a local path nor a
  valid module path

Suggested user-facing diagnostic example:

```text
package import not found: github.com/acme/math/vector
selected module: github.com/acme/math v1.2.3
looked for package directory: vector/
help: run `RR install github.com/acme/math@latest` or check the import path
```

## Backward Compatibility Rules

- legacy projects with only `main.rr` and relative imports must continue to
  work
- managed projects should prefer `src/main.rr` and `src/lib.rr`
- `RR run .`, `RR build .`, and `RR watch .` must still fall back to root
  `main.rr` for legacy projects
- local file imports are never reinterpreted as package imports
- existing R package interop syntax such as `import r "graphics"` is unaffected

## Recommended Implementation Phases

### Phase 1: Foundation

- add `rr.mod`
- add `RR new --bin|--lib`
- add `RR init --bin|--lib`
- add manifest parsing/writing
- add project-root detection via `rr.mod`
- add cargo-like `src/` scaffolding
- add `Build/` directory policy

### Phase 2: Dependency Install

- add `RR install`
- add GitHub normalization
- add shared module cache
- add checksum recording
- add `rr.lock`

### Phase 3: Compiler Integration

- add package import classification
- add package-directory resolution
- add synthetic package entry generation under `Build/pkg/`
- integrate resolver into `run`, `build`, and `watch`
- move watch output and incremental caches into `Build/`

### Phase 4: Workflow Completion

- add `RR remove`
- add `RR mod tidy`
- add `RR mod vendor`
- add `replace` support
- add offline and vendor-aware builds

### Phase 5: Nice-To-Haves

- tool dependencies
- global executable install into `~/.rr/bin`
- private GitHub support

## Recommended V1 UX

The intended end-to-end flow should look like this:

```bash
RR new github.com/acme/demo
cd demo
RR install https://github.com/acme/math@latest
```

`rr.mod` becomes:

```text
module github.com/acme/demo

rr 8.0

require github.com/acme/math v1.2.3
```

User code:

```rr
// src/main.rr
import "github.com/acme/math"

fn main() {
  print(add_one(41))
}

main()
```

Then:

```bash
RR run .
```

works without the user caring where the dependency is cached locally.

## Final Recommendation

RR should not build a generic package manager first.

RR should build a Go-style module system adapted to RR's existing import model:

- `rr.mod` as the project manifest
- `rr.lock` as the exact solved graph
- `RR new` and `RR init` with cargo-like `src/` scaffolding
- `RR install` for GitHub-backed dependency acquisition
- MVS-style version selection
- directory packages bridged through synthetic entry files
- project-local build artifacts and incremental state under `Build/`

That gives RR a package manager that feels familiar, stays deterministic, and
fits the compiler architecture already in the repository.
