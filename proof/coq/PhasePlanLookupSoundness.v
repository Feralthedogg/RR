Require Import RRProofs.MirInvariantBundle.
Require Import RRProofs.PhasePlanSoundness.
Require Import RRProofs.PhasePlanCollectionSoundness.
From Stdlib Require Import List Bool.
From Stdlib Require Import PeanoNat.
Import ListNotations.
Open Scope bool_scope.

Module RRPhasePlanLookupSoundness.

Import RRMirInvariantBundle.
Import RRPhasePlanSoundness.
Import RRPhasePlanCollectionSoundness.

Fixpoint lookup_collected_plan
    (function_id : nat)
    (plans : list reduced_function_phase_plan) : option reduced_function_phase_plan :=
  match plans with
  | [] => None
  | plan :: rest =>
      if Nat.eqb (plan_function_id plan) function_id then
        Some plan
      else
        lookup_collected_plan function_id rest
  end.

Lemma lookup_singleton_eligible_returns_plan :
  forall mode trace_requested fast_dev function_id features,
    lookup_collected_plan function_id
      (collect_function_phase_plans mode trace_requested fast_dev
        [{| entry_function_id := function_id;
            entry_features := features;
            entry_present := true;
            entry_conservative := false;
            entry_self_recursive := false;
            entry_selected := true |}]) =
      Some (build_function_phase_plan function_id mode trace_requested fast_dev features).
Proof.
  intros mode trace_requested fast_dev function_id features.
  simpl. rewrite Nat.eqb_refl. reflexivity.
Qed.

Lemma lookup_singleton_missing_returns_none :
  forall mode trace_requested fast_dev function_id features,
    lookup_collected_plan function_id
      (collect_function_phase_plans mode trace_requested fast_dev
        [{| entry_function_id := function_id;
            entry_features := features;
            entry_present := false;
            entry_conservative := false;
            entry_self_recursive := false;
            entry_selected := true |}]) =
      None.
Proof. reflexivity. Qed.

Lemma lookup_singleton_conservative_returns_none :
  forall mode trace_requested fast_dev function_id features,
    lookup_collected_plan function_id
      (collect_function_phase_plans mode trace_requested fast_dev
        [{| entry_function_id := function_id;
            entry_features := features;
            entry_present := true;
            entry_conservative := true;
            entry_self_recursive := false;
            entry_selected := true |}]) =
      None.
Proof. reflexivity. Qed.

Lemma lookup_singleton_self_recursive_returns_none :
  forall mode trace_requested fast_dev function_id features,
    lookup_collected_plan function_id
      (collect_function_phase_plans mode trace_requested fast_dev
        [{| entry_function_id := function_id;
            entry_features := features;
            entry_present := true;
            entry_conservative := false;
            entry_self_recursive := true;
            entry_selected := true |}]) =
      None.
Proof. reflexivity. Qed.

Lemma lookup_singleton_unselected_returns_none :
  forall mode trace_requested fast_dev function_id features,
    lookup_collected_plan function_id
      (collect_function_phase_plans mode trace_requested fast_dev
        [{| entry_function_id := function_id;
            entry_features := features;
            entry_present := true;
            entry_conservative := false;
            entry_self_recursive := false;
            entry_selected := false |}]) =
      None.
Proof. reflexivity. Qed.

Lemma lookup_collected_plan_preserves_verify_ir :
  forall function_id plans plan fn,
    lookup_collected_plan function_id plans = Some plan ->
    optimizer_eligible fn ->
    optimizer_eligible (plan_selected_pipeline plan fn).
Proof.
  intros function_id plans plan fn Hlookup H.
  exact (selected_plan_preserves_verify_ir plan fn H).
Qed.

Lemma lookup_collected_plan_preserves_semantics :
  forall function_id plans plan fn ρ,
    lookup_collected_plan function_id plans = Some plan ->
    exec_entry (plan_selected_pipeline plan fn) ρ = exec_entry fn ρ.
Proof.
  intros function_id plans plan fn ρ Hlookup.
  exact (selected_plan_preserves_semantics plan fn ρ).
Qed.

End RRPhasePlanLookupSoundness.
