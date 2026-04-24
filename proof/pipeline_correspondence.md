# Lowering / Codegen Proof ↔ Rust Correspondence

This note ties the reduced lowering/codegen/pipeline proof layers in `proof/`
to the concrete Rust compiler pipeline in `src/mir/`, `src/codegen/`, and
`src/compiler/pipeline/`.

As with [optimizer_correspondence.md](/Users/feral/Desktop/Programming/RR/proof/optimizer_correspondence.md:1),
the goal is pragmatic rather than grand:

- identify which proof file approximates which Rust stage
- make the current abstraction boundary explicit
- keep the next 1:1 connection target obvious

## Lowering

Proof layers:
- [proof/lean/RRProofs/LoweringSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/LoweringSubset.lean:1)
- [proof/coq/LoweringSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/LoweringSubset.v:1)

Core proof claim:
- a reduced source expression fragment
  - const
  - unary neg
  - binary add
  - record literal
  - field access
  lowers to a MIR-like expression fragment without changing evaluation

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:705)
  `MirLowerer::lower_fn`
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  expression and assignment emission into `FnIR`

Current gap:
- proof is expression-centric
- Rust lowering is statement/CFG-producing and tracks names, blocks, and
  side conditions beyond the subset

## If → Phi Lowering

Proof layers:
- [proof/lean/RRProofs/LoweringIfPhiSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/LoweringIfPhiSubset.lean:1)
- [proof/coq/LoweringIfPhiSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/LoweringIfPhiSubset.v:1)
- [proof/coq/LoweringIfPhiGenericSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/LoweringIfPhiGenericSubset.v:1)

Core proof claim:
- reduced source `if` lowers to a MIR-like `phi` join form
- generic and concrete true/false/nested-field cases preserve evaluation

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:466)
  branch lowering scaffolding
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering

Current gap:
- proof models a reduced join expression rather than the full Rust block graph
- Rust lowering also carries ownership metadata and later verifier obligations

## Let / Local Lowering

Proof layers:
- [proof/lean/RRProofs/LoweringLetSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/LoweringLetSubset.lean:1)
- [proof/coq/PipelineLetSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineLetSubset.v:1)
- [proof/coq/PipelineLetGenericSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineLetGenericSubset.v:1)

Core proof claim:
- reduced local reads, field reads, nested field reads, and local `add`
  preserve evaluation through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:558)
  local binding / SSA name setup
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:578)
  recursive variable read reconstruction

Current gap:
- proof does not model full local environment mutation over blocks
- Rust lowering includes shadowing, assignment emission, and CFG placement

## Codegen

Proof layers:
- [proof/lean/RRProofs/CodegenSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/CodegenSubset.lean:1)
- [proof/coq/CodegenSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/CodegenSubset.v:1)
- [proof/coq/CodegenGenericSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/CodegenGenericSubset.v:1)

Core proof claim:
- a reduced MIR-like expression fragment emits to an R-like expression fragment
  without changing evaluation

Primary Rust correspondence:
- [src/codegen/mir_emit.rs](/Users/feral/Desktop/Programming/RR/src/codegen/mir_emit.rs:259)
  structured code emission entry
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured traversal
- [src/codegen/emit/resolve.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/resolve.rs:499)
  expression rendering for index/call forms

Current gap:
- proof covers expression evaluation only
- Rust codegen also performs statement rendering, structured control-flow
  reconstruction, and emitted-R cleanup

## Assign / Phi Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineAssignPhiSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineAssignPhiSubset.lean:1)
- [proof/coq/PipelineAssignPhiSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineAssignPhiSubset.v:1)
- [proof/coq/PipelineAssignPhiGenericSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineAssignPhiGenericSubset.v:1)

Core proof claim:
- reduced branch-local reassignment lowered through a `phi`-merged value and
  then emitted to R-like code still preserves evaluation

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  `Phi` placeholder introduction
- [src/mir/opt/de_ssa.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/de_ssa.rs:1)
  later `Phi` elimination before codegen
- [src/mir/opt.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt.rs:1593)
  de-SSA before codegen

Current gap:
- proof speaks in reduced merged-value semantics
- Rust path spans lowering, verification, de-SSA, and statement emission

## Statement / Program Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineStmtSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineStmtSubset.lean:1)
- [proof/coq/PipelineStmtSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineStmtSubset.v:1)
- [proof/coq/PipelineStmtGenericSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineStmtGenericSubset.v:1)

Core proof claim:
- reduced straight-line and branch/program fragments preserve execution through
  lowering and R-like codegen

Primary Rust correspondence:
- [src/compiler/pipeline/compile_api.rs](/Users/feral/Desktop/Programming/RR/src/compiler/pipeline/compile_api.rs:262)
  `compile_with_pipeline_request`
- [src/compiler/pipeline/phases/source_emit.rs](/Users/feral/Desktop/Programming/RR/src/compiler/pipeline/phases/source_emit.rs:1771)
  source-to-MIR function collection and lowering
- [src/compiler/pipeline/phases/source_emit.rs](/Users/feral/Desktop/Programming/RR/src/compiler/pipeline/phases/source_emit.rs:1063)
  `emit_r_functions_cached`

Current gap:
- proof uses reduced program fragments
- Rust path includes caching, root selection, emitted-R rewrites, and runtime
  injection

## CFG Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineCfgSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineCfgSubset.lean:1)
- [proof/coq/PipelineCfgSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineCfgSubset.v:1)
- [proof/coq/PipelineCfgGenericSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineCfgGenericSubset.v:1)

Core proof claim:
- a tiny explicit `then/else/join` CFG wrapper preserves evaluation through
  lowering and emission

Primary Rust correspondence:
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:272)
  CFG-side structural obligations before emission
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured CFG-to-R reconstruction
- [src/compiler/pipeline/compile_api.rs](/Users/feral/Desktop/Programming/RR/src/compiler/pipeline/compile_api.rs:444)
  `verify_emittable_program`

Current gap:
- proof uses tiny explicit CFG fragments
- Rust path still includes richer loop/block metadata and emitted-R cleanup

## Block / Env Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineBlockEnvSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineBlockEnvSubset.lean:1)
- [proof/coq/PipelineBlockEnvSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineBlockEnvSubset.v:1)

Core proof claim:
- a reduced explicit block shell carrying block id, incoming local
  environment, ordered statements, and return expression preserves evaluation
  through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:558)
  local binding / read environment setup
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/mir_emit.rs](/Users/feral/Desktop/Programming/RR/src/codegen/mir_emit.rs:259)
  structured function/block emission entry

Current gap:
- proof still uses one ordered block shell rather than a real CFG of blocks
- Rust path still tracks SSA ids, block ownership, verifier side conditions,
  and emitted-R cleanup beyond the subset

## Function / Env Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnEnvSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnEnvSubset.lean:1)
- [proof/coq/PipelineFnEnvSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnEnvSubset.v:1)

Core proof claim:
- a reduced explicit function shell carrying
  - function name
  - entry/body-head metadata
  - ordered block/env list
  preserves per-block results through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:705)
  `MirLowerer::lower_fn`
- [src/codegen/mir_emit.rs](/Users/feral/Desktop/Programming/RR/src/codegen/mir_emit.rs:252)
  function emission entry
- [src/compiler/pipeline/compile_api.rs](/Users/feral/Desktop/Programming/RR/src/compiler/pipeline/compile_api.rs:263)
  top-level compile pipeline entry

Current gap:
- proof still uses ordered block lists instead of a real predecessor graph
- Rust path still includes verifier obligations, SSA ids, and structured CFG
  reconstruction beyond the subset

## Function / CFG Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgSubset.lean:1)
- [proof/coq/PipelineFnCfgSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgSubset.v:1)

Core proof claim:
- a reduced explicit function shell carrying
  - function metadata
  - predecessor map
  - ordered block/env list
  preserves reduced per-block results through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:705)
  `MirLowerer::lower_fn`
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:466)
  branch/block scaffolding
- [src/codegen/mir_emit.rs](/Users/feral/Desktop/Programming/RR/src/codegen/mir_emit.rs:252)
  function emission entry

Current gap:
- proof now carries predecessor data, but still not a real CFG execution
- Rust path still tracks SSA ids, phi placeholders, verifier obligations, and
  structured CFG reconstruction beyond the subset

## Function / CFG Execution Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgExecSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgExecSubset.lean:1)
- [proof/coq/PipelineFnCfgExecSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgExecSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell plus an explicit execution path
  witness preserves selected-path results through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:466)
  branch/block scaffolding
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:705)
  `MirLowerer::lower_fn`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured CFG-to-R reconstruction

Current gap:
- proof now carries a selected path witness, but still not a real small-step
  CFG execution semantics
- Rust path still includes phi placeholders, verifier obligations, and
  structure recovery beyond the subset

## Function / CFG Small-Step Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgSmallStepSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgSmallStepSubset.lean:1)
- [proof/coq/PipelineFnCfgSmallStepSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgSmallStepSubset.v:1)

Core proof claim:
- a reduced tiny trace machine over the selected CFG path preserves execution
  trace results through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:466)
  branch/block scaffolding
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured CFG-to-R reconstruction

Current gap:
- proof now has a reduced small-step trace, but still not the full concrete
  `FnIR` block/value operational semantics
- Rust path still includes phi placeholders, verifier obligations, and richer
  structure recovery beyond the subset

## Function / CFG Branch-Exec Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgBranchExecSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgBranchExecSubset.lean:1)
- [proof/coq/PipelineFnCfgBranchExecSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgBranchExecSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit `then` / `else`
  path choice preserves the chosen branch trace through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:466)
  branch/block scaffolding
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured CFG branch reconstruction

Current gap:
- proof now carries explicit branch choice, but still not a reduced `phi` /
  join merge operational semantics for converging paths
- Rust path still includes verifier obligations, phi placeholders, and richer
  structured reconstruction beyond the subset

## Function / CFG Phi-Exec Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgPhiExecSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgPhiExecSubset.lean:1)
- [proof/coq/PipelineFnCfgPhiExecSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgPhiExecSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit branch choice and a
  reduced `phi`/join merge result preserves the chosen merged result through
  lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction

Current gap:
- proof now carries a reduced join-merge result, but still not a full reduced
  operational semantics for branch execution plus explicit join block state
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Join-State Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgJoinStateSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgJoinStateSubset.lean:1)
- [proof/coq/PipelineFnCfgJoinStateSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgJoinStateSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit branch choice,
  reduced `phi`/join merge, and explicit join-local environment preserves the
  join-state result through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/opt/de_ssa.rs](/Users/feral/Desktop/Programming/RR/src/mir/opt/de_ssa.rs:1)
  later join-state realization before codegen
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction

Current gap:
- proof now carries explicit join-local state, but still not a reduced
  operational semantics for whole-CFG join block execution after merge
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Join-Exec Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgJoinExecSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgJoinExecSubset.lean:1)
- [proof/coq/PipelineFnCfgJoinExecSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgJoinExecSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit join-local state and
  explicit join block execution (`join stmts + join ret`) preserves the
  join-block result through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured join-block reconstruction

Current gap:
- proof now carries explicit join-block execution, but still not a reduced
  whole-CFG operational semantics with explicit post-join continuation
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Post-Join Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgPostJoinSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgPostJoinSubset.lean:1)
- [proof/coq/PipelineFnCfgPostJoinSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgPostJoinSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit join block
  execution plus an explicit post-join continuation block preserves the
  continuation result through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured join-block reconstruction and follow-up block emission

Current gap:
- proof now carries explicit post-join continuation, but still not a reduced
  whole-CFG operational semantics for iterative multi-block execution after
  join
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Iterative Post-Join Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgIterExecSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgIterExecSubset.lean:1)
- [proof/coq/PipelineFnCfgIterExecSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgIterExecSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit join-block
  execution and an ordered list of post-join continuation blocks preserves the
  final continuation result through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured join-block reconstruction and post-join ordered emission

Current gap:
- proof now carries reduced iterative post-join execution, but still not a
  whole-CFG operational semantics for repeated graph transitions driven by
  explicit branch/join control state
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Control-State Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgControlStateSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgControlStateSubset.lean:1)
- [proof/coq/PipelineFnCfgControlStateSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgControlStateSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit join execution and
  an explicit control-state transition machine for post-join continuation
  preserves the final continuation result through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction with follow-up control flow

Current gap:
- proof now carries explicit post-join control-state transitions, but still
  not a richer reduced semantics for arbitrary graph re-entry, loops, or
  repeated branch/join cycling
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Graph-State Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgGraphStateSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgGraphStateSubset.lean:1)
- [proof/coq/PipelineFnCfgGraphStateSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgGraphStateSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit join execution and
  an explicit `pc + step-table` graph-state shell preserves the final
  continuation result through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction with ordered follow-up control flow

Current gap:
- proof now carries explicit graph-state execution, but still not a richer
  reduced semantics for loops, arbitrary re-entry, or repeated branch/join
  cycling over non-linear control graphs
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Reentry Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgReentrySubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgReentrySubset.lean:1)
- [proof/coq/PipelineFnCfgReentrySubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgReentrySubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit join execution and
  an explicit re-entry trace over step indices preserves the final
  continuation result through lowering and emission, even when the same
  continuation step is revisited

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction with follow-up control-flow revisits

Current gap:
- proof now carries explicit re-entry / revisit traces, but still not a richer
  reduced semantics for arbitrary loop headers, fixed-point branch cycling, or
  non-trace-driven graph exploration
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Loop-Cycle Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgLoopCycleSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgLoopCycleSubset.lean:1)
- [proof/coq/PipelineFnCfgLoopCycleSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgLoopCycleSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit re-entry traces and
  repeated branch/join cycles over an accumulator-like loop state preserves
  the final cycle result through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction with repeated follow-up control flow

Current gap:
- proof now carries repeated branch/join cycle iteration, but still not a
  richer reduced semantics for open-ended fixed-point convergence or general
  loop-header graph execution
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Loop-Fixpoint Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgLoopFixpointSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgLoopFixpointSubset.lean:1)
- [proof/coq/PipelineFnCfgLoopFixpointSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgLoopFixpointSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with repeated branch/join cycles
  and an explicit stability witness preserves that fixed-point witness through
  lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit fixed-point witness, but still not a richer
  reduced semantics for automatic convergence discovery or general loop-header
  fixed-point computation
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Loop-Discover Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgLoopDiscoverSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgLoopDiscoverSubset.lean:1)
- [proof/coq/PipelineFnCfgLoopDiscoverSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgLoopDiscoverSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with an explicit worklist and a
  selected stable candidate preserves that discovery result through lowering
  and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit discovery/worklist shell, but still not a
  reduced semantics for automatically updating the candidate set or proving
  convergence discovery by iteration
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Loop-Worklist Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgLoopWorklistSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgLoopWorklistSubset.lean:1)
- [proof/coq/PipelineFnCfgLoopWorklistSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgLoopWorklistSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with an explicit worklist
  selection and a `pending -> done` update shell preserves that updated
  worklist state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit worklist update shell, but still not a reduced
  semantics for multiple update rounds, candidate insertion, or automatic
  convergence discovery by iteration
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Loop-Queue Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgLoopQueueSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgLoopQueueSubset.lean:1)
- [proof/coq/PipelineFnCfgLoopQueueSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgLoopQueueSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with an ordered queue of worklist
  rounds preserves the resulting drained update list through lowering and
  emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit queue drain shell, but still not a reduced
  semantics for candidate insertion, priority changes, or automatic multi-round
  convergence discovery by iteration
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Loop-Scheduler Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgLoopSchedulerSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgLoopSchedulerSubset.lean:1)
- [proof/coq/PipelineFnCfgLoopSchedulerSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgLoopSchedulerSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with an ordered scheduler of queue
  batches preserves the resulting batch-evaluation trace through lowering and
  emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit scheduler shell, but still not a reduced
  semantics for dynamic queue growth, priority changes, or automatic
  convergence discovery by repeated scheduling
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Loop-Dynamic-Scheduler Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgLoopDynamicSchedulerSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgLoopDynamicSchedulerSubset.lean:1)
- [proof/coq/PipelineFnCfgLoopDynamicSchedulerSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgLoopDynamicSchedulerSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit scheduler reinserts
  preserves the resulting dynamically scheduled batch trace through lowering
  and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries explicit reinsertion batches, but still not a reduced
  semantics for priority-based insertion, dynamic candidate growth, or
  automatic convergence discovery by repeated scheduling
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Loop-Priority Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgLoopPrioritySubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgLoopPrioritySubset.lean:1)
- [proof/coq/PipelineFnCfgLoopPrioritySubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgLoopPrioritySubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit priority labels over
  pending and reinserted batches preserves the resulting priority-labeled
  scheduler trace through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries explicit priority labels, but still not a reduced
  semantics for dynamic priority recomputation or policy-driven candidate
  promotion by repeated scheduling
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Loop-Policy Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgLoopPolicySubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgLoopPolicySubset.lean:1)
- [proof/coq/PipelineFnCfgLoopPolicySubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgLoopPolicySubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit priority-rewrite
  rules preserves the resulting policy-normalized scheduler trace through
  lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries explicit priority-rewrite policy, but still not a reduced
  semantics for policy recomputation driven by newly discovered costs or
  repeated scheduler feedback
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Loop-Adaptive-Policy Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgLoopAdaptivePolicySubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgLoopAdaptivePolicySubset.lean:1)
- [proof/coq/PipelineFnCfgLoopAdaptivePolicySubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgLoopAdaptivePolicySubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with feedback-driven priority rule
  recomputation preserves the resulting adaptive policy trace through lowering
  and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries explicit feedback-driven rule recomputation, but still not
  a reduced semantics for closed-loop policy learning or repeated feedback
  adaptation across multiple scheduler rounds
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Loop-Closed-Loop Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgLoopClosedLoopSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgLoopClosedLoopSubset.lean:1)
- [proof/coq/PipelineFnCfgLoopClosedLoopSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgLoopClosedLoopSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with repeated adaptive feedback
  rounds preserves the resulting closed-loop adaptive trace through lowering
  and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries repeated adaptive rounds, but still not a reduced
  semantics for open-ended learning loops, policy saturation, or repeated
  adaptive convergence discovery
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Loop-Meta-Iteration Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgLoopMetaIterSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgLoopMetaIterSubset.lean:1)
- [proof/coq/PipelineFnCfgLoopMetaIterSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgLoopMetaIterSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with repeated adaptive rounds and
  an explicit last-summary witness preserves that meta-iteration summary
  through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit meta-iteration summary shell, but still not a
  reduced semantics for discovering that summary by open-ended convergence
  rather than reading it from a bounded closed-loop trace
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Summary-Protocol Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgSummaryProtocolSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgSummaryProtocolSubset.lean:1)
- [proof/coq/PipelineFnCfgSummaryProtocolSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgSummaryProtocolSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with repeated meta-iteration
  rounds and an explicit stable-summary protocol preserves that summary trace
  through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit summary protocol shell, but still not a
  reduced semantics for discovering stability by open-ended convergence rather
  than carrying a bounded summary trace
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Convergence-Protocol Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgConvergenceProtocolSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgConvergenceProtocolSubset.lean:1)
- [proof/coq/PipelineFnCfgConvergenceProtocolSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgConvergenceProtocolSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with an explicit
  `summary unchanged => halt` witness preserves the resulting convergence
  summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit convergence protocol shell, but still not a
  reduced semantics for discovering the halt condition by open-ended dynamic
  search rather than transporting a bounded witness
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Halt-Discover Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgHaltDiscoverSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgHaltDiscoverSubset.lean:1)
- [proof/coq/PipelineFnCfgHaltDiscoverSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgHaltDiscoverSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with an explicit halt-search
  shell preserves the discovered halt summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit halt-discovery shell, but still not a reduced
  semantics for open-ended halt search or dynamic convergence discovery beyond
  a bounded search space
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with an explicit
  `completed + frontier` worklist shell preserves the discovered halt summary
  through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit pending/completed open-search shell, but still
  not a reduced semantics for dynamic frontier growth or repeated open-ended
  queue discovery beyond one split worklist
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Dynamic Open-Search Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgDynamicOpenSearchSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgDynamicOpenSearchSubset.lean:1)
- [proof/coq/PipelineFnCfgDynamicOpenSearchSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgDynamicOpenSearchSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit frontier growth
  preserves the discovered halt summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one explicit dynamic frontier-growth step, but still not a
  reduced semantics for repeated open-ended queue discovery or policy-guided
  expansion beyond one update
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Scheduler Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchSchedulerSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchSchedulerSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchSchedulerSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchSchedulerSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit scheduled
  open-search rounds preserves the discovered halt summary through lowering
  and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries repeated scheduled open-search rounds, but still not a
  reduced semantics for adaptive queue reordering or policy-guided scheduling
  beyond one fixed schedule shell
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Priority Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchPrioritySubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchPrioritySubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchPrioritySubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchPrioritySubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit priority-labeled
  open-search rounds preserves the discovered halt summary through lowering
  and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries priority-labeled open-search rounds, but still not a
  reduced semantics for adaptive reprioritization or feedback-driven
  open-search policy updates beyond one tagged schedule shell
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Policy Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchPolicySubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchPolicySubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchPolicySubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchPolicySubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit priority-rewrite
  rules over open-search rounds preserves the discovered halt summary through
  lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one explicit priority-rewrite policy step, but still not a
  reduced semantics for feedback-driven adaptive reprioritization or repeated
  policy updates beyond one rewrite shell
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Adaptive Policy Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchAdaptivePolicySubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchAdaptivePolicySubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchAdaptivePolicySubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchAdaptivePolicySubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with feedback-driven
  recomputation of open-search priority rules preserves the discovered halt
  summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one explicit adaptive-rule recomputation step, but still
  not a reduced semantics for repeated closed-loop reprioritization or
  convergence of adaptive open-search policy updates
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Closed-Loop Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchClosedLoopSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchClosedLoopSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchClosedLoopSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchClosedLoopSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with repeated closed-loop
  adaptive-policy rounds preserves the discovered halt summary trace through
  lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries repeated closed-loop open-search rounds, but still not a
  reduced semantics for meta-iteration or explicit convergence discovery over
  adaptive open-search summaries
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Meta-Iteration Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchMetaIterSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchMetaIterSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchMetaIterSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchMetaIterSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with last-summary extraction over
  repeated open-search closed-loop rounds preserves the discovered halt
  summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries last-summary extraction, but still not a reduced
  convergence protocol or explicit halt-discovery layer for repeated
  open-search summaries
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Summary-Protocol Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchSummaryProtocolSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchSummaryProtocolSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchSummaryProtocolSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchSummaryProtocolSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit stable-summary
  protocol rounds preserves the discovered halt summary through lowering and
  emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries stable-summary protocol rounds, but still not a reduced
  convergence protocol or explicit halt witness for repeated open-search
  summaries
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Convergence-Protocol Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchConvergenceProtocolSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchConvergenceProtocolSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchConvergenceProtocolSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchConvergenceProtocolSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit
  `summary unchanged => halt` witness over repeated open-search summaries
  preserves the discovered halt summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit convergence shell, but still not a reduced
  halt-discovery/search-space layer for repeated open-search summaries
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Halt-Discover Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchHaltDiscoverSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchHaltDiscoverSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchHaltDiscoverSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchHaltDiscoverSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with an explicit halt-search
  shell over repeated open-search summaries preserves the discovered halt
  summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit halt-discovery shell, but still not a reduced
  open-ended search/worklist layer for repeated open-search summaries
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Frontier Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchFrontierSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchFrontierSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell that reopens the discovered halt
  summary into an explicit `completed + frontier` shell preserves the
  resulting frontier state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries a reopened frontier shell, but still not a reduced
  dynamic-growth/update layer for repeated open-search frontier evolution
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Dynamic-Frontier Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchDynamicFrontierSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchDynamicFrontierSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchDynamicFrontierSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchDynamicFrontierSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit frontier growth
  after reopening the halt-discovered summary preserves the resulting
  frontier state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one explicit dynamic frontier-growth step, but still not a
  reduced scheduler/policy layer for repeated reopened-frontier evolution
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Frontier-Scheduler Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierSchedulerSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierSchedulerSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchFrontierSchedulerSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchFrontierSchedulerSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit scheduled rounds
  over reopened dynamic frontier states preserves the resulting frontier
  state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries repeated scheduled frontier rounds, but still not a
  reduced priority/policy layer for reopened-frontier evolution
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Frontier-Priority Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierPrioritySubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierPrioritySubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchFrontierPrioritySubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchFrontierPrioritySubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit priority-labeled
  reopened-frontier rounds preserves the resulting frontier state through
  lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries priority-labeled frontier rounds, but still not a reduced
  policy/adaptive reprioritization layer for reopened-frontier evolution
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Frontier-Policy Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierPolicySubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierPolicySubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchFrontierPolicySubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchFrontierPolicySubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit priority-rewrite
  rules over reopened-frontier rounds preserves the resulting frontier state
  through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one explicit frontier-policy rewrite step, but still not a
  reduced adaptive reprioritization layer for reopened-frontier evolution
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Frontier-Adaptive-Policy Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierAdaptivePolicySubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierAdaptivePolicySubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchFrontierAdaptivePolicySubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchFrontierAdaptivePolicySubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with feedback-driven
  recomputation of reopened-frontier priority rules preserves the resulting
  frontier state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one explicit adaptive frontier-policy recomputation step,
  but still not a reduced closed-loop or meta-iteration layer for reopened
  frontier evolution
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Frontier-Closed-Loop Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierClosedLoopSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierClosedLoopSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchFrontierClosedLoopSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchFrontierClosedLoopSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with repeated closed-loop rounds
  over reopened frontier-adaptive-policy states preserves the resulting
  frontier trace through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries repeated closed-loop frontier rounds, but still not a
  reduced meta-iteration or convergence-discovery layer for reopened frontier
  evolution
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Frontier-Meta-Iteration Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierMetaIterSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierMetaIterSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchFrontierMetaIterSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchFrontierMetaIterSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with last-summary extraction over
  repeated reopened-frontier closed-loop rounds preserves the resulting
  frontier summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries last-summary extraction, but still not a reopened-frontier
  summary/convergence protocol or explicit halt-discovery layer
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Frontier-Summary-Protocol Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierSummaryProtocolSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierSummaryProtocolSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchFrontierSummaryProtocolSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchFrontierSummaryProtocolSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit stable-summary
  protocol rounds over reopened frontier evolution preserves the resulting
  frontier summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries stable-summary protocol rounds, but still not a reopened
  frontier convergence protocol or explicit halt-discovery/search-space layer
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Frontier-Convergence-Protocol Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierConvergenceProtocolSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierConvergenceProtocolSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchFrontierConvergenceProtocolSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchFrontierConvergenceProtocolSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit
  `summary unchanged => halt` witness over reopened frontier summaries
  preserves the resulting frontier halt summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit frontier convergence shell, but still not a
  reopened-frontier halt-discovery/search-space layer
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Frontier-Halt-Discover Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierHaltDiscoverSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierHaltDiscoverSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchFrontierHaltDiscoverSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchFrontierHaltDiscoverSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with an explicit halt-search
  shell over reopened frontier summaries preserves the discovered halt
  summary through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries an explicit reopened-frontier halt-discovery shell, but
  still not a reopened-frontier reopen/update layer that starts a new search
  cycle
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Frontier-Reopen Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchFrontierReopenSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchFrontierReopenSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell that reopens the
  halt-discovered frontier summary into an explicit `completed + frontier`
  shell preserves the resulting frontier state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries a reopened frontier shell after halt discovery, but still
  not the next dynamic-growth/update layer that restarts search over that
  reopened frontier
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Frontier-Reopen-Dynamic Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenDynamicSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenDynamicSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchFrontierReopenDynamicSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchFrontierReopenDynamicSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit frontier growth
  after reopening the halt-discovered frontier preserves the resulting
  frontier state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one reopened-frontier dynamic-growth step, but still not a
  repeated scheduler/policy layer over the restarted frontier cycle
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Frontier-Reopen-Scheduler Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenSchedulerSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenSchedulerSubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchFrontierReopenSchedulerSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchFrontierReopenSchedulerSubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with an explicit scheduled-round
  wrapper over reopened-frontier dynamic growth preserves the resulting
  frontier state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one reopened-frontier scheduled-round step, but still not
  priority/policy/adaptive layers over the restarted frontier cycle
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Frontier-Reopen-Priority Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenPrioritySubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenPrioritySubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchFrontierReopenPrioritySubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchFrontierReopenPrioritySubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit priority-tagged
  reopened-frontier scheduled rounds preserves the resulting frontier state
  through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one reopened-frontier priority step, but still not
  policy/adaptive layers over the restarted frontier cycle
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Function / CFG Open-Search Frontier-Reopen-Policy Pipeline

Proof layers:
- [proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenPolicySubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/PipelineFnCfgOpenSearchFrontierReopenPolicySubset.lean:1)
- [proof/coq/PipelineFnCfgOpenSearchFrontierReopenPolicySubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/PipelineFnCfgOpenSearchFrontierReopenPolicySubset.v:1)

Core proof claim:
- a reduced predecessor-aware function shell with explicit policy-rewrite
  normalization over reopened-frontier priority rounds preserves the resulting
  frontier state through lowering and emission

Primary Rust correspondence:
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:588)
  cycle-breaking `Phi` placeholders during lowering
- [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:773)
  statement emission into `FnIR`
- [src/codegen/emit/structured.rs](/Users/feral/Desktop/Programming/RR/src/codegen/emit/structured.rs:25)
  structured branch/join reconstruction around stabilized follow-up flow

Current gap:
- proof now carries one reopened-frontier policy-normalization step, but still
  not adaptive or closed-loop layers over the restarted frontier cycle
- Rust path still includes verifier obligations, phi placeholders, de-SSA, and
  richer structured reconstruction beyond the subset

## Immediate Next Steps

The most direct next connection points are:

1. attach short Rust source comments at
   [src/mir/lower_hir.rs](/Users/feral/Desktop/Programming/RR/src/mir/lower_hir.rs:705),
   [src/codegen/mir_emit.rs](/Users/feral/Desktop/Programming/RR/src/codegen/mir_emit.rs:259),
   and
   [src/compiler/pipeline/compile_api.rs](/Users/feral/Desktop/Programming/RR/src/compiler/pipeline/compile_api.rs:262)
   that point at the matching proof files
2. lift one reduced proof from expression-level semantics to a small
   block/value environment model closer to real `FnIR`
3. continue moving Coq generic theorems into stable `*GenericSubset.v`
   companions where Rocq compile pathology makes in-file generic proofs brittle
