Require Import LoweringSubset.
Require Import LoweringIfPhiSubset.

Import RRLoweringSubset.
Import RRLoweringIfPhiSubset.

Module RRLoweringIfPhiGenericSubset.

Lemma lower_if_phi_preserves_eval_fuel :
  forall fuel expr,
    eval_mir_if_phi_fuel fuel (lower_if_phi expr) =
      eval_src_if_fuel fuel expr.
Proof.
  apply RRLoweringIfPhiSubset.lower_if_phi_preserves_eval_fuel.
Qed.

Lemma lower_if_phi_preserves_eval :
  forall expr,
    eval_mir_if_phi (lower_if_phi expr) = eval_src_if expr.
Proof.
  apply RRLoweringIfPhiSubset.lower_if_phi_preserves_eval.
Qed.

End RRLoweringIfPhiGenericSubset.
