//! Tachyon execution and final runtime-injection helpers for the compiler
//! pipeline.
//!
//! This module runs the optimize-or-stabilize phase, reports phase summaries,
//! and produces the self-contained output artifact that wraps emitted code with
//! the required runtime subset and configuration.
use super::*;

use crate::compiler::pipeline::compile_output_cache_salt;
use crate::error::{RRCode, RRException};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CachedOptimizedProgramArtifact {
    pub(crate) schema: String,
    pub(crate) compiler_version: String,
    pub(crate) opt_level: String,
    pub(crate) compile_mode: String,
    #[serde(default)]
    pub(crate) phase_ordering_mode: String,
    pub(crate) functions: Vec<(String, crate::mir::def::FnIR)>,
    pub(crate) pulse_stats: crate::mir::opt::TachyonPulseStats,
    pub(crate) pass_timings: crate::mir::opt::TachyonPassTimings,
    pub(crate) active_pass_groups: Vec<String>,
    pub(crate) plan_summary: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CachedOptimizedFunctionArtifact {
    pub(crate) schema: String,
    pub(crate) compiler_version: String,
    pub(crate) opt_level: String,
    pub(crate) compile_mode: String,
    pub(crate) phase_ordering_mode: String,
    pub(crate) function_name: String,
    #[serde(default)]
    pub(crate) input_hash: u64,
    #[serde(default)]
    pub(crate) dependency_hash: u64,
    #[serde(default)]
    pub(crate) global_summary_hash: u64,
    pub(crate) function: crate::mir::def::FnIR,
}

pub(crate) struct OptimizedProgramStoreRequest<'a> {
    pub(crate) optimize: bool,
    pub(crate) optimized_cache_hit: bool,
    pub(crate) cache_root: Option<&'a Path>,
    pub(crate) cache_key: &'a str,
    pub(crate) opt_level: OptLevel,
    pub(crate) compile_mode: CompileMode,
    pub(crate) phase_ordering_mode_label: &'a str,
    pub(crate) all_fns: &'a FxHashMap<String, crate::mir::def::FnIR>,
    pub(crate) input_fns: Option<&'a FxHashMap<String, crate::mir::def::FnIR>>,
    pub(crate) run_profile: &'a crate::mir::opt::TachyonRunProfile,
}

pub(crate) struct OptimizedProgramArtifactStoreRequest<'a> {
    pub(crate) cache_root: &'a Path,
    pub(crate) key: &'a str,
    pub(crate) opt_level: OptLevel,
    pub(crate) compile_mode: CompileMode,
    pub(crate) phase_ordering_mode: &'a str,
    pub(crate) all_fns: &'a FxHashMap<String, crate::mir::def::FnIR>,
    pub(crate) input_fns: Option<&'a FxHashMap<String, crate::mir::def::FnIR>>,
    pub(crate) run_profile: &'a crate::mir::opt::TachyonRunProfile,
}

struct OptimizedFunctionArtifactLookup<'a> {
    cache_root: &'a Path,
    key: &'a str,
    opt_level: OptLevel,
    compile_mode: CompileMode,
    phase_ordering_mode: &'a str,
    function_name: &'a str,
    input_hash: u64,
    dependency_hash: u64,
    global_summary_hash: u64,
}

pub(crate) struct TachyonPhaseRequest<'a> {
    pub(crate) ui: &'a CliLog,
    pub(crate) total_steps: usize,
    pub(crate) optimize: bool,
    pub(crate) opt_level: OptLevel,
    pub(crate) compile_mode: CompileMode,
    pub(crate) all_fns: &'a mut FxHashMap<String, crate::mir::def::FnIR>,
    pub(crate) scheduler: &'a CompilerScheduler,
    pub(crate) optimized_mir_cache_root: Option<&'a Path>,
}

pub(crate) fn stable_hash_bytes_local(bytes: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET_BASIS;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

pub(crate) fn semantic_env_fingerprint(
    keys: &[&'static str],
) -> Vec<(&'static str, Option<String>)> {
    keys.iter()
        .map(|key| (*key, std::env::var(key).ok()))
        .collect()
}

pub(crate) fn optimized_mir_semantic_env_fingerprint() -> Vec<(&'static str, Option<String>)> {
    const KEYS: &[&str] = &[
        "RR_DISABLE_VECTORIZE",
        "RR_INLINE_ALLOW_LOOPS",
        "RR_INLINE_LOCAL_ROUNDS",
        "RR_INLINE_MAX_BLOCKS",
        "RR_INLINE_MAX_CALLER_INSTRS",
        "RR_INLINE_MAX_CALLSITE_COST",
        "RR_INLINE_MAX_COST",
        "RR_INLINE_MAX_FN_GROWTH_PCT",
        "RR_INLINE_MAX_INSTRS",
        "RR_INLINE_MAX_KERNEL_COST",
        "RR_INLINE_MAX_TOTAL_INSTRS",
        "RR_INLINE_MAX_UNIT_GROWTH_PCT",
        "RR_INLINE_O3_ALLOW_LOOPS",
        "RR_INLINE_O3_MAX_BLOCKS",
        "RR_INLINE_O3_MAX_CALLER_INSTRS",
        "RR_INLINE_O3_MAX_CALLSITE_COST",
        "RR_INLINE_O3_MAX_COST",
        "RR_INLINE_O3_MAX_FN_GROWTH_PCT",
        "RR_INLINE_O3_MAX_INSTRS",
        "RR_INLINE_O3_MAX_KERNEL_COST",
        "RR_INLINE_O3_MAX_TOTAL_INSTRS",
        "RR_INLINE_O3_MAX_UNIT_GROWTH_PCT",
        "RR_OPT_FUEL",
        "RR_OPT_FUEL_TRACE",
        "RR_OUTLINE_ENABLE",
        "RR_OUTLINE_BRANCH_MIN_REGION_IR",
        "RR_OUTLINE_LOOP_MIN_REGION_IR",
        "RR_OUTLINE_MIN_PARENT_IR",
        "RR_OUTLINE_MIN_REGION_IR",
        "RR_OUTLINE_TRACE",
        "RR_PHASE_ORDERING",
        "RR_POLY_BACKEND",
        "RR_POLY_ENABLE",
        "RR_POLY_FISSION",
        "RR_POLY_GENERIC_FISSION",
        "RR_POLY_GENERIC_MIR",
        "RR_POLY_R_CODE_SIZE_MODEL",
        "RR_POLY_R_COST_MODEL",
        "RR_POLY_R_MAX_CODE_GROWTH",
        "RR_POLY_R_MIN_TILE_VOLUME",
        "RR_POLY_SKEW_2D",
        "RR_POLY_TILE_1D",
        "RR_POLY_TILE_2D",
        "RR_POLY_TILE_3D",
        "RR_POLY_TILE_COLS",
        "RR_POLY_TILE_DEPTH",
        "RR_POLY_TILE_ROWS",
        "RR_POLY_TILE_SIZE",
        "RR_PROFILE_USE",
        "RR_PROFILE_USE_PATH",
        "RR_UNROLL_ENABLE",
        "RR_UNROLL_MAX_FACTOR",
        "RR_UNROLL_MAX_GROWTH_IR",
        "RR_UNROLL_MAX_PARTIAL_TRIP",
        "RR_UNROLL_MAX_TRIP",
        "RR_UNROLL_PARTIAL_ENABLE",
        "RR_UNROLL_TRACE",
    ];
    let mut fingerprint = semantic_env_fingerprint(KEYS);
    fingerprint.push(("RR_HAS_ISL", option_env!("RR_HAS_ISL").map(str::to_string)));
    fingerprint.push((
        "RR_ISL_LINK_MODE",
        option_env!("RR_ISL_LINK_MODE").map(str::to_string),
    ));
    fingerprint
}

pub(crate) fn optimized_program_cache_key(
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
    opt_level: OptLevel,
    compile_mode: CompileMode,
    phase_ordering_mode: &str,
) -> crate::error::RR<String> {
    let mut functions: Vec<(String, crate::mir::def::FnIR)> = all_fns
        .iter()
        .map(|(name, fn_ir)| (name.clone(), fn_ir.clone()))
        .collect();
    functions.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
    let payload = serde_json::to_vec(&(
        opt_level.label(),
        compile_mode.as_str(),
        phase_ordering_mode,
        optimized_mir_semantic_env_fingerprint(),
        &functions,
    ))
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

pub(crate) fn optimized_program_artifact_path(cache_root: &Path, key: &str) -> PathBuf {
    cache_root.join(format!("{}.json", key))
}

pub(crate) fn optimized_function_artifact_path(cache_root: &Path, key: &str) -> PathBuf {
    cache_root.join("functions").join(format!("{}.json", key))
}

pub(crate) fn optimized_function_cache_key(
    name: &str,
    opt_level: OptLevel,
    compile_mode: CompileMode,
    phase_ordering_mode: &str,
    input_hash: u64,
    dependency_hash: u64,
    global_summary_hash: u64,
) -> crate::error::RR<String> {
    let payload = serde_json::to_vec(&(
        name,
        opt_level.label(),
        compile_mode.as_str(),
        phase_ordering_mode,
        optimized_mir_semantic_env_fingerprint(),
        input_hash,
        dependency_hash,
        global_summary_hash,
    ))
    .map_err(|e| {
        InternalCompilerError::new(
            Stage::Opt,
            format!(
                "failed to serialize optimized MIR function cache key: {}",
                e
            ),
        )
        .into_exception()
    })?;
    Ok(format!("{:016x}", stable_hash_bytes_local(&payload)))
}

fn stable_json_hash<T: Serialize>(label: &str, value: &T) -> crate::error::RR<u64> {
    let payload = serde_json::to_vec(value).map_err(|e| {
        InternalCompilerError::new(Stage::Opt, format!("failed to serialize {}: {}", label, e))
            .into_exception()
    })?;
    Ok(stable_hash_bytes_local(&payload))
}

fn optimized_function_input_hash(fn_ir: &crate::mir::def::FnIR) -> crate::error::RR<u64> {
    stable_json_hash("optimized MIR function input hash", fn_ir)
}

fn optimized_function_signature_hash(fn_ir: &crate::mir::def::FnIR) -> crate::error::RR<u64> {
    stable_json_hash(
        "optimized MIR function signature hash",
        &(
            &fn_ir.name,
            &fn_ir.params,
            &fn_ir.param_ty_hints,
            &fn_ir.param_term_hints,
            &fn_ir.ret_ty_hint,
            &fn_ir.ret_term_hint,
            fn_ir.unsupported_dynamic,
            fn_ir.opaque_interop,
        ),
    )
}

fn optimized_function_global_summary_hash(
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
) -> crate::error::RR<u64> {
    let mut entries = Vec::with_capacity(all_fns.len());
    for (name, fn_ir) in all_fns {
        let mut callees = direct_user_calls(fn_ir, all_fns);
        callees.sort();
        entries.push((
            name.clone(),
            optimized_function_signature_hash(fn_ir)?,
            callees,
        ));
    }
    entries.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
    stable_json_hash("optimized MIR global function summary", &entries)
}

fn optimized_function_dependency_hash(
    fn_ir: &crate::mir::def::FnIR,
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
) -> crate::error::RR<u64> {
    let mut dependencies = Vec::new();
    for callee in direct_user_calls(fn_ir, all_fns) {
        let Some(callee_fn) = all_fns.get(&callee) else {
            continue;
        };
        dependencies.push((
            callee,
            optimized_function_input_hash(callee_fn)?,
            optimized_function_signature_hash(callee_fn)?,
        ));
    }
    dependencies.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
    stable_json_hash(
        "optimized MIR direct function dependency hash",
        &(
            optimized_function_signature_hash(fn_ir)?,
            dependencies,
            fn_ir.requires_conservative_optimization(),
        ),
    )
}

fn direct_user_calls(
    fn_ir: &crate::mir::def::FnIR,
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
) -> Vec<String> {
    let mut calls = FxHashSet::default();
    for value in &fn_ir.values {
        if let crate::mir::def::ValueKind::Call { callee, .. } = &value.kind
            && all_fns.contains_key(callee)
        {
            calls.insert(callee.clone());
        }
    }
    calls.into_iter().collect()
}

fn optimized_function_cache_key_for_input(
    name: &str,
    input_fn: &crate::mir::def::FnIR,
    all_input_fns: &FxHashMap<String, crate::mir::def::FnIR>,
    opt_level: OptLevel,
    compile_mode: CompileMode,
    phase_ordering_mode: &str,
    global_summary_hash: u64,
) -> crate::error::RR<(String, u64, u64)> {
    let input_hash = optimized_function_input_hash(input_fn)?;
    let dependency_hash = optimized_function_dependency_hash(input_fn, all_input_fns)?;
    let key = optimized_function_cache_key(
        name,
        opt_level,
        compile_mode,
        phase_ordering_mode,
        input_hash,
        dependency_hash,
        global_summary_hash,
    )?;
    Ok((key, input_hash, dependency_hash))
}

pub(crate) fn load_optimized_program_artifact(
    cache_root: &Path,
    key: &str,
    opt_level: OptLevel,
    compile_mode: CompileMode,
    phase_ordering_mode: &str,
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
        || artifact.phase_ordering_mode != phase_ordering_mode
    {
        return Ok(None);
    }
    Ok(Some(artifact))
}

pub(crate) fn store_optimized_program_artifact(
    request: OptimizedProgramArtifactStoreRequest<'_>,
) -> crate::error::RR<()> {
    let recovery_root = request.cache_root.parent().unwrap_or(request.cache_root);
    fs::create_dir_all(request.cache_root).map_err(|e| {
        crate::compiler::incremental::attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Opt,
                format!(
                    "failed to create cache directory '{}': {}",
                    request.cache_root.display(),
                    e
                ),
            ),
            Some(recovery_root),
        )
    })?;
    let mut functions: Vec<(String, crate::mir::def::FnIR)> = request
        .all_fns
        .iter()
        .map(|(name, fn_ir)| (name.clone(), fn_ir.clone()))
        .collect();
    functions.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
    let artifact = CachedOptimizedProgramArtifact {
        schema: "rr-optimized-mir-artifact".to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        opt_level: request.opt_level.label().to_string(),
        compile_mode: request.compile_mode.as_str().to_string(),
        phase_ordering_mode: request.phase_ordering_mode.to_string(),
        functions,
        pulse_stats: request.run_profile.pulse_stats,
        pass_timings: request.run_profile.pass_timings.clone(),
        active_pass_groups: request.run_profile.active_pass_groups.clone(),
        plan_summary: request.run_profile.plan_summary.clone(),
    };
    let payload = serde_json::to_vec_pretty(&artifact).map_err(|e| {
        InternalCompilerError::new(
            Stage::Opt,
            format!("failed to serialize optimized MIR artifact: {}", e),
        )
        .into_exception()
    })?;
    fs::write(
        optimized_program_artifact_path(request.cache_root, request.key),
        payload,
    )
    .map_err(|e| {
        crate::compiler::incremental::attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Opt,
                format!(
                    "failed to write optimized MIR artifact '{}': {}",
                    request.key, e
                ),
            ),
            Some(recovery_root),
        )
    })?;
    store_optimized_function_artifacts(
        request.cache_root,
        request.opt_level,
        request.compile_mode,
        request.phase_ordering_mode,
        request.all_fns,
        request.input_fns,
    )
}

pub(crate) fn store_optimized_function_artifacts(
    cache_root: &Path,
    opt_level: OptLevel,
    compile_mode: CompileMode,
    phase_ordering_mode: &str,
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
    input_fns: Option<&FxHashMap<String, crate::mir::def::FnIR>>,
) -> crate::error::RR<()> {
    let function_root = cache_root.join("functions");
    fs::create_dir_all(&function_root).map_err(|e| {
        InternalCompilerError::new(
            Stage::Opt,
            format!(
                "failed to create optimized MIR function cache '{}': {}",
                function_root.display(),
                e
            ),
        )
        .into_exception()
    })?;

    let input_fns = input_fns.unwrap_or(all_fns);
    let global_summary_hash = optimized_function_global_summary_hash(input_fns)?;
    for (name, fn_ir) in all_fns {
        let input_fn = input_fns.get(name).unwrap_or(fn_ir);
        let (key, input_hash, dependency_hash) = optimized_function_cache_key_for_input(
            name,
            input_fn,
            input_fns,
            opt_level,
            compile_mode,
            phase_ordering_mode,
            global_summary_hash,
        )?;
        let artifact = CachedOptimizedFunctionArtifact {
            schema: "rr-optimized-mir-function-artifact".to_string(),
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            opt_level: opt_level.label().to_string(),
            compile_mode: compile_mode.as_str().to_string(),
            phase_ordering_mode: phase_ordering_mode.to_string(),
            function_name: name.clone(),
            input_hash,
            dependency_hash,
            global_summary_hash,
            function: fn_ir.clone(),
        };
        let payload = serde_json::to_vec_pretty(&artifact).map_err(|e| {
            InternalCompilerError::new(
                Stage::Opt,
                format!("failed to serialize optimized MIR function artifact: {}", e),
            )
            .into_exception()
        })?;
        fs::write(optimized_function_artifact_path(cache_root, &key), payload).map_err(|e| {
            InternalCompilerError::new(
                Stage::Opt,
                format!(
                    "failed to write optimized MIR function artifact '{}': {}",
                    key, e
                ),
            )
            .into_exception()
        })?;
    }
    Ok(())
}

pub(crate) fn build_tachyon_engine(
    opt_level: OptLevel,
    compile_mode: CompileMode,
) -> (crate::mir::opt::TachyonEngine, String) {
    let phase_ordering_default_mode =
        crate::mir::opt::TachyonEngine::phase_ordering_default_mode_for_opt_level(opt_level);
    let phase_ordering_mode =
        crate::mir::opt::TachyonEngine::phase_ordering_mode_for_opt_level(opt_level);
    let tachyon =
        crate::mir::opt::TachyonEngine::with_phase_ordering_default_mode_compile_mode_and_opt_level(
            phase_ordering_default_mode,
            compile_mode,
            opt_level,
        );
    (tachyon, phase_ordering_mode.label().to_string())
}

pub(crate) fn emit_interop_warnings(
    ui: &CliLog,
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
) {
    let mut fallback_msgs = Vec::new();
    let mut opaque_msgs = Vec::new();
    for fn_ir in all_fns.values() {
        if fn_ir.unsupported_dynamic {
            fallback_msgs.push(fallback_warning_message(fn_ir));
        }
        if fn_ir.opaque_interop {
            opaque_msgs.push(opaque_warning_message(fn_ir));
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
}

pub(crate) fn fallback_warning_message(fn_ir: &crate::mir::def::FnIR) -> String {
    if fn_ir.fallback_reasons.is_empty() {
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
    }
}

pub(crate) fn opaque_warning_message(fn_ir: &crate::mir::def::FnIR) -> String {
    if fn_ir.opaque_reasons.is_empty() {
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
    }
}

pub(crate) fn load_cached_optimized_program_if_available(
    ui: &CliLog,
    all_fns: &mut FxHashMap<String, crate::mir::def::FnIR>,
    cache_root: Option<&Path>,
    cache_key: &str,
    opt_level: OptLevel,
    compile_mode: CompileMode,
    phase_ordering_mode_label: &str,
) -> crate::error::RR<Option<crate::mir::opt::TachyonRunProfile>> {
    let Some(cache_root) = cache_root else {
        return Ok(None);
    };
    let Some(artifact) = load_optimized_program_artifact(
        cache_root,
        cache_key,
        opt_level,
        compile_mode,
        phase_ordering_mode_label,
    )?
    else {
        return Ok(None);
    };

    all_fns.clear();
    all_fns.extend(artifact.functions);
    ui.trace("tachyon-cache", "optimized MIR cache hit");
    Ok(Some(crate::mir::opt::TachyonRunProfile {
        pulse_stats: artifact.pulse_stats,
        pass_timings: artifact.pass_timings,
        active_pass_groups: artifact.active_pass_groups,
        plan_summary: artifact.plan_summary,
    }))
}

pub(crate) fn load_cached_optimized_functions_if_complete(
    ui: &CliLog,
    all_fns: &mut FxHashMap<String, crate::mir::def::FnIR>,
    cache_root: Option<&Path>,
    opt_level: OptLevel,
    compile_mode: CompileMode,
    phase_ordering_mode_label: &str,
) -> crate::error::RR<Option<crate::mir::opt::TachyonRunProfile>> {
    let Some(cache_root) = cache_root else {
        return Ok(None);
    };
    if all_fns.is_empty() {
        return Ok(None);
    }

    let input_fns = all_fns.clone();
    let global_summary_hash = optimized_function_global_summary_hash(&input_fns)?;
    let mut restored = Vec::with_capacity(input_fns.len());
    let mut names: Vec<_> = input_fns.keys().cloned().collect();
    names.sort();

    for name in names {
        let Some(input_fn) = input_fns.get(&name) else {
            return Ok(None);
        };
        let (key, input_hash, dependency_hash) = optimized_function_cache_key_for_input(
            &name,
            input_fn,
            &input_fns,
            opt_level,
            compile_mode,
            phase_ordering_mode_label,
            global_summary_hash,
        )?;
        let Some(artifact) = load_optimized_function_artifact(OptimizedFunctionArtifactLookup {
            cache_root,
            key: &key,
            opt_level,
            compile_mode,
            phase_ordering_mode: phase_ordering_mode_label,
            function_name: &name,
            input_hash,
            dependency_hash,
            global_summary_hash,
        })?
        else {
            return Ok(None);
        };
        if optimized_function_requires_missing_internal_helper(&artifact.function) {
            return Ok(None);
        }
        restored.push((name, artifact.function));
    }

    all_fns.clear();
    all_fns.extend(restored);
    ui.trace("tachyon-cache", "optimized MIR function cache complete hit");
    let mut profile = crate::mir::opt::TachyonRunProfile::default();
    profile.pulse_stats.optimized_mir_function_hits = all_fns.len();
    Ok(Some(profile))
}

fn load_optimized_function_artifact(
    lookup: OptimizedFunctionArtifactLookup<'_>,
) -> crate::error::RR<Option<CachedOptimizedFunctionArtifact>> {
    let path = optimized_function_artifact_path(lookup.cache_root, lookup.key);
    if !path.is_file() {
        return Ok(None);
    }
    let payload = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };
    let artifact: CachedOptimizedFunctionArtifact = match serde_json::from_slice(&payload) {
        Ok(artifact) => artifact,
        Err(_) => return Ok(None),
    };
    if artifact.schema != "rr-optimized-mir-function-artifact"
        || artifact.compiler_version != env!("CARGO_PKG_VERSION")
        || artifact.opt_level != lookup.opt_level.label()
        || artifact.compile_mode != lookup.compile_mode.as_str()
        || artifact.phase_ordering_mode != lookup.phase_ordering_mode
        || artifact.function_name != lookup.function_name
        || artifact.input_hash != lookup.input_hash
        || artifact.dependency_hash != lookup.dependency_hash
        || artifact.global_summary_hash != lookup.global_summary_hash
    {
        return Ok(None);
    }
    Ok(Some(artifact))
}

fn optimized_function_requires_missing_internal_helper(fn_ir: &crate::mir::def::FnIR) -> bool {
    fn_ir.values.iter().any(|value| {
        matches!(
            &value.kind,
            crate::mir::def::ValueKind::Call { callee, .. }
                if callee.starts_with("__rr_outline_")
        )
    })
}

pub(crate) fn run_tachyon_optimizer(
    tachyon: &crate::mir::opt::TachyonEngine,
    ui: &CliLog,
    scheduler: &CompilerScheduler,
    all_fns: &mut FxHashMap<String, crate::mir::def::FnIR>,
    mut elapsed: impl FnMut() -> Duration,
) -> crate::mir::opt::TachyonRunProfile {
    if ui.detailed && ui.slow_step_ms > 0 {
        run_tachyon_optimizer_with_progress(tachyon, ui, scheduler, all_fns, &mut elapsed)
    } else {
        tachyon.run_program_with_profile_and_scheduler(all_fns, scheduler)
    }
}

pub(crate) fn run_tachyon_optimizer_with_progress(
    tachyon: &crate::mir::opt::TachyonEngine,
    ui: &CliLog,
    scheduler: &CompilerScheduler,
    all_fns: &mut FxHashMap<String, crate::mir::def::FnIR>,
    elapsed: &mut impl FnMut() -> Duration,
) -> crate::mir::opt::TachyonRunProfile {
    let slow_after = Duration::from_millis(ui.slow_step_ms as u64);
    let repeat_after = Duration::from_millis(ui.slow_step_repeat_ms as u64);
    let mut last_report = Duration::ZERO;
    let mut last_marker: Option<(crate::mir::opt::TachyonProgressTier, usize)> = None;
    let mut progress_cb = |event: crate::mir::opt::TachyonProgress| {
        let elapsed_now = elapsed();
        if elapsed_now < slow_after {
            return;
        }
        if elapsed_now.saturating_sub(last_report) < repeat_after {
            return;
        }
        let marker = (event.tier, event.completed);
        if last_marker == Some(marker) {
            return;
        }
        last_marker = Some(marker);
        last_report = elapsed_now;
        ui.trace(
            "progress",
            &format!(
                "tier={} {}/{} fn={} elapsed={}",
                event.tier.label(),
                event.completed,
                event.total,
                event.function,
                format_duration(elapsed_now)
            ),
        );
    };
    tachyon.run_program_with_profile_and_progress_scheduler(all_fns, scheduler, &mut progress_cb)
}

pub(crate) fn store_optimized_program_if_needed(
    request: OptimizedProgramStoreRequest<'_>,
) -> crate::error::RR<()> {
    if !request.optimize || request.optimized_cache_hit {
        return Ok(());
    }
    let Some(cache_root) = request.cache_root else {
        return Ok(());
    };
    store_optimized_program_artifact(OptimizedProgramArtifactStoreRequest {
        cache_root,
        key: request.cache_key,
        opt_level: request.opt_level,
        compile_mode: request.compile_mode,
        phase_ordering_mode: request.phase_ordering_mode_label,
        all_fns: request.all_fns,
        input_fns: request.input_fns,
        run_profile: request.run_profile,
    })
}

pub(crate) fn dump_debug_fnir_if_requested(all_fns: &FxHashMap<String, crate::mir::def::FnIR>) {
    let Some(debug_names) = std::env::var_os("RR_DEBUG_FNIR") else {
        return;
    };
    let wanted: std::collections::HashSet<String> = debug_names
        .to_string_lossy()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if wanted.is_empty() {
        return;
    }
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

pub(crate) fn emit_tachyon_phase_summary(
    ui: &CliLog,
    optimize: bool,
    function_count: usize,
    elapsed: Duration,
    pulse_stats: &crate::mir::opt::TachyonPulseStats,
) {
    if optimize {
        if std::env::var_os("RR_VERBOSE_LOG").is_some() {
            emit_verbose_tachyon_stats(ui, pulse_stats);
        }
        ui.step_line_ok(&format!("Finished in {}", format_duration(elapsed)));
    } else {
        ui.step_line_ok(&format!(
            "Stabilized {} MIR functions in {}",
            function_count,
            format_duration(elapsed)
        ));
    }
}

pub(crate) fn emit_verbose_tachyon_stats(
    ui: &CliLog,
    pulse_stats: &crate::mir::opt::TachyonPulseStats,
) {
    ui.step_line_ok(&format!(
        "Vectorized: {} | Reduced: {} | Simplified: {} loops",
        pulse_stats.vectorized, pulse_stats.reduced, pulse_stats.simplified_loops
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
    emit_verbose_vector_stats(ui, pulse_stats);
    emit_verbose_phase_order_stats(ui, pulse_stats);
    emit_verbose_poly_stats(ui, pulse_stats);
    emit_verbose_proof_stats(ui, pulse_stats);
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
}

pub(crate) fn emit_verbose_vector_stats(
    ui: &CliLog,
    pulse_stats: &crate::mir::opt::TachyonPulseStats,
) {
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
    if pulse_stats.vector_candidate_total == 0 {
        return;
    }
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
    emit_verbose_vector_fallback_stats(ui, pulse_stats);
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
    if pulse_stats.vector_candidate_call_maps > 0 || pulse_stats.vector_applied_call_maps > 0 {
        ui.step_line_ok(&format!(
            "CallMap: cand direct {} | runtime {} | apply direct {} | runtime {}",
            pulse_stats.vector_candidate_call_map_direct,
            pulse_stats.vector_candidate_call_map_runtime,
            pulse_stats.vector_applied_call_map_direct,
            pulse_stats.vector_applied_call_map_runtime
        ));
    }
}

pub(crate) fn emit_verbose_vector_fallback_stats(
    ui: &CliLog,
    pulse_stats: &crate::mir::opt::TachyonPulseStats,
) {
    if pulse_stats.vector_legacy_poly_fallback_candidate_total == 0
        && pulse_stats.vector_legacy_poly_fallback_applied_total == 0
    {
        return;
    }
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

pub(crate) fn emit_verbose_phase_order_stats(
    ui: &CliLog,
    pulse_stats: &crate::mir::opt::TachyonPulseStats,
) {
    if pulse_stats.phase_profile_balanced_functions == 0
        && pulse_stats.phase_profile_compute_heavy_functions == 0
        && pulse_stats.phase_profile_control_flow_heavy_functions == 0
        && pulse_stats.phase_schedule_fallbacks == 0
        && pulse_stats.control_flow_structural_skip_functions == 0
    {
        return;
    }
    ui.step_line_ok(&format!(
        "PhaseOrder: balanced {} | compute {} | control {} | fallback {} | ctrl-skip {}",
        pulse_stats.phase_profile_balanced_functions,
        pulse_stats.phase_profile_compute_heavy_functions,
        pulse_stats.phase_profile_control_flow_heavy_functions,
        pulse_stats.phase_schedule_fallbacks,
        pulse_stats.control_flow_structural_skip_functions
    ));
}

pub(crate) fn emit_verbose_poly_stats(
    ui: &CliLog,
    pulse_stats: &crate::mir::opt::TachyonPulseStats,
) {
    if pulse_stats.poly_loops_seen == 0 && pulse_stats.poly_scops_detected == 0 {
        return;
    }
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

pub(crate) fn emit_verbose_proof_stats(
    ui: &CliLog,
    pulse_stats: &crate::mir::opt::TachyonPulseStats,
) {
    if pulse_stats.proof_certified == 0
        && pulse_stats.proof_applied == 0
        && pulse_stats.proof_apply_failed == 0
        && pulse_stats.proof_fallback_pattern == 0
    {
        return;
    }
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

pub(crate) fn run_tachyon_phase(
    request: TachyonPhaseRequest<'_>,
) -> crate::error::RR<TachyonPhaseMetrics> {
    let TachyonPhaseRequest {
        ui,
        total_steps,
        optimize,
        opt_level,
        compile_mode,
        all_fns,
        scheduler,
        optimized_mir_cache_root,
    } = request;
    let (tachyon, phase_ordering_mode_label) = build_tachyon_engine(opt_level, compile_mode);
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
    emit_interop_warnings(ui, all_fns);

    let mut run_profile = crate::mir::opt::TachyonRunProfile::default();
    let optimized_cache_key = if optimize {
        optimized_program_cache_key(
            all_fns,
            opt_level,
            compile_mode,
            phase_ordering_mode_label.as_str(),
        )?
    } else {
        String::new()
    };
    let input_fns = optimize.then(|| all_fns.clone());
    let mut optimized_cache_hit = false;
    if optimize
        && let Some(profile) = load_cached_optimized_program_if_available(
            ui,
            all_fns,
            optimized_mir_cache_root,
            optimized_cache_key.as_str(),
            opt_level,
            compile_mode,
            phase_ordering_mode_label.as_str(),
        )?
    {
        run_profile = profile;
        run_profile.pulse_stats.optimized_mir_function_hits = all_fns.len();
        optimized_cache_hit = true;
    } else if optimize
        && let Some(profile) = load_cached_optimized_functions_if_complete(
            ui,
            all_fns,
            optimized_mir_cache_root,
            opt_level,
            compile_mode,
            phase_ordering_mode_label.as_str(),
        )?
    {
        run_profile = profile;
        optimized_cache_hit = true;
    } else if optimize {
        run_profile =
            run_tachyon_optimizer(&tachyon, ui, scheduler, all_fns, || step_opt.elapsed());
        run_profile.pulse_stats.optimized_mir_function_misses = all_fns.len();
    } else {
        tachyon.stabilize_for_codegen_relaxed_start(all_fns);
    }
    crate::mir::semantics::validate_program(all_fns)?;
    crate::mir::semantics::validate_runtime_safety(all_fns)?;
    store_optimized_program_if_needed(OptimizedProgramStoreRequest {
        optimize,
        optimized_cache_hit,
        cache_root: optimized_mir_cache_root,
        cache_key: optimized_cache_key.as_str(),
        opt_level,
        compile_mode,
        phase_ordering_mode_label: phase_ordering_mode_label.as_str(),
        all_fns,
        input_fns: input_fns.as_ref(),
        run_profile: &run_profile,
    })?;
    maybe_write_pulse_stats_json(&run_profile.pulse_stats);
    dump_debug_fnir_if_requested(all_fns);
    let elapsed = step_opt.elapsed();
    emit_tachyon_phase_summary(
        ui,
        optimize,
        all_fns.len(),
        elapsed,
        &run_profile.pulse_stats,
    );

    Ok(TachyonPhaseMetrics {
        elapsed_ns: elapsed.as_nanos(),
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
