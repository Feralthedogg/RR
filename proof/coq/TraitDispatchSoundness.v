From Stdlib Require Import String List Bool.
Import ListNotations.
Open Scope string_scope.

Definition reduced_trait_type := string.

Record reduced_trait_impl := {
  trait_name : string;
  for_type : reduced_trait_type;
  method_name : string;
  target_name : string;
  negative : bool
}.

Definition resolves_trait_method
    (trait method : string)
    (receiver : reduced_trait_type)
    (entry : reduced_trait_impl) : bool :=
  String.eqb entry.(trait_name) trait
  && String.eqb entry.(method_name) method
  && String.eqb entry.(for_type) receiver
  && negb entry.(negative).

Fixpoint resolve_trait_target
    (impls : list reduced_trait_impl)
    (trait method : string)
    (receiver : reduced_trait_type) : option string :=
  match impls with
  | [] => None
  | entry :: tail =>
      if resolves_trait_method trait method receiver entry
      then Some entry.(target_name)
      else resolve_trait_target tail trait method receiver
  end.

Theorem trait_resolution_preserves_static_dispatch_target :
  forall entry tail trait method receiver,
    resolves_trait_method trait method receiver entry = true ->
    resolve_trait_target (entry :: tail) trait method receiver =
      Some entry.(target_name).
Proof.
  intros entry tail trait method receiver Hres.
  simpl. rewrite Hres. reflexivity.
Qed.

Theorem negative_impl_does_not_resolve :
  forall entry trait method receiver,
    entry.(negative) = true ->
    resolves_trait_method trait method receiver entry = false.
Proof.
  intros entry trait method receiver Hneg.
  unfold resolves_trait_method.
  rewrite Hneg.
  repeat rewrite andb_false_r.
  reflexivity.
Qed.

Inductive reduced_operator :=
  | OpAdd
  | OpSub
  | OpMul
  | OpDiv
  | OpMod
  | OpMatMul
  | OpNeg
  | OpIndex.

Definition reduced_operator_trait (op : reduced_operator) : string * string :=
  match op with
  | OpAdd => ("Add", "add")
  | OpSub => ("Sub", "sub")
  | OpMul => ("Mul", "mul")
  | OpDiv => ("Div", "div")
  | OpMod => ("Mod", "mod")
  | OpMatMul => ("MatMul", "matmul")
  | OpNeg => ("Neg", "neg")
  | OpIndex => ("Index", "index")
  end.

Theorem neg_operator_maps_to_neg_trait :
  reduced_operator_trait OpNeg = ("Neg", "neg").
Proof. reflexivity. Qed.

Record reduced_trait_metadata := {
  metadata_name : string;
  metadata_public : bool
}.

Fixpoint exported_trait_metadata
    (entries : list reduced_trait_metadata) : list reduced_trait_metadata :=
  match entries with
  | [] => []
  | entry :: tail =>
      if entry.(metadata_public)
      then entry :: exported_trait_metadata tail
      else exported_trait_metadata tail
  end.

Theorem exported_trait_metadata_keeps_public_head :
  forall entry tail,
    entry.(metadata_public) = true ->
    exported_trait_metadata (entry :: tail) =
      entry :: exported_trait_metadata tail.
Proof.
  intros entry tail Hpub.
  simpl. rewrite Hpub. reflexivity.
Qed.

Theorem exported_trait_metadata_drops_private_head :
  forall entry tail,
    entry.(metadata_public) = false ->
    exported_trait_metadata (entry :: tail) = exported_trait_metadata tail.
Proof.
  intros entry tail Hpriv.
  simpl. rewrite Hpriv. reflexivity.
Qed.

Record reduced_generic_instance := {
  generic_name : string;
  concrete_types : list reduced_trait_type;
  monomorphized_name : string
}.

Fixpoint string_list_beq (a b : list string) : bool :=
  match a, b with
  | [], [] => true
  | x :: xs, y :: ys => String.eqb x y && string_list_beq xs ys
  | _, _ => false
  end.

Definition matches_generic_instance
    (generic : string)
    (tys : list reduced_trait_type)
    (inst : reduced_generic_instance) : bool :=
  String.eqb inst.(generic_name) generic
  && string_list_beq inst.(concrete_types) tys.

Fixpoint resolve_monomorphized_target
    (instances : list reduced_generic_instance)
    (generic : string)
    (tys : list reduced_trait_type) : option string :=
  match instances with
  | [] => None
  | inst :: tail =>
      if matches_generic_instance generic tys inst
      then Some inst.(monomorphized_name)
      else resolve_monomorphized_target tail generic tys
  end.

Theorem monomorphization_preserves_resolved_target :
  forall inst tail generic tys,
    matches_generic_instance generic tys inst = true ->
    resolve_monomorphized_target (inst :: tail) generic tys =
      Some inst.(monomorphized_name).
Proof.
  intros inst tail generic tys Hmatch.
  simpl. rewrite Hmatch. reflexivity.
Qed.

Definition repeated_param_pair_overlaps_exact
    (left_component right_component : reduced_trait_type) : bool :=
  String.eqb left_component right_component.

Theorem repeated_param_pair_rejects_inconsistent_exact_types :
  repeated_param_pair_overlaps_exact "int" "float" = false.
Proof. reflexivity. Qed.

Theorem repeated_param_pair_accepts_consistent_exact_type :
  forall component,
    repeated_param_pair_overlaps_exact component component = true.
Proof.
  intros component.
  unfold repeated_param_pair_overlaps_exact.
  apply String.eqb_refl.
Qed.

Record reduced_assoc_projection := {
  base_type : reduced_trait_type;
  owner_trait : string;
  assoc_name : string;
  resolved_type : reduced_trait_type
}.

Definition matches_assoc_projection
    (base owner assoc : string)
    (entry : reduced_assoc_projection) : bool :=
  String.eqb entry.(base_type) base
  && String.eqb entry.(owner_trait) owner
  && String.eqb entry.(assoc_name) assoc.

Fixpoint resolve_assoc_projection
    (entries : list reduced_assoc_projection)
    (base owner assoc : string) : option reduced_trait_type :=
  match entries with
  | [] => None
  | entry :: tail =>
      if matches_assoc_projection base owner assoc entry
      then Some entry.(resolved_type)
      else resolve_assoc_projection tail base owner assoc
  end.

Theorem qualified_assoc_projection_preserves_owner_resolution :
  forall entry tail base owner assoc,
    matches_assoc_projection base owner assoc entry = true ->
    resolve_assoc_projection (entry :: tail) base owner assoc =
      Some entry.(resolved_type).
Proof.
  intros entry tail base owner assoc Hmatch.
  simpl. rewrite Hmatch. reflexivity.
Qed.

Theorem qualified_assoc_projection_ignores_sibling_owner :
  forall sibling entry tail base owner assoc,
    matches_assoc_projection base owner assoc sibling = false ->
    matches_assoc_projection base owner assoc entry = true ->
    resolve_assoc_projection (sibling :: entry :: tail) base owner assoc =
      Some entry.(resolved_type).
Proof.
  intros sibling entry tail base owner assoc Hsibling Hmatch.
  simpl. rewrite Hsibling. simpl. rewrite Hmatch. reflexivity.
Qed.
