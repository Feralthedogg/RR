Require Import VerifyIrRustErrorLite.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRVerifyIrRustErrorLite.

Module RRVerifyIrCheckOrderLite.

Record verify_ir_check_order_lite_case : Type := {
  check_entry_block_error : option verify_error_rust_lite;
  check_body_head_block_error : option verify_error_rust_lite;
  check_body_head_reachability_error : option verify_error_rust_lite;
  check_entry_terminator_error : option verify_error_rust_lite;
  check_body_head_terminator_error : option verify_error_rust_lite;
  check_value_id_error : option verify_error_rust_lite;
  check_non_phi_owner_error : option verify_error_rust_lite;
  check_self_reference_error : option verify_error_rust_lite;
  check_param_index_error : option verify_error_rust_lite;
  check_operand_error : option verify_error_rust_lite;
  check_call_arg_names_error : option verify_error_rust_lite;
  check_intrinsic_arity_error : option verify_error_rust_lite;
  check_non_phi_cycle_error : option verify_error_rust_lite;
  check_block_id_error : option verify_error_rust_lite;
  check_block_target_error : option verify_error_rust_lite;
  check_entry_pred_error : option verify_error_rust_lite;
  check_phi_shape_error : option verify_error_rust_lite;
  check_phi_edge_error : option verify_error_rust_lite;
  check_flow_error : option verify_error_rust_lite;
  check_bad_terminator_error : option verify_error_rust_lite;
  check_undefined_var_error : option verify_error_rust_lite;
  check_reachable_phi_error : option verify_error_rust_lite;
}.

Definition verify_ir_check_order_lite
    (c : verify_ir_check_order_lite_case)
    : option verify_error_rust_lite :=
  match check_entry_block_error c with
  | Some err => Some err
  | None =>
      match check_body_head_block_error c with
      | Some err => Some err
      | None =>
          match check_body_head_reachability_error c with
          | Some err => Some err
          | None =>
              match check_entry_terminator_error c with
              | Some err => Some err
              | None =>
                  match check_body_head_terminator_error c with
                  | Some err => Some err
                  | None =>
                      match check_value_id_error c with
                      | Some err => Some err
                      | None =>
                          match check_non_phi_owner_error c with
                          | Some err => Some err
                          | None =>
                              match check_self_reference_error c with
                              | Some err => Some err
                              | None =>
                                  match check_param_index_error c with
                                  | Some err => Some err
                                  | None =>
                                      match check_operand_error c with
                                      | Some err => Some err
                                      | None =>
                                          match check_call_arg_names_error c with
                                          | Some err => Some err
                                          | None =>
                                              match check_intrinsic_arity_error c with
                                              | Some err => Some err
                                              | None =>
                                                  match check_non_phi_cycle_error c with
                                                  | Some err => Some err
                                                  | None =>
                                                      match check_block_id_error c with
                                                      | Some err => Some err
                                                      | None =>
                                                          match check_block_target_error c with
                                                          | Some err => Some err
                                                          | None =>
                                                              match check_entry_pred_error c with
                                                              | Some err => Some err
                                                              | None =>
                                                                  match check_phi_shape_error c with
                                                                  | Some err => Some err
                                                                  | None =>
                                                                      match check_phi_edge_error c with
                                                                      | Some err => Some err
                                                                      | None =>
                                                                          match check_flow_error c with
                                                                          | Some err => Some err
                                                                          | None =>
                                                                              match check_bad_terminator_error c with
                                                                              | Some err => Some err
                                                                              | None => check_undefined_var_error c
                                                                              end
                                                                          end
                                                                      end
                                                                  end
                                                              end
                                                          end
                                                      end
                                                  end
                                              end
                                          end
                                      end
                                  end
                              end
                          end
                      end
                  end
              end
          end
      end
  end.

Definition verify_emittable_check_order_lite
    (c : verify_ir_check_order_lite_case)
    : option verify_error_rust_lite :=
  match verify_ir_check_order_lite c with
  | Some err => Some err
  | None => check_reachable_phi_error c
  end.

Definition example_entry_check_dominates : verify_ir_check_order_lite_case :=
  {| check_entry_block_error := Some RBadBlock;
     check_body_head_block_error := Some RBadBlock;
     check_body_head_reachability_error := Some RInvalidBodyHead;
     check_entry_terminator_error := Some RInvalidEntryTerminator;
     check_body_head_terminator_error := Some RInvalidBodyHeadTerminator;
     check_value_id_error := Some RBadValue;
     check_non_phi_owner_error := Some RInvalidPhiOwner;
     check_self_reference_error := Some RSelfReferentialValue;
     check_param_index_error := Some RInvalidParamIndex;
     check_operand_error := Some RBadValue;
     check_call_arg_names_error := Some RInvalidCallArgNames;
     check_intrinsic_arity_error := Some RInvalidIntrinsicArity;
     check_non_phi_cycle_error := Some RNonPhiValueCycle;
     check_block_id_error := Some RBadBlock;
     check_block_target_error := Some RBadBlock;
     check_entry_pred_error := Some RInvalidEntryPredecessor;
     check_phi_shape_error := Some RInvalidPhiArgs;
     check_phi_edge_error := Some RInvalidPhiEdgeValue;
     check_flow_error := Some RUseBeforeDef;
     check_bad_terminator_error := Some RBadTerminator;
     check_undefined_var_error := Some RUndefinedVar;
     check_reachable_phi_error := Some RReachablePhi |}.

Definition example_value_check_dominates_later : verify_ir_check_order_lite_case :=
  {| check_entry_block_error := None;
     check_body_head_block_error := None;
     check_body_head_reachability_error := None;
     check_entry_terminator_error := None;
     check_body_head_terminator_error := None;
     check_value_id_error := None;
     check_non_phi_owner_error := None;
     check_self_reference_error := None;
     check_param_index_error := Some RInvalidParamIndex;
     check_operand_error := Some RBadValue;
     check_call_arg_names_error := Some RInvalidCallArgNames;
     check_intrinsic_arity_error := Some RInvalidIntrinsicArity;
     check_non_phi_cycle_error := Some RNonPhiValueCycle;
     check_block_id_error := Some RBadBlock;
     check_block_target_error := Some RBadBlock;
     check_entry_pred_error := Some RInvalidEntryPredecessor;
     check_phi_shape_error := Some RInvalidPhiArgs;
     check_phi_edge_error := Some RInvalidPhiEdgeValue;
     check_flow_error := Some RUseBeforeDef;
     check_bad_terminator_error := Some RBadTerminator;
     check_undefined_var_error := Some RUndefinedVar;
     check_reachable_phi_error := Some RReachablePhi |}.

Definition example_phi_check_dominates_flow : verify_ir_check_order_lite_case :=
  {| check_entry_block_error := None;
     check_body_head_block_error := None;
     check_body_head_reachability_error := None;
     check_entry_terminator_error := None;
     check_body_head_terminator_error := None;
     check_value_id_error := None;
     check_non_phi_owner_error := None;
     check_self_reference_error := None;
     check_param_index_error := None;
     check_operand_error := None;
     check_call_arg_names_error := None;
     check_intrinsic_arity_error := None;
     check_non_phi_cycle_error := None;
     check_block_id_error := None;
     check_block_target_error := None;
     check_entry_pred_error := None;
     check_phi_shape_error := Some RInvalidPhiArgs;
     check_phi_edge_error := Some RInvalidPhiEdgeValue;
     check_flow_error := Some RUseBeforeDef;
     check_bad_terminator_error := Some RBadTerminator;
     check_undefined_var_error := Some RUndefinedVar;
     check_reachable_phi_error := Some RReachablePhi |}.

Definition example_undefined_var_dominates_reachable_phi : verify_ir_check_order_lite_case :=
  {| check_entry_block_error := None;
     check_body_head_block_error := None;
     check_body_head_reachability_error := None;
     check_entry_terminator_error := None;
     check_body_head_terminator_error := None;
     check_value_id_error := None;
     check_non_phi_owner_error := None;
     check_self_reference_error := None;
     check_param_index_error := None;
     check_operand_error := None;
     check_call_arg_names_error := None;
     check_intrinsic_arity_error := None;
     check_non_phi_cycle_error := None;
     check_block_id_error := None;
     check_block_target_error := None;
     check_entry_pred_error := None;
     check_phi_shape_error := None;
     check_phi_edge_error := None;
     check_flow_error := None;
     check_bad_terminator_error := None;
     check_undefined_var_error := Some RUndefinedVar;
     check_reachable_phi_error := Some RReachablePhi |}.

Definition example_check_order_clean : verify_ir_check_order_lite_case :=
  {| check_entry_block_error := None;
     check_body_head_block_error := None;
     check_body_head_reachability_error := None;
     check_entry_terminator_error := None;
     check_body_head_terminator_error := None;
     check_value_id_error := None;
     check_non_phi_owner_error := None;
     check_self_reference_error := None;
     check_param_index_error := None;
     check_operand_error := None;
     check_call_arg_names_error := None;
     check_intrinsic_arity_error := None;
     check_non_phi_cycle_error := None;
     check_block_id_error := None;
     check_block_target_error := None;
     check_entry_pred_error := None;
     check_phi_shape_error := None;
     check_phi_edge_error := None;
     check_flow_error := None;
     check_bad_terminator_error := None;
     check_undefined_var_error := None;
     check_reachable_phi_error := None |}.

Lemma example_entry_check_dominates_rejects :
  verify_ir_check_order_lite example_entry_check_dominates = Some RBadBlock.
Proof.
  reflexivity.
Qed.

Lemma example_value_check_dominates_later_rejects :
  verify_ir_check_order_lite example_value_check_dominates_later = Some RInvalidParamIndex.
Proof.
  reflexivity.
Qed.

Lemma example_phi_check_dominates_flow_rejects :
  verify_ir_check_order_lite example_phi_check_dominates_flow = Some RInvalidPhiArgs.
Proof.
  reflexivity.
Qed.

Lemma example_undefined_var_dominates_reachable_phi_rejects :
  verify_emittable_check_order_lite example_undefined_var_dominates_reachable_phi =
    Some RUndefinedVar.
Proof.
  reflexivity.
Qed.

Lemma example_check_order_clean_accepts :
  verify_emittable_check_order_lite example_check_order_clean = None.
Proof.
  reflexivity.
Qed.

End RRVerifyIrCheckOrderLite.
