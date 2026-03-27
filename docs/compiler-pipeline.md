# Compiler Pipeline

This page is the end-to-end compile manual for RR.

Primary implementation entrypoint:

- `src/compiler/pipeline.rs::compile_with_configs()`

CLI entrypoints in `src/main.rs` call into this pipeline API.

## Audience

Read this page when you need to understand:

- where a diagnostic originated
- which phase owns a transformation
- which invariants must hold between phases

## Pipeline Synopsis

`RR source`
`-> parse`
`-> HIR lowering`
`-> HIR canonicalization`
`-> MIR synthesis`
`-> type analysis`
`-> Tachyon optimization or stabilization`
`-> structurization`
`-> MIR-to-R emission`
`-> runtime subset injection`
`-> self-contained .R artifact`

## Pipeline Contract

Inputs:

- source text
- optimization level
- type/native/parallel policy
- incremental policy

Outputs:

- emitted `.R`
- source map
- structured diagnostics on failure

Hard rules:

- parser/HIR/MIR errors are reported as structured `RRException`
- codegen must see phi-free MIR
- runtime injection must emit only referenced helper subsets plus required bootstrap
- incremental reuse must not silently change final emitted semantics

## Phase Boundary Invariants

Important boundaries to keep in mind:

- HIR canonicalization should remove surface sugar before MIR lowering
- MIR verification and type analysis operate on SSA-like MIR, not emitted R
- codegen should only see de-SSA, structurized MIR
- runtime injection should never invent semantics that are absent from emitted R

## Phase Table

| Phase | Purpose | Main implementation |
| --- | --- | --- |
| Source Analysis | parse modules, imports, HIR lowering | `src/syntax`, `src/hir` |
| Canonicalization | desugar surface forms | `src/hir/desugar.rs` |
| SSA Graph Synthesis | build MIR CFG and SSA values | `src/mir/lower_hir.rs` |
| Type Analysis | infer/prove MIR value states | `src/typeck/solver.rs` |
| Tachyon | optimize or stabilize MIR | `src/mir/opt.rs` |
| R Code Emission | structurize CFG and print R | `src/mir/structurizer.rs`, `src/codegen/mir_emit.rs` |
| Runtime Injection | prepend helper subset and policy | `src/runtime`, `src/compiler/pipeline.rs` |

## Detailed Phase Notes

### 1. Source Analysis

RR:

- parses each module
- resolves imports
- lowers AST to HIR
- collects symbol and function-arity tables

Relevant paths:

- `src/syntax`
- `src/hir/lower.rs`

### 2. Canonicalization

HIR desugaring normalizes:

- compound assignments
- shorthand function forms
- some surface sugar before MIR lowering

Relevant path:

- `src/hir/desugar.rs`

### 3. SSA Graph Synthesis

HIR functions lower into MIR (`FnIR`):

- blocks
- SSA-like values
- deferred phi placeholders
- explicit stores/evals/returns

Relevant path:

- `src/mir/lower_hir.rs`

### 4. Type Analysis

Type analysis computes:

- `FnIR.inferred_ret_ty`
- `FnIR.inferred_ret_term`
- per-value type state and structural type term

This stage feeds:

- guard removal
- intrinsic selection
- runtime check elision
- typed parallel wrapper eligibility

Relevant paths:

- `src/typeck/solver.rs`
- `src/typeck/term.rs`

### 5. Tachyon

`-O0` runs stabilization only.

`-O1/-O2` run the optimizing pipeline:

- always tier
- selective heavy tier
- bounded interprocedural inlining
- global de-SSA before emission

Relevant path:

- `src/mir/opt.rs`

Detailed optimizer behavior lives in [Tachyon Engine](optimization.md).

### 6. R Code Emission

Emission is split into:

- CFG structurization
- expression/instruction lowering
- source-map generation
- emitted-R cleanup and shape canonicalization

Relevant paths:

- `src/mir/structurizer.rs`
- `src/codegen/mir_emit.rs`
- `src/compiler/r_peephole.rs`

The final emitted-R cleanup stage is intentionally conservative. It exists to
improve readability and stabilize artifact shape without changing meaning. In
practice this stage is where RR now fixes or canonicalizes items such as:

- stale tail writebacks after loop-carried whole-slice updates
- repeated fresh-allocation replays that would overwrite computed state
- dead `.arg_*` parameter aliases in helper-style wrappers
- immediate scalar/index temporaries that are used once and can be inlined
- trivial loop index aliases such as `ii <- i`

This stage is also where many emitted-artifact regression fences are anchored,
so a change that regresses generated R shape should usually be documented and
covered here rather than treated as an optimizer-only issue.

By default the emitted artifact is treated as a whole-program result rather than
a definition-preserving transpilation dump. That means:

- the pipeline may trim unreachable top-level `Sym_*` definitions
- emitted-R cleanup may strip helper definitions that are not reachable from the
  synthesized entry closure

If the caller needs a source-preserving artifact, use
`CompileOutputOptions { preserve_all_defs: true, .. }` or the CLI flag
`--preserve-all-defs`.

### 7. Runtime Injection

RR scans emitted R for referenced `rr_*` symbols and injects:

- the minimal helper subset needed by the artifact
- transitive helper dependencies
- bootstrap state in `.rr_env`
- compile-time policy assignments

Relevant paths:

- `src/runtime/runtime_prelude.R`
- `src/runtime/source.rs`
- `src/runtime/subset.rs`

## O0 vs O1/O2

`-O0` is not “raw MIR dumped to R”.

It still runs mandatory codegen-safety work:

- helper canonicalization needed for valid emission
- de-SSA
- cleanup after de-SSA

`-O1/-O2` additionally run optimization passes that may:

- vectorize loops
- reduce loops
- eliminate checks when proven safe
- inline and simplify interprocedurally

## Type and Backend Configuration Priority

Configuration resolves in this order:

1. explicit CLI flags or explicit API config values
2. defaults

The emitted artifact then appends compile-time policy values directly so the
final runtime behavior matches the compile that produced it without consulting
ambient backend or parallel environment overrides.

## Incremental Compile

Incremental compile is implemented in `src/compiler/incremental.rs`.

The important policy points are:

- default CLI mode is `auto`
- normal compile uses phase 1 and phase 2 when possible
- live sessions such as `watch` may also use phase 3 memory reuse
- `--strict-incremental-verify` rebuilds and compares cached output instead of trusting it blindly

## Validation Boundaries

RR validates at important boundaries:

- semantic validation
- runtime-safety static validation
- MIR verifier
- post-optimization/codegen-ready invariants

If a boundary fails, the pipeline is expected to reject with diagnostics instead
of silently emitting a best-effort artifact.
