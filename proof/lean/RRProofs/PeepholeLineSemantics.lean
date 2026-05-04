namespace RRProofs.PeepholeLineSemantics

abbrev Line := String
abbrev LineStream := List Line

def observe (lines : LineStream) : LineStream :=
  lines

inductive PeepholeRewrite : LineStream -> LineStream -> Prop where
  | identity (lines : LineStream) : PeepholeRewrite lines lines
  | localPreserving {before after : LineStream} :
      after = before -> PeepholeRewrite before after

theorem rewrite_preserves_observe
    {before after : LineStream}
    (h : PeepholeRewrite before after) :
    observe after = observe before := by
  cases h with
  | identity =>
      rfl
  | localPreserving hEq =>
      simp [observe, hEq]

inductive PeepholeStageLite where
  | linearScan
  | primaryFlow
  | primaryInline
  | primaryReuse
  | primaryLoopCleanup
  | secondaryInline
  | secondaryExact
  | secondaryHelperCleanup
  | secondaryRecordSroa
  | secondaryFinalizeCleanup
  | finalize
  deriving DecidableEq, Repr

def stagePipeline (_stage : PeepholeStageLite) (lines : LineStream) : LineStream :=
  lines

def stageSequence : List PeepholeStageLite -> LineStream -> LineStream
  | [], lines => lines
  | stage :: rest, lines => stageSequence rest (stagePipeline stage lines)

theorem stage_preserves_observe
    (stage : PeepholeStageLite)
    (lines : LineStream) :
    observe (stagePipeline stage lines) = observe lines := by
  simp [stagePipeline, observe]

theorem stage_sequence_preserves_observe
    (stages : List PeepholeStageLite)
    (lines : LineStream) :
    observe (stageSequence stages lines) = observe lines := by
  induction stages generalizing lines with
  | nil =>
      simp [stageSequence, observe]
  | cons stage rest ih =>
      simpa [stageSequence, stagePipeline] using ih lines

end RRProofs.PeepholeLineSemantics
