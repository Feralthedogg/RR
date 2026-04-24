Require Import LoweringSubset.
Require Import CodegenSubset.

Import RRLoweringSubset.
Import RRCodegenSubset.

Module RRCodegenGenericSubset.

Lemma emit_r_preserves_eval_fuel :
  forall fuel expr,
    (mir_expr_depth expr < fuel)%nat ->
    eval_r_expr (emit_r expr) = eval_mir_fuel fuel expr.
Proof.
  apply RRCodegenSubset.emit_r_preserves_eval_fuel.
Qed.

Lemma emit_r_preserves_eval :
  forall expr,
    eval_r_expr (emit_r expr) = eval_mir_fuel (S (mir_expr_depth expr)) expr.
Proof.
  apply RRCodegenSubset.emit_r_preserves_eval.
Qed.

End RRCodegenGenericSubset.
