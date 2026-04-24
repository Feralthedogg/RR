Require Import RRProofs.MirInvariantBundle.
Require Import RRProofs.PhasePlanSoundness.
From Stdlib Require Import List Bool.
Import ListNotations.
Open Scope bool_scope.

Module RRPhasePlanCollectionSoundness.

Import RRMirInvariantBundle.
Import RRPhasePlanSoundness.

Record reduced_function_inventory_entry : Type := {
  entry_function_id : nat;
  entry_features : reduced_plan_features;
  entry_present : bool;
  entry_conservative : bool;
  entry_self_recursive : bool;
  entry_selected : bool;
}.

Definition collect_single_function_phase_plan
    (mode : reduced_phase_ordering_mode)
    (trace_requested fast_dev : bool)
    (entry : reduced_function_inventory_entry) : option reduced_function_phase_plan :=
  if entry_present entry &&
      negb (entry_conservative entry) &&
      negb (entry_self_recursive entry) &&
      entry_selected entry then
    Some (build_function_phase_plan
      (entry_function_id entry) mode trace_requested fast_dev (entry_features entry))
  else
    None.

Fixpoint collect_function_phase_plans
    (mode : reduced_phase_ordering_mode)
    (trace_requested fast_dev : bool)
    (entries : list reduced_function_inventory_entry) : list reduced_function_phase_plan :=
  match entries with
  | [] => []
  | entry :: rest =>
      match collect_single_function_phase_plan mode trace_requested fast_dev entry with
      | Some plan => plan :: collect_function_phase_plans mode trace_requested fast_dev rest
      | None => collect_function_phase_plans mode trace_requested fast_dev rest
      end
  end.

Lemma collect_single_skips_missing :
  forall mode trace_requested fast_dev function_id features conservative self_recursive selected,
    collect_single_function_phase_plan mode trace_requested fast_dev
      {| entry_function_id := function_id;
         entry_features := features;
         entry_present := false;
         entry_conservative := conservative;
         entry_self_recursive := self_recursive;
         entry_selected := selected |} = None.
Proof. reflexivity. Qed.

Lemma collect_single_skips_conservative :
  forall mode trace_requested fast_dev function_id features present self_recursive selected,
    collect_single_function_phase_plan mode trace_requested fast_dev
      {| entry_function_id := function_id;
         entry_features := features;
         entry_present := present;
         entry_conservative := true;
         entry_self_recursive := self_recursive;
         entry_selected := selected |} = None.
Proof.
  intros mode trace_requested fast_dev function_id features present self_recursive selected.
  destruct present; reflexivity.
Qed.

Lemma collect_single_skips_self_recursive :
  forall mode trace_requested fast_dev function_id features present conservative selected,
    collect_single_function_phase_plan mode trace_requested fast_dev
      {| entry_function_id := function_id;
         entry_features := features;
         entry_present := present;
         entry_conservative := conservative;
         entry_self_recursive := true;
         entry_selected := selected |} = None.
Proof.
  intros mode trace_requested fast_dev function_id features present conservative selected.
  destruct present, conservative; reflexivity.
Qed.

Lemma collect_single_skips_unselected :
  forall mode trace_requested fast_dev function_id features present conservative self_recursive,
    collect_single_function_phase_plan mode trace_requested fast_dev
      {| entry_function_id := function_id;
         entry_features := features;
         entry_present := present;
         entry_conservative := conservative;
         entry_self_recursive := self_recursive;
         entry_selected := false |} = None.
Proof.
  intros mode trace_requested fast_dev function_id features present conservative self_recursive.
  destruct present, conservative, self_recursive; reflexivity.
Qed.

Lemma collect_single_builds_plan_when_eligible :
  forall mode trace_requested fast_dev function_id features,
    collect_single_function_phase_plan mode trace_requested fast_dev
      {| entry_function_id := function_id;
         entry_features := features;
         entry_present := true;
         entry_conservative := false;
         entry_self_recursive := false;
         entry_selected := true |}
    = Some (build_function_phase_plan function_id mode trace_requested fast_dev features).
Proof. reflexivity. Qed.

Lemma collected_plan_preserves_verify_ir :
  forall mode trace_requested fast_dev entry plan fn,
    collect_single_function_phase_plan mode trace_requested fast_dev entry = Some plan ->
    optimizer_eligible fn ->
    optimizer_eligible (plan_selected_pipeline plan fn).
Proof.
  intros mode trace_requested fast_dev entry plan fn Hcollect H.
  exact (selected_plan_preserves_verify_ir plan fn H).
Qed.

Lemma collected_plan_preserves_semantics :
  forall mode trace_requested fast_dev entry plan fn ρ,
    collect_single_function_phase_plan mode trace_requested fast_dev entry = Some plan ->
    exec_entry (plan_selected_pipeline plan fn) ρ = exec_entry fn ρ.
Proof.
  intros mode trace_requested fast_dev entry plan fn ρ Hcollect.
  exact (selected_plan_preserves_semantics plan fn ρ).
Qed.

Lemma all_collected_plans_preserve_verify_ir :
  forall mode trace_requested fast_dev entries plan fn,
    In plan (collect_function_phase_plans mode trace_requested fast_dev entries) ->
    optimizer_eligible fn ->
    optimizer_eligible (plan_selected_pipeline plan fn).
Proof.
  intros mode trace_requested fast_dev entries plan fn Hmem H.
  exact (selected_plan_preserves_verify_ir plan fn H).
Qed.

Lemma all_collected_plans_preserve_semantics :
  forall mode trace_requested fast_dev entries plan fn ρ,
    In plan (collect_function_phase_plans mode trace_requested fast_dev entries) ->
    exec_entry (plan_selected_pipeline plan fn) ρ = exec_entry fn ρ.
Proof.
  intros mode trace_requested fast_dev entries plan fn ρ Hmem.
  exact (selected_plan_preserves_semantics plan fn ρ).
Qed.

End RRPhasePlanCollectionSoundness.
