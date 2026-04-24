Require Import VerifyIrArgEnvTraversalSubset.
Require Import VerifyIrArgEnvSubset.
Require Import VerifyIrArgListTraversalSubset.
Require Import VerifyIrFlowLite.
Require Import VerifyIrMustDefSubset.
Require Import VerifyIrMustDefFixedPointSubset.
Require Import VerifyIrMustDefConvergenceSubset.
Require Import VerifyIrValueEnvSubset.
Require Import VerifyIrValueKindTraversalSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrValueEnvSubset.
Import RRVerifyIrArgEnvSubset.
Import RRVerifyIrArgEnvTraversalSubset.
Import RRVerifyIrArgListTraversalSubset.
Import RRVerifyIrFlowLite.
Import RRVerifyIrMustDefSubset.
Import RRVerifyIrMustDefFixedPointSubset.
Import RRVerifyIrMustDefConvergenceSubset.
Import RRVerifyIrValueKindTraversalSubset.

Module RRVerifyIrEnvScanComposeSubset.

Record env_scan_compose_case : Type := {
  compose_arg_env_list_clean : Prop;
  compose_value_kind_list_clean : Prop;
  compose_arg_env_field_clean : Prop;
  compose_value_kind_field_clean : Prop;
}.

Definition env_scan_compose_all_clean (c : env_scan_compose_case) : Prop :=
  compose_arg_env_list_clean c /\
  compose_value_kind_list_clean c /\
  compose_arg_env_field_clean c /\
  compose_value_kind_field_clean c.

Definition env_scan_cross_clean
    (call_case field_case : env_scan_compose_case) : Prop :=
  compose_arg_env_list_clean call_case /\
  compose_value_kind_list_clean call_case /\
  compose_arg_env_field_clean field_case /\
  compose_value_kind_field_clean field_case.

Lemma env_scan_compose_all_clean_of_components :
  forall c,
    compose_arg_env_list_clean c ->
    compose_value_kind_list_clean c ->
    compose_arg_env_field_clean c ->
    compose_value_kind_field_clean c ->
    env_scan_compose_all_clean c.
Proof.
  intros c HArgList HVkList HArgField HVkField.
  repeat split; assumption.
Qed.

Lemma env_scan_compose_components_of_all_clean :
  forall c,
    env_scan_compose_all_clean c ->
    compose_arg_env_list_clean c /\
    compose_value_kind_list_clean c /\
    compose_arg_env_field_clean c /\
    compose_value_kind_field_clean c.
Proof.
  intros c H.
  exact H.
Qed.

Lemma env_scan_cross_clean_of_all_clean :
  forall call_case field_case,
    env_scan_compose_all_clean call_case ->
    env_scan_compose_all_clean field_case ->
    env_scan_cross_clean call_case field_case.
Proof.
  intros call_case field_case HCall HField.
  destruct HCall as [HArgList [HVkList [_ _]]].
  destruct HField as [_ [_ [HArgField HVkField]]].
  repeat split; assumption.
Qed.

Lemma env_scan_cross_clean_components :
  forall call_case field_case,
    env_scan_cross_clean call_case field_case ->
    compose_arg_env_list_clean call_case /\
    compose_value_kind_list_clean call_case /\
    compose_arg_env_field_clean field_case /\
    compose_value_kind_field_clean field_case.
Proof.
  intros call_case field_case H.
  exact H.
Qed.

Definition mk_list_compose_case
    (env : value_env) (phi arg : value_id) (es : list env_expr)
    (defined : def_set) (vk_es : list vk_expr) : env_scan_compose_case :=
  {| compose_arg_env_list_clean :=
       first_missing_arg_env_expr_list (merged_env env phi arg) es = None;
     compose_value_kind_list_clean :=
       first_undefined_vk_list defined vk_es = None;
     compose_arg_env_field_clean := True;
     compose_value_kind_field_clean := True |}.

Definition mk_field_compose_case
    (env : value_env) (phi arg : value_id) (fs : list env_field_arg)
    (defined : def_set) (vk_fs : list field_arg) : env_scan_compose_case :=
  {| compose_arg_env_list_clean := True;
     compose_value_kind_list_clean := True;
     compose_arg_env_field_clean :=
       first_missing_arg_env_fields (merged_env env phi arg) fs = None;
     compose_value_kind_field_clean :=
       first_undefined_field_args defined vk_fs = None |}.

Lemma mk_list_compose_case_all_clean_of_clean :
  forall env phi arg es defined vk_es,
    first_missing_arg_env_expr_list (merged_env env phi arg) es = None ->
    first_undefined_vk_list defined vk_es = None ->
    env_scan_compose_all_clean
      (mk_list_compose_case env phi arg es defined vk_es).
Proof.
  intros env phi arg es defined vk_es HArgEnv HVk.
  apply env_scan_compose_all_clean_of_components.
  - exact HArgEnv.
  - exact HVk.
  - exact I.
  - exact I.
Qed.

Lemma mk_field_compose_case_all_clean_of_clean :
  forall env phi arg fs defined vk_fs,
    first_missing_arg_env_fields (merged_env env phi arg) fs = None ->
    first_undefined_field_args defined vk_fs = None ->
    env_scan_compose_all_clean
      (mk_field_compose_case env phi arg fs defined vk_fs).
Proof.
  intros env phi arg fs defined vk_fs HArgEnv HVk.
  apply env_scan_compose_all_clean_of_components.
  - exact I.
  - exact I.
  - exact HArgEnv.
  - exact HVk.
Qed.

Lemma mk_list_compose_case_all_clean_of_loads_defined :
  forall env phi arg es defined vk_es,
    first_missing_arg_env_expr_list (merged_env env phi arg) es = None ->
    loads_defined_vk_list defined vk_es ->
    env_scan_compose_all_clean
      (mk_list_compose_case env phi arg es defined vk_es).
Proof.
  intros env phi arg es defined vk_es HArgEnv HDefined.
  apply mk_list_compose_case_all_clean_of_clean.
  - exact HArgEnv.
  - exact (first_undefined_vk_list_none_of_loads_defined defined vk_es HDefined).
Qed.

Lemma mk_field_compose_case_all_clean_of_fields_defined :
  forall env phi arg fs defined vk_fs,
    first_missing_arg_env_fields (merged_env env phi arg) fs = None ->
    fields_defined defined vk_fs ->
    env_scan_compose_all_clean
      (mk_field_compose_case env phi arg fs defined vk_fs).
Proof.
  intros env phi arg fs defined vk_fs HArgEnv HDefined.
  apply mk_field_compose_case_all_clean_of_clean.
  - exact HArgEnv.
  - exact (first_undefined_field_args_none_of_fields_defined defined vk_fs HDefined).
Qed.

Lemma mk_cross_compose_clean_of_clean :
  forall call_env call_phi call_arg call_es call_defined call_vk_es
         field_env field_phi field_arg field_fs field_defined field_vk_fs,
    first_missing_arg_env_expr_list
      (merged_env call_env call_phi call_arg) call_es = None ->
    first_undefined_vk_list call_defined call_vk_es = None ->
    first_missing_arg_env_fields
      (merged_env field_env field_phi field_arg) field_fs = None ->
    first_undefined_field_args field_defined field_vk_fs = None ->
    env_scan_cross_clean
      (mk_list_compose_case call_env call_phi call_arg call_es call_defined call_vk_es)
      (mk_field_compose_case field_env field_phi field_arg field_fs
        field_defined field_vk_fs).
Proof.
  intros call_env call_phi call_arg call_es call_defined call_vk_es
    field_env field_phi field_arg field_fs field_defined field_vk_fs
    HCallArgEnv HCallVk HFieldArgEnv HFieldVk.
  apply env_scan_cross_clean_of_all_clean.
  - exact (mk_list_compose_case_all_clean_of_clean _ _ _ _ _ _ HCallArgEnv HCallVk).
  - exact (mk_field_compose_case_all_clean_of_clean _ _ _ _ _ _ HFieldArgEnv HFieldVk).
Qed.

Lemma mk_cross_compose_clean_of_defined :
  forall call_env call_phi call_arg call_es call_defined call_vk_es
         field_env field_phi field_arg field_fs field_defined field_vk_fs,
    first_missing_arg_env_expr_list
      (merged_env call_env call_phi call_arg) call_es = None ->
    loads_defined_vk_list call_defined call_vk_es ->
    first_missing_arg_env_fields
      (merged_env field_env field_phi field_arg) field_fs = None ->
    fields_defined field_defined field_vk_fs ->
    env_scan_cross_clean
      (mk_list_compose_case call_env call_phi call_arg call_es call_defined call_vk_es)
      (mk_field_compose_case field_env field_phi field_arg field_fs
        field_defined field_vk_fs).
Proof.
  intros call_env call_phi call_arg call_es call_defined call_vk_es
    field_env field_phi field_arg field_fs field_defined field_vk_fs
    HCallArgEnv HCallDefined HFieldArgEnv HFieldDefined.
  apply env_scan_cross_clean_of_all_clean.
  - exact (mk_list_compose_case_all_clean_of_loads_defined _ _ _ _ _ _
      HCallArgEnv HCallDefined).
  - exact (mk_field_compose_case_all_clean_of_fields_defined _ _ _ _ _ _
      HFieldArgEnv HFieldDefined).
Qed.

Definition call_compose_case : env_scan_compose_case :=
  mk_list_compose_case
    example_env 9%nat 1%nat example_call_env_args
    (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
      example_stable_assign_map 5%nat example_stable_seed 3%nat)
    example_call_args.

Definition field_compose_case : env_scan_compose_case :=
  mk_field_compose_case
    example_env 12%nat 7%nat example_record_env_fields
    (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
      example_stable_assign_map 5%nat example_stable_seed 3%nat)
    example_record_fields.

Lemma call_compose_case_clean :
  compose_arg_env_list_clean call_compose_case /\
  compose_value_kind_list_clean call_compose_case.
Proof.
  split.
  - exact example_call_env_args_scan_clean_from_selected_eval.
  - exact example_call_args_scan_clean.
Qed.

Lemma field_compose_case_clean :
  compose_arg_env_field_clean field_compose_case /\
  compose_value_kind_field_clean field_compose_case.
Proof.
  split.
  - exact example_record_env_fields_scan_clean_from_selected_eval.
  - exact example_record_fields_scan_clean.
Qed.

Lemma compose_cases_all_clean :
  compose_arg_env_list_clean call_compose_case /\
  compose_value_kind_list_clean call_compose_case /\
  compose_arg_env_field_clean field_compose_case /\
  compose_value_kind_field_clean field_compose_case.
Proof.
  destruct call_compose_case_clean as [HCallEnv HCallVk].
  destruct field_compose_case_clean as [HFieldEnv HFieldVk].
  repeat split; assumption.
Qed.

Lemma cross_compose_cases_all_clean :
  env_scan_cross_clean call_compose_case field_compose_case.
Proof.
  exact
    (mk_cross_compose_clean_of_clean
      example_env 9%nat 1%nat example_call_env_args
      (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
        example_stable_assign_map 5%nat example_stable_seed 3%nat)
      example_call_args
      example_env 12%nat 7%nat example_record_env_fields
      (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
        example_stable_assign_map 5%nat example_stable_seed 3%nat)
      example_record_fields
      example_call_env_args_scan_clean_from_selected_eval
      example_call_args_scan_clean
      example_record_env_fields_scan_clean_from_selected_eval
      example_record_fields_scan_clean).
Qed.

Lemma call_compose_case_all_clean :
  env_scan_compose_all_clean call_compose_case.
Proof.
  exact
    (mk_list_compose_case_all_clean_of_clean
      example_env 9%nat 1%nat example_call_env_args
      (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
        example_stable_assign_map 5%nat example_stable_seed 3%nat)
      example_call_args
      example_call_env_args_scan_clean_from_selected_eval
      example_call_args_scan_clean).
Qed.

Lemma field_compose_case_all_clean :
  env_scan_compose_all_clean field_compose_case.
Proof.
  exact
    (mk_field_compose_case_all_clean_of_clean
      example_env 12%nat 7%nat example_record_env_fields
      (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
        example_stable_assign_map 5%nat example_stable_seed 3%nat)
      example_record_fields
      example_record_env_fields_scan_clean_from_selected_eval
      example_record_fields_scan_clean).
Qed.

End RRVerifyIrEnvScanComposeSubset.
