# RR Documentation

This directory documents RR as implemented in this repository.
Current compiler line: `RR Tachyon v4.0.0`.

The docs are organized for three audiences:

- users who want to compile or run RR programs
- contributors who need to understand the pipeline and runtime
- reviewers who need a map from behavior to source code

## Start Here

If you are using RR:

- [Getting Started](getting-started.md): build RR, compile a file, run a project
- [Writing RR for Performance and Safety](writing-rr.md): user-facing guide for optimization-friendly and safe `.rr` code
- [CLI Reference](cli.md): command forms, options, watch/build behavior
- [Configuration](configuration.md): environment variables and optimizer/runtime knobs
- [Language Reference](language.md): syntax and supported forms
- [R Interop](r-interop.md): supported R package interop surface and fallback tiers

If you are working on the compiler:

- [Compiler Pipeline](compiler-pipeline.md): end-to-end compile phases
- [IR Model](ir-model.md): HIR/MIR structure and purpose
- [Tachyon Engine](optimization.md): optimization passes and vectorization coverage
- [Runtime and Errors](runtime-and-errors.md): emitted runtime helpers and diagnostics model
- [Testing and Quality Gates](testing.md): integration, perf, and fuzz coverage
- [Contributing Audit Checklist](contributing-audit.md): final manual and command-based verification pass
  - includes `scripts/contributing_audit.sh` for baseline command + static rule checks
  - includes `scripts/verify_cleanroom.sh` for strict clean-worktree verification of a scoped patch

If you need behavior limits:

- [Compatibility and Limits](compatibility.md)

## Documentation Map

- `getting-started.md`
  - minimal setup and first successful compile/run flow
- `writing-rr.md`
  - how to structure `.rr` programs so current RR optimization and safety checks work well
- `cli.md`
  - command-line surface and execution modes
- `language.md`
  - surface syntax and language behavior
- `r-interop.md`
  - direct/opaque/hybrid R package interop model and supported package surface
- `compiler-pipeline.md`
  - pipeline phases, pass order, and validation points
- `optimization.md`
  - Tachyon optimizer strategy and current vectorization/runtime helper lowering
- `runtime-and-errors.md`
  - runtime helper contract, backend policy, and diagnostics
- `configuration.md`
  - environment-driven behavior switches
- `testing.md`
  - test families, perf gates, fuzzing
- `contributing-audit.md`
  - post-change verification checklist for compiler contributors

## VitePress

Docs are served from the `docs/` directory with VitePress.

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

- frontend: lexer/parser/HIR in `src/syntax` and `src/hir`
- core IR: SSA-like MIR in `src/mir`
- optimizer: `TachyonEngine` in `src/mir/opt.rs`
- backend: MIR-to-R emission in `src/codegen/mir_emit.rs`
- runtime: embedded R helper library in `src/runtime/mod.rs`
- diagnostics: structured `RRException` in `src/error.rs`
