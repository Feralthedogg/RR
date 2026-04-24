Require Import RRProofs.MirInvariantBundle.
Require Import RRProofs.ProgramOptPlanSoundness.
Require Import RRProofs.PhasePlanSoundness.
Require Import RRProofs.PhasePlanCollectionSoundness.
Require Import RRProofs.PhasePlanLookupSoundness.
Require Import RRProofs.PhasePlanSummarySoundness.
From Stdlib Require Import List Bool.
Import ListNotations.
Open Scope bool_scope.

Module RRProgramPhasePipelineSoundness.

Import RRMirInvariantBundle.
Import RRProgramOptPlanSoundness.
Import RRPhasePlanSoundness.
Import RRPhasePlanCollectionSoundness.
Import RRPhasePlanLookupSoundness.
Import RRPhasePlanSummarySoundness.

Record reduced_program_phase_entry : Type := {
  rppe_function_id : nat;
  rppe_features : reduced_plan_features;
  rppe_ir_size : nat;
  rppe_score : nat;
  rppe_hot_weight : nat;
  rppe_present : bool;
  rppe_conservative : bool;
  rppe_self_recursive : bool;
}.

Definition budget_entry_of (entry : reduced_program_phase_entry)
    : reduced_program_function_entry :=
  {| rpf_function_id := rppe_function_id entry;
     rpf_ir_size := rppe_ir_size entry;
     rpf_score := rppe_score entry;
     rpf_hot_weight := rppe_hot_weight entry;
     rpf_conservative := rppe_conservative entry |}.

Definition selected_by_program_plan
    (plan : reduced_program_opt_plan)
    (entry : reduced_program_phase_entry) : bool :=
  if rpp_selective_mode plan then
    existsb (Nat.eqb (rppe_function_id entry)) (rpp_selected_functions plan)
  else
    true.

Definition phase_inventory_entry_of
    (plan : reduced_program_opt_plan)
    (entry : reduced_program_phase_entry) : reduced_function_inventory_entry :=
  {| entry_function_id := rppe_function_id entry;
     entry_features := rppe_features entry;
     entry_present := rppe_present entry;
     entry_conservative := rppe_conservative entry;
     entry_self_recursive := rppe_self_recursive entry;
     entry_selected := selected_by_program_plan plan entry |}.

Definition collect_program_phase_plans
    (mode : reduced_phase_ordering_mode)
    (trace_requested fast_dev run_heavy_tier : bool)
    (plan : reduced_program_opt_plan)
    (entries : list reduced_program_phase_entry) : list reduced_function_phase_plan :=
  if run_heavy_tier then
    collect_function_phase_plans mode trace_requested fast_dev
      (map (phase_inventory_entry_of plan) entries)
  else
    [].

Definition program_phase_summary_entries
    (ordered_function_ids : list nat)
    (mode : reduced_phase_ordering_mode)
    (trace_requested fast_dev run_heavy_tier : bool)
    (plan : reduced_program_opt_plan)
    (entries : list reduced_program_phase_entry) : list reduced_plan_summary_entry :=
  plan_summary_entries ordered_function_ids
    (collect_program_phase_plans mode trace_requested fast_dev run_heavy_tier plan entries).

Lemma non_selective_plan_marks_every_entry_selected :
  forall plan entry,
    rpp_selective_mode plan = false ->
    selected_by_program_plan plan entry = true.
Proof.
  intros plan entry H.
  unfold selected_by_program_plan. rewrite H. reflexivity.
Qed.

Lemma selective_plan_marks_membership_selected :
  forall plan entry,
    rpp_selective_mode plan = true ->
    selected_by_program_plan plan entry =
      existsb (Nat.eqb (rppe_function_id entry)) (rpp_selected_functions plan).
Proof.
  intros plan entry H.
  unfold selected_by_program_plan. rewrite H. reflexivity.
Qed.

Lemma heavy_tier_disabled_collects_no_phase_plans :
  forall mode trace_requested fast_dev plan entries,
    collect_program_phase_plans mode trace_requested fast_dev false plan entries = [].
Proof. reflexivity. Qed.

Lemma heavy_tier_disabled_emits_no_phase_summary :
  forall ordered_function_ids mode trace_requested fast_dev plan entries,
    program_phase_summary_entries ordered_function_ids mode trace_requested fast_dev false plan entries = [].
Proof.
  intros ordered_function_ids mode trace_requested fast_dev plan entries.
  induction ordered_function_ids as [| function_id rest IH].
  - reflexivity.
  - simpl. exact IH.
Qed.

Lemma program_phase_lookup_preserves_verify_ir :
  forall mode trace_requested fast_dev run_heavy_tier plan entries function_id selected_plan fn,
    lookup_collected_plan function_id
      (collect_program_phase_plans mode trace_requested fast_dev run_heavy_tier plan entries)
      = Some selected_plan ->
    optimizer_eligible fn ->
    optimizer_eligible (plan_selected_pipeline selected_plan fn).
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries function_id selected_plan fn Hlookup H.
  exact (lookup_collected_plan_preserves_verify_ir function_id _ selected_plan fn Hlookup H).
Qed.

Lemma program_phase_lookup_preserves_semantics :
  forall mode trace_requested fast_dev run_heavy_tier plan entries function_id selected_plan fn ρ,
    lookup_collected_plan function_id
      (collect_program_phase_plans mode trace_requested fast_dev run_heavy_tier plan entries)
      = Some selected_plan ->
    exec_entry (plan_selected_pipeline selected_plan fn) ρ = exec_entry fn ρ.
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries function_id selected_plan fn ρ Hlookup.
  exact (lookup_collected_plan_preserves_semantics function_id _ selected_plan fn ρ Hlookup).
Qed.

Lemma program_phase_summary_hit_emits_entry :
  forall mode trace_requested fast_dev run_heavy_tier plan entries function_id selected_plan,
    lookup_collected_plan function_id
      (collect_program_phase_plans mode trace_requested fast_dev run_heavy_tier plan entries)
      = Some selected_plan ->
    In (summarize_plan selected_plan)
      (program_phase_summary_entries [function_id] mode trace_requested fast_dev run_heavy_tier plan entries).
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries function_id selected_plan Hlookup.
  exact (summary_lookup_hit_emits_entry function_id
    (collect_program_phase_plans mode trace_requested fast_dev run_heavy_tier plan entries)
    selected_plan Hlookup).
Qed.

Lemma program_phase_summary_miss_skips_entry :
  forall mode trace_requested fast_dev run_heavy_tier plan entries function_id,
    lookup_collected_plan function_id
      (collect_program_phase_plans mode trace_requested fast_dev run_heavy_tier plan entries)
      = None ->
    program_phase_summary_entries [function_id] mode trace_requested fast_dev run_heavy_tier plan entries = [].
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries function_id Hlookup.
  exact (summary_lookup_miss_skips_entry function_id
    (collect_program_phase_plans mode trace_requested fast_dev run_heavy_tier plan entries)
    Hlookup).
Qed.

End RRProgramPhasePipelineSoundness.
