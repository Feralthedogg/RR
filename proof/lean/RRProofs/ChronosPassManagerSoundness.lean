import RRProofs.ProgramPostTierStagesSoundness

namespace RRProofs.ChronosPassManagerSoundness

open RRProofs.MirInvariantBundle
open RRProofs.OptimizerPipelineSoundness
open RRProofs.ProgramPostTierStagesSoundness

inductive ChronosStageLite where
  | functionEntryCanonicalization
  | alwaysTier
  | phaseOrderStandard
  | phaseOrderUnroll
  | functionFinalPolish
  | programOutlining
  | programInline
  | programRecordSpecialization
  | programInlineCleanup
  | programFreshAlias
  | programPostDeSsa
  | prepareForCodegen
  deriving DecidableEq, Repr

def chronosStagePipeline (stage : ChronosStageLite) (fn : MirFnLite) : MirFnLite :=
  match stage with
  | .functionEntryCanonicalization => identityPass fn
  | .alwaysTier => alwaysTierPipeline fn
  | .phaseOrderStandard => programInnerPreDeSsaPipeline fn
  | .phaseOrderUnroll => identityPass fn
  | .functionFinalPolish => alwaysTierPipeline fn
  | .programOutlining => identityPass fn
  | .programInline => identityPass fn
  | .programRecordSpecialization => identityPass fn
  | .programInlineCleanup => inlineCleanupStage fn
  | .programFreshAlias => freshAliasStage fn
  | .programPostDeSsa => programPostDeSsaPipeline fn
  | .prepareForCodegen => prepareForCodegenPipeline fn

def chronosReducedSchedule (fn : MirFnLite) : MirFnLite :=
  prepareForCodegenPipeline
    (programPostTierPipeline
      (programInnerPreDeSsaPipeline fn))

theorem chronos_stage_preserves_verify_ir
    (stage : ChronosStageLite) {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (chronosStagePipeline stage fn) := by
  cases stage <;> simp [chronosStagePipeline]
  · exact identity_pass_preserves_verify_ir_bundle h
  · exact always_tier_preserves_verify_ir h
  · exact program_inner_pre_dessa_preserves_verify_ir h
  · exact identity_pass_preserves_verify_ir_bundle h
  · exact always_tier_preserves_verify_ir h
  · exact identity_pass_preserves_verify_ir_bundle h
  · exact identity_pass_preserves_verify_ir_bundle h
  · exact identity_pass_preserves_verify_ir_bundle h
  · exact inline_cleanup_stage_preserves_verify_ir h
  · exact fresh_alias_stage_preserves_verify_ir h
  · exact program_post_dessa_preserves_verify_ir h
  · exact prepare_for_codegen_preserves_verify_ir h

theorem chronos_stage_preserves_semantics
    (stage : ChronosStageLite) (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (chronosStagePipeline stage fn) env = execEntry fn env := by
  cases stage <;> simp [chronosStagePipeline]
  · exact identity_pass_preserves_semantics fn env
  · exact always_tier_preserves_semantics fn env
  · exact program_inner_pre_dessa_preserves_semantics fn env
  · exact identity_pass_preserves_semantics fn env
  · exact always_tier_preserves_semantics fn env
  · exact identity_pass_preserves_semantics fn env
  · exact identity_pass_preserves_semantics fn env
  · exact identity_pass_preserves_semantics fn env
  · exact inline_cleanup_stage_preserves_semantics fn env
  · exact fresh_alias_stage_preserves_semantics fn env
  · exact program_post_dessa_preserves_semantics fn env
  · exact prepare_for_codegen_preserves_semantics fn env

theorem chronos_reduced_schedule_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (chronosReducedSchedule fn) := by
  unfold chronosReducedSchedule
  exact prepare_for_codegen_preserves_verify_ir
    (program_post_tier_pipeline_preserves_verify_ir
      (program_inner_pre_dessa_preserves_verify_ir h))

theorem chronos_reduced_schedule_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (chronosReducedSchedule fn) env = execEntry fn env := by
  unfold chronosReducedSchedule
  calc
    execEntry
        (prepareForCodegenPipeline
          (programPostTierPipeline (programInnerPreDeSsaPipeline fn))) env
        = execEntry
            (programPostTierPipeline (programInnerPreDeSsaPipeline fn)) env := by
            exact prepare_for_codegen_preserves_semantics _ _
    _ = execEntry (programInnerPreDeSsaPipeline fn) env := by
          exact program_post_tier_pipeline_preserves_semantics _ _
    _ = execEntry fn env := by
          exact program_inner_pre_dessa_preserves_semantics fn env

def fuelExhaustedSkip (fn : MirFnLite) : MirFnLite :=
  identityPass fn

theorem fuel_exhausted_skip_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (fuelExhaustedSkip fn) := by
  exact identity_pass_preserves_verify_ir_bundle h

theorem fuel_exhausted_skip_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (fuelExhaustedSkip fn) env = execEntry fn env := by
  exact identity_pass_preserves_semantics fn env

end RRProofs.ChronosPassManagerSoundness
