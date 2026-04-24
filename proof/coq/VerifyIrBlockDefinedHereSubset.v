Require Import VerifyIrBlockAssignBranchSubset.
Require Import VerifyIrBlockAssignChainSubset.
Require Import VerifyIrBlockAssignStoreSubset.
Require Import VerifyIrBlockFlowSubset.
Require Import VerifyIrBlockRecordSubset.
Require Import VerifyIrMustDefSubset.
Require Import VerifyIrValueFullRecordSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrBlockAssignBranchSubset.
Import RRVerifyIrBlockAssignChainSubset.
Import RRVerifyIrBlockAssignStoreSubset.
Import RRVerifyIrBlockFlowSubset.
Import RRVerifyIrMustDefSubset.
Import RRVerifyIrBlockRecordSubset.
Import RRVerifyIrValueFullRecordSubset.

Module RRVerifyIrBlockDefinedHereSubset.

Fixpoint scan_defined_vars (defined : def_set) (instrs : list instr_record_lite) : def_set :=
  match instrs with
  | [] => defined
  | instr :: instrs' => scan_defined_vars (List.app defined (instr_record_writes instr)) instrs'
  end.

Definition final_defined_vars (defined : def_set) (bb : actual_block_record_lite) : def_set :=
  scan_defined_vars defined (actual_block_instrs bb).

Lemma step_instr_flow_fst_eq_scan_seed :
  forall table defined required instr,
    fst (step_instr_flow table (defined, required) instr) =
    scan_defined_vars defined [instr].
Proof.
  intros table defined required instr. reflexivity.
Qed.

Lemma fold_step_instr_flow_fst_eq_scan_defined_vars :
  forall table instrs defined required,
    fst (fold_left (step_instr_flow table) instrs (defined, required)) =
    scan_defined_vars defined instrs.
Proof.
  intros table instrs.
  induction instrs as [|instr instrs IH]; intros defined required.
  - reflexivity.
  - simpl.
    exact (IH (List.app defined (instr_record_writes instr))
      (snd (step_instr_flow table (defined, required) instr))).
Qed.

Lemma final_defined_vars_eq_fold_step_instr_flow :
  forall table defined bb,
    final_defined_vars defined bb =
    fst (fold_left (step_instr_flow table) (actual_block_instrs bb) (defined, [])).
Proof.
  intros table defined bb.
  unfold final_defined_vars.
  symmetry.
  apply fold_step_instr_flow_fst_eq_scan_defined_vars.
Qed.

Lemma in_scan_defined_vars_of_in_init :
  forall defined instrs v,
    In v defined ->
    In v (scan_defined_vars defined instrs).
Proof.
  intros defined instrs v H.
  induction instrs as [|instr instrs IH] in defined, H |- *.
  - simpl. exact H.
  - simpl.
    apply IH.
    apply in_or_app.
    left. exact H.
Qed.

Lemma in_final_defined_vars_of_in_init :
  forall defined bb v,
    In v defined ->
    In v (final_defined_vars defined bb).
Proof.
  intros defined bb v H.
  unfold final_defined_vars.
  apply in_scan_defined_vars_of_in_init.
  exact H.
Qed.

Lemma example_good_actual_block_final_defined_vars :
  final_defined_vars ["y"] example_good_actual_block = ["y"; "x"].
Proof.
  reflexivity.
Qed.

Lemma example_assign_chain_block_final_defined_vars :
  final_defined_vars ["y"] example_assign_chain_block = ["y"; "loop"; "x"].
Proof.
  reflexivity.
Qed.

Lemma example_assign_branch_block_final_defined_vars :
  final_defined_vars ["y"] example_assign_branch_block = ["y"; "loop"; "x"].
Proof.
  reflexivity.
Qed.

Lemma example_assign_store1d_block_final_defined_vars :
  final_defined_vars ["y"] example_assign_store1d_block = ["y"; "loop"; "x"].
Proof.
  reflexivity.
Qed.

Lemma example_assign_store2d_block_final_defined_vars :
  final_defined_vars ["y"] example_assign_store2d_block = ["y"; "loop"; "x"].
Proof.
  reflexivity.
Qed.

Lemma example_assign_store3d_block_final_defined_vars :
  final_defined_vars ["y"] example_assign_store3d_block = ["y"; "loop"; "x"].
Proof.
  reflexivity.
Qed.

Lemma example_assign_chain_block_preserves_incoming_y :
  In "y" (final_defined_vars ["y"] example_assign_chain_block).
Proof.
  apply in_final_defined_vars_of_in_init.
  simpl. auto.
Qed.

End RRVerifyIrBlockDefinedHereSubset.
