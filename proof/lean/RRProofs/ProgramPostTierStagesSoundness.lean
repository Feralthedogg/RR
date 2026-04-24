import RRProofs.OptimizerPipelineSoundness

namespace RRProofs.ProgramPostTierStagesSoundness

open RRProofs.MirInvariantBundle
open RRProofs.OptimizerPipelineSoundness

def inlineCleanupStage (fn : MirFnLite) : MirFnLite :=
  identityPass fn

def freshAliasStage (fn : MirFnLite) : MirFnLite :=
  identityPass fn

def deSsaProgramStage (fn : MirFnLite) : MirFnLite :=
  prepareForCodegenPipeline fn

def programPostTierPipeline (fn : MirFnLite) : MirFnLite :=
  deSsaProgramStage (freshAliasStage (inlineCleanupStage fn))

theorem inline_cleanup_stage_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (inlineCleanupStage fn) := by
  exact identity_pass_preserves_verify_ir_bundle h

theorem inline_cleanup_stage_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (inlineCleanupStage fn) env = execEntry fn env := by
  exact identity_pass_preserves_semantics fn env

theorem fresh_alias_stage_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (freshAliasStage fn) := by
  exact identity_pass_preserves_verify_ir_bundle h

theorem fresh_alias_stage_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (freshAliasStage fn) env = execEntry fn env := by
  exact identity_pass_preserves_semantics fn env

theorem de_ssa_program_stage_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (deSsaProgramStage fn) := by
  exact prepare_for_codegen_preserves_verify_ir h

theorem de_ssa_program_stage_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (deSsaProgramStage fn) env = execEntry fn env := by
  exact prepare_for_codegen_preserves_semantics fn env

theorem program_post_tier_pipeline_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (programPostTierPipeline fn) := by
  unfold programPostTierPipeline inlineCleanupStage freshAliasStage deSsaProgramStage
  exact prepare_for_codegen_preserves_verify_ir
    (identity_pass_preserves_verify_ir_bundle
      (identity_pass_preserves_verify_ir_bundle h))

theorem program_post_tier_pipeline_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (programPostTierPipeline fn) env = execEntry fn env := by
  unfold programPostTierPipeline inlineCleanupStage freshAliasStage deSsaProgramStage
  calc
    execEntry (prepareForCodegenPipeline (identityPass (identityPass fn))) env
        = execEntry (identityPass (identityPass fn)) env := by
            exact prepare_for_codegen_preserves_semantics _ _
    _ = execEntry fn env := by
          simp [identity_pass_preserves_semantics]

end RRProofs.ProgramPostTierStagesSoundness
