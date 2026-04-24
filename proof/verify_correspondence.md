# Verify Proof ↔ Rust Correspondence

This note ties the reduced verifier proof layers in `proof/` to the concrete
Rust verifier in [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:1).

The goal is the same as the other correspondence notes:

- identify which proof layer approximates which Rust check group
- make the current abstraction boundary explicit
- keep the next 1:1 refinement target obvious

## Struct Layer

Proof layers:
- [proof/lean/RRProofs/VerifyIrStructLite.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrStructLite.lean:1)
- [proof/coq/VerifyIrStructLite.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrStructLite.v:1)

Core proof claim:
- `body_head` must be reachable
- self-recursive `body_head != entry` functions must have a direct entry edge
  and param-copy-only entry prologue
- entry root / branch target / loop-header shape invariants hold
- `Phi` ownership, predecessor distinctness, and edge-availability hold
- parameter index / call-name / self-reference / non-`Phi` cycle checks hold

Primary Rust correspondence:
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:272)
  entry/body-head and loop-header structural checks
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:526)
  `Phi` shape against CFG predecessors
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:307)
  self-recursive entry prologue restriction

Current gap:
- proof packages booleans/flags rather than the full Rust `FnIR`
- Rust verifier also reasons about exact predecessor sets and inferred owner
  blocks on the concrete CFG

## Flow Layer

Proof layers:
- [proof/lean/RRProofs/VerifyIrMustDefSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrMustDefSubset.lean:1)
- [proof/coq/VerifyIrMustDefSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrMustDefSubset.v:1)
- [proof/lean/RRProofs/VerifyIrMustDefFixedPointSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrMustDefFixedPointSubset.lean:1)
- [proof/coq/VerifyIrMustDefFixedPointSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrMustDefFixedPointSubset.v:1)
- [proof/lean/RRProofs/VerifyIrMustDefConvergenceSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrMustDefConvergenceSubset.lean:1)
- [proof/coq/VerifyIrMustDefConvergenceSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrMustDefConvergenceSubset.v:1)
- [proof/lean/RRProofs/VerifyIrUseTraversalSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrUseTraversalSubset.lean:1)
- [proof/coq/VerifyIrUseTraversalSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrUseTraversalSubset.v:1)
- [proof/lean/RRProofs/VerifyIrValueKindTraversalSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrValueKindTraversalSubset.lean:1)
- [proof/coq/VerifyIrValueKindTraversalSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrValueKindTraversalSubset.v:1)
- [proof/lean/RRProofs/VerifyIrArgListTraversalSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrArgListTraversalSubset.lean:1)
- [proof/coq/VerifyIrArgListTraversalSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrArgListTraversalSubset.v:1)
- [proof/lean/RRProofs/VerifyIrEnvScanComposeSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrEnvScanComposeSubset.lean:1)
- [proof/coq/VerifyIrEnvScanComposeSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrEnvScanComposeSubset.v:1)
- [proof/lean/RRProofs/VerifyIrConsumerMetaSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrConsumerMetaSubset.lean:1)
- [proof/coq/VerifyIrConsumerMetaSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrConsumerMetaSubset.v:1)
- [proof/lean/RRProofs/VerifyIrConsumerGraphSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrConsumerGraphSubset.lean:1)
- [proof/coq/VerifyIrConsumerGraphSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrConsumerGraphSubset.v:1)
- [proof/lean/RRProofs/VerifyIrChildDepsSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrChildDepsSubset.lean:1)
- [proof/coq/VerifyIrChildDepsSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrChildDepsSubset.v:1)
- [proof/lean/RRProofs/VerifyIrValueDepsWalkSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrValueDepsWalkSubset.lean:1)
- [proof/coq/VerifyIrValueDepsWalkSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrValueDepsWalkSubset.v:1)
- [proof/lean/RRProofs/VerifyIrValueTableWalkSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrValueTableWalkSubset.lean:1)
- [proof/coq/VerifyIrValueTableWalkSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrValueTableWalkSubset.v:1)
- [proof/lean/RRProofs/VerifyIrValueKindTableSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrValueKindTableSubset.lean:1)
- [proof/coq/VerifyIrValueKindTableSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrValueKindTableSubset.v:1)
- [proof/lean/RRProofs/VerifyIrValueRecordSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrValueRecordSubset.lean:1)
- [proof/coq/VerifyIrValueRecordSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrValueRecordSubset.v:1)
- [proof/lean/RRProofs/VerifyIrValueFullRecordSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrValueFullRecordSubset.lean:1)
- [proof/coq/VerifyIrValueFullRecordSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrValueFullRecordSubset.v:1)
- [proof/lean/RRProofs/VerifyIrFnRecordSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrFnRecordSubset.lean:1)
- [proof/coq/VerifyIrFnRecordSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrFnRecordSubset.v:1)
- [proof/lean/RRProofs/VerifyIrFnMetaSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrFnMetaSubset.lean:1)
- [proof/coq/VerifyIrFnMetaSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrFnMetaSubset.v:1)
- [proof/lean/RRProofs/VerifyIrFnParamMetaSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrFnParamMetaSubset.lean:1)
- [proof/coq/VerifyIrFnParamMetaSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrFnParamMetaSubset.v:1)
- [proof/lean/RRProofs/VerifyIrFnHintMapSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrFnHintMapSubset.lean:1)
- [proof/coq/VerifyIrFnHintMapSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrFnHintMapSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockRecordSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockRecordSubset.lean:1)
- [proof/coq/VerifyIrBlockRecordSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockRecordSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockFlowSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockFlowSubset.lean:1)
- [proof/coq/VerifyIrBlockFlowSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockFlowSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockMustDefSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockMustDefSubset.lean:1)
- [proof/coq/VerifyIrBlockMustDefSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockMustDefSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockMustDefComposeSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockMustDefComposeSubset.lean:1)
- [proof/coq/VerifyIrBlockMustDefComposeSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockMustDefComposeSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockAssignFlowSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockAssignFlowSubset.lean:1)
- [proof/coq/VerifyIrBlockAssignFlowSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockAssignFlowSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockAssignChainSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockAssignChainSubset.lean:1)
- [proof/coq/VerifyIrBlockAssignChainSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockAssignChainSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockAssignBranchSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockAssignBranchSubset.lean:1)
- [proof/coq/VerifyIrBlockAssignBranchSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockAssignBranchSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockAssignStoreSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockAssignStoreSubset.lean:1)
- [proof/coq/VerifyIrBlockAssignStoreSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockAssignStoreSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockDefinedHereSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockDefinedHereSubset.lean:1)
- [proof/coq/VerifyIrBlockDefinedHereSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockDefinedHereSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockExecutableSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockExecutableSubset.lean:1)
- [proof/coq/VerifyIrBlockExecutableSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockExecutableSubset.v:1)
- [proof/lean/RRProofs/VerifyIrTwoBlockExecutableSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrTwoBlockExecutableSubset.lean:1)
- [proof/coq/VerifyIrTwoBlockExecutableSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrTwoBlockExecutableSubset.v:1)
- [proof/lean/RRProofs/VerifyIrJoinExecutableSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrJoinExecutableSubset.lean:1)
- [proof/coq/VerifyIrJoinExecutableSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrJoinExecutableSubset.v:1)
- [proof/lean/RRProofs/VerifyIrCfgExecutableSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrCfgExecutableSubset.lean:1)
- [proof/coq/VerifyIrCfgExecutableSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrCfgExecutableSubset.v:1)
- [proof/lean/RRProofs/VerifyIrCfgReachabilitySubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrCfgReachabilitySubset.lean:1)
- [proof/coq/VerifyIrCfgReachabilitySubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrCfgReachabilitySubset.v:1)
- [proof/lean/RRProofs/VerifyIrCfgConvergenceSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrCfgConvergenceSubset.lean:1)
- [proof/coq/VerifyIrCfgConvergenceSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrCfgConvergenceSubset.v:1)
- [proof/lean/RRProofs/VerifyIrCfgWorklistSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrCfgWorklistSubset.lean:1)
- [proof/coq/VerifyIrCfgWorklistSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrCfgWorklistSubset.v:1)
- [proof/lean/RRProofs/VerifyIrCfgOrderWorklistSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrCfgOrderWorklistSubset.lean:1)
- [proof/coq/VerifyIrCfgOrderWorklistSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrCfgOrderWorklistSubset.v:1)
- [proof/lean/RRProofs/VerifyIrCfgFixedPointSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrCfgFixedPointSubset.lean:1)
- [proof/coq/VerifyIrCfgFixedPointSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrCfgFixedPointSubset.v:1)
- [proof/lean/RRProofs/VerifyIrFlowLite.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrFlowLite.lean:1)
- [proof/coq/VerifyIrFlowLite.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrFlowLite.v:1)

Core proof claim:
- predecessor out-set intersection computes reduced must-defined join facts
- local assignment extends that must-defined set monotonically
- reachable-predecessor filtering and one reduced fixed-point step preserve
  those join facts into the next out-set map
- stable reduced out-set maps remain unchanged under further iteration
- reduced recursive load/wrapper traversal returns `none` whenever every
  observed load is must-defined
- reduced `ValueKind`-named wrappers such as `Intrinsic`, `RecordLit`,
  `FieldSet`, `Index*`, `Range`, and `Binary` also preserve the absence of
  undefined loads under the same must-defined assumption
- reduced arg-list and named-field-list scans for `Call`, `Intrinsic`, and
  `RecordLit` also preserve the absence of undefined loads under the same
  must-defined assumption
- env-selected scans and `ValueKind` arg/field scans can be packaged together
  under reusable compose-case and cross-case theorems, and reduced generic
  list/field composition theorems now quantify directly over selected-env
  clean facts and value-kind clean facts, with concrete call/record examples
  as instances
- those reduced composition theorems can then be re-packaged under explicit
  heterogeneous consumer constructors for `Call`, `Intrinsic`, and `RecordLit`
- those heterogeneous consumer constructors can then be lifted into a reduced
  `node-id + seen + fuel` graph so shared children and recursive wrapper
  parents approximate the concrete `ValueId` traversal discipline
- reduced child-edge extraction for non-`Phi` nodes now also mirrors the
  exact helper shape for unary wrappers, arg lists, field lists, and `Index*`
  nodes used before recursive traversal in Rust
- full reduced `value_dependencies` now also includes `Phi` arg lists and is
  lifted into a reduced seen/fuel stack walk approximating
  `depends_on_phi_in_block_except`
- that reduced seen/fuel walk is now also rephrased over an explicit
  `ValueId -> table row` lookup with stored `phi_block` metadata, closer to
  the concrete `FnIR.values` table shape
- those explicit table rows are now also refined to actual `ValueKind`-named
  payload constructors, rather than only reduced dependency tags
- those rows are now also lifted again to a reduced `Value` record carrying
  `id`, `kind`, `origin_var`, `phi_block`, and `escape`
- that reduced `Value` record is now also extended with `span`, `facts`,
  `value_ty`, and `value_term`, so nearly all fields of the concrete record are
  represented
- those reduced full `Value` rows are now also packaged into a small
  `FnIR`-style record carrying `name`, `params`, `values`, `blocks`, `entry`,
  and `body_head`
- that small reduced `FnIR` shell is now also refined again with reduced
  `user_name`, return-hint, inferred-return, and fallback/interop metadata
  while keeping the current verifier-facing value/table walk theorems
  projected onto the same smaller shell
- that same reduced function shell is now also refined again with
  `param_default_r_exprs`, `param_spans`, `param_ty_hints`,
  `param_term_hints`, and `param_hint_spans`, still projecting the current
  verifier-facing walks onto the same smaller shell
- that same reduced function shell is now also refined with reduced
  `call_semantics` and `memory_layout_hints` maps, still projecting the
  current verifier-facing walks onto the same smaller shell
- that same reduced function shell is now also refined with reduced
  `Block`/`Terminator` payloads carrying explicit instruction lists and
  terminator operands, still projecting the current verifier-facing walks
  onto the same smaller shell
- those reduced block payloads are now also connected back to reduced
  `UseBeforeDef` obligations by looking operand ids up through the reduced
  value table's `origin_var` field and packaging the resulting requirements
  as `VerifyIrFlowLite` blocks
- that block-flow bridge is now also composed directly with the reduced
  must-defined chain, so reduced join facts can certify explicit block payloads
  as `UseBeforeDef`-clean
- that same bridge is now also lifted to generic `required ⊆ defs`
  packaging, and multi-read block payloads can be certified clean from
  multiple reduced join facts at once
- local `assign` writes are now also packaged explicitly, so a reduced
  block may consume one incoming must-defined source var and then satisfy
  later reads of the destination var from block-local writes
- that same block-local write story is now also extended to a two-step local
  def chain, closer to the concrete `defined_here` growth across several
  `Assign` instructions before a later read
- that same local def-chain story is now also extended through a branch
  terminator, closer to the concrete case where `defined_here` must also
  discharge `If { cond, .. }` after preceding `Assign` instructions
- that same local def-chain story is now also extended through
  `StoreIndex1D/2D/3D`, closer to the concrete case where `defined_here` must
  also discharge store operands after preceding `Assign` instructions
- the sequential `defined_here` growth itself is now also packaged as a
  reusable reduced theorem, closer to the concrete loop that updates
  `defined_here` after each `Assign`
- those reusable block-local flow and `defined_here` theorems are now also
  packaged back into a single-block executable theorem, closer to the concrete
  verifier's ordered per-block acceptance story
- that executable packaging is now also extended to an ordered two-block case,
  closer to the concrete verifier's multi-block acceptance order after
  predecessor-selected `in_defs` are fixed
- that same executable packaging is now also extended to a join-shaped
  three-block case with left/right sibling blocks and a join block, closer to
  the small ordered bundles the concrete verifier reasons about after
  predecessor-selected `in_defs` are fixed
- that same join packaging is now also lifted into an explicit CFG witness
  record carrying reduced predecessor-map and block-order data, closer to the
  concrete verifier's explicit CFG reasoning surface
- that same CFG witness is now also tied directly to reduced
  `reachable/preds/outDefs` data, so the join block's incoming defs are
  justified through reduced `stepInDefs`
- that same reduced CFG witness is now also tied to a stable reduced out-map
  witness, so once the must-defined iteration has converged, iterated out-def
  maps can be re-used directly to justify reduced CFG acceptance
- that same stable reduced out-map witness is now also tied to a reduced
  join-focused worklist `changed` bit, closer to the concrete
  `compute_must_defined_vars` loop that stops once no block update changes the
  current out-def map
- that same reduced worklist story is now also lifted to a small block-order
  aggregation over left/right/join and then packaged as a reduced whole-CFG
  fixed-point checker, closer to the concrete `changed` loop and its
  `if !changed { break; }` exit condition
- required loads must already be defined on the path that reaches an
  instruction or terminator

Primary Rust correspondence:
- [src/mir/def.rs](/Users/feral/Desktop/Programming/RR/src/mir/def.rs:118)
  concrete `FnIR` record layout
- [src/mir/def.rs](/Users/feral/Desktop/Programming/RR/src/mir/def.rs:213)
  concrete `Value` record layout
- [src/mir/def.rs](/Users/feral/Desktop/Programming/RR/src/mir/def.rs:561)
  concrete `value_dependencies`
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:929)
  concrete `non_phi_dependencies`
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:608)
  instruction/terminator use-before-def checks
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:729)
  `Phi` edge availability against predecessor out-def sets
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:1041)
  concrete `compute_must_defined_vars`
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:1124)
  recursive `first_undefined_load_in_value`

Current gap:
- proof now models the core predecessor-intersection / local-assign step, but
  still uses reduced lists/functions rather than the full `FnIR` CFG/worklist
  state and full termination/convergence argument
- wrapper traversal now has reduced `ValueKind`-named cases and reduced
  arg-list forms, but it still does not model exact heterogeneous field/arg
  metadata or the full `ValueId` graph
- Rust verifier computes concrete must-defined sets over the real CFG

## Phi Edge Value Environment

Proof layers:
- [proof/lean/RRProofs/VerifyIrValueEnvSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrValueEnvSubset.lean:1)
- [proof/coq/VerifyIrValueEnvSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrValueEnvSubset.v:1)
- [proof/lean/RRProofs/VerifyIrArgEnvSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrArgEnvSubset.lean:1)
- [proof/coq/VerifyIrArgEnvSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrArgEnvSubset.v:1)
- [proof/lean/RRProofs/VerifyIrArgEnvTraversalSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrArgEnvTraversalSubset.lean:1)
- [proof/coq/VerifyIrArgEnvTraversalSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrArgEnvTraversalSubset.v:1)
- [proof/lean/RRProofs/VerifyIrEnvScanComposeSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrEnvScanComposeSubset.lean:1)
- [proof/coq/VerifyIrEnvScanComposeSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrEnvScanComposeSubset.v:1)
- [proof/lean/RRProofs/VerifyIrConsumerMetaSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrConsumerMetaSubset.lean:1)
- [proof/coq/VerifyIrConsumerMetaSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrConsumerMetaSubset.v:1)
- [proof/lean/RRProofs/VerifyIrConsumerGraphSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrConsumerGraphSubset.lean:1)
- [proof/coq/VerifyIrConsumerGraphSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrConsumerGraphSubset.v:1)
- [proof/lean/RRProofs/VerifyIrChildDepsSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrChildDepsSubset.lean:1)
- [proof/coq/VerifyIrChildDepsSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrChildDepsSubset.v:1)
- [proof/lean/RRProofs/VerifyIrValueDepsWalkSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrValueDepsWalkSubset.lean:1)
- [proof/coq/VerifyIrValueDepsWalkSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrValueDepsWalkSubset.v:1)
- [proof/lean/RRProofs/VerifyIrValueTableWalkSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrValueTableWalkSubset.lean:1)
- [proof/coq/VerifyIrValueTableWalkSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrValueTableWalkSubset.v:1)
- [proof/lean/RRProofs/VerifyIrValueKindTableSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrValueKindTableSubset.lean:1)
- [proof/coq/VerifyIrValueKindTableSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrValueKindTableSubset.v:1)
- [proof/lean/RRProofs/VerifyIrValueRecordSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrValueRecordSubset.lean:1)
- [proof/coq/VerifyIrValueRecordSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrValueRecordSubset.v:1)
- [proof/lean/RRProofs/VerifyIrValueFullRecordSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrValueFullRecordSubset.lean:1)
- [proof/coq/VerifyIrValueFullRecordSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrValueFullRecordSubset.v:1)
- [proof/lean/RRProofs/VerifyIrFnRecordSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrFnRecordSubset.lean:1)
- [proof/coq/VerifyIrFnRecordSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrFnRecordSubset.v:1)
- [proof/lean/RRProofs/VerifyIrFnMetaSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrFnMetaSubset.lean:1)
- [proof/coq/VerifyIrFnMetaSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrFnMetaSubset.v:1)
- [proof/lean/RRProofs/VerifyIrFnParamMetaSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrFnParamMetaSubset.lean:1)
- [proof/coq/VerifyIrFnParamMetaSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrFnParamMetaSubset.v:1)
- [proof/lean/RRProofs/VerifyIrFnHintMapSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrFnHintMapSubset.lean:1)
- [proof/coq/VerifyIrFnHintMapSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrFnHintMapSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockRecordSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockRecordSubset.lean:1)
- [proof/coq/VerifyIrBlockRecordSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockRecordSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockFlowSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockFlowSubset.lean:1)
- [proof/coq/VerifyIrBlockFlowSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockFlowSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockMustDefSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockMustDefSubset.lean:1)
- [proof/coq/VerifyIrBlockMustDefSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockMustDefSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockMustDefComposeSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockMustDefComposeSubset.lean:1)
- [proof/coq/VerifyIrBlockMustDefComposeSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockMustDefComposeSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockAssignFlowSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockAssignFlowSubset.lean:1)
- [proof/coq/VerifyIrBlockAssignFlowSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockAssignFlowSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockAssignChainSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockAssignChainSubset.lean:1)
- [proof/coq/VerifyIrBlockAssignChainSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockAssignChainSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockAssignBranchSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockAssignBranchSubset.lean:1)
- [proof/coq/VerifyIrBlockAssignBranchSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockAssignBranchSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockAssignStoreSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockAssignStoreSubset.lean:1)
- [proof/coq/VerifyIrBlockAssignStoreSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockAssignStoreSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockDefinedHereSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockDefinedHereSubset.lean:1)
- [proof/coq/VerifyIrBlockDefinedHereSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockDefinedHereSubset.v:1)
- [proof/lean/RRProofs/VerifyIrBlockExecutableSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrBlockExecutableSubset.lean:1)
- [proof/coq/VerifyIrBlockExecutableSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrBlockExecutableSubset.v:1)
- [proof/lean/RRProofs/VerifyIrTwoBlockExecutableSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrTwoBlockExecutableSubset.lean:1)
- [proof/coq/VerifyIrTwoBlockExecutableSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrTwoBlockExecutableSubset.v:1)
- [proof/lean/RRProofs/VerifyIrJoinExecutableSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrJoinExecutableSubset.lean:1)
- [proof/coq/VerifyIrJoinExecutableSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrJoinExecutableSubset.v:1)
- [proof/lean/RRProofs/VerifyIrCfgExecutableSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrCfgExecutableSubset.lean:1)
- [proof/coq/VerifyIrCfgExecutableSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrCfgExecutableSubset.v:1)
- [proof/lean/RRProofs/VerifyIrCfgReachabilitySubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrCfgReachabilitySubset.lean:1)
- [proof/coq/VerifyIrCfgReachabilitySubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrCfgReachabilitySubset.v:1)

Core proof claim:
- an explicit `ValueId`/`BlockId` environment can model predecessor-selected
  `Phi` values directly
- rewriting a consumer from the merged `Phi` id to the predecessor-selected
  source id preserves evaluation
- that same predecessor-selected rewrite also preserves reduced arg-list and
  field-list consumer evaluation
- that same predecessor-selected rewrite also preserves reduced missing-use
  scans over arg lists and field lists
- those env-selected scan facts can also be packaged alongside reduced
  `ValueKind` arg/field scan facts for the same concrete correspondence cases,
  and are now linked by reduced generic list/field composition theorems rather
  than example-only packaging
- those same reduced composition theorems are now also lifted under explicit
  heterogeneous consumer metadata for `Call`, `Intrinsic`, and `RecordLit`
- those heterogeneous consumer metadata cases are now also lifted into a
  reduced graph with shared node ids, seen sets, and fuel-based recursion
  closer to the concrete recursive traversal
- the exact non-`Phi` child extraction shape used to seed that recursion is
  now also modeled directly for unary wrappers, arg lists, field lists, and
  `Index*` nodes
- the full `value_dependencies` shape, including `Phi` arg lists, is now also
  modeled directly and composed into a reduced stack walk for
  `depends_on_phi_in_block_except`
- that reduced stack walk is now also lifted to an explicit lookup-table model
  with stored `phi_block` metadata closer to the concrete `FnIR.values` table
- those explicit table rows are now also refined to actual `ValueKind`-named
  payload constructors, closer to the top-level row kinds stored in
  `FnIR.values`
- those rows are now also lifted to a reduced `Value` record carrying the
  main per-value fields used by the concrete table
- that reduced `Value` record is now also extended with reduced
  `span/facts/value_ty/value_term` fields so nearly all top-level fields of the
  concrete `Value` row are represented
- those reduced rows are now also packaged into a small `FnIR` shell carrying
  `name/params/values/blocks/entry/body_head`
- that small `FnIR` shell is now also refined with reduced `user_name`,
  return-hint, inferred-return, and fallback/interop metadata while still
  projecting the current verifier-relevant walk back onto the same shell
- that same reduced `FnIR` shell is now also refined with reduced parameter
  defaults, per-parameter spans, and per-parameter type/term/hint-span lists
  while still projecting the current verifier-relevant walk back onto the same
  shell
- that same reduced `FnIR` shell is now also refined with reduced
  `call_semantics` and `memory_layout_hints` maps while still projecting the
  current verifier-relevant walk back onto the same shell
- that same reduced `FnIR` shell is now also refined with reduced
  `Block`/`Terminator` payloads carrying explicit instruction lists and
  terminator operands while still projecting the current verifier-relevant
  walk back onto the same shell
- those reduced block payloads are now also connected back to reduced
  `UseBeforeDef` obligations through `origin_var` lookup over the reduced
  value table, closer to the concrete instruction/terminator operand checks
- that same block-flow bridge is now also composed with reduced must-defined
  join facts, closer to the concrete `in_defs` / instruction-operand story
- that same bridge is now also lifted to generic `required ⊆ defs`
  packaging, closer to the concrete story that every operand-derived required
  load must already lie in the block's incoming must-defined set
- block-local writes are now also packaged explicitly, closer to the concrete
  story that `defined_here` grows after each `Assign` and may discharge later
  operand uses within the same block
- that same `defined_here` story is now also extended over a two-step local
  def chain before a later read, closer to the concrete sequential block scan
- that same sequential block story is now also extended to a branch
  terminator condition after the local def chain
- that same sequential block story is now also extended to store operand
  bundles after the local def chain
- the sequential `defined_here` growth itself is now also isolated as a
  reusable reduced theorem over block scans
- those reusable block-local theorems are now also packaged back into a
  single-block executable acceptance theorem over reduced `VerifyIrFlowLite`
- that single-block executable packaging is now also extended to an ordered
  two-block acceptance theorem over reduced `VerifyIrFlowLite`
- that same executable packaging is now also extended to a join-shaped
  three-block acceptance theorem over reduced `VerifyIrFlowLite`
- that same packaging is now also lifted into an explicit CFG witness record
  carrying reduced predecessor/order data
- that same explicit CFG witness is now also tied directly to reduced
  `reachable/preds/outDefs` computation data

Primary Rust correspondence:
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:559)
  reachable predecessor matching for `Phi` args
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:603)
  `Phi` edge-availability and current-block-phi exclusion
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:976)
  `infer_phi_owner_block`
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:949)
  `depends_on_phi_in_block_except`

Current gap:
- proof still abstracts away the full `FnIR` graph and uses a reduced explicit
  environment model
- proof now composes env rewriting with reduced list consumers, but it still
  does not connect those consumers back to the concrete `ValueId` graph used by
  `first_undefined_load_in_value`
- proof now has a reduced table-driven walk with `ValueKind`-named rows, but
  it still does not use the full concrete `Value` payload fields or exact
  verifier stack/update discipline
- the new reduced full `Value` record still compresses
  `span/facts/value_ty/value_term/escape` into small tags rather than the full
  concrete payloads
- the new reduced `FnIR` shell still omits many concrete fields such as
  return/type hints, fallback flags, interop metadata, and call semantics
- Rust verifier also combines owner-block inference, predecessor filtering, and
  use-before-def propagation over the concrete CFG

## Executable Layer

Proof layers:
- [proof/lean/RRProofs/VerifyIrExecutableLite.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrExecutableLite.lean:1)
- [proof/coq/VerifyIrExecutableLite.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrExecutableLite.v:1)

Core proof claim:
- block ids / value ids / intrinsic arities / terminators are structurally
  executable
- emittable MIR must also eliminate reachable `Phi`

Primary Rust correspondence:
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:307)
  operand/id/arity checking during value validation
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:807)
  `verify_emittable_ir`

Current gap:
- proof collapses several Rust check groups into coarse booleans
- Rust verifier still traverses full values/instructions and exact ids

## Rust Error Name Layer

Proof layers:
- [proof/lean/RRProofs/VerifyIrRustErrorLite.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/VerifyIrRustErrorLite.lean:1)
- [proof/coq/VerifyIrRustErrorLite.v](/Users/feral/Desktop/Programming/RR/proof/coq/VerifyIrRustErrorLite.v:1)

Core proof claim:
- reduced proof-side failures map onto Rust-enum-shaped verifier names

Primary Rust correspondence:
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:7)
  `VerifyError`
- [src/mir/verify.rs](/Users/feral/Desktop/Programming/RR/src/mir/verify.rs:91)
  displayed user-facing error categories

Current gap:
- proof maps names, not full diagnostic payloads or spans
- Rust side still contains richer source attribution and staging details

## Immediate Next Steps

The most direct next refinements are:

1. lift one `VerifyIrStructLite` boolean bundle into a reduced explicit CFG
   predecessor map closer to real `FnIR`
2. replace the current reduced lookup-table / seen / fuel walk with a closer
   approximation of the real heterogeneous `FnIR.values` table, exact
   `ValueKind` payloads, and concrete stack discipline used by
   `depends_on_phi_in_block_except` and `first_undefined_load_in_value`
3. document which verifier checks are intentionally restricted to
   self-recursive/TCO shapes versus general emittable MIR
