---
search: false
---

# RR Documentation

This directory is the RR manual set. It documents RR as implemented in this
repository, but the reading order is user-first: how to write RR, compile it,
and run it comes before compiler structure. Compiler contributor manuals live
under `docs/compiler/`.

Current compiler line: `RR Tachyon v1.2.0`.

## Manual Organization

The RR docs are split into four manual sets:

- `Guide`
  - user-oriented
  - build, compile, run, watch, and performance-minded authoring
- `Reference`
  - interface-oriented
  - syntax, CLI, interop, configuration, and limits
- `Compiler Dev Docs`
  - implementation-oriented
  - pipeline, IR, optimizer, runtime, and diagnostics
- `Development`
  - verification-oriented
  - tests, audit steps, and regression workflows

This keeps "how do I use RR?", "why did RR emit this?", and "how do I verify a
compiler change?" separate instead of mixing them into one page.

## Start Here

If you are using RR:

- [Getting Started](getting-started.md)
- [RR for R Users](r-for-r-users.md)
- [Writing RR for Performance and Safety](writing-rr.md)
- [CLI Reference](cli.md)
- [Language Reference](language.md)

If you are validating generated output:

- [Configuration](configuration.md)
- [Compatibility and Limits](compatibility.md)
- [R Interop](r-interop.md)

If you are working on the compiler:

- [Compiler Dev Docs](compiler/index.md)
- [`CONTRIBUTING.md`](https://github.com/Feralthedogg/RR/blob/main/CONTRIBUTING.md)
- [Compiler Pipeline](compiler/pipeline.md)
- [Adaptive Phase Ordering Design](compiler/adaptive-phase-ordering.md)
- [Parallel Compilation Design](compiler/parallel-compilation.md)
- [IR Model](compiler/ir-model.md)
- [Tachyon Engine](compiler/optimization.md)
- [Testing and Quality Gates](compiler/testing.md)
- [Contributing Audit Checklist](compiler/contributing-audit.md)

## Documentation Conventions

- User-facing pages state commands, workflows, guarantees, and limits before implementation detail.
- Internal pages state phase boundaries, invariants, and failure modes before code layout.
- Testing pages are treated as product contract, not as contributor-only notes.
- Pages prefer exact file paths, pass names, flags, and helper names over vague summaries.

## Project Snapshot

- frontend
  - lexer/parser/HIR in `src/syntax` and `src/hir`
- middle end
  - SSA-like MIR in `src/mir`
  - optimizer entry in `src/mir/opt.rs`
- backend
  - MIR-to-R emission in `src/codegen/mir_emit.rs`
- runtime
  - embedded R runtime in `src/runtime`
- diagnostics
  - structured `RRException` in `src/error.rs`

## VitePress

Docs are served from `docs/` with VitePress.

```bash
cd docs
pnpm install
pnpm docs:dev
```

Build and preview:

```bash
pnpm docs:build
pnpm docs:preview
```
