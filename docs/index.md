---
layout: home

hero:
  name: RR
  text: "Write RR, Run R"
  tagline: "Learn RR from the user side first: write `.rr`, compile it into self-contained `.R`, and run it with familiar R tooling."
  actions:
    - theme: brand
      text: Start Here
      link: /getting-started
    - theme: alt
      text: What's New in 2.0
      link: /whats-new-2.0
    - theme: alt
      text: RR for R Users
      link: /r-for-r-users

features:
  - title: First Compile
    details: Go from a fresh checkout to a working `.R` artifact in a few commands.
  - title: Learn the Language
    details: Pick up RR syntax, type hints, matrix and dataframe guidance, and the patterns RR understands well.
  - title: Build Real Projects
    details: Use `RR run`, `RR build`, and `RR watch` for local runs, mirrored build trees, and edit-compile loops.
  - title: R Interop and Limits
    details: Check configuration, package interop, and supported limits when you need exact behavior.
---

## Start Here

If your goal is "show me how to use RR", start here:

1. Read [Getting Started](/getting-started) for the shortest path from checkout to a running program.
2. Read [What Is New in 2.0](/whats-new-2.0) if you are upgrading from RR 1.x.
3. Read [RR for R Users](/r-for-r-users) if you already know R and want the RR mental model quickly.
4. Keep [CLI Reference](/cli) nearby once you want exact command forms and flags.

Build RR, compile one file, and run it:

```bash
cargo build
target/debug/RR main.rr -o main.R -O2
Rscript --vanilla main.R
```

## Common Workflows

### Compile one file

```bash
RR input.rr -o out.R -O2
```

### Run a project entry

```bash
RR run . -O2
```

### Build a directory tree

```bash
RR build . --out-dir build -O2
```

### Watch and rebuild

```bash
RR watch . -O2
```

## Current Surface Notes

- `unsafe r { ... }` is the read/write raw R escape hatch. RR preserves the raw
  R body and treats the affected function conservatively.
- `unsafe r(read) { ... }` is the narrower read-only form for probes and logging.
  It can read RR-visible locals without forcing opaque interop or post-block
  local reloads, but assigning to RR locals inside the block is outside the
  stable contract.
- The exact trust boundary is documented in [Language Reference](/language),
  [R Interop](/r-interop), and [Unsafe Boundaries](/compiler/unsafe-boundaries).

## User Docs

Use these pages when you want to write RR code and get work done.

- [Getting Started](/getting-started)
- [What Is New in 2.0](/whats-new-2.0)
- [RR for R Users](/r-for-r-users)
- [Writing RR for Performance and Safety](/writing-rr)
- [CLI Reference](/cli)
- [Language Reference](/language)
- [Configuration](/configuration)
- [R Interop](/r-interop)
- [Compatibility and Limits](/compatibility)
- [Package Manager Design](/package-manager-design)

## Compiler Docs

Use these pages when you want the implementation-side view:

- [Compiler Overview](/compiler/)
- [Compiler Pipeline](/compiler/pipeline)
- [Parallel Compilation](/compiler/parallel-compilation)
- [IR Model](/compiler/ir-model)
- [Tachyon Optimizer](/compiler/optimization)
- [Adaptive Phase Ordering](/compiler/adaptive-phase-ordering)
- [Compile-Time Reduction](/compiler/compile-time-reduction)
- [MIR SROA Design](/compiler/sroa)
- [Runtime and Error Model](/compiler/runtime-and-errors)
- [Unsafe Boundaries](/compiler/unsafe-boundaries)
- [Testing and Quality Gates](/compiler/testing)
- [Contributing Audit Checklist](/compiler/contributing-audit)

## Reading Paths

If you are new to RR:

1. Read [Getting Started](/getting-started).
2. Read [What Is New in 2.0](/whats-new-2.0) if you are upgrading existing code.
3. Read [RR for R Users](/r-for-r-users).
4. Read [CLI Reference](/cli).
5. Read [Language Reference](/language).

If you are shipping RR code:

1. Read [CLI Reference](/cli).
2. Read [Configuration](/configuration).
3. Read [Writing RR for Performance and Safety](/writing-rr).

If you are debugging or modifying RR itself:

1. Read [Compiler Overview](/compiler/).
2. Read [Compiler Pipeline](/compiler/pipeline).
3. Read [IR Model](/compiler/ir-model).
4. Read [Tachyon Optimizer](/compiler/optimization).
5. Read [Testing and Quality Gates](/compiler/testing).

## Documentation Principles

- Pages describe RR as implemented in this repository, not as an aspirational design.
- User docs start with workflows, commands, and examples before compiler structure.
- User docs avoid implementation-detail walkthroughs in the main reading flow.
- Testing pages are treated as part of the product contract, not as contributor-only notes.
