From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Bool.
From Stdlib Require Import Arith.

Open Scope string_scope.

Module RRVectorizeOriginMemoSubset.

Definition tiny_value_id := nat.

Inductive tiny_kind : Type :=
| TKLoad : string -> tiny_kind
| TKOther : tiny_kind.

Record tiny_node : Type := {
  tn_id : tiny_value_id;
  tn_origin_var : option string;
  tn_kind : tiny_kind;
}.

Definition boundary_rewrite (target : string) (replacement : tiny_value_id)
    (node : tiny_node) : tiny_value_id :=
  match tn_origin_var node with
  | Some origin =>
      if String.eqb origin target then
        match tn_kind node with
        | TKLoad v => if String.eqb v target then tn_id node else replacement
        | TKOther => replacement
        end
      else tn_id node
  | None => tn_id node
  end.

Lemma boundary_rewrite_exact_load_keeps_root :
  forall target replacement root,
    boundary_rewrite target replacement
      {| tn_id := root; tn_origin_var := Some target; tn_kind := TKLoad target |} = root.
Proof.
  intros. unfold boundary_rewrite. simpl. repeat rewrite String.eqb_refl. reflexivity.
Qed.

Lemma boundary_rewrite_origin_nonload_uses_replacement :
  forall target replacement root,
    boundary_rewrite target replacement
      {| tn_id := root; tn_origin_var := Some target; tn_kind := TKOther |} = replacement.
Proof.
  intros. unfold boundary_rewrite. simpl. rewrite String.eqb_refl. reflexivity.
Qed.

Lemma boundary_rewrite_origin_mismatched_load_uses_replacement :
  forall target other replacement root,
    other <> target ->
    boundary_rewrite target replacement
      {| tn_id := root; tn_origin_var := Some target; tn_kind := TKLoad other |} = replacement.
Proof.
  intros target other replacement root Hneq.
  unfold boundary_rewrite. simpl.
  rewrite String.eqb_refl.
  apply String.eqb_neq in Hneq.
  now rewrite Hneq.
Qed.

Lemma boundary_rewrite_unrelated_keeps_root :
  forall target other replacement root,
    other <> target ->
    boundary_rewrite target replacement
      {| tn_id := root; tn_origin_var := Some other; tn_kind := TKOther |} = root.
Proof.
  intros target other replacement root Hneq.
  unfold boundary_rewrite. simpl.
  apply String.eqb_neq in Hneq.
  now rewrite Hneq.
Qed.

Definition tiny_memo := tiny_value_id -> option tiny_value_id.

Definition memoized_result (memo : tiny_memo) (root computed : tiny_value_id) : tiny_value_id :=
  match memo root with
  | Some mapped => mapped
  | None => computed
  end.

Definition record_memo (memo : tiny_memo) (root mapped : tiny_value_id) : tiny_memo :=
  fun q => if Nat.eqb q root then Some mapped else memo q.

Lemma memoized_result_hit_reuses :
  forall memo root computed mapped,
    memo root = Some mapped ->
    memoized_result memo root computed = mapped.
Proof.
  intros. unfold memoized_result. now rewrite H.
Qed.

Lemma memoized_result_miss_uses_computed :
  forall memo root computed,
    memo root = None ->
    memoized_result memo root computed = computed.
Proof.
  intros. unfold memoized_result. now rewrite H.
Qed.

Lemma record_memo_reuses_recorded_root :
  forall memo root mapped computed,
    memoized_result (record_memo memo root mapped) root computed = mapped.
Proof.
  intros. unfold memoized_result, record_memo. now rewrite Nat.eqb_refl.
Qed.

Definition allocate_rewrite_id (next root : tiny_value_id) (changed : bool) : tiny_value_id :=
  if changed then next else root.

Lemma allocate_rewrite_id_unchanged_reuses_root :
  forall next root,
    allocate_rewrite_id next root false = root.
Proof.
  reflexivity.
Qed.

Lemma allocate_rewrite_id_changed_uses_fresh :
  forall next root,
    allocate_rewrite_id next root true = next.
Proof.
  reflexivity.
Qed.

End RRVectorizeOriginMemoSubset.
