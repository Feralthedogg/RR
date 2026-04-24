From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Bool.
From Stdlib Require Import Arith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.

Module RRMirSemanticsLite.

Inductive mir_value : Type :=
| MVInt : Z -> mir_value
| MVBool : bool -> mir_value
| MVNull : mir_value
| MVArray : list mir_value -> mir_value
| MVRecord : list (string * mir_value) -> mir_value.

Definition env : Type := list (string * mir_value).

Fixpoint lookup_env (ρ : env) (name : string) : option mir_value :=
  match ρ with
  | [] => None
  | (field, value) :: rest =>
      if String.eqb field name then Some value else lookup_env rest name
  end.

Fixpoint update_env (ρ : env) (name : string) (value : mir_value) : env :=
  match ρ with
  | [] => [(name, value)]
  | (field, current) :: rest =>
      if String.eqb field name
      then (field, value) :: rest
      else (field, current) :: update_env rest name value
  end.

Fixpoint update_at {A : Type} (xs : list A) (idx : nat) (value : A)
    : option (list A) :=
  match xs, idx with
  | [], _ => None
  | _ :: rest, O => Some (value :: rest)
  | head :: rest, S idx' =>
      match update_at rest idx' value with
      | Some tail => Some (head :: tail)
      | None => None
      end
  end.

Definition write_index1d_value (base : mir_value) (idx : nat) (value : mir_value)
    : option mir_value :=
  match base with
  | MVArray items =>
      match update_at items idx value with
      | Some updated => Some (MVArray updated)
      | None => None
      end
  | _ => None
  end.

Definition write_index2d_value
    (base : mir_value) (row col : nat) (value : mir_value)
    : option mir_value :=
  match base with
  | MVArray rows =>
      match nth_error rows row with
      | Some row_value =>
          match write_index1d_value row_value col value with
          | Some updated_row =>
              match update_at rows row updated_row with
              | Some updated_rows => Some (MVArray updated_rows)
              | None => None
              end
          | None => None
          end
      | None => None
      end
  | _ => None
  end.

Definition write_index3d_value
    (base : mir_value) (i j k : nat) (value : mir_value)
    : option mir_value :=
  match base with
  | MVArray planes =>
      match nth_error planes i with
      | Some plane =>
          match write_index2d_value plane j k value with
          | Some updated_plane =>
              match update_at planes i updated_plane with
              | Some updated_planes => Some (MVArray updated_planes)
              | None => None
              end
          | None => None
          end
      | None => None
      end
  | _ => None
  end.

Inductive mir_expr : Type :=
| MEConst : mir_value -> mir_expr
| MELoad : string -> mir_expr
| MEAdd : mir_expr -> mir_expr -> mir_expr
| MEMul : mir_expr -> mir_expr -> mir_expr
| MENeg : mir_expr -> mir_expr
| MELt : mir_expr -> mir_expr -> mir_expr.

Fixpoint eval_expr (ρ : env) (e : mir_expr) : option mir_value :=
  match e with
  | MEConst v => Some v
  | MELoad name => lookup_env ρ name
  | MEAdd lhs rhs =>
      match eval_expr ρ lhs, eval_expr ρ rhs with
      | Some (MVInt l), Some (MVInt r) => Some (MVInt (l + r))
      | _, _ => None
      end
  | MEMul lhs rhs =>
      match eval_expr ρ lhs, eval_expr ρ rhs with
      | Some (MVInt l), Some (MVInt r) => Some (MVInt (l * r))
      | _, _ => None
      end
  | MENeg arg =>
      match eval_expr ρ arg with
      | Some (MVInt z) => Some (MVInt (- z))
      | _ => None
      end
  | MELt lhs rhs =>
      match eval_expr ρ lhs, eval_expr ρ rhs with
      | Some (MVInt l), Some (MVInt r) => Some (MVBool (Z.ltb l r))
      | _, _ => None
      end
  end.

Inductive mir_instr : Type :=
| MIAssign : string -> mir_expr -> mir_instr
| MIEval : mir_expr -> mir_instr
| MIStoreIndex1D : string -> mir_expr -> mir_expr -> mir_instr
| MIStoreIndex2D : string -> mir_expr -> mir_expr -> mir_expr -> mir_instr
| MIStoreIndex3D : string -> mir_expr -> mir_expr -> mir_expr -> mir_expr -> mir_instr.

Inductive mir_term : Type :=
| MTGoto : nat -> mir_term
| MTIf : mir_expr -> nat -> nat -> mir_term
| MTRet : option mir_expr -> mir_term
| MTUnreachable : mir_term.

Record mir_phi_arm : Type := {
  phi_pred : nat;
  phi_expr : mir_expr;
}.

Record mir_phi : Type := {
  phi_dst : string;
  phi_arms : list mir_phi_arm;
}.

Record mir_block : Type := {
  block_id : nat;
  block_phis : list mir_phi;
  block_instrs : list mir_instr;
  block_term : mir_term;
}.

Fixpoint find_phi_arm (pred : nat) (arms : list mir_phi_arm) : option mir_phi_arm :=
  match arms with
  | [] => None
  | arm :: rest =>
      if Nat.eqb arm.(phi_pred) pred then Some arm else find_phi_arm pred rest
  end.

Fixpoint eval_phi_assignments (pred : nat) (phis : list mir_phi) (ρ : env)
    : option (list (string * mir_value)) :=
  match phis with
  | [] => Some []
  | phi :: rest =>
      match find_phi_arm pred phi.(phi_arms), eval_phi_assignments pred rest ρ with
      | Some arm, Some tail =>
          match eval_expr ρ arm.(phi_expr) with
          | Some value => Some ((phi.(phi_dst), value) :: tail)
          | None => None
          end
      | _, _ => None
      end
  end.

Definition apply_phi_nodes (pred : nat) (phis : list mir_phi) (ρ : env) : option env :=
  match eval_phi_assignments pred phis ρ with
  | Some assigns =>
      Some (
        fold_left
          (fun acc pair =>
             match pair with
             | (dst, value) => update_env acc dst value
             end)
          assigns
          ρ
      )
  | None => None
  end.

Definition as_nat_index (value : mir_value) : option nat :=
  match value with
  | MVInt z => Some (Z.to_nat z)
  | _ => None
  end.

Definition exec_instr (ρ : env) (instr : mir_instr) : option env :=
  match instr with
  | MIAssign dst rhs =>
      match eval_expr ρ rhs with
      | Some value => Some (update_env ρ dst value)
      | None => None
      end
  | MIEval rhs =>
      match eval_expr ρ rhs with
      | Some _ => Some ρ
      | None => None
      end
  | MIStoreIndex1D base idx rhs =>
      match lookup_env ρ base, eval_expr ρ idx, eval_expr ρ rhs with
      | Some base_value, Some idx_value, Some rhs_value =>
          match as_nat_index idx_value with
          | Some idx_nat =>
              match write_index1d_value base_value idx_nat rhs_value with
              | Some updated => Some (update_env ρ base updated)
              | None => None
              end
          | None => None
          end
      | _, _, _ => None
      end
  | MIStoreIndex2D base row col rhs =>
      match lookup_env ρ base, eval_expr ρ row, eval_expr ρ col, eval_expr ρ rhs with
      | Some base_value, Some row_value, Some col_value, Some rhs_value =>
          match as_nat_index row_value, as_nat_index col_value with
          | Some row_nat, Some col_nat =>
              match write_index2d_value base_value row_nat col_nat rhs_value with
              | Some updated => Some (update_env ρ base updated)
              | None => None
              end
          | _, _ => None
          end
      | _, _, _, _ => None
      end
  | MIStoreIndex3D base i j k rhs =>
      match lookup_env ρ base, eval_expr ρ i, eval_expr ρ j, eval_expr ρ k, eval_expr ρ rhs with
      | Some base_value, Some i_value, Some j_value, Some k_value, Some rhs_value =>
          match as_nat_index i_value, as_nat_index j_value, as_nat_index k_value with
          | Some i_nat, Some j_nat, Some k_nat =>
              match write_index3d_value base_value i_nat j_nat k_nat rhs_value with
              | Some updated => Some (update_env ρ base updated)
              | None => None
              end
          | _, _, _ => None
          end
      | _, _, _, _, _ => None
      end
  end.

Fixpoint exec_instrs (ρ : env) (instrs : list mir_instr) : option env :=
  match instrs with
  | [] => Some ρ
  | instr :: rest =>
      match exec_instr ρ instr with
      | Some ρ' => exec_instrs ρ' rest
      | None => None
      end
  end.

Inductive block_exit : Type :=
| BXJump : nat -> env -> block_exit
| BXDone : option mir_value -> env -> block_exit
| BXStuck : block_exit.

Definition exec_term (ρ : env) (term : mir_term) : block_exit :=
  match term with
  | MTGoto target => BXJump target ρ
  | MTIf cond then_blk else_blk =>
      match eval_expr ρ cond with
      | Some (MVBool true) => BXJump then_blk ρ
      | Some (MVBool false) => BXJump else_blk ρ
      | _ => BXStuck
      end
  | MTRet None => BXDone None ρ
  | MTRet (Some expr) =>
      match eval_expr ρ expr with
      | Some value => BXDone (Some value) ρ
      | None => BXStuck
      end
  | MTUnreachable => BXStuck
  end.

Definition exec_block_entry (pred : nat) (blk : mir_block) (ρ : env) : block_exit :=
  match apply_phi_nodes pred blk.(block_phis) ρ with
  | Some ρ' =>
      match exec_instrs ρ' blk.(block_instrs) with
      | Some ρ'' => exec_term ρ'' blk.(block_term)
      | None => BXStuck
      end
  | None => BXStuck
  end.

Definition observable_of_exit (out : block_exit) : option (option mir_value) :=
  match out with
  | BXJump _ _ => None
  | BXDone result _ => Some result
  | BXStuck => None
  end.

Lemma apply_phi_nodes_nil :
  forall pred ρ,
    apply_phi_nodes pred [] ρ = Some ρ.
Proof.
  intros pred ρ. reflexivity.
Qed.

Lemma exec_instrs_app :
  forall ρ prefix suffix,
    exec_instrs ρ (prefix ++ suffix) =
      match exec_instrs ρ prefix with
      | Some ρ' => exec_instrs ρ' suffix
      | None => None
      end.
Proof.
  intros ρ prefix.
  revert ρ.
  induction prefix as [|instr rest IH]; intros ρ suffix; simpl.
  - reflexivity.
  - destruct (exec_instr ρ instr) as [ρ'|] eqn:Hexec; simpl.
    + apply IH.
    + reflexivity.
Qed.

End RRMirSemanticsLite.
