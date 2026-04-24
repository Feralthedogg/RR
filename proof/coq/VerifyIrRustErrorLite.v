Require Import VerifyIrLite.
Require Import VerifyIrStructLite.
Require Import VerifyIrFlowLite.
Require Import VerifyIrExecutableLite.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRVerifyIrLite.
Import RRVerifyIrStructLite.
Import RRVerifyIrFlowLite.
Import RRVerifyIrExecutableLite.

Module RRVerifyIrRustErrorLite.

Inductive verify_error_rust_lite : Type :=
| RBadValue
| RBadBlock
| RBadOperand
| RBadTerminator
| RUseBeforeDef
| RInvalidPhiArgs
| RInvalidPhiSource
| RInvalidPhiOwner
| RInvalidPhiOwnerBlock
| RInvalidParamIndex
| RInvalidCallArgNames
| RSelfReferentialValue
| RNonPhiValueCycle
| RInvalidBodyHead
| RInvalidBodyHeadEntryEdge
| RInvalidEntryPrologue
| RInvalidEntryPredecessor
| RInvalidEntryTerminator
| RInvalidBranchTargets
| RInvalidLoopHeaderSplit
| RInvalidLoopHeaderPredecessors
| RInvalidBodyHeadTerminator
| RInvalidPhiPlacement
| RInvalidPhiPredecessorAliases
| RInvalidPhiEdgeValue
| RUndefinedVar
| RReachablePhi
| RInvalidIntrinsicArity.

Definition verify_error_lite_to_rust (err : verify_error_lite)
    : verify_error_rust_lite :=
  match err with
  | EUndefinedVar _ => RUndefinedVar
  | EInvalidPhiSource => RInvalidPhiSource
  | EReachablePhi => RReachablePhi
  end.

Definition verify_error_struct_lite_to_rust (err : verify_error_struct_lite)
    : verify_error_rust_lite :=
  match err with
  | EBase base => verify_error_lite_to_rust base
  | EInvalidBodyHead => RInvalidBodyHead
  | EInvalidBodyHeadEntryEdge => RInvalidBodyHeadEntryEdge
  | EInvalidEntryPrologue => RInvalidEntryPrologue
  | EInvalidBodyHeadTerminator => RInvalidBodyHeadTerminator
  | EInvalidEntryPredecessor => RInvalidEntryPredecessor
  | EInvalidEntryTerminator => RInvalidEntryTerminator
  | EInvalidBranchTargets => RInvalidBranchTargets
  | EInvalidLoopHeaderSplit => RInvalidLoopHeaderSplit
  | EInvalidLoopHeaderPredecessors => RInvalidLoopHeaderPredecessors
  | EInvalidPhiPlacement => RInvalidPhiPlacement
  | EInvalidPhiPredecessorAliases => RInvalidPhiPredecessorAliases
  | EInvalidPhiEdgeValue => RInvalidPhiEdgeValue
  | EMissingPhiBlock => RInvalidPhiArgs
  | ENonPhiCarriesPhiBlock => RInvalidPhiOwner
  | EInvalidPhiOwnerBlock => RInvalidPhiOwnerBlock
  | EInvalidParamIndex => RInvalidParamIndex
  | EInvalidCallArgNames => RInvalidCallArgNames
  | ESelfReferentialValue => RSelfReferentialValue
  | ENonPhiValueCycle => RNonPhiValueCycle
  end.

Definition verify_error_flow_lite_to_rust (err : verify_error_flow_lite)
    : verify_error_rust_lite :=
  match err with
  | EFlowBase base => verify_error_struct_lite_to_rust base
  | EUseBeforeDef _ => RUseBeforeDef
  end.

Definition verify_error_executable_lite_to_rust
    (err : verify_error_executable_lite)
    : verify_error_rust_lite :=
  match err with
  | EBadBlock => RBadBlock
  | EBadValue => RBadValue
  | EInvalidIntrinsicArity => RInvalidIntrinsicArity
  | EBadTerminator => RBadTerminator
  | EExecBase base => verify_error_flow_lite_to_rust base
  | EReachablePhiExec => RReachablePhi
  end.

Definition verify_ir_rust_lite
    (c : verify_ir_executable_lite_case)
    : option verify_error_rust_lite :=
  option_map verify_error_executable_lite_to_rust (verify_ir_executable_lite c).

Definition verify_emittable_rust_lite
    (c : verify_ir_executable_lite_case)
    : option verify_error_rust_lite :=
  option_map verify_error_executable_lite_to_rust
    (verify_emittable_executable_lite c).

Lemma example_bad_entry_case_maps_to_bad_block :
  verify_ir_rust_lite example_bad_entry_case = Some RBadBlock.
Proof.
  reflexivity.
Qed.

Lemma example_intrinsic_arity_case_maps_to_invalid_intrinsic_arity :
  verify_ir_rust_lite example_intrinsic_arity_case = Some RInvalidIntrinsicArity.
Proof.
  reflexivity.
Qed.

Lemma example_reachable_phi_executable_case_maps_to_reachable_phi :
  verify_emittable_rust_lite example_reachable_phi_executable_case = Some RReachablePhi.
Proof.
  reflexivity.
Qed.

Lemma example_executable_clean_case_rust_accepts :
  verify_emittable_rust_lite example_executable_clean_case = None.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_phi_edge_value_maps_to_rust_name :
  option_map verify_error_struct_lite_to_rust
    (verify_ir_struct_lite example_invalid_phi_edge_value_case) =
    Some RInvalidPhiEdgeValue.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_entry_prologue_maps_to_rust_name :
  option_map verify_error_struct_lite_to_rust
    (verify_ir_struct_lite example_invalid_entry_prologue_case) =
    Some RInvalidEntryPrologue.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_branch_targets_maps_to_rust_name :
  option_map verify_error_struct_lite_to_rust
    (verify_ir_struct_lite example_invalid_branch_targets_case) =
    Some RInvalidBranchTargets.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_loop_header_split_maps_to_rust_name :
  option_map verify_error_struct_lite_to_rust
    (verify_ir_struct_lite example_invalid_loop_header_split_case) =
    Some RInvalidLoopHeaderSplit.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_loop_header_preds_maps_to_rust_name :
  option_map verify_error_struct_lite_to_rust
    (verify_ir_struct_lite example_invalid_loop_header_preds_case) =
    Some RInvalidLoopHeaderPredecessors.
Proof.
  reflexivity.
Qed.

End RRVerifyIrRustErrorLite.
