//! Tachyon execution and final runtime-injection helpers for the compiler
//! pipeline.
//!
//! This module runs the optimize-or-stabilize phase, reports phase summaries,
//! and produces the self-contained output artifact that wraps emitted code with
//! the required runtime subset and configuration.

use super::super::*;
use crate::compiler::pipeline::compile_output_cache_salt;
use crate::error::{RRCode, RRException};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CachedOptimizedProgramArtifact {
    schema: String,
    compiler_version: String,
    opt_level: String,
    compile_mode: String,
    functions: Vec<(String, crate::mir::def::FnIR)>,
    pulse_stats: crate::mir::opt::TachyonPulseStats,
    pass_timings: crate::mir::opt::TachyonPassTimings,
    active_pass_groups: Vec<String>,
    plan_summary: Vec<String>,
}

fn stable_hash_bytes_local(bytes: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET_BASIS;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn optimized_program_cache_key(
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
    opt_level: OptLevel,
    compile_mode: CompileMode,
) -> crate::error::RR<String> {
    let mut functions: Vec<(String, crate::mir::def::FnIR)> = all_fns
        .iter()
        .map(|(name, fn_ir)| (name.clone(), fn_ir.clone()))
        .collect();
    functions.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
    let payload = serde_json::to_vec(&(opt_level.label(), compile_mode.as_str(), &functions))
        .map_err(|e| {
            InternalCompilerError::new(
                Stage::Opt,
                format!("failed to serialize optimized MIR cache key payload: {}", e),
            )
            .into_exception()
        })?;
    Ok(format!(
        "{:016x}",
        stable_hash_bytes_local(payload.as_slice()) ^ compile_output_cache_salt()
    ))
}

fn optimized_program_artifact_path(cache_root: &Path, key: &str) -> PathBuf {
    cache_root.join(format!("{}.json", key))
}

fn load_optimized_program_artifact(
    cache_root: &Path,
    key: &str,
    opt_level: OptLevel,
    compile_mode: CompileMode,
) -> crate::error::RR<Option<CachedOptimizedProgramArtifact>> {
    let path = optimized_program_artifact_path(cache_root, key);
    if !path.is_file() {
        return Ok(None);
    }
    let payload = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };
    let artifact: CachedOptimizedProgramArtifact = match serde_json::from_slice(&payload) {
        Ok(artifact) => artifact,
        Err(_) => return Ok(None),
    };
    if artifact.schema != "rr-optimized-mir-artifact"
        || artifact.compiler_version != env!("CARGO_PKG_VERSION")
        || artifact.opt_level != opt_level.label()
        || artifact.compile_mode != compile_mode.as_str()
    {
        return Ok(None);
    }
    Ok(Some(artifact))
}

fn store_optimized_program_artifact(
    cache_root: &Path,
    key: &str,
    opt_level: OptLevel,
    compile_mode: CompileMode,
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
    run_profile: &crate::mir::opt::TachyonRunProfile,
) -> crate::error::RR<()> {
    let recovery_root = cache_root.parent().unwrap_or(cache_root);
    fs::create_dir_all(cache_root).map_err(|e| {
        crate::compiler::incremental::attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Opt,
                format!(
                    "failed to create cache directory '{}': {}",
                    cache_root.display(),
                    e
                ),
            ),
            Some(recovery_root),
        )
    })?;
    let mut functions: Vec<(String, crate::mir::def::FnIR)> = all_fns
        .iter()
        .map(|(name, fn_ir)| (name.clone(), fn_ir.clone()))
        .collect();
    functions.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
    let artifact = CachedOptimizedProgramArtifact {
        schema: "rr-optimized-mir-artifact".to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        opt_level: opt_level.label().to_string(),
        compile_mode: compile_mode.as_str().to_string(),
        functions,
        pulse_stats: run_profile.pulse_stats,
        pass_timings: run_profile.pass_timings.clone(),
        active_pass_groups: run_profile.active_pass_groups.clone(),
        plan_summary: run_profile.plan_summary.clone(),
    };
    let payload = serde_json::to_vec_pretty(&artifact).map_err(|e| {
        InternalCompilerError::new(
            Stage::Opt,
            format!("failed to serialize optimized MIR artifact: {}", e),
        )
        .into_exception()
    })?;
    fs::write(optimized_program_artifact_path(cache_root, key), payload).map_err(|e| {
        crate::compiler::incremental::attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Opt,
                format!("failed to write optimized MIR artifact '{}': {}", key, e),
            ),
            Some(recovery_root),
        )
    })
}

pub(crate) fn run_tachyon_phase(
    ui: &CliLog,
    total_steps: usize,
    optimize: bool,
    opt_level: OptLevel,
    compile_mode: CompileMode,
    all_fns: &mut FxHashMap<String, crate::mir::def::FnIR>,
    scheduler: &CompilerScheduler,
    optimized_mir_cache_root: Option<&Path>,
) -> crate::error::RR<TachyonPhaseMetrics> {
    let phase_ordering_default_mode =
        crate::mir::opt::TachyonEngine::phase_ordering_default_mode_for_opt_level(opt_level);
    let tachyon = crate::mir::opt::TachyonEngine::with_phase_ordering_default_mode_and_compile_mode(
        phase_ordering_default_mode,
        compile_mode,
    );
    let step_opt = ui.step_start(
        4,
        total_steps,
        if optimize {
            "Tachyon Optimization"
        } else {
            "Tachyon Stabilization"
        },
        if optimize {
            "execute aggressive passes"
        } else {
            "execute safe stabilization passes"
        },
    );
    let mut fallback_msgs = Vec::new();
    let mut opaque_msgs = Vec::new();
    for fn_ir in all_fns.values() {
        if fn_ir.unsupported_dynamic {
            let msg = if fn_ir.fallback_reasons.is_empty() {
                format!(
                    "Hybrid fallback enabled for {} (dynamic feature)",
                    fn_ir.name
                )
            } else {
                format!(
                    "Hybrid fallback enabled for {}: {}",
                    fn_ir.name,
                    fn_ir.fallback_reasons.join(", ")
                )
            };
            fallback_msgs.push(msg);
        }
        if fn_ir.opaque_interop {
            let msg = if fn_ir.opaque_reasons.is_empty() {
                format!(
                    "Opaque interop enabled for {} (package/runtime call)",
                    fn_ir.name
                )
            } else {
                format!(
                    "Opaque interop enabled for {}: {}",
                    fn_ir.name,
                    fn_ir.opaque_reasons.join(", ")
                )
            };
            opaque_msgs.push(msg);
        }
    }
    fallback_msgs.sort();
    for msg in fallback_msgs {
        ui.warn(&msg);
    }
    opaque_msgs.sort();
    for msg in opaque_msgs {
        ui.warn(&msg);
    }
    let mut run_profile = crate::mir::opt::TachyonRunProfile::default();
    let optimized_cache_key = if optimize {
        optimized_program_cache_key(all_fns, opt_level, compile_mode)?
    } else {
        String::new()
    };
    let mut optimized_cache_hit = false;
    if optimize
        && let Some(cache_root) = optimized_mir_cache_root
        && let Some(artifact) = load_optimized_program_artifact(
            cache_root,
            optimized_cache_key.as_str(),
            opt_level,
            compile_mode,
        )?
    {
        all_fns.clear();
        all_fns.extend(artifact.functions.into_iter());
        run_profile = crate::mir::opt::TachyonRunProfile {
            pulse_stats: artifact.pulse_stats,
            pass_timings: artifact.pass_timings,
            active_pass_groups: artifact.active_pass_groups,
            plan_summary: artifact.plan_summary,
        };
        optimized_cache_hit = true;
        ui.trace("tachyon-cache", "optimized MIR cache hit");
    } else if optimize {
        if ui.detailed && ui.slow_step_ms > 0 {
            let slow_after = Duration::from_millis(ui.slow_step_ms as u64);
            let repeat_after = Duration::from_millis(ui.slow_step_repeat_ms as u64);
            let mut last_report = Duration::ZERO;
            let mut last_marker: Option<(crate::mir::opt::TachyonProgressTier, usize)> = None;
            let mut progress_cb = |event: crate::mir::opt::TachyonProgress| {
                let elapsed = step_opt.elapsed();
                if elapsed < slow_after {
                    return;
                }
                if elapsed.saturating_sub(last_report) < repeat_after {
                    return;
                }
                let marker = (event.tier, event.completed);
                if last_marker == Some(marker) {
                    return;
                }
                last_marker = Some(marker);
                last_report = elapsed;
                ui.trace(
                    "progress",
                    &format!(
                        "tier={} {}/{} fn={} elapsed={}",
                        event.tier.label(),
                        event.completed,
                        event.total,
                        event.function,
                        format_duration(elapsed)
                    ),
                );
            };
            run_profile = tachyon.run_program_with_profile_and_progress_scheduler(
                all_fns,
                scheduler,
                &mut progress_cb,
            );
        } else {
            run_profile = tachyon.run_program_with_profile_and_scheduler(all_fns, scheduler);
        }
    } else {
        tachyon.stabilize_for_codegen_relaxed_start(all_fns);
    }
    crate::mir::semantics::validate_program(all_fns)?;
    crate::mir::semantics::validate_runtime_safety(all_fns)?;
    if optimize
        && !optimized_cache_hit
        && let Some(cache_root) = optimized_mir_cache_root
    {
        store_optimized_program_artifact(
            cache_root,
            optimized_cache_key.as_str(),
            opt_level,
            compile_mode,
            all_fns,
            &run_profile,
        )?;
    }
    maybe_write_pulse_stats_json(&run_profile.pulse_stats);
    if let Some(debug_names) = std::env::var_os("RR_DEBUG_FNIR") {
        let wanted: std::collections::HashSet<String> = debug_names
            .to_string_lossy()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        if !wanted.is_empty() {
            let dump_all = wanted.contains("*") || wanted.contains("all");
            let mut names: Vec<_> = all_fns
                .keys()
                .filter(|n| dump_all || wanted.contains(*n))
                .collect();
            names.sort();
            for name in names {
                if let Some(fn_ir) = all_fns.get(name) {
                    eprintln!("=== RR_DEBUG_FNIR {} ===\n{:#?}", name, fn_ir);
                }
            }
        }
    }
    let pulse_stats = &run_profile.pulse_stats;
    if optimize {
        ui.step_line_ok(&format!(
            "Vectorized: {} | Reduced: {} | Simplified: {} loops",
            pulse_stats.vectorized, pulse_stats.reduced, pulse_stats.simplified_loops
        ));
        if pulse_stats.vector_loops_seen > 0 && pulse_stats.vector_skipped > 0 {
            ui.step_line_ok(&format!(
                "VecSkip: {}/{} (no-iv {} | bound {} | cfg {} | indirect {} | store {} | no-pattern {})",
                pulse_stats.vector_skipped,
                pulse_stats.vector_loops_seen,
                pulse_stats.vector_skip_no_iv,
                pulse_stats.vector_skip_non_canonical_bound,
                pulse_stats.vector_skip_unsupported_cfg_shape,
                pulse_stats.vector_skip_indirect_index_access,
                pulse_stats.vector_skip_store_effects,
                pulse_stats.vector_skip_no_supported_pattern
            ));
        }
        if pulse_stats.vector_candidate_total > 0 {
            ui.step_line_ok(&format!(
                "VecCand: {} total | red {} | cond {} | recur {} | shifted {} | call {} | expr {} | scatter {} | cube {} | map {} | multi {}",
                pulse_stats.vector_candidate_total,
                pulse_stats.vector_candidate_reductions,
                pulse_stats.vector_candidate_conditionals,
                pulse_stats.vector_candidate_recurrences,
                pulse_stats.vector_candidate_shifted,
                pulse_stats.vector_candidate_call_maps,
                pulse_stats.vector_candidate_expr_maps,
                pulse_stats.vector_candidate_scatters,
                pulse_stats.vector_candidate_cube_slices,
                pulse_stats.vector_candidate_basic_maps,
                pulse_stats.vector_candidate_multi_exprs
            ));
            ui.step_line_ok(&format!(
                "VecApply: {} total | red {} | cond {} | recur {} | shifted {} | call {} | expr {} | scatter {} | cube {} | map {} | multi {}",
                pulse_stats.vector_applied_total,
                pulse_stats.vector_applied_reductions,
                pulse_stats.vector_applied_conditionals,
                pulse_stats.vector_applied_recurrences,
                pulse_stats.vector_applied_shifted,
                pulse_stats.vector_applied_call_maps,
                pulse_stats.vector_applied_expr_maps,
                pulse_stats.vector_applied_scatters,
                pulse_stats.vector_applied_cube_slices,
                pulse_stats.vector_applied_basic_maps,
                pulse_stats.vector_applied_multi_exprs
            ));
            if pulse_stats.vector_legacy_poly_fallback_candidate_total > 0
                || pulse_stats.vector_legacy_poly_fallback_applied_total > 0
            {
                ui.step_line_ok(&format!(
                    "VecFallback: cand {} | apply {} | red cand {} | red apply {} | map cand {} | map apply {}",
                    pulse_stats.vector_legacy_poly_fallback_candidate_total,
                    pulse_stats.vector_legacy_poly_fallback_applied_total,
                    pulse_stats.vector_legacy_poly_fallback_candidate_reductions,
                    pulse_stats.vector_legacy_poly_fallback_applied_reductions,
                    pulse_stats.vector_legacy_poly_fallback_candidate_maps,
                    pulse_stats.vector_legacy_poly_fallback_applied_maps
                ));
            }
            ui.step_line_ok(&format!(
                "VecShape: cand 2d {} | 3d {} | apply 2d {} | 3d {} | Trip tiny {} | small {} | medium {} | large {}",
                pulse_stats.vector_candidate_2d,
                pulse_stats.vector_candidate_3d,
                pulse_stats.vector_applied_2d,
                pulse_stats.vector_applied_3d,
                pulse_stats.vector_trip_tier_tiny,
                pulse_stats.vector_trip_tier_small,
                pulse_stats.vector_trip_tier_medium,
                pulse_stats.vector_trip_tier_large
            ));
            if pulse_stats.vector_candidate_call_maps > 0
                || pulse_stats.vector_applied_call_maps > 0
            {
                ui.step_line_ok(&format!(
                    "CallMap: cand direct {} | runtime {} | apply direct {} | runtime {}",
                    pulse_stats.vector_candidate_call_map_direct,
                    pulse_stats.vector_candidate_call_map_runtime,
                    pulse_stats.vector_applied_call_map_direct,
                    pulse_stats.vector_applied_call_map_runtime
                ));
            }
        }
        if pulse_stats.phase_profile_balanced_functions > 0
            || pulse_stats.phase_profile_compute_heavy_functions > 0
            || pulse_stats.phase_profile_control_flow_heavy_functions > 0
            || pulse_stats.phase_schedule_fallbacks > 0
            || pulse_stats.control_flow_structural_skip_functions > 0
        {
            ui.step_line_ok(&format!(
                "PhaseOrder: balanced {} | compute {} | control {} | fallback {} | ctrl-skip {}",
                pulse_stats.phase_profile_balanced_functions,
                pulse_stats.phase_profile_compute_heavy_functions,
                pulse_stats.phase_profile_control_flow_heavy_functions,
                pulse_stats.phase_schedule_fallbacks,
                pulse_stats.control_flow_structural_skip_functions
            ));
        }
        if pulse_stats.poly_loops_seen > 0 || pulse_stats.poly_scops_detected > 0 {
            ui.step_line_ok(&format!(
                "Poly: loops {} | scops {} | reject cfg {} | affine {} | effect {} | stmts {} | accesses {} | dep {} | sched {}/{}",
                pulse_stats.poly_loops_seen,
                pulse_stats.poly_scops_detected,
                pulse_stats.poly_rejected_cfg_shape,
                pulse_stats.poly_rejected_non_affine,
                pulse_stats.poly_rejected_effects,
                pulse_stats.poly_affine_stmt_count,
                pulse_stats.poly_access_relation_count,
                pulse_stats.poly_dependence_solved,
                pulse_stats.poly_schedule_applied,
                pulse_stats.poly_schedule_attempted,
            ));
            ui.step_line_ok(&format!(
                "PolySched: attempt id {} | xchg {} | sk2 {} | t1 {} | t2 {} | t3 {} | apply id {} | xchg {} | sk2 {} | t1 {} | t2 {} | t3 {}",
                pulse_stats.poly_schedule_attempted_identity,
                pulse_stats.poly_schedule_attempted_interchange,
                pulse_stats.poly_schedule_attempted_skew2d,
                pulse_stats.poly_schedule_attempted_tile1d,
                pulse_stats.poly_schedule_attempted_tile2d,
                pulse_stats.poly_schedule_attempted_tile3d,
                pulse_stats.poly_schedule_applied_identity,
                pulse_stats.poly_schedule_applied_interchange,
                pulse_stats.poly_schedule_applied_skew2d,
                pulse_stats.poly_schedule_applied_tile1d,
                pulse_stats.poly_schedule_applied_tile2d,
                pulse_stats.poly_schedule_applied_tile3d,
            ));
            if pulse_stats.poly_schedule_auto_fission_selected > 0
                || pulse_stats.poly_schedule_auto_fuse_selected > 0
                || pulse_stats.poly_schedule_auto_skew2d_selected > 0
                || pulse_stats.poly_schedule_backend_hint_selected > 0
            {
                ui.step_line_ok(&format!(
                    "PolyAuto: fuse {} | fission {} | skew {} | backend-hint {}",
                    pulse_stats.poly_schedule_auto_fuse_selected,
                    pulse_stats.poly_schedule_auto_fission_selected,
                    pulse_stats.poly_schedule_auto_skew2d_selected,
                    pulse_stats.poly_schedule_backend_hint_selected,
                ));
            }
        }
        if pulse_stats.proof_certified > 0
            || pulse_stats.proof_applied > 0
            || pulse_stats.proof_apply_failed > 0
            || pulse_stats.proof_fallback_pattern > 0
        {
            ui.step_line_ok(&format!(
                "Proof: cert {} | apply {} | fail {} | fallback {}",
                pulse_stats.proof_certified,
                pulse_stats.proof_applied,
                pulse_stats.proof_apply_failed,
                pulse_stats.proof_fallback_pattern
            ));
            let proof_reason_detail = crate::mir::opt::v_opt::format_proof_fallback_counts(
                &pulse_stats.proof_fallback_reason_counts,
            );
            if !proof_reason_detail.is_empty() {
                ui.step_line_ok(&format!("ProofWhy: {}", proof_reason_detail));
            }
        }
        ui.step_line_ok(&format!(
            "Passes: SCCP {} | GVN {} | LICM {} | BCE {} | TCO {} | DCE {}",
            pulse_stats.sccp_hits,
            pulse_stats.gvn_hits,
            pulse_stats.licm_hits,
            pulse_stats.bce_hits,
            pulse_stats.tco_hits,
            pulse_stats.dce_hits
        ));
        ui.step_line_ok(&format!(
            "Infra: Intrinsics {} | FreshAlloc {} | Simplify {} | Inline rounds {} | De-SSA {}",
            pulse_stats.intrinsics_hits,
            pulse_stats.fresh_alloc_hits,
            pulse_stats.simplify_hits,
            pulse_stats.inline_rounds,
            pulse_stats.de_ssa_hits
        ));
        ui.step_line_ok(&format!(
            "Budget: IR {}/{} | MaxFn {}/{} | AlwaysFns {} | OptimizedFns {} | SkippedFns {}{}",
            pulse_stats.total_program_ir,
            pulse_stats.full_opt_ir_limit,
            pulse_stats.max_function_ir,
            pulse_stats.full_opt_fn_limit,
            pulse_stats.always_tier_functions,
            pulse_stats.optimized_functions,
            pulse_stats.skipped_functions,
            if pulse_stats.selective_budget_mode {
                " | selective"
            } else {
                ""
            }
        ));
        ui.step_line_ok(&format!(
            "Finished in {}",
            format_duration(step_opt.elapsed())
        ));
    } else {
        ui.step_line_ok(&format!(
            "Stabilized {} MIR functions in {}",
            all_fns.len(),
            format_duration(step_opt.elapsed())
        ));
    }

    Ok(TachyonPhaseMetrics {
        elapsed_ns: step_opt.elapsed().as_nanos(),
        optimized_mir_cache_hit: optimized_cache_hit,
        pulse_stats: run_profile.pulse_stats,
        pass_timings: run_profile.pass_timings,
        disabled_pass_groups: compile_mode
            .disabled_pass_groups()
            .iter()
            .map(|name| (*name).to_string())
            .collect(),
        active_pass_groups: run_profile.active_pass_groups,
        plan_summary: run_profile.plan_summary,
    })
}

pub(crate) fn inject_runtime_prelude(
    entry_path: &str,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    mut final_output: String,
    top_level_calls: &[String],
) -> String {
    if !final_output.is_empty() {
        final_output.insert_str(0, "# --- RR generated code (from user RR source) ---\n");
    }
    for call in top_level_calls {
        if !final_output.ends_with('\n') {
            final_output.push('\n');
        }
        if !final_output.contains("# --- RR synthesized entrypoints (auto-generated) ---\n")
            && !top_level_calls.is_empty()
        {
            final_output.push_str("# --- RR synthesized entrypoints (auto-generated) ---\n");
        }
        final_output.push_str(&format!("{}()\n", call));
    }

    let mut with_runtime = String::new();
    let runtime_roots = runtime_roots_for_output(&final_output, true);
    with_runtime.push_str(&crate::runtime::render_runtime_subset(&runtime_roots));
    if !with_runtime.ends_with('\n') {
        with_runtime.push('\n');
    }
    append_runtime_configuration(
        &mut with_runtime,
        entry_path,
        type_cfg,
        parallel_cfg,
        true,
        &runtime_roots,
    );
    with_runtime.push_str(&final_output);
    with_runtime
}
