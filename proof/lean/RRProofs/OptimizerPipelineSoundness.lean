import RRProofs.MirInvariantBundle
import RRProofs.DataflowOptSoundness
import RRProofs.CfgOptSoundness
import RRProofs.LoopOptSoundness
import RRProofs.DeSsaBoundarySoundness

namespace RRProofs.OptimizerPipelineSoundness

open RRProofs.MirInvariantBundle
open RRProofs.DataflowOptSoundness
open RRProofs.CfgOptSoundness
open RRProofs.LoopOptSoundness
open RRProofs.DeSsaBoundarySoundness

def alwaysTierCfgStage (fn : MirFnLite) : MirFnLite :=
  identityPass fn

def alwaysTierDataflowStage (fn : MirFnLite) : MirFnLite :=
  identityPass fn

def alwaysTierLoopStage (fn : MirFnLite) : MirFnLite :=
  identityPass fn

def alwaysTierCleanupStage (fn : MirFnLite) : MirFnLite :=
  identityPass fn

def alwaysTierPipeline (fn : MirFnLite) : MirFnLite :=
  alwaysTierCleanupStage
    (alwaysTierLoopStage
      (alwaysTierDataflowStage
        (alwaysTierCfgStage fn)))

def programInnerPreDeSsaPipeline (fn : MirFnLite) : MirFnLite :=
  alwaysTierPipeline fn

def postDeSsaBoundaryStage (fn : MirFnLite) : MirFnLite :=
  identityPass fn

def postDeSsaCleanupStage (fn : MirFnLite) : MirFnLite :=
  identityPass fn

def programPostDeSsaPipeline (fn : MirFnLite) : MirFnLite :=
  postDeSsaCleanupStage (postDeSsaBoundaryStage (programInnerPreDeSsaPipeline fn))

def prepareForCodegenPipeline (fn : MirFnLite) : MirFnLite :=
  programPostDeSsaPipeline fn

def optimizerPipeline (fn : MirFnLite) : MirFnLite :=
  prepareForCodegenPipeline fn

theorem alwaysTierPipeline_eq_identity (fn : MirFnLite) :
    alwaysTierPipeline fn = identityPass fn := by
  rfl

theorem programInnerPreDeSsaPipeline_eq_identity (fn : MirFnLite) :
    programInnerPreDeSsaPipeline fn = identityPass fn := by
  rfl

theorem optimizerPipeline_eq_identity (fn : MirFnLite) :
    optimizerPipeline fn = identityPass fn := by
  rfl

theorem always_tier_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (alwaysTierPipeline fn) := by
  unfold alwaysTierPipeline alwaysTierCfgStage alwaysTierDataflowStage
    alwaysTierLoopStage alwaysTierCleanupStage
  exact loop_opt_identity_preserves_verify_ir_bundle
    (identity_dataflow_layer_preserves_verify_ir_bundle
      (identity_pass_preserves_verify_ir_bundle
        (identity_pass_preserves_verify_ir_bundle h)))

theorem always_tier_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (alwaysTierPipeline fn) env = execEntry fn env := by
  unfold alwaysTierPipeline alwaysTierCfgStage alwaysTierDataflowStage
    alwaysTierLoopStage alwaysTierCleanupStage
  simp [identity_pass_preserves_semantics]

theorem always_tier_cfg_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (alwaysTierCfgStage fn) := by
  exact identity_pass_preserves_verify_ir_bundle h

theorem always_tier_cfg_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (alwaysTierCfgStage fn) env = execEntry fn env := by
  exact identity_pass_preserves_semantics fn env

theorem always_tier_dataflow_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (alwaysTierDataflowStage fn) := by
  exact identity_dataflow_layer_preserves_verify_ir_bundle h

theorem always_tier_dataflow_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (alwaysTierDataflowStage fn) env = execEntry fn env := by
  exact identity_pass_preserves_semantics fn env

theorem always_tier_loop_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (alwaysTierLoopStage fn) := by
  exact loop_opt_identity_preserves_verify_ir_bundle h

theorem always_tier_loop_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (alwaysTierLoopStage fn) env = execEntry fn env := by
  exact identity_pass_preserves_semantics fn env

theorem always_tier_cleanup_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (alwaysTierCleanupStage fn) := by
  exact identity_pass_preserves_verify_ir_bundle h

theorem always_tier_cleanup_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (alwaysTierCleanupStage fn) env = execEntry fn env := by
  exact identity_pass_preserves_semantics fn env

theorem program_inner_pre_dessa_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (programInnerPreDeSsaPipeline fn) := by
  unfold programInnerPreDeSsaPipeline
  exact always_tier_preserves_verify_ir h

theorem program_inner_pre_dessa_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (programInnerPreDeSsaPipeline fn) env = execEntry fn env := by
  unfold programInnerPreDeSsaPipeline
  exact always_tier_preserves_semantics fn env

theorem program_post_dessa_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (programPostDeSsaPipeline fn) := by
  unfold programPostDeSsaPipeline postDeSsaBoundaryStage postDeSsaCleanupStage
  exact de_ssa_boundary_identity_preserves_verify_ir_bundle
    (de_ssa_boundary_identity_preserves_verify_ir_bundle
      (program_inner_pre_dessa_preserves_verify_ir h))

theorem program_post_dessa_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (programPostDeSsaPipeline fn) env = execEntry fn env := by
  unfold programPostDeSsaPipeline postDeSsaBoundaryStage postDeSsaCleanupStage
  calc
    execEntry (identityPass (identityPass (programInnerPreDeSsaPipeline fn))) env
        = execEntry (programInnerPreDeSsaPipeline fn) env := by
            simp [identity_pass_preserves_semantics]
    _ = execEntry fn env := program_inner_pre_dessa_preserves_semantics fn env

theorem post_dessa_boundary_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (postDeSsaBoundaryStage fn) := by
  exact de_ssa_boundary_identity_preserves_verify_ir_bundle h

theorem post_dessa_boundary_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (postDeSsaBoundaryStage fn) env = execEntry fn env := by
  exact de_ssa_boundary_identity_preserves_semantics fn env

theorem post_dessa_cleanup_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (postDeSsaCleanupStage fn) := by
  exact identity_pass_preserves_verify_ir_bundle h

theorem post_dessa_cleanup_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (postDeSsaCleanupStage fn) env = execEntry fn env := by
  exact identity_pass_preserves_semantics fn env

theorem prepare_for_codegen_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (prepareForCodegenPipeline fn) := by
  unfold prepareForCodegenPipeline
  exact program_post_dessa_preserves_verify_ir h

theorem prepare_for_codegen_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (prepareForCodegenPipeline fn) env = execEntry fn env := by
  unfold prepareForCodegenPipeline
  exact program_post_dessa_preserves_semantics fn env

theorem optimizer_pipeline_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (optimizerPipeline fn) := by
  unfold optimizerPipeline
  exact prepare_for_codegen_preserves_verify_ir h

theorem optimizer_pipeline_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (optimizerPipeline fn) env = execEntry fn env := by
  unfold optimizerPipeline
  exact prepare_for_codegen_preserves_semantics fn env

end RRProofs.OptimizerPipelineSoundness
