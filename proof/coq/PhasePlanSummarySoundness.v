Require Import MirInvariantBundle.
Require Import PhaseOrderOptimizerSoundness.
Require Import PhasePlanSoundness.
Require Import PhasePlanLookupSoundness.
From Stdlib Require Import List Bool.
Import ListNotations.
Open Scope bool_scope.

Module RRPhasePlanSummarySoundness.

Import RRMirInvariantBundle.
Import RRPhaseOrderOptimizerSoundness.
Import RRPhasePlanSoundness.
Import RRPhasePlanLookupSoundness.

Record reduced_plan_summary_entry : Type := {
  summary_function_id : nat;
  summary_schedule : reduced_phase_schedule;
  summary_profile : reduced_phase_profile_kind;
  summary_pass_groups : list reduced_pass_group;
}.

Definition summarize_plan (plan : reduced_function_phase_plan)
    : reduced_plan_summary_entry :=
  {| summary_function_id := plan_function_id plan;
     summary_schedule := plan_schedule plan;
     summary_profile := plan_profile plan;
     summary_pass_groups := plan_pass_groups plan |}.

Fixpoint plan_summary_entries
    (ordered_function_ids : list nat)
    (plans : list reduced_function_phase_plan) : list reduced_plan_summary_entry :=
  match ordered_function_ids with
  | [] => []
  | function_id :: rest =>
      match lookup_collected_plan function_id plans with
      | Some plan => summarize_plan plan :: plan_summary_entries rest plans
      | None => plan_summary_entries rest plans
      end
  end.

Lemma summary_lookup_hit_emits_entry :
  forall function_id plans plan,
    lookup_collected_plan function_id plans = Some plan ->
    In (summarize_plan plan) (plan_summary_entries [function_id] plans).
Proof.
  intros function_id plans plan Hlookup.
  simpl. rewrite Hlookup. simpl. auto.
Qed.

Lemma summary_lookup_miss_skips_entry :
  forall function_id plans,
    lookup_collected_plan function_id plans = None ->
    plan_summary_entries [function_id] plans = [].
Proof.
  intros function_id plans Hlookup.
  simpl. rewrite Hlookup. reflexivity.
Qed.

Lemma summary_entry_exposes_schedule :
  forall plan,
    summary_schedule (summarize_plan plan) = plan_schedule plan.
Proof. reflexivity. Qed.

Lemma summary_entry_exposes_profile :
  forall plan,
    summary_profile (summarize_plan plan) = plan_profile plan.
Proof. reflexivity. Qed.

Lemma summary_entry_exposes_pass_groups :
  forall plan,
    summary_pass_groups (summarize_plan plan) = plan_pass_groups plan.
Proof. reflexivity. Qed.

Lemma summary_lookup_preserves_verify_ir :
  forall function_id plans plan fn,
    lookup_collected_plan function_id plans = Some plan ->
    optimizer_eligible fn ->
    optimizer_eligible (plan_selected_pipeline plan fn).
Proof.
  intros function_id plans plan fn Hlookup H.
  exact (selected_plan_preserves_verify_ir plan fn H).
Qed.

Lemma summary_lookup_preserves_semantics :
  forall function_id plans plan fn ρ,
    lookup_collected_plan function_id plans = Some plan ->
    exec_entry (plan_selected_pipeline plan fn) ρ = exec_entry fn ρ.
Proof.
  intros function_id plans plan fn ρ Hlookup.
  exact (selected_plan_preserves_semantics plan fn ρ).
Qed.

End RRPhasePlanSummarySoundness.
