From Stdlib Require Import List String.

Import ListNotations.
Open Scope string_scope.

Module RRPeepholeLineSemantics.

Definition line := string.
Definition line_stream := list line.

Definition observe (lines : line_stream) : line_stream := lines.

Inductive peephole_rewrite : line_stream -> line_stream -> Prop :=
  | PRIdentity : forall lines, peephole_rewrite lines lines
  | PRLocalPreserving :
      forall before after, after = before -> peephole_rewrite before after.

Lemma rewrite_preserves_observe :
  forall before after,
    peephole_rewrite before after ->
    observe after = observe before.
Proof.
  intros before after Hrewrite.
  inversion Hrewrite; subst; reflexivity.
Qed.

Inductive peephole_stage_lite : Type :=
  | PeepholeLinearScan
  | PeepholePrimaryFlow
  | PeepholePrimaryInline
  | PeepholePrimaryReuse
  | PeepholePrimaryLoopCleanup
  | PeepholeSecondaryInline
  | PeepholeSecondaryExact
  | PeepholeSecondaryHelperCleanup
  | PeepholeSecondaryRecordSroa
  | PeepholeSecondaryFinalizeCleanup
  | PeepholeFinalize.

Definition stage_pipeline
    (_stage : peephole_stage_lite)
    (lines : line_stream) : line_stream :=
  lines.

Fixpoint stage_sequence
    (stages : list peephole_stage_lite)
    (lines : line_stream) : line_stream :=
  match stages with
  | [] => lines
  | stage :: rest => stage_sequence rest (stage_pipeline stage lines)
  end.

Lemma stage_preserves_observe :
  forall stage lines,
    observe (stage_pipeline stage lines) = observe lines.
Proof.
  reflexivity.
Qed.

Lemma stage_sequence_preserves_observe :
  forall stages lines,
    observe (stage_sequence stages lines) = observe lines.
Proof.
  induction stages as [|stage rest IH]; intros lines; simpl.
  - reflexivity.
  - exact (IH lines).
Qed.

End RRPeepholeLineSemantics.
