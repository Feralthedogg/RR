Require Import VerifyIrBlockRecordSubset.
Require Import VerifyIrConsumerGraphSubset.
Require Import VerifyIrFnHintMapSubset.
Require Import VerifyIrFnRecordSubset.
Require Import VerifyIrFlowLite.
Require Import VerifyIrStructLite.
Require Import VerifyIrValueFullRecordSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrBlockRecordSubset.
Import RRVerifyIrConsumerGraphSubset.
Import RRVerifyIrFnHintMapSubset.
Import RRVerifyIrFnRecordSubset.
Import RRVerifyIrFlowLite.
Import RRVerifyIrStructLite.
Import RRVerifyIrValueFullRecordSubset.

Module RRVerifyIrBlockFlowSubset.

Fixpoint filter_map_string
    (f : consumer_node_id -> option string) (xs : list consumer_node_id) : list string :=
  match xs with
  | [] => []
  | x :: rest =>
      match f x with
      | Some y => y :: filter_map_string f rest
      | None => filter_map_string f rest
      end
  end.

Definition lookup_actual_value_origin_var
    (table : actual_value_full_table_lite) (root : consumer_node_id) : option string :=
  match lookup_actual_value_full_row table root with
  | Some row => actual_full_origin_var row
  | None => None
  end.

Definition value_ids_to_vars
    (table : actual_value_full_table_lite) (ids : list consumer_node_id) : list string :=
  filter_map_string (lookup_actual_value_origin_var table) ids.

Definition instr_record_reads
    (table : actual_value_full_table_lite) (instr : instr_record_lite) : list string :=
  match instr with
  | IRAssign _ src _ => value_ids_to_vars table [src]
  | IREval val _ => value_ids_to_vars table [val]
  | IRStoreIndex1D base idx val _ => value_ids_to_vars table [base; idx; val]
  | IRStoreIndex2D base r c val _ => value_ids_to_vars table [base; r; c; val]
  | IRStoreIndex3D base i j k val _ => value_ids_to_vars table [base; i; j; k; val]
  end.

Definition instr_record_writes (instr : instr_record_lite) : list string :=
  match instr with
  | IRAssign dst _ _ => [dst]
  | IREval _ _ => []
  | IRStoreIndex1D _ _ _ _ => []
  | IRStoreIndex2D _ _ _ _ _ => []
  | IRStoreIndex3D _ _ _ _ _ _ => []
  end.

Definition terminator_record_reads
    (table : actual_value_full_table_lite) (term : terminator_record_lite) : list string :=
  match term with
  | TRLGoto _ => []
  | TRLBranch cond _ _ => value_ids_to_vars table [cond]
  | TRLRet (Some val) => value_ids_to_vars table [val]
  | TRLRet None => []
  | TRLUnreachable => []
  end.

Definition missing_vars (defined reads : list string) : list string :=
  filter (fun v => if in_dec String.string_dec v defined then false else true) reads.

Definition step_instr_flow
    (table : actual_value_full_table_lite)
    (state : list string * list string) (instr : instr_record_lite)
    : list string * list string :=
  let defined := fst state in
  let required := snd state in
  let required' := List.app required (missing_vars defined (instr_record_reads table instr)) in
  let defined' := List.app defined (instr_record_writes instr) in
  (defined', required').

Definition block_required_vars
    (table : actual_value_full_table_lite)
    (init_defined : list string) (bb : actual_block_record_lite) : list string :=
  let state := fold_left (step_instr_flow table) (actual_block_instrs bb) (init_defined, []) in
  let defined := fst state in
  let required := snd state in
  List.app required (missing_vars defined (terminator_record_reads table (actual_block_term bb))).

Definition flow_case_of_actual_block
    (table : actual_value_full_table_lite)
    (init_defined : list string) (bb : actual_block_record_lite) : flow_block_case :=
  {| flow_defined := init_defined;
     flow_required := block_required_vars table init_defined bb |}.

Fixpoint flow_cases_of_actual_blocks
    (table : actual_value_full_table_lite)
    (init_defs : list (list string))
    (blocks : list actual_block_record_lite) : list flow_block_case :=
  match init_defs, blocks with
  | defs :: defs_rest, bb :: blocks_rest =>
      flow_case_of_actual_block table defs bb ::
      flow_cases_of_actual_blocks table defs_rest blocks_rest
  | _, _ => []
  end.

Definition flow_lite_case_of_fn_block
    (base : verify_ir_struct_lite_case)
    (fn_block : fn_block_record_lite)
    (init_defs : list (list string)) : verify_ir_flow_lite_case :=
  {| flow_base := base;
     flow_blocks_case :=
       flow_cases_of_actual_blocks
         (fn_record_values (fn_block_record_to_fn_record fn_block))
         init_defs
         (fn_block_blocks fn_block) |}.

Definition example_bad_actual_block : actual_block_record_lite :=
  {| actual_block_id := 10%nat;
     actual_block_instrs := [IREval 3%nat SSource];
     actual_block_term := TRLRet (Some 4%nat) |}.

Definition example_good_actual_block : actual_block_record_lite :=
  {| actual_block_id := 11%nat;
     actual_block_instrs := [IRAssign "x" 4%nat SSource; IREval 3%nat SSource];
     actual_block_term := TRLRet (Some 3%nat) |}.

Definition example_bad_fn_block_record : fn_block_record_lite :=
  {| fn_block_shell := example_fn_hint_map_record;
     fn_block_blocks := [example_bad_actual_block] |}.

Definition example_good_fn_block_record : fn_block_record_lite :=
  {| fn_block_shell := example_fn_hint_map_record;
     fn_block_blocks := [example_good_actual_block] |}.

Definition example_bad_flow_lite_case : verify_ir_flow_lite_case :=
  flow_lite_case_of_fn_block example_flow_base example_bad_fn_block_record [["y"]].

Definition example_good_flow_lite_case : verify_ir_flow_lite_case :=
  flow_lite_case_of_fn_block example_flow_base example_good_fn_block_record [["y"]].

Lemma example_bad_actual_block_required :
  flow_case_of_actual_block example_actual_value_full_table ["y"] example_bad_actual_block =
    {| flow_defined := ["y"]; flow_required := ["x"] |}.
Proof.
  reflexivity.
Qed.

Lemma example_good_actual_block_required :
  flow_case_of_actual_block example_actual_value_full_table ["y"] example_good_actual_block =
    {| flow_defined := ["y"]; flow_required := [] |}.
Proof.
  reflexivity.
Qed.

Lemma example_bad_flow_lite_case_rejects :
  verify_ir_flow_lite example_bad_flow_lite_case = Some (EUseBeforeDef "x").
Proof.
  reflexivity.
Qed.

Lemma example_good_flow_lite_case_accepts :
  verify_ir_flow_lite example_good_flow_lite_case = None.
Proof.
  reflexivity.
Qed.

End RRVerifyIrBlockFlowSubset.
