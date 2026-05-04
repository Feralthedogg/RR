use super::*;
pub struct IncrementalCompileRequest<'a> {
    pub entry_path: &'a str,
    pub entry_input: &'a str,
    pub opt_level: OptLevel,
    pub type_cfg: TypeConfig,
    pub parallel_cfg: ParallelConfig,
    pub compiler_parallel_cfg: CompilerParallelConfig,
    pub options: IncrementalOptions,
    pub output_options: CompileOutputOptions,
    pub session: Option<&'a mut IncrementalSession>,
    pub profile: Option<&'a mut CompileProfile>,
}

pub fn compile_with_configs_incremental(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    options: IncrementalOptions,
    session: Option<&mut IncrementalSession>,
) -> RR<IncrementalCompileOutput> {
    compile_incremental_request(IncrementalCompileRequest {
        entry_path,
        entry_input,
        opt_level,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg: CompilerParallelConfig::default(),
        options,
        output_options: CompileOutputOptions::default(),
        session,
        profile: None,
    })
}

pub fn module_tree_fingerprint(entry_path: &str, entry_input: &str) -> RR<u64> {
    crate::pkg::with_project_root_hint(entry_path, || {
        let modules = collect_module_fingerprints(entry_path, entry_input)?;
        let mut payload = String::new();
        payload.push_str(CACHE_VERSION);
        for module in modules {
            payload.push('|');
            payload.push_str(&module.canonical_path.to_string_lossy());
            payload.push(':');
            payload.push_str(&module.content_hash.to_string());
        }
        Ok(stable_hash_bytes(payload.as_bytes()))
    })
}

pub fn module_tree_snapshot(entry_path: &str, entry_input: &str) -> RR<Vec<(PathBuf, u64)>> {
    crate::pkg::with_project_root_hint(entry_path, || {
        Ok(collect_module_fingerprints(entry_path, entry_input)?
            .into_iter()
            .map(|module| (module.canonical_path, module.content_hash))
            .collect())
    })
}

pub fn compile_incremental_request(
    mut request: IncrementalCompileRequest<'_>,
) -> RR<IncrementalCompileOutput> {
    let entry_path = request.entry_path;
    let entry_input = request.entry_input;
    let opt_level = request.opt_level;
    let type_cfg = request.type_cfg;
    let parallel_cfg = request.parallel_cfg;
    let compiler_parallel_cfg = request.compiler_parallel_cfg;
    let options = request.options;
    let output_options = request.output_options;
    let session = request.session.take();
    let mut profile = request.profile.take();

    let mut resolved = options.resolve(session.is_some());
    if raw_r_debug_dump_requested() {
        // Raw debug dumps are emitted during the normal pipeline before the
        // peephole pass. Cached final artifacts cannot reproduce that side
        // output, so bypass the artifact-hit tiers while keeping phase2 emit
        // cache reuse available.
        resolved.phase1 = false;
        resolved.phase3 = false;
    }
    let inputs = build_artifact_key_inputs(
        entry_path,
        entry_input,
        opt_level,
        type_cfg,
        parallel_cfg,
        resolved,
        output_options,
    )?;
    if !resolved.enabled || (!resolved.phase1 && !resolved.phase2 && !resolved.phase3) {
        let (r_code, source_map) = compile_with_profile_request(CompileWithProfileRequest {
            entry_path,
            entry_input,
            opt_level,
            type_cfg,
            parallel_cfg,
            compiler_parallel_cfg,
            output_opts: output_options,
            profile: profile.as_deref_mut(),
        })?;
        if let Some(profile) = profile.as_deref_mut() {
            profile.incremental.enabled = false;
        }
        return Ok(IncrementalCompileOutput {
            r_code,
            source_map,
            stats: IncrementalStats::default(),
        });
    }

    let mut stats = IncrementalStats::default();
    let mut strict_expected = Vec::new();
    let cache_key = if resolved.phase1 || resolved.phase3 {
        Some(build_artifact_key(&inputs))
    } else {
        None
    };
    let cache_root = cache_root_for_entry(entry_path);

    if resolved.phase3
        && let Some(cache_key) = cache_key.as_ref()
        && let Some(s) = session.as_ref()
        && let Some(hit) = s.phase3_artifacts.get(cache_key)
    {
        stats.phase3_memory_hit = true;
        if resolved.strict_verify {
            strict_expected.push((StrictArtifactTier::Phase3Memory, hit.clone()));
        } else {
            store_latest_build_meta(&cache_root, &inputs)?;
            maybe_fill_incremental_profile(profile.as_deref_mut(), &stats, output_options);
            return Ok(IncrementalCompileOutput {
                r_code: hit.r_code.clone(),
                source_map: hit.source_map.clone(),
                stats,
            });
        }
    }

    if resolved.phase1
        && let Some(cache_key) = cache_key.as_ref()
        && let Some(hit) = load_artifact(&cache_root, cache_key)?
    {
        stats.phase1_artifact_hit = true;
        if resolved.strict_verify {
            strict_expected.push((StrictArtifactTier::Phase1Disk, hit.clone()));
        } else {
            if resolved.phase3
                && let Some(s) = session
            {
                s.phase3_artifacts.insert(cache_key.clone(), hit.clone());
            }
            store_latest_build_meta(&cache_root, &inputs)?;
            maybe_fill_incremental_profile(profile.as_deref_mut(), &stats, output_options);
            return Ok(IncrementalCompileOutput {
                r_code: hit.r_code,
                source_map: hit.source_map,
                stats,
            });
        }
    }

    stats.miss_reasons = derive_incremental_miss_reasons(&cache_root, &inputs);

    let optimized_mir_cache_root = cache_root.join("optimized-mir");
    let (r_code, source_map) = if resolved.phase2 {
        let fn_cache = DiskFnEmitCache::new(cache_root.join("function-emits"));
        let (code, map, hits, misses) =
            compile_with_configs_using_emit_cache_and_compiler_parallel(
                crate::compiler::pipeline::CompilePipelineRequest {
                    entry_path,
                    entry_input,
                    opt_level,
                    type_cfg,
                    parallel_cfg,
                    compiler_parallel_cfg,
                    cache: Some(&fn_cache),
                    optimized_mir_cache_root: Some(optimized_mir_cache_root.clone()),
                    output_opts: output_options,
                    profile: profile.as_deref_mut(),
                },
            )?;
        stats.phase2_emit_hits = hits;
        stats.phase2_emit_misses = misses;
        (code, map)
    } else {
        let (code, map, _, _) = compile_with_configs_using_emit_cache_and_compiler_parallel(
            crate::compiler::pipeline::CompilePipelineRequest {
                entry_path,
                entry_input,
                opt_level,
                type_cfg,
                parallel_cfg,
                compiler_parallel_cfg,
                cache: None,
                optimized_mir_cache_root: Some(optimized_mir_cache_root),
                output_opts: output_options,
                profile: profile.as_deref_mut(),
            },
        )?;
        (code, map)
    };
    let built = CachedArtifact { r_code, source_map };

    if resolved.phase1 {
        let cache_key = required_artifact_cache_key(cache_key.as_ref(), "storing phase1 artifact")?;
        store_artifact(&cache_root, cache_key, &built)?;
    }
    if resolved.phase3
        && let Some(s) = session
    {
        let cache_key = required_artifact_cache_key(cache_key.as_ref(), "storing phase3 artifact")?;
        s.phase3_artifacts.insert(cache_key.clone(), built.clone());
    }
    if resolved.strict_verify {
        stats.strict_verification_checked = !strict_expected.is_empty();
        for (tier, expected) in &strict_expected {
            verify_strict_artifact_match(*tier, &cache_root, expected, &built)?;
        }
        stats.strict_verification_passed = stats.strict_verification_checked;
    }
    store_latest_build_meta(&cache_root, &inputs)?;
    maybe_fill_incremental_profile(profile, &stats, output_options);
    Ok(IncrementalCompileOutput {
        r_code: built.r_code,
        source_map: built.source_map,
        stats,
    })
}
