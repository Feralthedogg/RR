import RRProofs.ProgramRunProfileInnerSoundness

namespace RRProofs.ProgramApiWrapperSoundness

open RRProofs.MirInvariantBundle
open RRProofs.PhasePlanSoundness
open RRProofs.ProgramOptPlanSoundness
open RRProofs.ProgramPhasePipelineSoundness
open RRProofs.ProgramRunProfileInnerSoundness

def runProgramWithProfileAndSchedulerPipeline :=
  runProgramInnerFunctionPipeline

def runProgramWithSchedulerPipeline :=
  runProgramWithProfileAndSchedulerPipeline

def runProgramWithStatsPipeline :=
  runProgramWithSchedulerPipeline

def runProgramPipeline :=
  runProgramWithStatsPipeline

theorem run_program_with_profile_and_scheduler_preserves_verify_ir
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible
      (runProgramWithProfileAndSchedulerPipeline mode traceRequested fastDev runHeavyTier plan entries entry fn) := by
  exact run_program_inner_function_preserves_verify_ir mode traceRequested fastDev runHeavyTier plan entries entry h

theorem run_program_with_profile_and_scheduler_preserves_semantics
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry
      (runProgramWithProfileAndSchedulerPipeline mode traceRequested fastDev runHeavyTier plan entries entry fn) env
      = execEntry fn env := by
  exact run_program_inner_function_preserves_semantics mode traceRequested fastDev runHeavyTier plan entries entry fn env

theorem run_program_with_scheduler_preserves_verify_ir
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible
      (runProgramWithSchedulerPipeline mode traceRequested fastDev runHeavyTier plan entries entry fn) := by
  exact run_program_with_profile_and_scheduler_preserves_verify_ir mode traceRequested fastDev runHeavyTier plan entries entry h

theorem run_program_with_scheduler_preserves_semantics
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry
      (runProgramWithSchedulerPipeline mode traceRequested fastDev runHeavyTier plan entries entry fn) env
      = execEntry fn env := by
  exact run_program_with_profile_and_scheduler_preserves_semantics mode traceRequested fastDev runHeavyTier plan entries entry fn env

theorem run_program_with_stats_preserves_verify_ir
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible
      (runProgramWithStatsPipeline mode traceRequested fastDev runHeavyTier plan entries entry fn) := by
  exact run_program_with_scheduler_preserves_verify_ir mode traceRequested fastDev runHeavyTier plan entries entry h

theorem run_program_with_stats_preserves_semantics
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry
      (runProgramWithStatsPipeline mode traceRequested fastDev runHeavyTier plan entries entry fn) env
      = execEntry fn env := by
  exact run_program_with_scheduler_preserves_semantics mode traceRequested fastDev runHeavyTier plan entries entry fn env

theorem run_program_preserves_verify_ir
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible
      (runProgramPipeline mode traceRequested fastDev runHeavyTier plan entries entry fn) := by
  exact run_program_with_stats_preserves_verify_ir mode traceRequested fastDev runHeavyTier plan entries entry h

theorem run_program_preserves_semantics
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry
      (runProgramPipeline mode traceRequested fastDev runHeavyTier plan entries entry fn) env
      = execEntry fn env := by
  exact run_program_with_stats_preserves_semantics mode traceRequested fastDev runHeavyTier plan entries entry fn env

end RRProofs.ProgramApiWrapperSoundness
