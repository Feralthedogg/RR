Require Import VerifyIrFlowLite.
Require Import VerifyIrMustDefSubset.
Require Import VerifyIrMustDefFixedPointSubset.
Require Import VerifyIrMustDefConvergenceSubset.
Require Import VerifyIrValueKindTraversalSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import Bool.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrFlowLite.
Import RRVerifyIrMustDefSubset.
Import RRVerifyIrMustDefFixedPointSubset.
Import RRVerifyIrMustDefConvergenceSubset.
Import RRVerifyIrValueKindTraversalSubset.

Module RRVerifyIrArgListTraversalSubset.

Definition field_arg := (string * vk_expr)%type.

Fixpoint first_undefined_vk_list (defined : def_set) (es : list vk_expr)
    : option string :=
  match es with
  | [] => None
  | e :: rest =>
      match first_undefined_vk defined e with
      | Some v => Some v
      | None => first_undefined_vk_list defined rest
      end
  end.

Fixpoint loads_defined_vk_list (defined : def_set) (es : list vk_expr) : Prop :=
  match es with
  | [] => True
  | e :: rest =>
      (loads_defined_vk defined e /\ loads_defined_vk_list defined rest)%type
  end.

Fixpoint first_undefined_field_args (defined : def_set) (fs : list field_arg)
    : option string :=
  match fs with
  | [] => None
  | (_, e) :: rest =>
      match first_undefined_vk defined e with
      | Some v => Some v
      | None => first_undefined_field_args defined rest
      end
  end.

Fixpoint fields_defined (defined : def_set) (fs : list field_arg) : Prop :=
  match fs with
  | [] => True
  | (_, e) :: rest =>
      (loads_defined_vk defined e /\ fields_defined defined rest)%type
  end.

Lemma first_undefined_vk_list_none_of_loads_defined :
  forall defined es,
    loads_defined_vk_list defined es ->
    first_undefined_vk_list defined es = None.
Proof.
  intros defined es.
  induction es as [|e rest IH]; intros H.
  - reflexivity.
  - simpl in H. destruct H as [Hhead Hrest].
    simpl.
    rewrite (first_undefined_vk_none_of_loads_defined defined e Hhead).
    exact (IH Hrest).
Qed.

Lemma first_undefined_field_args_none_of_fields_defined :
  forall defined fs,
    fields_defined defined fs ->
    first_undefined_field_args defined fs = None.
Proof.
  intros defined fs.
  induction fs as [|(name, e) rest IH]; intros H.
  - reflexivity.
  - simpl in H. destruct H as [Hhead Hrest].
    simpl.
    rewrite (first_undefined_vk_none_of_loads_defined defined e Hhead).
    exact (IH Hrest).
Qed.

Definition example_call_args : list vk_expr :=
  [ VKLoad "x"
  ; VKBinary (VKLoad "tmp") (VKLoad "x")
  ; VKFieldGet (VKLoad "tmp")
  ].

Definition example_intrinsic_args : list vk_expr :=
  [ VKLoad "x"
  ; VKUnary (VKLoad "tmp")
  ; VKRange (VKLoad "x") (VKLoad "tmp")
  ].

Definition example_record_fields : list field_arg :=
  [ ("a", VKLoad "x")
  ; ("b", VKFieldGet (VKLoad "tmp"))
  ; ("c", VKBinary (VKLoad "x") (VKLoad "tmp"))
  ].

Lemma example_call_args_scan_clean :
  first_undefined_vk_list
    (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
      example_stable_assign_map 5%nat example_stable_seed 3%nat)
    example_call_args = None.
Proof.
  rewrite example_stable_seed_iterate_five_block3.
  apply first_undefined_vk_list_none_of_loads_defined.
  simpl. repeat split; simpl; auto.
Qed.

Lemma example_intrinsic_args_scan_clean :
  first_undefined_vk_list
    (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
      example_stable_assign_map 5%nat example_stable_seed 3%nat)
    example_intrinsic_args = None.
Proof.
  rewrite example_stable_seed_iterate_five_block3.
  apply first_undefined_vk_list_none_of_loads_defined.
  simpl. repeat split; simpl; auto.
Qed.

Lemma example_record_fields_scan_clean :
  first_undefined_field_args
    (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
      example_stable_assign_map 5%nat example_stable_seed 3%nat)
    example_record_fields = None.
Proof.
  rewrite example_stable_seed_iterate_five_block3.
  apply first_undefined_field_args_none_of_fields_defined.
  simpl. repeat split; simpl; auto.
Qed.

End RRVerifyIrArgListTraversalSubset.
