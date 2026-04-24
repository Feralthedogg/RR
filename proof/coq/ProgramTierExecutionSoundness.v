Require Import RRProofs.MirInvariantBundle.
Require Import RRProofs.PhaseOrderOptimizerSoundness.
Require Import RRProofs.PhasePlanSoundness.
Require Import RRProofs.ProgramOptPlanSoundness.
Require Import RRProofs.PhasePlanLookupSoundness.
Require Import RRProofs.ProgramPhasePipelineSoundness.
From Stdlib Require Import List Bool.
Import ListNotations.
Open Scope bool_scope.

Module RRProgramTierExecutionSoundness.

Import RRMirInvariantBundle.
Import RRPhaseOrderOptimizerSoundness.
Import RRPhasePlanSoundness.
Import RRProgramOptPlanSoundness.
Import RRPhasePlanLookupSoundness.
Import RRProgramPhasePipelineSoundness.

Inductive reduced_heavy_tier_decision : Type :=
| RHTSkipConservative
| RHTSkipSelfRecursive
| RHTSkipHeavyTierDisabled
| RHTSkipBudget
| RHTUseCollectedPlan
| RHTUseLegacyPlan.

Definition legacy_function_phase_plan
    (function_id : nat)
    (mode : reduced_phase_ordering_mode)
    (trace_requested : bool)
    (features : reduced_plan_features) : reduced_function_phase_plan :=
  {| plan_function_id := function_id;
     plan_mode := mode;
     plan_profile := RPPBalanced;
     plan_schedule := RPSBalanced;
     plan_pass_groups := default_pass_groups_for_schedule RPSBalanced;
     plan_features := features;
     plan_trace_requested := trace_requested |}.

Definition execute_heavy_tier_decision
    (mode : reduced_phase_ordering_mode)
    (trace_requested : bool)
    (entry : reduced_program_phase_entry)
    (decision : reduced_heavy_tier_decision)
    (selected_plan : option reduced_function_phase_plan)
    (fn : mir_fn_lite) : mir_fn_lite :=
  match decision with
  | RHTSkipConservative
  | RHTSkipSelfRecursive
  | RHTSkipHeavyTierDisabled
  | RHTSkipBudget => fn
  | RHTUseCollectedPlan =>
      match selected_plan with
      | Some plan => plan_selected_pipeline plan fn
      | None => fn
      end
  | RHTUseLegacyPlan =>
      plan_selected_pipeline
        (legacy_function_phase_plan (rppe_function_id entry) mode trace_requested (rppe_features entry))
        fn
  end.

Definition execute_program_heavy_function
    (mode : reduced_phase_ordering_mode)
    (trace_requested fast_dev run_heavy_tier : bool)
    (plan : reduced_program_opt_plan)
    (entries : list reduced_program_phase_entry)
    (entry : reduced_program_phase_entry)
    (fn : mir_fn_lite) : mir_fn_lite :=
  if rppe_conservative entry then
    execute_heavy_tier_decision mode trace_requested entry RHTSkipConservative None fn
  else if rppe_self_recursive entry then
    execute_heavy_tier_decision mode trace_requested entry RHTSkipSelfRecursive None fn
  else if negb run_heavy_tier then
    execute_heavy_tier_decision mode trace_requested entry RHTSkipHeavyTierDisabled None fn
  else if negb (selected_by_program_plan plan entry) then
    execute_heavy_tier_decision mode trace_requested entry RHTSkipBudget None fn
  else
    match lookup_collected_plan (rppe_function_id entry)
      (collect_program_phase_plans mode trace_requested fast_dev run_heavy_tier plan entries) with
    | Some selected_plan =>
        execute_heavy_tier_decision mode trace_requested entry RHTUseCollectedPlan (Some selected_plan) fn
    | None =>
        execute_heavy_tier_decision mode trace_requested entry RHTUseLegacyPlan None fn
    end.

Lemma execute_program_heavy_function_preserves_verify_ir :
  forall mode trace_requested fast_dev run_heavy_tier plan entries entry fn,
    optimizer_eligible fn ->
    optimizer_eligible
      (execute_program_heavy_function mode trace_requested fast_dev run_heavy_tier plan entries entry fn).
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries entry fn H.
  unfold execute_program_heavy_function.
  destruct (rppe_conservative entry) eqn:Hconservative.
  - exact H.
  - destruct (rppe_self_recursive entry) eqn:Hself.
    + exact H.
    + destruct run_heavy_tier eqn:Hheavy.
      * destruct (selected_by_program_plan plan entry) eqn:Hselected.
        -- destruct (lookup_collected_plan (rppe_function_id entry)
             (collect_program_phase_plans mode trace_requested fast_dev true plan entries)) eqn:Hlookup.
           ++ simpl.
              exact (program_phase_lookup_preserves_verify_ir
                mode trace_requested fast_dev true plan entries (rppe_function_id entry) r fn Hlookup H).
           ++ simpl.
              exact (selected_plan_preserves_verify_ir
                (legacy_function_phase_plan (rppe_function_id entry) mode trace_requested (rppe_features entry))
                fn H).
        -- exact H.
      * exact H.
Qed.

Lemma execute_program_heavy_function_preserves_semantics :
  forall mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ,
    exec_entry
      (execute_program_heavy_function mode trace_requested fast_dev run_heavy_tier plan entries entry fn) ρ
      = exec_entry fn ρ.
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ.
  unfold execute_program_heavy_function.
  destruct (rppe_conservative entry) eqn:Hconservative.
  - reflexivity.
  - destruct (rppe_self_recursive entry) eqn:Hself.
    + reflexivity.
    + destruct run_heavy_tier eqn:Hheavy.
      * destruct (selected_by_program_plan plan entry) eqn:Hselected.
        -- destruct (lookup_collected_plan (rppe_function_id entry)
             (collect_program_phase_plans mode trace_requested fast_dev true plan entries)) eqn:Hlookup.
           ++ simpl.
              exact (program_phase_lookup_preserves_semantics
                mode trace_requested fast_dev true plan entries (rppe_function_id entry) r fn ρ Hlookup).
           ++ simpl.
              exact (selected_plan_preserves_semantics
                (legacy_function_phase_plan (rppe_function_id entry) mode trace_requested (rppe_features entry))
                fn ρ).
        -- reflexivity.
      * reflexivity.
Qed.

End RRProgramTierExecutionSoundness.
