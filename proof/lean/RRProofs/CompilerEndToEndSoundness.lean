import RRProofs.PipelineStmtSubset
import RRProofs.ProgramApiWrapperSoundness

namespace RRProofs.CompilerEndToEndSoundness

open RRProofs.MirInvariantBundle
open RRProofs.ProgramApiWrapperSoundness
open RRProofs.ProgramOptPlanSoundness
open RRProofs.ProgramPhasePipelineSoundness
open RRProofs.PhasePlanSoundness

structure ReducedCompilerArtifact where
  emittedProgram : RProgram
  optimizedFn : MirFnLite
deriving Repr

def compileArtifact
    (src : SrcProgram)
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    (fn : MirFnLite) : ReducedCompilerArtifact :=
  { emittedProgram := emitRProgram (lowerProgram src)
  , optimizedFn := runProgramPipeline mode traceRequested fastDev runHeavyTier plan entries entry fn
  }

theorem compiler_frontend_preserves_eval
    (src : SrcProgram) :
    evalRProgram (compileArtifact src .balanced false false false
      { programLimit := 0, fnLimit := 0, totalIr := 0, maxFnIr := 0, selectiveMode := false, selectedFunctions := [] }
      [] { functionId := 0, features := balancedSample, irSize := 0, score := 0, hotWeight := 0, present := true, conservative := false, selfRecursive := false }
      { entry := 0, bodyHead := 0, blocks := [], unsupportedDynamic := false, opaqueInterop := false }).emittedProgram
      = evalSrcProgram src := by
  simp [compileArtifact, lowerEmitProgram_preserves_eval]

theorem compiler_optimizer_preserves_verify_ir
    (src : SrcProgram)
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (compileArtifact src mode traceRequested fastDev runHeavyTier plan entries entry fn).optimizedFn := by
  exact run_program_preserves_verify_ir mode traceRequested fastDev runHeavyTier plan entries entry h

theorem compiler_optimizer_preserves_semantics
    (src : SrcProgram)
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (compileArtifact src mode traceRequested fastDev runHeavyTier plan entries entry fn).optimizedFn env
      = execEntry fn env := by
  exact run_program_preserves_semantics mode traceRequested fastDev runHeavyTier plan entries entry fn env

theorem compiler_end_to_end_preserves_observables
    (src : SrcProgram)
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    {fn : MirFnLite} (_h : OptimizerEligible fn)
    (env : RRProofs.MirSemanticsLite.Env) :
    evalRProgram (compileArtifact src mode traceRequested fastDev runHeavyTier plan entries entry fn).emittedProgram
      = evalSrcProgram src
      ∧ execEntry (compileArtifact src mode traceRequested fastDev runHeavyTier plan entries entry fn).optimizedFn env
        = execEntry fn env := by
  constructor
  · simp [compileArtifact, lowerEmitProgram_preserves_eval]
  · exact run_program_preserves_semantics mode traceRequested fastDev runHeavyTier plan entries entry fn env

end RRProofs.CompilerEndToEndSoundness
