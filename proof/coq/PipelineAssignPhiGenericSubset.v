Require Import PipelineAssignPhiSubset.
Require Import CodegenSubset.

Import RRPipelineAssignPhiSubset.
Import RRCodegenSubset.

Module RRPipelineAssignPhiGenericSubset.

Lemma emit_r_assign_phi_preserves_eval :
  forall expr,
    eval_r_assign_phi (emit_r_assign_phi expr) = eval_mir_assign_phi_codegen expr.
Proof.
  apply RRPipelineAssignPhiSubset.emit_r_assign_phi_preserves_eval.
Qed.

End RRPipelineAssignPhiGenericSubset.
