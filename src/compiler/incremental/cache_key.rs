use super::*;
pub(crate) fn build_artifact_key_inputs(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    options: IncrementalOptions,
    output_options: CompileOutputOptions,
) -> RR<ArtifactKeyInputs> {
    let modules = collect_module_fingerprints(entry_path, entry_input)?;
    let normalized_entry = normalize_module_path(Path::new(entry_path));
    let entry_content_hash = modules
        .iter()
        .find(|module| module.canonical_path == normalized_entry)
        .map(|module| module.content_hash)
        .unwrap_or_else(|| stable_hash_bytes(entry_input.as_bytes()));
    let import_fingerprint = compute_import_fingerprint(&modules, &normalized_entry);
    let dependency_graph = build_dependency_graph(&modules);
    Ok(ArtifactKeyInputs {
        modules,
        dependency_graph,
        entry_content_hash,
        import_fingerprint,
        opt_level,
        phase_ordering_mode: crate::mir::opt::TachyonEngine::phase_ordering_mode_for_opt_level(
            opt_level,
        )
        .label()
        .to_string(),
        type_cfg,
        parallel_cfg,
        options,
        output_options,
    })
}

pub(crate) fn compute_import_fingerprint(modules: &[ModuleFingerprint], entry_path: &Path) -> u64 {
    let mut payload = String::new();
    for module in modules {
        if module.canonical_path == entry_path {
            continue;
        }
        payload.push('|');
        payload.push_str(&module.canonical_path.to_string_lossy());
        payload.push(':');
        payload.push_str(&module.content_hash.to_string());
    }
    stable_hash_bytes(payload.as_bytes())
}

pub(crate) fn build_dependency_graph(modules: &[ModuleFingerprint]) -> IncrementalDependencyGraph {
    let mut reverse_deps: FxHashMap<PathBuf, Vec<PathBuf>> = FxHashMap::default();
    for module in modules {
        for import in &module.direct_imports {
            reverse_deps
                .entry(import.clone())
                .or_default()
                .push(module.canonical_path.clone());
        }
    }

    let mut nodes = Vec::with_capacity(modules.len());
    for module in modules {
        let mut direct_imports = module.direct_imports.clone();
        direct_imports.sort();
        let mut reverse = reverse_deps
            .remove(&module.canonical_path)
            .unwrap_or_default();
        reverse.sort();
        nodes.push(IncrementalModuleNode {
            canonical_path: module.canonical_path.clone(),
            direct_imports,
            reverse_deps: reverse,
            exported_symbol_fingerprint: module.exported_symbol_fingerprint,
            function_body_fingerprint: module.function_body_fingerprint,
        });
    }
    nodes.sort_by(|lhs, rhs| lhs.canonical_path.cmp(&rhs.canonical_path));

    let mut payload = String::new();
    for node in &nodes {
        payload.push_str(&node.canonical_path.to_string_lossy());
        payload.push('|');
        payload.push_str(&node.exported_symbol_fingerprint.to_string());
        payload.push('|');
        payload.push_str(&node.function_body_fingerprint.to_string());
        payload.push('|');
        for import in &node.direct_imports {
            payload.push_str(&import.to_string_lossy());
            payload.push(',');
        }
        payload.push('|');
        for dep in &node.reverse_deps {
            payload.push_str(&dep.to_string_lossy());
            payload.push(',');
        }
        payload.push('\n');
    }

    IncrementalDependencyGraph {
        nodes,
        fingerprint: stable_hash_bytes(payload.as_bytes()),
    }
}

pub(crate) fn normalize_module_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub(crate) fn import_regex() -> RR<&'static Regex> {
    if let Some(re) = IMPORT_RE.get() {
        return Ok(re);
    }

    let compiled = Regex::new(IMPORT_PATTERN).map_err(|e| {
        InternalCompilerError::new(
            Stage::Parse,
            format!("failed to compile import regex: {}", e),
        )
        .into_exception()
    })?;
    let _ = IMPORT_RE.set(compiled);
    IMPORT_RE.get().ok_or_else(|| {
        InternalCompilerError::new(Stage::Parse, "failed to initialize import regex")
            .into_exception()
    })
}

pub(crate) fn build_artifact_key(inputs: &ArtifactKeyInputs) -> String {
    let mut payload = String::new();
    payload.push_str(CACHE_VERSION);
    payload.push('|');
    payload.push_str(inputs.opt_level.label());
    payload.push('|');
    payload.push_str(&inputs.phase_ordering_mode);
    payload.push('|');
    payload.push_str(inputs.type_cfg.mode.as_str());
    payload.push('|');
    payload.push_str(inputs.type_cfg.native_backend.as_str());
    payload.push('|');
    payload.push_str(inputs.parallel_cfg.mode.as_str());
    payload.push('|');
    payload.push_str(inputs.parallel_cfg.backend.as_str());
    payload.push('|');
    payload.push_str(&inputs.parallel_cfg.threads.to_string());
    payload.push('|');
    payload.push_str(&inputs.parallel_cfg.min_trip.to_string());
    payload.push('|');
    payload.push_str(if inputs.options.phase2 { "p2" } else { "nop2" });
    payload.push('|');
    payload.push_str(if inputs.options.strict_verify {
        "strict"
    } else {
        "nostrict"
    });
    payload.push('|');
    payload.push_str(if inputs.output_options.inject_runtime {
        "runtime"
    } else {
        "helper-only"
    });
    payload.push('|');
    payload.push_str(if inputs.output_options.preserve_all_defs {
        "preserve-defs"
    } else {
        "strip-defs"
    });
    payload.push('|');
    payload.push_str(if inputs.output_options.strict_let {
        "strict-let"
    } else {
        "legacy-implicit-decl"
    });
    payload.push('|');
    payload.push_str(if inputs.output_options.warn_implicit_decl {
        "warn-implicit-decl"
    } else {
        "silent-implicit-decl"
    });
    payload.push('|');
    payload.push_str(inputs.output_options.compile_mode.as_str());
    payload.push('|');
    payload.push_str(&compile_output_cache_salt().to_string());
    payload.push('|');
    payload.push_str(&inputs.dependency_graph.fingerprint.to_string());
    for module in &inputs.modules {
        payload.push('|');
        payload.push_str(&module.canonical_path.to_string_lossy());
        payload.push(':');
        payload.push_str(&module.content_hash.to_string());
        payload.push(':');
        payload.push_str(&module.exported_symbol_fingerprint.to_string());
        payload.push(':');
        payload.push_str(&module.function_body_fingerprint.to_string());
    }
    let hash = stable_hash_bytes(payload.as_bytes());
    format!("{:016x}", hash)
}
