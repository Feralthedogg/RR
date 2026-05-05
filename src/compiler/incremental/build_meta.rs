use super::*;
pub(crate) fn latest_build_meta_path(cache_root: &Path) -> PathBuf {
    cache_root.join("latest-build.meta")
}

pub(crate) fn store_latest_build_meta(cache_root: &Path, inputs: &ArtifactKeyInputs) -> RR<()> {
    if let Some(parent) = latest_build_meta_path(cache_root).parent() {
        fs::create_dir_all(parent).map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to create incremental metadata dir '{}': {}",
                        parent.display(),
                        e
                    ),
                ),
                Some(cache_root),
            )
        })?;
    }
    let payload = format!(
        concat!(
            "entry_content_hash={}\n",
            "import_fingerprint={}\n",
            "opt_level={}\n",
            "phase_ordering_mode={}\n",
            "type_mode={}\n",
            "native_backend={}\n",
            "parallel_mode={}\n",
            "parallel_backend={}\n",
            "parallel_threads={}\n",
            "parallel_min_trip={}\n",
            "inject_runtime={}\n",
            "preserve_all_defs={}\n",
            "strict_let={}\n",
            "warn_implicit_decl={}\n",
            "compile_mode={}\n",
            "phase2={}\n",
            "strict_verify={}\n"
        ),
        inputs.entry_content_hash,
        inputs.import_fingerprint,
        inputs.opt_level.label(),
        inputs.phase_ordering_mode,
        inputs.type_cfg.mode.as_str(),
        inputs.type_cfg.native_backend.as_str(),
        inputs.parallel_cfg.mode.as_str(),
        inputs.parallel_cfg.backend.as_str(),
        inputs.parallel_cfg.threads,
        inputs.parallel_cfg.min_trip,
        inputs.output_options.inject_runtime,
        inputs.output_options.preserve_all_defs,
        inputs.output_options.strict_let,
        inputs.output_options.warn_implicit_decl,
        inputs.output_options.compile_mode.as_str(),
        inputs.options.phase2,
        inputs.options.strict_verify,
    );
    fs::write(latest_build_meta_path(cache_root), payload).map_err(|e| {
        attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to write incremental metadata '{}': {}",
                    latest_build_meta_path(cache_root).display(),
                    e
                ),
            ),
            Some(cache_root),
        )
    })
}

pub(crate) fn load_latest_build_meta(cache_root: &Path) -> Option<StoredBuildMeta> {
    let path = latest_build_meta_path(cache_root);
    let text = fs::read_to_string(path).ok()?;
    let mut meta = StoredBuildMeta::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "entry_content_hash" => meta.entry_content_hash = value.parse().ok()?,
            "import_fingerprint" => meta.import_fingerprint = value.parse().ok()?,
            "opt_level" => meta.opt_level = value.to_string(),
            "phase_ordering_mode" => meta.phase_ordering_mode = value.to_string(),
            "type_mode" => meta.type_mode = value.to_string(),
            "native_backend" => meta.native_backend = value.to_string(),
            "parallel_mode" => meta.parallel_mode = value.to_string(),
            "parallel_backend" => meta.parallel_backend = value.to_string(),
            "parallel_threads" => meta.parallel_threads = value.parse().ok()?,
            "parallel_min_trip" => meta.parallel_min_trip = value.parse().ok()?,
            "inject_runtime" => meta.inject_runtime = value.parse().ok()?,
            "preserve_all_defs" => meta.preserve_all_defs = value.parse().ok()?,
            "strict_let" => meta.strict_let = value.parse().ok()?,
            "warn_implicit_decl" => meta.warn_implicit_decl = value.parse().ok()?,
            "compile_mode" => meta.compile_mode = value.to_string(),
            "phase2" => meta.phase2 = value.parse().ok()?,
            "strict_verify" => meta.strict_verify = value.parse().ok()?,
            _ => {}
        }
    }
    Some(meta)
}

pub(crate) fn derive_incremental_miss_reasons(
    cache_root: &Path,
    inputs: &ArtifactKeyInputs,
) -> Vec<String> {
    let Some(previous) = load_latest_build_meta(cache_root) else {
        return vec!["cold_start".to_string()];
    };

    let mut reasons = Vec::new();
    if previous.entry_content_hash != inputs.entry_content_hash {
        reasons.push("entry_changed".to_string());
    }
    if previous.import_fingerprint != inputs.import_fingerprint {
        reasons.push("import_fingerprint_changed".to_string());
    }
    if previous.opt_level != inputs.opt_level.label() {
        reasons.push("opt_level_changed".to_string());
    }
    if previous.phase_ordering_mode != inputs.phase_ordering_mode {
        reasons.push("phase_ordering_changed".to_string());
    }
    if previous.type_mode != inputs.type_cfg.mode.as_str()
        || previous.native_backend != inputs.type_cfg.native_backend.as_str()
        || previous.parallel_mode != inputs.parallel_cfg.mode.as_str()
        || previous.parallel_backend != inputs.parallel_cfg.backend.as_str()
        || previous.parallel_threads != inputs.parallel_cfg.threads
        || previous.parallel_min_trip != inputs.parallel_cfg.min_trip
    {
        reasons.push("compile_config_changed".to_string());
    }
    if previous.inject_runtime != inputs.output_options.inject_runtime
        || previous.preserve_all_defs != inputs.output_options.preserve_all_defs
        || previous.strict_let != inputs.output_options.strict_let
        || previous.warn_implicit_decl != inputs.output_options.warn_implicit_decl
        || previous.compile_mode != inputs.output_options.compile_mode.as_str()
    {
        reasons.push("output_options_changed".to_string());
    }
    if previous.phase2 != inputs.options.phase2
        || previous.strict_verify != inputs.options.strict_verify
    {
        reasons.push("incremental_options_changed".to_string());
    }
    if reasons.is_empty() {
        reasons.push("artifact_unavailable".to_string());
    }
    reasons
}
