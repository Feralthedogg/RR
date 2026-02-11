# RR Documentation

This directory documents RR as implemented in this repository.
It is organized by usage first, then internals.

## Index

- [`docs/getting-started.md`](getting-started.md)
- [`docs/cli.md`](cli.md)
- [`docs/language.md`](language.md)
- [`docs/ir-model.md`](ir-model.md)
- [`docs/compiler-pipeline.md`](compiler-pipeline.md)
- [`docs/optimization.md`](optimization.md)
- [`docs/runtime-and-errors.md`](runtime-and-errors.md)
- [`docs/configuration.md`](configuration.md)
- [`docs/testing.md`](testing.md)
- [`docs/compatibility.md`](compatibility.md)

## Project Snapshot

- Surface language: R-first syntax (`<-`, `function`, dotted identifiers) with legacy aliases (`let`, `fn`)
- Frontend: lexer + parser + AST (`src/syntax`)
- Mid-level: HIR lowering + desugaring (`src/hir`)
- Core IR: MIR SSA-like CFG (`src/mir`)
- Optimizer: `TachyonEngine` (`src/mir/opt.rs`)
- Backend: structured MIR-to-R emitter (`src/codegen/mir_emit.rs`)
- Runtime: embedded R helper library (`src/runtime/mod.rs`)
- Diagnostics: structured `RRException` with color palettes (`src/error.rs`)
