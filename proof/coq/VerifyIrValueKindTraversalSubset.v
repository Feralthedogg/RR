Require Import VerifyIrFlowLite.
Require Import VerifyIrMustDefSubset.
Require Import VerifyIrMustDefFixedPointSubset.
Require Import VerifyIrMustDefConvergenceSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import Bool.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrFlowLite.
Import RRVerifyIrMustDefSubset.
Import RRVerifyIrMustDefFixedPointSubset.
Import RRVerifyIrMustDefConvergenceSubset.

Module RRVerifyIrValueKindTraversalSubset.

Inductive vk_expr : Type :=
| VKConst
| VKLoad : string -> vk_expr
| VKLen : vk_expr -> vk_expr
| VKIndices : vk_expr -> vk_expr
| VKFieldGet : vk_expr -> vk_expr
| VKRange : vk_expr -> vk_expr -> vk_expr
| VKUnary : vk_expr -> vk_expr
| VKBinary : vk_expr -> vk_expr -> vk_expr
| VKIntrinsic : vk_expr -> vk_expr -> vk_expr
| VKCall : vk_expr -> vk_expr -> vk_expr
| VKRecordLit : vk_expr -> vk_expr -> vk_expr
| VKFieldSet : vk_expr -> vk_expr -> vk_expr
| VKIndex1D : vk_expr -> vk_expr -> vk_expr
| VKIndex2D : vk_expr -> vk_expr -> vk_expr -> vk_expr
| VKIndex3D : vk_expr -> vk_expr -> vk_expr -> vk_expr -> vk_expr.

Fixpoint first_undefined_vk (defined : def_set) (e : vk_expr) : option string :=
  match e with
  | VKConst => None
  | VKLoad v =>
      if in_dec String.string_dec v defined then None else Some v
  | VKLen base
  | VKIndices base
  | VKFieldGet base
  | VKUnary base => first_undefined_vk defined base
  | VKRange start final
  | VKBinary start final
  | VKIntrinsic start final
  | VKCall start final
  | VKRecordLit start final
  | VKFieldSet start final
  | VKIndex1D start final =>
      match first_undefined_vk defined start with
      | Some v => Some v
      | None => first_undefined_vk defined final
      end
  | VKIndex2D base r c =>
      match first_undefined_vk defined base with
      | Some v => Some v
      | None =>
          match first_undefined_vk defined r with
          | Some v => Some v
          | None => first_undefined_vk defined c
          end
      end
  | VKIndex3D base i j k =>
      match first_undefined_vk defined base with
      | Some v => Some v
      | None =>
          match first_undefined_vk defined i with
          | Some v => Some v
          | None =>
              match first_undefined_vk defined j with
              | Some v => Some v
              | None => first_undefined_vk defined k
              end
          end
      end
  end.

Fixpoint loads_defined_vk (defined : def_set) (e : vk_expr) : Prop :=
  match e with
  | VKConst => True
  | VKLoad v => In v defined
  | VKLen base
  | VKIndices base
  | VKFieldGet base
  | VKUnary base => loads_defined_vk defined base
  | VKRange start final
  | VKBinary start final
  | VKIntrinsic start final
  | VKCall start final
  | VKRecordLit start final
  | VKFieldSet start final
  | VKIndex1D start final =>
      (loads_defined_vk defined start /\ loads_defined_vk defined final)%type
  | VKIndex2D base r c =>
      (loads_defined_vk defined base /\ loads_defined_vk defined r /\
       loads_defined_vk defined c)%type
  | VKIndex3D base i j k =>
      (loads_defined_vk defined base /\ loads_defined_vk defined i /\
       loads_defined_vk defined j /\ loads_defined_vk defined k)%type
  end.

Lemma first_undefined_vk_none_of_loads_defined :
  forall defined e,
    loads_defined_vk defined e ->
    first_undefined_vk defined e = None.
Proof.
  intros defined e.
  induction e as
      [|v|base IH|base IH|base IH|start IHs final IHf|base IH|start IHs final IHf
      |start IHs final IHf|start IHs final IHf|start IHs final IHf|start IHs final IHf
      |start IHs final IHf|base IHb r IHr c IHc|base IHb i IHi j IHj k IHk];
    intros H.
  - reflexivity.
  - simpl in H. simpl.
    destruct (in_dec String.string_dec v defined).
    + reflexivity.
    + contradiction.
  - exact (IH H).
  - exact (IH H).
  - exact (IH H).
  - simpl in H. destruct H as [Hs Hf]. simpl. rewrite (IHs Hs). exact (IHf Hf).
  - exact (IH H).
  - simpl in H. destruct H as [Hs Hf]. simpl. rewrite (IHs Hs). exact (IHf Hf).
  - simpl in H. destruct H as [Hs Hf]. simpl. rewrite (IHs Hs). exact (IHf Hf).
  - simpl in H. destruct H as [Hs Hf]. simpl. rewrite (IHs Hs). exact (IHf Hf).
  - simpl in H. destruct H as [Hs Hf]. simpl. rewrite (IHs Hs). exact (IHf Hf).
  - simpl in H. destruct H as [Hs Hf]. simpl. rewrite (IHs Hs). exact (IHf Hf).
  - simpl in H. destruct H as [Hs Hf]. simpl. rewrite (IHs Hs). exact (IHf Hf).
  - simpl in H. destruct H as [Hb [Hr Hc]].
    simpl. rewrite (IHb Hb). rewrite (IHr Hr). exact (IHc Hc).
  - simpl in H. destruct H as [Hb [Hi [Hj Hk]]].
    simpl. rewrite (IHb Hb). rewrite (IHi Hi). rewrite (IHj Hj). exact (IHk Hk).
Qed.

Definition example_intrinsic_vk : vk_expr :=
  VKIntrinsic (VKLoad "x") (VKFieldGet (VKLoad "tmp")).

Definition example_record_fieldset_vk : vk_expr :=
  VKFieldSet (VKRecordLit (VKLoad "x") VKConst) (VKLoad "tmp").

Definition example_index3d_vk : vk_expr :=
  VKIndex3D (VKLoad "base") (VKLoad "i") (VKLoad "j") (VKLoad "k").

Definition example_range_binary_vk : vk_expr :=
  VKBinary (VKRange (VKLoad "x") (VKLoad "tmp")) (VKUnary (VKLoad "x")).

Lemma example_intrinsic_vk_scan_clean :
  first_undefined_vk
    (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
      example_stable_assign_map 5%nat example_stable_seed 3%nat)
    example_intrinsic_vk = None.
Proof.
  rewrite example_stable_seed_iterate_five_block3.
  apply first_undefined_vk_none_of_loads_defined.
  simpl. auto.
Qed.

Lemma example_record_fieldset_vk_scan_clean :
  first_undefined_vk
    (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
      example_stable_assign_map 5%nat example_stable_seed 3%nat)
    example_record_fieldset_vk = None.
Proof.
  rewrite example_stable_seed_iterate_five_block3.
  apply first_undefined_vk_none_of_loads_defined.
  simpl. auto.
Qed.

Lemma example_index3d_vk_scan_clean :
  first_undefined_vk ["base"; "i"; "j"; "k"] example_index3d_vk = None.
Proof.
  apply first_undefined_vk_none_of_loads_defined.
  simpl. repeat split; simpl; auto.
Qed.

Lemma example_range_binary_vk_scan_clean :
  first_undefined_vk
    (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
      example_stable_assign_map 5%nat example_stable_seed 3%nat)
    example_range_binary_vk = None.
Proof.
  rewrite example_stable_seed_iterate_five_block3.
  apply first_undefined_vk_none_of_loads_defined.
  simpl. auto.
Qed.

End RRVerifyIrValueKindTraversalSubset.
