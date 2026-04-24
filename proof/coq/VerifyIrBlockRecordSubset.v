Require Import VerifyIrConsumerGraphSubset.
Require Import VerifyIrFnRecordSubset.
Require Import VerifyIrFnHintMapSubset.
Require Import VerifyIrValueFullRecordSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrConsumerGraphSubset.
Import RRVerifyIrFnRecordSubset.
Import RRVerifyIrFnHintMapSubset.
Import RRVerifyIrValueFullRecordSubset.

Module RRVerifyIrBlockRecordSubset.

Inductive instr_record_lite : Type :=
| IRAssign : string -> consumer_node_id -> span_tag -> instr_record_lite
| IREval : consumer_node_id -> span_tag -> instr_record_lite
| IRStoreIndex1D : consumer_node_id -> consumer_node_id -> consumer_node_id ->
    span_tag -> instr_record_lite
| IRStoreIndex2D : consumer_node_id -> consumer_node_id -> consumer_node_id -> consumer_node_id ->
    span_tag -> instr_record_lite
| IRStoreIndex3D : consumer_node_id -> consumer_node_id -> consumer_node_id -> consumer_node_id ->
    consumer_node_id -> span_tag -> instr_record_lite.

Inductive terminator_record_lite : Type :=
| TRLGoto : nat -> terminator_record_lite
| TRLBranch : consumer_node_id -> nat -> nat -> terminator_record_lite
| TRLRet : option consumer_node_id -> terminator_record_lite
| TRLUnreachable : terminator_record_lite.

Record actual_block_record_lite : Type := {
  actual_block_id : nat;
  actual_block_instrs : list instr_record_lite;
  actual_block_term : terminator_record_lite;
}.

Definition terminator_record_to_term_tag (term : terminator_record_lite) : term_tag :=
  match term with
  | TRLGoto target => TGoto target
  | TRLBranch _ then_bb else_bb => TBranch then_bb else_bb
  | TRLRet _ => TRet
  | TRLUnreachable => TUnreachable
  end.

Definition actual_block_record_to_block_lite (bb : actual_block_record_lite) : block_lite :=
  {| block_id_lite := actual_block_id bb;
     block_term_lite := terminator_record_to_term_tag (actual_block_term bb) |}.

Record fn_block_record_lite : Type := {
  fn_block_shell : fn_hint_map_record_lite;
  fn_block_blocks : list actual_block_record_lite;
}.

Definition fn_block_record_to_fn_hint_map (fn_block : fn_block_record_lite) : fn_hint_map_record_lite :=
  fn_block_shell fn_block.

Definition fn_block_record_to_fn_record (fn_block : fn_block_record_lite) : fn_record_lite :=
  {| fn_record_name := fn_record_name (fn_hint_map_to_fn_record (fn_block_shell fn_block));
     fn_record_params := fn_record_params (fn_hint_map_to_fn_record (fn_block_shell fn_block));
     fn_record_values := fn_record_values (fn_hint_map_to_fn_record (fn_block_shell fn_block));
     fn_record_blocks := map actual_block_record_to_block_lite (fn_block_blocks fn_block);
     fn_record_entry := fn_record_entry (fn_hint_map_to_fn_record (fn_block_shell fn_block));
     fn_record_body_head := fn_record_body_head (fn_hint_map_to_fn_record (fn_block_shell fn_block)) |}.

Definition fn_block_record_lookup_value_deps
    (fn_block : fn_block_record_lite) (root : consumer_node_id)
    : option (list consumer_node_id) :=
  fn_hint_map_lookup_value_deps (fn_block_shell fn_block) root.

Definition fn_block_record_depends_on_phi_in_block_except_fuel
    (fuel : nat) (fn_block : fn_block_record_lite) (seen : list consumer_node_id)
    (root phi_block exempt : nat) : Prop :=
  fn_hint_map_depends_on_phi_in_block_except_fuel fuel
    (fn_block_shell fn_block) seen root phi_block exempt.

Lemma fn_block_record_lookup_value_deps_eq_fn_hint_map :
  forall fn_block root,
    fn_block_record_lookup_value_deps fn_block root =
    fn_hint_map_lookup_value_deps (fn_block_record_to_fn_hint_map fn_block) root.
Proof.
  intros fn_block root. reflexivity.
Qed.

Lemma fn_block_record_depends_on_phi_in_block_except_eq_fn_hint_map :
  forall fuel fn_block seen root phi_block exempt,
    fn_block_record_depends_on_phi_in_block_except_fuel fuel fn_block seen root phi_block exempt =
    fn_hint_map_depends_on_phi_in_block_except_fuel fuel
      (fn_block_record_to_fn_hint_map fn_block) seen root phi_block exempt.
Proof.
  intros fuel fn_block seen root phi_block exempt. reflexivity.
Qed.

Lemma fn_block_record_lookup_value_deps_eq_fn_record :
  forall fn_block root,
    fn_block_record_lookup_value_deps fn_block root =
    fn_record_lookup_value_deps (fn_block_record_to_fn_record fn_block) root.
Proof.
  intros fn_block root. destruct fn_block as [shell blocks]. reflexivity.
Qed.

Lemma fn_block_record_depends_on_phi_in_block_except_eq_fn_record :
  forall fuel fn_block seen root phi_block exempt,
    fn_block_record_depends_on_phi_in_block_except_fuel fuel fn_block seen root phi_block exempt =
    fn_record_depends_on_phi_in_block_except_fuel fuel
      (fn_block_record_to_fn_record fn_block) seen root phi_block exempt.
Proof.
  intros fuel fn_block seen root phi_block exempt.
  destruct fn_block as [shell blocks]. reflexivity.
Qed.

Definition example_actual_blocks : list actual_block_record_lite :=
  [ {| actual_block_id := 0%nat;
       actual_block_instrs := [IRAssign "tmp0" 0%nat SSource];
       actual_block_term := TRLGoto 1%nat |}
  ; {| actual_block_id := 1%nat;
       actual_block_instrs := [IREval 2%nat SSource];
       actual_block_term := TRLBranch 2%nat 2%nat 3%nat |}
  ; {| actual_block_id := 2%nat;
       actual_block_instrs := [];
       actual_block_term := TRLRet (Some 4%nat) |}
  ; {| actual_block_id := 3%nat;
       actual_block_instrs := [];
       actual_block_term := TRLUnreachable |}
  ].

Definition example_fn_block_record : fn_block_record_lite :=
  {| fn_block_shell := example_fn_hint_map_record;
     fn_block_blocks := example_actual_blocks |}.

Lemma example_actual_blocks_project :
  map actual_block_record_to_block_lite example_actual_blocks = fn_record_blocks example_fn_record.
Proof.
  reflexivity.
Qed.

Lemma example_fn_block_record_to_shell :
  fn_block_record_to_fn_hint_map example_fn_block_record = example_fn_hint_map_record.
Proof.
  reflexivity.
Qed.

Lemma example_fn_block_record_to_fn_record :
  fn_block_record_to_fn_record example_fn_block_record = example_fn_record.
Proof.
  reflexivity.
Qed.

Lemma example_fn_block_record_lookup_phi :
  fn_block_record_lookup_value_deps example_fn_block_record 1%nat = Some [3%nat].
Proof.
  reflexivity.
Qed.

Lemma example_fn_block_record_lookup_binary :
  fn_block_record_lookup_value_deps example_fn_block_record 6%nat = Some [6%nat; 1%nat].
Proof.
  reflexivity.
Qed.

Lemma example_fn_block_record_depends_direct_phi :
  fn_block_record_depends_on_phi_in_block_except_fuel 3%nat example_fn_block_record [] 0%nat 7%nat 99%nat.
Proof.
  exact example_fn_hint_map_depends_direct_phi.
Qed.

Lemma example_fn_block_record_depends_exempt_phi_through_arg :
  fn_block_record_depends_on_phi_in_block_except_fuel 3%nat example_fn_block_record [] 1%nat 7%nat 1%nat.
Proof.
  exact example_fn_hint_map_depends_exempt_phi_through_arg.
Qed.

Lemma example_fn_block_record_depends_other_block_ignored :
  ~ fn_block_record_depends_on_phi_in_block_except_fuel 3%nat example_fn_block_record [] 2%nat 7%nat 99%nat.
Proof.
  exact example_fn_hint_map_depends_other_block_ignored.
Qed.

End RRVerifyIrBlockRecordSubset.
