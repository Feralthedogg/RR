# IR Model (HIR and MIR)

## HIR (`src/hir/def.rs`)

HIR represents typed/semi-typed language structure before CFG lowering.

Key entities:

- `HirProgram` -> `HirModule` -> `HirItem`
- `HirFn` with params, attrs, local map, body
- `HirStmt`:
  - `Let`, `Assign`, `If`, `While`, `For`, `Return`, `Break`, `Next`, `Expr`
- `HirExpr`:
  - locals/globals/literals/calls/index/field
  - control expressions (`IfExpr`, `Match`, `Try`)
  - composite literals (`VectorLit`, `ListLit`)
  - lambda-related forms after lowering

Specialized forms:

- `HirTidyCall`/`TidyExpr` for tidy-style operations
- pattern nodes (`HirPat`) for match lowering

## MIR (`src/mir/def.rs`)

MIR is SSA-like, CFG-based, optimization-facing IR.

Core:

- `FnIR`:
  - `blocks: Vec<Block>`
  - `values: Vec<Value>`
  - `entry`, `body_head`
  - type metadata:
    - `param_ty_hints`, `param_term_hints`
    - `ret_ty_hint`, `ret_term_hint`
    - `inferred_ret_ty`, `inferred_ret_term`
  - `unsupported_dynamic` + `fallback_reasons`
- `Block`:
  - instruction list + terminator
- `ValueKind`:
  - SSA primitives: `Const`, `Phi`, `Param`, `Load`
  - structural primitives: `Len`, `Indices`, `Range`
  - compute: `Binary`, `Unary`, `Call`, `Intrinsic`
  - memory-like access: `Index1D`, `Index2D`, `Index3D`
- `Value` carries static analysis metadata used by optimizer/codegen:
  - `value_ty` (type lattice state)
  - `value_term` (structural generic term)
- `Instr`:
  - `Assign`, `Eval`
  - `StoreIndex1D`, `StoreIndex2D`, `StoreIndex3D`
- `Terminator`:
  - `Goto`, `If`, `Return`, `Unreachable`

## Key Invariants

- Phi nodes must be eliminated before codegen.
- Value/Block IDs must remain valid under transforms.
- `value_ty`/`value_term` facts must remain conservative under rewrites.
- Runtime safety validator checks static error cases (e.g., guaranteed division by zero).

## Why MIR Is Central

Most advanced behavior is implemented on MIR:

- SCCP
- GVN/CSE
- LICM
- TCO
- BCE
- Vectorization (`v_opt`)
- De-SSA with parallel copy
