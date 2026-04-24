Require Import PipelineLetSubset.
Require Import LoweringSubset.
Require Import CodegenSubset.

Import RRPipelineLetSubset.
Import RRLoweringSubset.
Import RRCodegenSubset.

Module RRPipelineLetGenericSubset.

Lemma lower_emit_preserves_eval :
  forall expr,
    eval_r_expr (emit_r (lower expr)) =
    eval_src_fuel (S (mir_expr_depth (lower expr))) expr.
Proof.
  apply RRPipelineLetSubset.lower_emit_preserves_eval.
Qed.

End RRPipelineLetGenericSubset.
