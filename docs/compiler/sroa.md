# MIR SROA Design

This page defines the intended MIR-level Scalar Replacement of Aggregates
(SROA) pass for RR.

Status: design plus a reduced MIR implementation. The current pass handles
pure record projections, local load aliases, functional `FieldSet` updates,
static nested record projections, branch-join record `Phi` projection, and a
conservative loop-carried record `Phi` subset by splitting aggregate phis into
fieldwise scalar phis. It also rematerializes scalarized aggregates at reduced
`Eval`, `Return`, generic `Call`, and `Intrinsic` argument boundaries. Unique
local record aliases whose fields depend on mutable `Load(var)` expressions are
snapshotted into hidden scalar temporaries at the alias assignment before the
aggregate is split, so later reassignments cannot change the projected field
value. Known RR callees can also be cloned into a reduced record-field
specialization when a scalarized record argument is only used through static
field projections, and known effect-free RR callees can be cloned into
field-return specializations for static projections of record return values.
Repeated projections of the same field through a unique local record-return
alias share a single scalar temporary. Direct projections of simple
single-return callees are also lowered to cloned scalar MIR values before helper
generation. Unique local record-return aliases try a reduced inline field-temp
lowering first, so multiple projected fields from a simple single-return callee
can avoid separate scalar-return helper calls. The
compiler also has a codegen-facing cleanup for let-lifted raw R `list(...)`
temps whose later uses are only static `[[field]]` projections. Those temps are
split into scalar hidden assignments after helper inlining, with concrete
records still materialized at unsupported boundaries. The compiler may still
emit helper calls, inline helper expressions, or let-lifted raw R expressions
outside this proven subset, and it does not yet perform full multi-return
aggregate ABI scalarization.

Primary implementation targets:

- `src/mir/def.rs`
- `src/mir/analyze/escape.rs`
- `src/mir/opt/sroa.rs`
- `src/mir/opt/`
- `src/codegen/`

## Goal

SROA should remove temporary RR record/list allocations when the aggregate is
only used through statically known fields.

For a trait-specialized `Vec2` style program, the optimizer should be able to
turn this shape:

```text
moved = { x: a.x + b.x, y: a.y + b.y }
translated = { x: moved.x + c.x, y: moved.y + c.y }
final = { x: translated.x * dt, y: translated.y * dt }
return final.x
```

into scalar MIR equivalent to:

```text
moved_x = a_x + b_x
moved_y = a_y + b_y
translated_x = moved_x + c_x
translated_y = moved_y + c_y
final_x = translated_x * dt
final_y = translated_y * dt
return final_x
```

The optimizer may still materialize a record at escape boundaries, such as an
unknown call, `return record`, or emitted R ABI boundary.

## Non-Goals For The First Implementation

The first SROA implementation must not claim to optimize every R list-like
value.

Excluded initially:

- dynamic field names
- aggregate field expressions that contain delayed `Load(var)` reads
- data frames
- S3/S4/R6 objects
- package calls or reflective R APIs
- mutation-heavy aggregates whose aliasing is not proved
- records passed to unknown user, package, or runtime calls without
  rematerialization
- arbitrary named list interop
- ABI changes for returning multiple scalar fields directly

These cases should either skip SROA or force a materialized `RecordLit` at the
boundary.

## MIR Model

The first-class aggregate forms are:

- `ValueKind::RecordLit { fields }`
- `ValueKind::FieldGet { base, field }`
- `ValueKind::FieldSet { base, field, value }`

SROA models an aggregate as a field map:

```text
Shape = ordered field names
FieldMap = field name -> scalar ValueId
```

A scalarized aggregate value has no runtime allocation by itself. It is a
compile-time bundle of scalar `ValueId`s.

The pass should preserve field order from `RecordLit`. Field order matters when
a record is rematerialized into R `list(...)`.

## Placement

SROA should run before de-SSA and before final codegen preparation.

Initial recommended placement:

```text
AlwaysTier:
  simplify_cfg
  sccp
  intrinsics
  type_specialize
  tco
  loop_opt
  sroa
  dce
  bce

HeavyTier:
  run inside the standard/cleanup clusters after inlining and before DCE
```

Rationale:

- trait/static dispatch and inlining expose record-producing helper bodies
- SCCP/GVN simplify field values before SROA decides eligibility
- SROA creates dead aggregate values, so DCE must run after it
- de-SSA must see fieldwise `Phi` values, not opaque aggregate phis

The first gated rollout should enable SROA only for `-O2` standard mode or an
explicit environment flag. After correctness and compile-time behavior are
stable, it can move into the always tier for eligible functions.

## Candidate Eligibility

A value can be scalarized when all of these are true:

- the aggregate shape is statically known
- field names are unique
- all field values are available where the aggregate is used
- field/update expressions are snapshot-safe and do not contain delayed
  `Load(var)` reads that could observe a later assignment, unless the aggregate
  is a unique local alias and the field expression is captured into a hidden
  SROA snapshot temp at the alias assignment
- every field access uses a static field name
- every aggregate use is one of the supported consumers
- rematerialization is possible at each escape boundary

Supported consumers for the first implementation:

- `FieldGet`
- `FieldSet` with a static field name
- `Assign` to another local aggregate alias, including transitive unique local
  alias chains
- nested `RecordLit` fields and functional `FieldSet` base/update values, only
  by rematerializing scalarized aggregate values first
- `Phi` where all incoming values have the same shape
- `Len`, `Indices`, and `Index1D`/`Index2D`/`Index3D` base operands, only by
  rematerializing the record first
- `Eval`, only by rematerializing the record first
- `Return`, only by rematerializing the record first
- generic `Call` arguments, only by rematerializing the record before the call
- `Intrinsic` arguments, only by rematerializing the record before the
  intrinsic boundary
- known RR `Call` arguments, by cloning the callee with fieldwise scalar
  parameters when the original parameter is used only through static
  `FieldGet` projections
- known effect-free RR `Call` return values, by cloning the callee to return a
  single statically projected field when the record result is consumed through
  `FieldGet`
- direct `FieldGet` projections of known effect-free RR record-return calls, by
  cloning simple single-return field expressions into the caller before falling
  back to scalar-return cloned helpers
- unique local aliases of known effect-free RR record-return calls, by
  directly materializing cloneable projected field expressions into hidden
  scalar temporaries before falling back to scalar-return cloned helpers
- known pure RR helper calls when the call is either inlined before SROA or the
  argument is rematerialized

Unsupported consumers must force skip or rematerialization:

- intrinsic-internal aggregate scalarization; intrinsic arguments are
  rematerialized at the boundary unless a future intrinsic declares fieldwise
  aggregate support
- `StoreIndex1D`, `StoreIndex2D`, `StoreIndex3D`
- named-argument call sites for record-argument specialization
- callees where the record parameter escapes directly, is mutated, is passed to
  another call, or is used through dynamic/non-field operations
- record-return specialization for effectful callees, recursive callees, missing
  fields, or whole-record return consumers
- direct inline record-return lowering for named-argument calls or callees with
  branching control flow, loads, nested calls, or other non-cloneable field
  expressions
- dynamic interop and opaque interop functions

## Escape And Rematerialization

SROA needs a stronger aggregate escape analysis than the current local escape
classification.

The analysis should classify each aggregate use:

```text
ProjectionUse     - field read, no materialization
UpdateUse         - functional field update, no materialization
AliasUse          - local alias, no materialization
PhiUse            - fieldwise merge required
MaterializeUse    - known boundary requiring a concrete record
RejectUse         - unsafe or unsupported use
```

At `MaterializeUse`, the pass emits or reuses a `RecordLit` built from the
current field map. The materialized value must dominate the consumer.

At `RejectUse`, the aggregate candidate is not scalarized.

When a unique local alias assignment would otherwise delay a mutable load, SROA
can insert a hidden snapshot assignment before the aggregate alias:

```text
x = ...
point = { x: x + 1, y: y }
x = ...
return point.x
```

is treated as:

```text
point__rr_sroa_snap_x = x + 1
x = ...
return point__rr_sroa_snap_x
```

This is only allowed for snapshot-safe expressions and is skipped if the field
expression depends on the alias being assigned.

## Rewrite Rules

### Record Literal

```text
v = RecordLit { x: a, y: b }
```

becomes a compile-time aggregate bundle:

```text
bundle(v) = { x -> a, y -> b }
```

No runtime record is emitted unless `v` later materializes.

### Field Get

```text
g = FieldGet { base: v, field: "x" }
```

becomes:

```text
replace g with bundle(v)["x"]
```

If `v` cannot be scalarized, the value is left unchanged.

Nested static projections are handled by a bounded local fixpoint. For example:

```text
body = { pos: { x: px, y: py }, mass: m }
g1 = FieldGet { base: body, field: "pos" }
g2 = FieldGet { base: g1, field: "x" }
```

is reduced in one SROA pass invocation to:

```text
replace g2 with px
```

The same reduced mechanism applies when the intermediate `pos` value is a
same-shape fieldwise `Phi`.

### Field Set

```text
u = FieldSet { base: v, field: "x", value: z }
```

becomes a new bundle:

```text
bundle(u) = bundle(v) with x -> z
```

This is valid because RR `FieldSet` is a functional update in MIR, not a
mutable in-place update.

### Phi

```text
p = Phi([(v1, b1), (v2, b2)])
```

where all incoming values have shape `{ x, y }`, becomes fieldwise phis:

```text
p_x = Phi([(v1_x, b1), (v2_x, b2)])
p_y = Phi([(v1_y, b1), (v2_y, b2)])
bundle(p) = { x -> p_x, y -> p_y }
```

The implemented subset applies this rewrite only for live demanded projections
in functions without index stores. Loop-carried aggregate phis are accepted only
when every incoming aggregate has the same static record shape and each incoming
field expression is side-effect free. Recursive field expressions, such as a
loop update `state = { x: state.x + 1L, y: state.y }`, become scalar loop phis
for `x` and `y`; unsupported materialization boundaries still force the
aggregate to remain concrete. Nested same-shape phis are handled when each
intermediate aggregate projection also has a statically known same-shape field
map.

### Return Or Unknown Boundary

```text
Return(v)
```

where `v` is scalarized becomes:

```text
m = RecordLit { x: v_x, y: v_y }
Return(m)
```

unless a later ABI design explicitly supports scalarized returns.

## Algorithm

1. Build a complete use graph for values, instructions, and terminators.
2. Discover aggregate candidates from `RecordLit`, `FieldSet`, and same-shape
   `Phi`.
3. Infer shapes and field maps to a fixed point.
4. Classify every use as projection, update, alias, phi, materialize, or
   reject.
5. Reject candidates with unsafe uses or inconsistent shapes.
6. Rewrite accepted candidates:
   - replace `FieldGet` uses with scalar field values
   - replace `FieldSet` results with updated field maps
   - split aggregate `Phi` nodes into fieldwise `Phi` nodes
   - insert rematerialized `RecordLit` values at escape boundaries
7. Run verifier after the pass.
8. Run DCE and simplify CFG after SROA to remove dead aggregate values.

The pass should be deterministic. New hidden locals should use a stable naming
scheme such as:

```text
.__rr_sroa_<aggregate_id>_<field>
```

If the pass only rewrites MIR `ValueId`s and does not introduce user-visible
locals, the codegen name policy can remain internal.

## Verifier Requirements

SROA must preserve:

- all `ValueId` dependencies are available on every predecessor edge
- fieldwise `Phi` nodes live in the correct `phi_block`
- rematerialized `RecordLit` field values dominate the materialization point
- removed aggregate values have no remaining required uses after DCE
- function metadata, fallback reasons, and interop flags are unchanged

The most important verifier failure mode to avoid is branch-local or loop-local
field values being used on predecessor edges where they are not available.

## Codegen Contract

After SROA, codegen should see either:

- scalar values only, with no aggregate allocation
- explicit `RecordLit` rematerialization at a boundary

Codegen should not need to rediscover aggregate scalarization from emitted R
text. Raw R peephole cleanup remains useful, but it should be a fallback cleanup
layer, not the primary SROA mechanism.

Current codegen-facing fallback: after helper inlining, simple raw emitted R
assignments of the form `tmp <- list(x = ..., y = ...)` are scalarized when all
later uses of `tmp` in the function are static `tmp[["field"]]` projections and
all field expressions are side-effect-free under the reduced raw-R purity
check. This closes the common trait/operator chain shape where helper inlining
creates temporary records after MIR SROA has already run. It intentionally skips
whole-record calls, dynamic field names, reassignment, and side-effecting field
expressions.

## Test Plan

Required tests:

- straight-line `Vec2` trait chain removes intermediate `list(...)`
  allocations
- `FieldGet(RecordLit(...))` scalarizes without changing output
- `FieldSet` functional update scalarizes only the changed field
- branch join with same-shape records creates fieldwise phis
- loop-carried record state creates fieldwise phis
- return of a scalarized record rematerializes once
- unknown call argument rematerializes before the call
- known RR call argument scalarizes through a fieldwise specialized callee
- known effect-free RR record return scalarizes through a field-return
  specialized callee
- dynamic or opaque interop skips SROA
- dataframe/list pattern matching behavior is unchanged

Recommended semantic smoke:

- compare `-O0` and `-O2` output for record-heavy numeric kernels
- include a tesseract-style particle/vector update workload
- assert emitted R does not contain avoidable `list(...)` allocations in the
  optimized hot path

## Proof Plan

The first proof target should be a reduced MIR record subset theorem:

```text
SroaRecordSubsetSoundness:
  If a function is well formed and SROA accepts a record candidate,
  then replacing accepted aggregate operations with fieldwise scalar
  operations preserves observable MIR execution.
```

The proof should be split into:

- field projection correctness
- functional field update correctness
- fieldwise phi correctness
- rematerialization correctness
- verifier invariant preservation

This is intentionally narrower than a full Rust/R object proof. The trusted
boundary is the static RR `RecordLit` subset. The current Lean/Coq companion
theorem family covers field projection replacement, alias-temp replacement,
direct call-return field replacement, alias-field-to-scalar-value replacement,
and the hidden `__rr_sroa_snap_*` snapshot-temp projection subset.

## Rollout

Phase 0: Design and claim boundary.

Phase 1: Add use graph and candidate analysis with diagnostics only.

Current implementation note: `src/mir/opt/sroa.rs` contains the analysis spine
and the reduced rewrite pass. Set `RR_SROA_TRACE=1` for per-function candidate
counts or `RR_SROA_TRACE=verbose` for per-candidate detail.

Phase 2: Implement straight-line `RecordLit` plus `FieldGet` replacement.

Current implementation note: Phase 2 is implemented for functions without
`If`, `Phi`, or store instructions. It replaces `FieldGet` from a pure
`RecordLit`, or from a single local `Load` alias assigned from such a record,
with the scalar field `ValueId`, then removes now-dead pure aggregate
assignments owned by the scalarized record. It intentionally skips records with
impure field values because removing the aggregate could otherwise remove
evaluation of an unread field. The pass is wired into the always tier, heavy
standard cleanup, structural cleanup, and post-inline cleanup so
trait-specialized records exposed by inlining can be reduced before DCE.

Phase 3: Add `FieldSet` functional update scalarization.

Current implementation note: Phase 3 handles straight-line functional updates
when the base aggregate is already scalarized, the updated field exists in the
shape, and the replacement value is pure. The updated aggregate receives a new
field map with only that field changed, so reading either the updated field or
an unchanged field can bypass `rr_field_set` / `list(...)` materialization.
Impure updates remain materialized to preserve evaluation.

Phase 4: Add branch join fieldwise phi splitting.

Current implementation note: Phase 4 is implemented for live demanded
projections. The pass creates one scalar `Phi` per field and leaves dead
aggregate phi values for later DCE/de-SSA cleanup. Demand is based on live MIR
uses and transitive unique local aliases, so rerunning SROA does not keep
growing dead scalar phi values.

Phase 5: Add loop-carried aggregate support.

Current implementation note: A reduced Phase 5 subset is implemented for
side-effect-free same-shape record recurrences in functions without index
stores. The pass can split loop-carried aggregate phis and rewrite recursive
field projections to the corresponding scalar field phi. The same bounded
fixpoint also reaches nested static projections such as `body.pos.x`, including
nested same-shape branch phis. It still does not handle arbitrary aggregate
escapes, dynamic field names, or mutation-heavy aliasing.

Phase 6: Add escape rematerialization for evals, returns, and known
boundaries.

Current implementation note: A reduced Phase 6 subset rematerializes
scalarized local aliases, functional `FieldSet` values, and same-shape aggregate
phis at `Eval`, `Return`, and generic `Call` argument boundaries. Aggregate phis
demanded only by these materialization boundaries are split into fieldwise
scalar phis, then rebuilt as a concrete `RecordLit` at the boundary. Unique
local alias chains are traced back to the original aggregate for both projection
demand and rematerialization demand. Nested aggregate values stored inside
`RecordLit` fields or used as `FieldSet` base/update values are also
rematerialized before the enclosing aggregate reaches codegen. Unique alias
field expressions containing mutable loads are captured into hidden snapshot
temps at the alias assignment before scalar replacement. Concrete-list
consumers such as `Len`, `Indices`, and `Index*` base operands are treated as
materialization boundaries, not scalarized internally. Intrinsic arguments are
also materialization boundaries, matching generic calls without changing the
intrinsic ABI. Materialization demand is seeded from live non-alias consumers,
so dead aggregate container values do not force fieldwise phi splitting. Known
RR callees are handled by a reduced record-argument specialization pass: if a
scalarized record argument maps to a
callee parameter that is used only by static `FieldGet` projections, the callee
is cloned with one scalar parameter per demanded field and the caller passes the
field values directly. The pass still does not scalarize unknown calls
internally, change the ABI to return multiple scalars, handle named-argument
specialization, or model dynamic/reflection-heavy R list observation.

Phase 6b: Add reduced field-return call specialization.

Current implementation note: A reduced field-return subset is implemented for
known, effect-free RR callees. If a call result is consumed by a static
`FieldGet`, the callee can be cloned into a `__rr_sroa_ret_<field>` function
whose terminators return only that field's scalar value. Unique local aliases of
pure record-return calls can be removed when every load of the alias is consumed
only by static field projections. Repeated projections of the same alias field
are let-bound to one hidden scalar temp, so `p.x + p.x` does not duplicate the
same scalar-return call. This is not a full multi-return ABI: different field
consumers may still become separate scalar-return calls, and whole-record
consumers still force the normal materialized record boundary.

Phase 7: Add reduced Lean/Rocq proof files and correspondence notes.

Current implementation note: the proof sidecars cover alias-field-to-temp,
alias-field-to-scalar-value, and direct-call-field-to-value rewrites in the
reduced integer expression model. This is still a reduced correspondence proof,
not a full mechanization of raw emitted R parsing or R list allocation
semantics.

Phase 8: Enable by default for eligible `-O2` functions, then evaluate moving
cheap straight-line SROA into the always tier.

## Risks

- R list field order and names must be preserved on materialization.
- Unknown calls can observe object identity-like behavior through R semantics;
  force materialization or skip.
- Loop phis can easily violate edge availability if split naively.
- More scalar values can increase MIR size and compile time.
- Debug dumps and emitted line mapping may become less direct.
- Existing raw R helper cleanup must not duplicate SROA work in a way that
  changes output shape.

The pass should prefer skipping to generating a questionable rewrite.
