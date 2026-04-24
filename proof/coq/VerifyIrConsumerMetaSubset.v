Require Import VerifyIrArgEnvTraversalSubset.
Require Import VerifyIrArgEnvSubset.
Require Import VerifyIrArgListTraversalSubset.
Require Import VerifyIrValueEnvSubset.
Require Import VerifyIrMustDefSubset.
Require Import VerifyIrMustDefFixedPointSubset.
Require Import VerifyIrMustDefConvergenceSubset.
Require Import VerifyIrValueKindTraversalSubset.
Require Import VerifyIrEnvScanComposeSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrArgEnvTraversalSubset.
Import RRVerifyIrArgEnvSubset.
Import RRVerifyIrArgListTraversalSubset.
Import RRVerifyIrValueEnvSubset.
Import RRVerifyIrMustDefSubset.
Import RRVerifyIrMustDefFixedPointSubset.
Import RRVerifyIrMustDefConvergenceSubset.
Import RRVerifyIrValueKindTraversalSubset.
Import RRVerifyIrEnvScanComposeSubset.

Module RRVerifyIrConsumerMetaSubset.

Inductive consumer_meta : Type :=
| CMCall :
    value_env -> value_id -> value_id -> list env_expr -> def_set -> list vk_expr ->
    consumer_meta
| CMIntrinsic :
    value_env -> value_id -> value_id -> list env_expr -> def_set -> list vk_expr ->
    consumer_meta
| CMRecordLit :
    value_env -> value_id -> value_id -> list env_field_arg -> def_set -> list field_arg ->
    consumer_meta.

Definition consumer_meta_clean (c : consumer_meta) : Prop :=
  match c with
  | CMCall env phi arg es defined vk_es =>
      env_scan_compose_all_clean (mk_list_compose_case env phi arg es defined vk_es)
  | CMIntrinsic env phi arg es defined vk_es =>
      env_scan_compose_all_clean (mk_list_compose_case env phi arg es defined vk_es)
  | CMRecordLit env phi arg fs defined vk_fs =>
      env_scan_compose_all_clean (mk_field_compose_case env phi arg fs defined vk_fs)
  end.

Lemma consumer_meta_clean_call_of_clean :
  forall env phi arg es defined vk_es,
    first_missing_arg_env_expr_list (merged_env env phi arg) es = None ->
    first_undefined_vk_list defined vk_es = None ->
    consumer_meta_clean (CMCall env phi arg es defined vk_es).
Proof.
  intros env phi arg es defined vk_es HArgEnv HVk.
  exact (mk_list_compose_case_all_clean_of_clean
    env phi arg es defined vk_es HArgEnv HVk).
Qed.

Lemma consumer_meta_clean_call_of_loads_defined :
  forall env phi arg es defined vk_es,
    first_missing_arg_env_expr_list (merged_env env phi arg) es = None ->
    loads_defined_vk_list defined vk_es ->
    consumer_meta_clean (CMCall env phi arg es defined vk_es).
Proof.
  intros env phi arg es defined vk_es HArgEnv HDefined.
  exact (mk_list_compose_case_all_clean_of_loads_defined
    env phi arg es defined vk_es HArgEnv HDefined).
Qed.

Lemma consumer_meta_clean_intrinsic_of_clean :
  forall env phi arg es defined vk_es,
    first_missing_arg_env_expr_list (merged_env env phi arg) es = None ->
    first_undefined_vk_list defined vk_es = None ->
    consumer_meta_clean (CMIntrinsic env phi arg es defined vk_es).
Proof.
  intros env phi arg es defined vk_es HArgEnv HVk.
  exact (mk_list_compose_case_all_clean_of_clean
    env phi arg es defined vk_es HArgEnv HVk).
Qed.

Lemma consumer_meta_clean_intrinsic_of_loads_defined :
  forall env phi arg es defined vk_es,
    first_missing_arg_env_expr_list (merged_env env phi arg) es = None ->
    loads_defined_vk_list defined vk_es ->
    consumer_meta_clean (CMIntrinsic env phi arg es defined vk_es).
Proof.
  intros env phi arg es defined vk_es HArgEnv HDefined.
  exact (mk_list_compose_case_all_clean_of_loads_defined
    env phi arg es defined vk_es HArgEnv HDefined).
Qed.

Lemma consumer_meta_clean_record_lit_of_clean :
  forall env phi arg fs defined vk_fs,
    first_missing_arg_env_fields (merged_env env phi arg) fs = None ->
    first_undefined_field_args defined vk_fs = None ->
    consumer_meta_clean (CMRecordLit env phi arg fs defined vk_fs).
Proof.
  intros env phi arg fs defined vk_fs HArgEnv HVk.
  exact (mk_field_compose_case_all_clean_of_clean
    env phi arg fs defined vk_fs HArgEnv HVk).
Qed.

Lemma consumer_meta_clean_record_lit_of_fields_defined :
  forall env phi arg fs defined vk_fs,
    first_missing_arg_env_fields (merged_env env phi arg) fs = None ->
    fields_defined defined vk_fs ->
    consumer_meta_clean (CMRecordLit env phi arg fs defined vk_fs).
Proof.
  intros env phi arg fs defined vk_fs HArgEnv HDefined.
  exact (mk_field_compose_case_all_clean_of_fields_defined
    env phi arg fs defined vk_fs HArgEnv HDefined).
Qed.

Definition example_call_consumer : consumer_meta :=
  CMCall
    example_env 9%nat 1%nat example_call_env_args
    (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
      example_stable_assign_map 5%nat example_stable_seed 3%nat)
    example_call_args.

Definition example_intrinsic_consumer : consumer_meta :=
  CMIntrinsic
    example_env 9%nat 1%nat example_call_env_args
    (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
      example_stable_assign_map 5%nat example_stable_seed 3%nat)
    example_intrinsic_args.

Definition example_record_consumer : consumer_meta :=
  CMRecordLit
    example_env 12%nat 7%nat example_record_env_fields
    (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
      example_stable_assign_map 5%nat example_stable_seed 3%nat)
    example_record_fields.

Lemma example_call_consumer_clean :
  consumer_meta_clean example_call_consumer.
Proof.
  exact
    (consumer_meta_clean_call_of_clean
      example_env 9%nat 1%nat example_call_env_args
      (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
        example_stable_assign_map 5%nat example_stable_seed 3%nat)
      example_call_args
      example_call_env_args_scan_clean_from_selected_eval
      example_call_args_scan_clean).
Qed.

Lemma example_intrinsic_consumer_clean :
  consumer_meta_clean example_intrinsic_consumer.
Proof.
  exact
    (consumer_meta_clean_intrinsic_of_clean
      example_env 9%nat 1%nat example_call_env_args
      (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
        example_stable_assign_map 5%nat example_stable_seed 3%nat)
      example_intrinsic_args
      example_call_env_args_scan_clean_from_selected_eval
      example_intrinsic_args_scan_clean).
Qed.

Lemma example_record_consumer_clean :
  consumer_meta_clean example_record_consumer.
Proof.
  exact
    (consumer_meta_clean_record_lit_of_clean
      example_env 12%nat 7%nat example_record_env_fields
      (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
        example_stable_assign_map 5%nat example_stable_seed 3%nat)
      example_record_fields
      example_record_env_fields_scan_clean_from_selected_eval
      example_record_fields_scan_clean).
Qed.

End RRVerifyIrConsumerMetaSubset.
