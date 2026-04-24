Require Import RRProofs.MirInvariantBundle.
Require Import RRProofs.PhaseOrderOptimizerSoundness.
From Stdlib Require Import List Bool.
Import ListNotations.
Open Scope bool_scope.

Module RRPhasePlanSoundness.

Import RRMirInvariantBundle.
Import RRPhaseOrderOptimizerSoundness.

Inductive reduced_phase_ordering_mode : Type :=
| RPOMOff
| RPOMBalanced
| RPOMAuto.

Inductive reduced_phase_profile_kind : Type :=
| RPPBalanced
| RPPComputeHeavy
| RPPControlFlowHeavy.

Inductive reduced_pass_group : Type :=
| RPGRequired
| RPGDevCheap
| RPGReleaseExpensive
| RPGExperimental.

Record reduced_plan_features : Type := {
  pf_ir_size : nat;
  pf_block_count : nat;
  pf_loop_count : nat;
  pf_canonical_loop_count : nat;
  pf_branch_terms : nat;
  pf_phi_count : nat;
  pf_arithmetic_values : nat;
  pf_intrinsic_values : nat;
  pf_call_values : nat;
  pf_side_effecting_calls : nat;
  pf_index_values : nat;
  pf_store_instrs : nat;
}.

Record reduced_function_phase_plan : Type := {
  plan_function_id : nat;
  plan_mode : reduced_phase_ordering_mode;
  plan_profile : reduced_phase_profile_kind;
  plan_schedule : reduced_phase_schedule;
  plan_pass_groups : list reduced_pass_group;
  plan_features : reduced_plan_features;
  plan_trace_requested : bool;
}.

Definition compute_phase_profile_scores (features : reduced_plan_features) : nat * nat :=
  let compute_score :=
    pf_canonical_loop_count features * 32 +
    pf_loop_count features * 16 +
    pf_arithmetic_values features * 2 +
    pf_intrinsic_values features * 4 +
    pf_index_values features * 2 +
    pf_store_instrs features * 2 in
  let control_score :=
    pf_branch_terms features * 18 +
    pf_phi_count features * 8 +
    pf_side_effecting_calls features * 16 in
  (compute_score, control_score).

Definition classify_phase_profile (features : reduced_plan_features)
    : reduced_phase_profile_kind :=
  let '(compute_score, control_score) := compute_phase_profile_scores features in
  let branch_density_high :=
    Nat.leb (Nat.max (pf_block_count features) 1)
      (pf_branch_terms features * 3) in
  let side_effects_light :=
    Nat.leb (pf_side_effecting_calls features * 4)
      (Nat.max (pf_call_values features) 1) in
  let compute_schedule_safe :=
    Nat.leb (pf_ir_size features) 256 &&
    Nat.leb (pf_block_count features) 16 &&
    negb (Nat.eqb (pf_canonical_loop_count features) 0) &&
    Nat.eqb (pf_side_effecting_calls features) 0 in
  let control_schedule_safe :=
    Nat.leb (pf_ir_size features) 128 &&
    Nat.leb (pf_block_count features) 12 &&
    Nat.eqb (pf_loop_count features) 0 in
  if compute_schedule_safe && side_effects_light &&
      Nat.leb (control_score + 24) compute_score then
    RPPComputeHeavy
  else if control_schedule_safe &&
      (Nat.leb (compute_score + 24) control_score || branch_density_high) then
    RPPControlFlowHeavy
  else
    RPPBalanced.

Definition choose_phase_schedule
    (mode : reduced_phase_ordering_mode)
    (profile : reduced_phase_profile_kind) : reduced_phase_schedule :=
  match mode with
  | RPOMOff | RPOMBalanced => RPSBalanced
  | RPOMAuto =>
      match profile with
      | RPPBalanced => RPSBalanced
      | RPPComputeHeavy => RPSComputeHeavy
      | RPPControlFlowHeavy => RPSControlFlowHeavy
      end
  end.

Definition default_pass_groups_for_schedule (schedule : reduced_phase_schedule)
    : list reduced_pass_group :=
  match schedule with
  | RPSBalanced | RPSControlFlowHeavy =>
      [RPGRequired; RPGDevCheap; RPGReleaseExpensive]
  | RPSComputeHeavy =>
      [RPGRequired; RPGDevCheap; RPGReleaseExpensive; RPGExperimental]
  end.

Definition adjust_pass_groups_for_fast_dev (fast_dev : bool)
    (groups : list reduced_pass_group) : list reduced_pass_group :=
  filter (fun g =>
    match g with
    | RPGRequired | RPGDevCheap => true
    | RPGReleaseExpensive | RPGExperimental => negb fast_dev
    end) groups.

Definition build_function_phase_plan
    (function_id : nat)
    (mode : reduced_phase_ordering_mode)
    (trace_requested fast_dev : bool)
    (features : reduced_plan_features) : reduced_function_phase_plan :=
  let profile :=
    match mode with
    | RPOMAuto => classify_phase_profile features
    | _ => RPPBalanced
    end in
  let schedule := choose_phase_schedule mode profile in
  {| plan_function_id := function_id;
     plan_mode := mode;
     plan_profile := profile;
     plan_schedule := schedule;
     plan_pass_groups :=
       adjust_pass_groups_for_fast_dev fast_dev
         (default_pass_groups_for_schedule schedule);
     plan_features := features;
     plan_trace_requested := trace_requested |}.

Definition plan_selected_pipeline (plan : reduced_function_phase_plan)
    (fn : mir_fn_lite) : mir_fn_lite :=
  phase_scheduled_pipeline (plan_schedule plan) fn.

Definition compute_heavy_sample : reduced_plan_features :=
  {| pf_ir_size := 64;
     pf_block_count := 6;
     pf_loop_count := 2;
     pf_canonical_loop_count := 2;
     pf_branch_terms := 1;
     pf_phi_count := 1;
     pf_arithmetic_values := 12;
     pf_intrinsic_values := 4;
     pf_call_values := 0;
     pf_side_effecting_calls := 0;
     pf_index_values := 3;
     pf_store_instrs := 2 |}.

Definition control_flow_heavy_sample : reduced_plan_features :=
  {| pf_ir_size := 48;
     pf_block_count := 4;
     pf_loop_count := 0;
     pf_canonical_loop_count := 0;
     pf_branch_terms := 3;
     pf_phi_count := 2;
     pf_arithmetic_values := 0;
     pf_intrinsic_values := 0;
     pf_call_values := 2;
     pf_side_effecting_calls := 2;
     pf_index_values := 0;
     pf_store_instrs := 0 |}.

Definition balanced_sample : reduced_plan_features :=
  {| pf_ir_size := 200;
     pf_block_count := 20;
     pf_loop_count := 1;
     pf_canonical_loop_count := 0;
     pf_branch_terms := 2;
     pf_phi_count := 1;
     pf_arithmetic_values := 1;
     pf_intrinsic_values := 0;
     pf_call_values := 2;
     pf_side_effecting_calls := 1;
     pf_index_values := 0;
     pf_store_instrs := 0 |}.

Lemma compute_heavy_sample_classifies_compute_heavy :
  classify_phase_profile compute_heavy_sample = RPPComputeHeavy.
Proof. reflexivity. Qed.

Lemma control_flow_heavy_sample_classifies_control_flow_heavy :
  classify_phase_profile control_flow_heavy_sample = RPPControlFlowHeavy.
Proof. reflexivity. Qed.

Lemma balanced_sample_classifies_balanced :
  classify_phase_profile balanced_sample = RPPBalanced.
Proof. reflexivity. Qed.

Lemma choose_phase_schedule_off_is_balanced :
  forall profile,
    choose_phase_schedule RPOMOff profile = RPSBalanced.
Proof.
  intros profile. destruct profile; reflexivity.
Qed.

Lemma choose_phase_schedule_balanced_mode_is_balanced :
  forall profile,
    choose_phase_schedule RPOMBalanced profile = RPSBalanced.
Proof.
  intros profile. destruct profile; reflexivity.
Qed.

Lemma choose_phase_schedule_auto_uses_profile :
  forall profile,
    choose_phase_schedule RPOMAuto profile =
      match profile with
      | RPPBalanced => RPSBalanced
      | RPPComputeHeavy => RPSComputeHeavy
      | RPPControlFlowHeavy => RPSControlFlowHeavy
      end.
Proof.
  intros profile. destruct profile; reflexivity.
Qed.

Lemma fast_dev_group_filter_drops_expensive_groups :
  forall schedule,
    adjust_pass_groups_for_fast_dev true (default_pass_groups_for_schedule schedule) =
      [RPGRequired; RPGDevCheap].
Proof.
  intros schedule. destruct schedule; reflexivity.
Qed.

Lemma build_phase_plan_non_auto_uses_balanced_profile :
  forall function_id trace_requested fast_dev features,
    plan_profile
      (build_function_phase_plan function_id RPOMBalanced trace_requested fast_dev features)
      = RPPBalanced.
Proof. reflexivity. Qed.

Lemma build_phase_plan_auto_uses_classified_profile :
  forall function_id trace_requested fast_dev features,
    plan_profile
      (build_function_phase_plan function_id RPOMAuto trace_requested fast_dev features)
      = classify_phase_profile features.
Proof. reflexivity. Qed.

Lemma build_phase_plan_schedule_matches_choice :
  forall function_id mode trace_requested fast_dev features,
    plan_schedule (build_function_phase_plan function_id mode trace_requested fast_dev features) =
      choose_phase_schedule mode
        (plan_profile (build_function_phase_plan function_id mode trace_requested fast_dev features)).
Proof.
  intros function_id mode trace_requested fast_dev features.
  destruct mode; reflexivity.
Qed.

Lemma selected_plan_preserves_verify_ir :
  forall plan fn,
    optimizer_eligible fn ->
    optimizer_eligible (plan_selected_pipeline plan fn).
Proof.
  intros plan fn H.
  unfold plan_selected_pipeline.
  destruct (plan_schedule plan).
  - exact (phase_schedule_balanced_preserves_verify_ir fn H).
  - exact (phase_schedule_compute_heavy_preserves_verify_ir fn H).
  - exact (phase_schedule_control_flow_heavy_preserves_verify_ir fn H).
Qed.

Lemma selected_plan_preserves_semantics :
  forall plan fn ρ,
    exec_entry (plan_selected_pipeline plan fn) ρ = exec_entry fn ρ.
Proof.
  intros plan fn ρ.
  unfold plan_selected_pipeline.
  destruct (plan_schedule plan).
  - exact (phase_schedule_balanced_preserves_semantics fn ρ).
  - exact (phase_schedule_compute_heavy_preserves_semantics fn ρ).
  - exact (phase_schedule_control_flow_heavy_preserves_semantics fn ρ).
Qed.

End RRPhasePlanSoundness.
