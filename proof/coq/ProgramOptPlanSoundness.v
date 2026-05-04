Require Import MirInvariantBundle.
From Stdlib Require Import List Bool PeanoNat.
Import ListNotations.
Open Scope bool_scope.

Module RRProgramOptPlanSoundness.

Record reduced_program_function_entry : Type := {
  rpf_function_id : nat;
  rpf_ir_size : nat;
  rpf_score : nat;
  rpf_hot_weight : nat;
  rpf_conservative : bool;
}.

Record reduced_program_opt_plan : Type := {
  rpp_program_limit : nat;
  rpp_fn_limit : nat;
  rpp_total_ir : nat;
  rpp_max_fn_ir : nat;
  rpp_selective_mode : bool;
  rpp_selected_functions : list nat;
}.

Definition entry_weighted_score (entry : reduced_program_function_entry) : nat :=
  rpf_score entry * rpf_hot_weight entry / 1024.

Definition entry_density (entry : reduced_program_function_entry) : nat :=
  entry_weighted_score entry * 1024 / Nat.max (rpf_ir_size entry) 1.

Fixpoint eligible_function_ids (entries : list reduced_program_function_entry) : list nat :=
  match entries with
  | [] => []
  | entry :: rest =>
      if rpf_conservative entry then
        eligible_function_ids rest
      else
        rpf_function_id entry :: eligible_function_ids rest
  end.

Definition select_under_budget (entries : list reduced_program_function_entry) : list nat :=
  eligible_function_ids entries.

Definition select_within_budget
    (program_limit soft_fn_limit : nat)
    (entries : list reduced_program_function_entry) : list nat :=
  let candidates := filter (fun entry =>
    negb (rpf_conservative entry) && Nat.leb (rpf_ir_size entry) soft_fn_limit) entries in
  let step (used_and_selected : nat * list nat) (entry : reduced_program_function_entry)
      : nat * list nat :=
    let '(used_budget, selected) := used_and_selected in
    if Nat.leb (used_budget + rpf_ir_size entry) program_limit then
      (used_budget + rpf_ir_size entry, selected ++ [rpf_function_id entry])
    else
      used_and_selected in
  snd (fold_left step candidates (0, [])).

Definition insert_by_ir_size
    (entry : reduced_program_function_entry)
    (entries : list reduced_program_function_entry) : list reduced_program_function_entry :=
  let fix go xs :=
    match xs with
    | [] => [entry]
    | x :: rest =>
        if Nat.ltb (rpf_ir_size entry) (rpf_ir_size x) ||
            ((Nat.eqb (rpf_ir_size entry) (rpf_ir_size x)) &&
             Nat.ltb (rpf_function_id entry) (rpf_function_id x)) then
          entry :: xs
        else
          x :: go rest
    end in
  go entries.

Definition sort_by_ir_size
    (entries : list reduced_program_function_entry) : list reduced_program_function_entry :=
  fold_right insert_by_ir_size [] entries.

Definition fallback_smallest_eligible (entries : list reduced_program_function_entry)
    : option nat :=
  match sort_by_ir_size (filter (fun entry => negb (rpf_conservative entry)) entries) with
  | [] => None
  | entry :: _ => Some (rpf_function_id entry)
  end.

Definition build_program_opt_plan
    (program_limit fn_limit total_ir max_fn_ir : nat)
    (entries : list reduced_program_function_entry) : reduced_program_opt_plan :=
  let needs_budget := Nat.ltb program_limit total_ir || Nat.ltb fn_limit max_fn_ir in
  if negb needs_budget then
    {| rpp_program_limit := program_limit;
       rpp_fn_limit := fn_limit;
       rpp_total_ir := total_ir;
       rpp_max_fn_ir := max_fn_ir;
       rpp_selective_mode := false;
       rpp_selected_functions := select_under_budget entries |}
  else
    let soft_fn_limit := Nat.min fn_limit (Nat.max 64 fn_limit) in
    let selected := select_within_budget program_limit soft_fn_limit entries in
    let selected' :=
      if List.length selected =? 0 then
        match fallback_smallest_eligible entries with
        | Some function_id => [function_id]
        | None => []
        end
      else
        selected in
    {| rpp_program_limit := program_limit;
       rpp_fn_limit := fn_limit;
       rpp_total_ir := total_ir;
       rpp_max_fn_ir := max_fn_ir;
       rpp_selective_mode := true;
       rpp_selected_functions := selected' |}.

Definition under_budget_sample_entries : list reduced_program_function_entry :=
  [ {| rpf_function_id := 1; rpf_ir_size := 10; rpf_score := 20; rpf_hot_weight := 1024; rpf_conservative := false |};
    {| rpf_function_id := 2; rpf_ir_size := 12; rpf_score := 18; rpf_hot_weight := 1024; rpf_conservative := false |};
    {| rpf_function_id := 3; rpf_ir_size := 8; rpf_score := 7; rpf_hot_weight := 1024; rpf_conservative := true |} ].

Definition over_budget_sample_entries : list reduced_program_function_entry :=
  [ {| rpf_function_id := 10; rpf_ir_size := 40; rpf_score := 100; rpf_hot_weight := 1024; rpf_conservative := false |};
    {| rpf_function_id := 11; rpf_ir_size := 60; rpf_score := 90; rpf_hot_weight := 1024; rpf_conservative := false |};
    {| rpf_function_id := 12; rpf_ir_size := 200; rpf_score := 5; rpf_hot_weight := 1024; rpf_conservative := false |} ].

Definition fallback_sample_entries : list reduced_program_function_entry :=
  [ {| rpf_function_id := 20; rpf_ir_size := 200; rpf_score := 5; rpf_hot_weight := 1024; rpf_conservative := false |};
    {| rpf_function_id := 21; rpf_ir_size := 80; rpf_score := 4; rpf_hot_weight := 1024; rpf_conservative := false |} ].

Lemma under_budget_plan_selects_all_safe :
  rpp_selected_functions (build_program_opt_plan 128 128 32 12 under_budget_sample_entries) = [1; 2].
Proof. reflexivity. Qed.

Lemma over_budget_plan_is_selective :
  rpp_selective_mode (build_program_opt_plan 100 50 300 200 over_budget_sample_entries) = true.
Proof. reflexivity. Qed.

Lemma over_budget_plan_selects_within_budget_prefix :
  rpp_selected_functions (build_program_opt_plan 100 50 300 200 over_budget_sample_entries) = [10].
Proof. reflexivity. Qed.

Lemma fallback_plan_selects_smallest_when_budget_selection_empty :
  rpp_selected_functions (build_program_opt_plan 10 10 280 200 fallback_sample_entries) = [21].
Proof. reflexivity. Qed.

End RRProgramOptPlanSoundness.
