import RRProofs.PhasePlanSoundness
import RRProofs.PhasePlanLookupSoundness
import RRProofs.PhasePlanSummarySoundness
import RRProofs.ProgramOptPlanSoundness
import RRProofs.OptimizerPipelineSoundness
import RRProofs.ProgramPhasePipelineSoundness
import RRProofs.ProgramTierExecutionSoundness
import RRProofs.ProgramPostTierStagesSoundness

namespace RRProofs.ProgramRunProfileInnerSoundness

open RRProofs.MirInvariantBundle
open RRProofs.PhasePlanSoundness
open RRProofs.PhasePlanLookupSoundness
open RRProofs.PhasePlanSummarySoundness
open RRProofs.ProgramOptPlanSoundness
open RRProofs.OptimizerPipelineSoundness
open RRProofs.ProgramPhasePipelineSoundness
open RRProofs.ProgramTierExecutionSoundness
open RRProofs.ProgramPostTierStagesSoundness

def runProgramInnerFunctionPipeline
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    (fn : MirFnLite) : MirFnLite :=
  programPostTierPipeline
    (executeProgramHeavyFunction mode traceRequested fastDev runHeavyTier plan entries entry
      (alwaysTierPipeline fn))

def runProgramInnerSummary
    (orderedFunctionIds : List Nat)
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry) : List ReducedPlanSummaryEntry :=
  programPhaseSummaryEntries orderedFunctionIds mode traceRequested fastDev runHeavyTier plan entries

theorem run_program_inner_function_preserves_verify_ir
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (runProgramInnerFunctionPipeline mode traceRequested fastDev runHeavyTier plan entries entry fn) := by
  unfold runProgramInnerFunctionPipeline
  exact program_post_tier_pipeline_preserves_verify_ir
    (execute_program_heavy_function_preserves_verify_ir mode traceRequested fastDev runHeavyTier plan entries entry
      (always_tier_preserves_verify_ir h))

theorem run_program_inner_function_preserves_semantics
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (runProgramInnerFunctionPipeline mode traceRequested fastDev runHeavyTier plan entries entry fn) env
      = execEntry fn env := by
  unfold runProgramInnerFunctionPipeline
  calc
    execEntry
        (programPostTierPipeline
          (executeProgramHeavyFunction mode traceRequested fastDev runHeavyTier plan entries entry
            (alwaysTierPipeline fn))) env
        = execEntry
            (executeProgramHeavyFunction mode traceRequested fastDev runHeavyTier plan entries entry
              (alwaysTierPipeline fn)) env := by
              exact program_post_tier_pipeline_preserves_semantics _ _
    _ = execEntry (alwaysTierPipeline fn) env := by
          exact execute_program_heavy_function_preserves_semantics mode traceRequested fastDev runHeavyTier plan entries entry _ _
    _ = execEntry fn env := by
          exact always_tier_preserves_semantics fn env

theorem run_program_inner_summary_hit_emits_singleton
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (functionId : Nat)
    (selectedPlan : ReducedFunctionPhasePlan)
    (hLookup :
      lookupCollectedPlan? functionId
        (collectProgramPhasePlans mode traceRequested fastDev runHeavyTier plan entries)
        = some selectedPlan) :
    summarizePlan selectedPlan ∈
      runProgramInnerSummary [functionId] mode traceRequested fastDev runHeavyTier plan entries := by
  unfold runProgramInnerSummary
  exact program_phase_summary_hit_emits_entry mode traceRequested fastDev runHeavyTier plan entries functionId selectedPlan hLookup

theorem run_program_inner_summary_miss_skips_singleton
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (functionId : Nat)
    (hLookup :
      lookupCollectedPlan? functionId
        (collectProgramPhasePlans mode traceRequested fastDev runHeavyTier plan entries)
        = none) :
    runProgramInnerSummary [functionId] mode traceRequested fastDev runHeavyTier plan entries = [] := by
  unfold runProgramInnerSummary
  exact program_phase_summary_miss_skips_entry mode traceRequested fastDev runHeavyTier plan entries functionId hLookup

end RRProofs.ProgramRunProfileInnerSoundness
