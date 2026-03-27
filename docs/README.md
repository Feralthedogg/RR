# RR Documentation

This directory is the RR manual set. It documents the compiler as implemented
in this repository, not as an aspirational future design.

Current compiler line: `RR Tachyon v7.0.0`.

## Manual Organization

The RR docs are split the way large systems projects split their manuals:

- `Guide`
  - task-oriented
  - build, compile, run, watch, and performance-minded authoring
- `Reference`
  - behavior-oriented
  - syntax, CLI, interop, configuration, and limits
- `Internals`
  - contract-oriented
  - pipeline, IR, optimizer, runtime, and diagnostics
- `Development`
  - verification-oriented
  - tests, audit steps, and regression workflows

This keeps “how do I use RR?”, “why did RR emit this?”, and “how do I verify a
compiler change?” separate instead of mixing them into one page.

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
- [Runtime and Error Model](runtime-and-errors.md)

If you are working on the compiler:

- [Compiler Pipeline](compiler-pipeline.md)
- [IR Model](ir-model.md)
- [Tachyon Engine](optimization.md)
- [Testing and Quality Gates](testing.md)
- [Contributing Audit Checklist](contributing-audit.md)

## Documentation Conventions

- User-facing pages state guarantees, limits, and expected workflows before implementation detail.
- Internal pages state phase boundaries, invariants, and failure modes before code layout.
- Testing pages are treated as product contract, not as contributor-only notes.
- Pages prefer exact file paths, pass names, flags, and helper names over vague summaries.

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
