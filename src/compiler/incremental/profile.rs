use super::*;
pub(crate) fn raw_r_debug_dump_requested() -> bool {
    env::var_os("RR_DEBUG_RAW_R_PATH").is_some()
}

pub(crate) fn maybe_fill_incremental_profile(
    profile: Option<&mut CompileProfile>,
    stats: &IncrementalStats,
    output_options: CompileOutputOptions,
) {
    let Some(profile) = profile else {
        return;
    };
    profile.compile_mode = output_options.compile_mode.as_str().to_string();
    profile.incremental.enabled = true;
    profile.incremental.phase1_artifact_hit = stats.phase1_artifact_hit;
    profile.incremental.phase2_emit_hits = stats.phase2_emit_hits;
    profile.incremental.phase2_emit_misses = stats.phase2_emit_misses;
    profile.incremental.phase3_memory_hit = stats.phase3_memory_hit;
    profile.incremental.strict_verification_checked = stats.strict_verification_checked;
    profile.incremental.strict_verification_passed = stats.strict_verification_passed;
    profile.incremental.miss_reasons = stats.miss_reasons.clone();
}
