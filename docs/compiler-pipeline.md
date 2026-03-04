# Compiler Pipeline

RR compile path in `src/compiler/pipeline.rs::compile()` uses a 6-step pipeline.

CLI entrypoints in `src/main.rs` call this pipeline API.

## High-Level Flow

`RR source`  
`-> Lexer/Parser (AST)`  
`-> HIR Lowering`  
`-> HIR Desugar`  
`-> MIR Lowering (SSA-like CFG)`  
`-> Type Analysis (strict/gradual)`  
`-> MIR Validation`  
`-> Tachyon Engine (opt or stabilization)`  
`-> MIR Structurizer + R emission`  
`-> Runtime injection`  
`-> self-contained .R output`

## Step-by-Step (matches CLI progress output)

1. `Source Analysis`
- Parse each module.
- Resolve imports.
- Lower AST to HIR.
- Collect global symbol table and arity info.

2. `Canonicalization`
- Run HIR desugaring (`src/hir/desugar.rs`).

3. `SSA Graph Synthesis`
- Lower each HIR function to MIR (`src/mir/lower_hir.rs`).
- Build blocks, SSA values, phi placeholders/backpatching.
- Seed MIR type metadata:
  - `FnIR.param_ty_hints`
  - `FnIR.param_term_hints`
  - `FnIR.ret_ty_hint`
  - `FnIR.ret_term_hint`
  - `Value.value_ty`
  - `Value.value_term`
- Run interprocedural fixed-point type analysis (`src/typeck/solver.rs`).
- Structural type terms (`src/typeck/term.rs`) and MIR constraints (`src/typeck/constraints.rs`)
  refine nested/container projections (for example `list<box<T>>` index/unbox flows).
- Respect type mode:
  - `strict` (default): fail on proven hint conflicts, call signature mismatches, and unresolved strict-only positions.
  - `gradual`: keep safe runtime path when proofs are unavailable.
- Populate:
  - `FnIR.inferred_ret_ty`
  - `FnIR.inferred_ret_term`
  - per-value type states/terms used by downstream passes.

4. `Tachyon Optimization` or `Tachyon Stabilization`
- `-O1/-O2`: full Tachyon optimization pipeline.
- `-O0`: stabilization-only path (still includes mandatory De-SSA before codegen).
- O1/O2 budget model:
  1. Tier A always pass for all safe functions.
  2. Tier B selective-heavy pass for scored functions under IR budget.
  3. Tier C inter-procedural inlining only when heavy tier is enabled.
- Type-directed pass order for optimized mode:
  1. `type_specialize`
  2. existing vectorization (`v_opt`)
  3. `bce`
  4. cleanup/de-ssa chain
- Guard removal policy: remove only when MIR type/range proof exists (`value_ty` + `value_term` + range facts);
  otherwise preserve original guard calls.
- Optional hotness hints (`RR_PROFILE_USE`) can bias selective Tier-B function choice without changing semantics.
- SCCP/analysis arithmetic is fail-safe: when compile-time arithmetic overflows or is invalid,
  optimization falls back to non-folded runtime evaluation instead of panicking.

5. `R Code Emission`
- Structurize CFG into high-level control shapes (`src/mir/structurizer.rs`).
- Emit R code from structured blocks (`src/codegen/mir_emit.rs`).
- Build RR-to-R source map entries.
- Emit intrinsics as runtime helper calls (`rr_intrinsic_*`) when type-specialized MIR values are present.

6. `Runtime Injection`
- Prepend embedded runtime (`src/runtime/mod.rs`).
- Set source label (`rr_set_source(...)`).
- Set compile-time mode flags in emitted runtime:
  - `rr_set_type_mode("strict|gradual")`
  - `rr_set_native_backend("off|optional|required")`
- Append top-level synthetic invocations.

## Required Validation Stages

RR validates before and after critical MIR phases:

- Semantic validation (`validate_program`)
- Runtime-safety static validation (`validate_runtime_safety`)
- MIR structural verifier (`src/mir/verify.rs`)
- strict type diagnostics (`E1010`, `E1011`, `E1012`) during type analysis

Multiple diagnostics are aggregated and reported together when possible.

## Type/Native/Parallel Configuration Priority

Configuration is resolved in this order:

1. CLI flags (`--type-mode`, `--native-backend`, `--parallel-*`)
2. environment (`RR_TYPE_MODE`, `RR_NATIVE_BACKEND`, `RR_PARALLEL_*`)
3. defaults (`strict`, `off`, `off/auto`)

## Error Flow

The compiler pipeline returns `RR<T>` (`Result<T, RRException>`):

- pipeline layers return structured errors
- CLI decides final process exit code
- the compile core itself does not terminate the process directly

## Legacy IR Path

`src/legacy/ir/*` still exists as a legacy/experimental layer.
Main production pipeline uses HIR -> MIR -> codegen path.
