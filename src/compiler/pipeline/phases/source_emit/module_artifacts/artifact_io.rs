use super::*;
pub(crate) fn build_cached_module_artifact(
    module_path: &Path,
    module: &crate::hir::def::HirModule,
    lowerer: &crate::hir::lower::Lowerer,
    source_metadata: crate::syntax::ast::Program,
) -> crate::error::RR<CachedModuleArtifact> {
    let (source_len, source_mtime_ns) = source_file_signature(module_path).map_err(|e| {
        RRException::new(
            "RR.ParseError",
            RRCode::E0001,
            Stage::Parse,
            format!(
                "failed to read module metadata '{}': {}",
                module_path.display(),
                e
            ),
        )
    })?;
    let symbol_entries = lowerer.symbols_snapshot();
    let mut symbol_map = FxHashMap::default();
    for (id, name) in &symbol_entries {
        symbol_map.insert(*id, name.clone());
    }
    let public_symbols = collect_public_symbols_from_module(module, &symbol_map);
    let public_function_arities = collect_public_function_arities(module, &symbol_map);
    let emit_roots = collect_emit_roots(module, &symbol_map);
    let module_fingerprint = serde_json::to_vec(module)
        .map(|bytes| stable_hash_bytes(&bytes))
        .unwrap_or(0);
    Ok(CachedModuleArtifact {
        schema: "rr-module-artifact".to_string(),
        schema_version: 2,
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        canonical_path: module_path.to_string_lossy().to_string(),
        source_len,
        source_mtime_ns,
        public_symbols,
        public_function_arities,
        emit_roots,
        module_fingerprint,
        symbols: symbol_entries
            .into_iter()
            .map(|(id, name)| (id.0, name))
            .collect(),
        source_metadata: Some(source_metadata),
        module: module.clone(),
    })
}

pub(crate) fn store_module_artifact(
    cache_root: &Path,
    module_path: &Path,
    module: &crate::hir::def::HirModule,
    lowerer: &crate::hir::lower::Lowerer,
    source_metadata: crate::syntax::ast::Program,
) -> crate::error::RR<()> {
    fs::create_dir_all(cache_root).map_err(|e| {
        RRException::new(
            "RR.CompilerError",
            RRCode::ICE9001,
            Stage::Codegen,
            format!(
                "failed to create module artifact cache dir '{}': {}",
                cache_root.display(),
                e
            ),
        )
    })?;
    let artifact = build_cached_module_artifact(module_path, module, lowerer, source_metadata)?;
    let payload = serde_json::to_vec_pretty(&artifact).map_err(|e| {
        InternalCompilerError::new(
            Stage::Codegen,
            format!(
                "failed to serialize module artifact '{}': {}",
                module_path.display(),
                e
            ),
        )
        .into_exception()
    })?;
    fs::write(module_artifact_path(cache_root, module_path), payload).map_err(|e| {
        RRException::new(
            "RR.CompilerError",
            RRCode::ICE9001,
            Stage::Codegen,
            format!(
                "failed to write module artifact '{}': {}",
                module_path.display(),
                e
            ),
        )
    })?;
    Ok(())
}

pub(crate) fn load_module_artifact(
    cache_root: &Path,
    module_path: &Path,
    mod_id: crate::hir::def::ModuleId,
    lowerer: &mut crate::hir::lower::Lowerer,
) -> crate::error::RR<Option<crate::hir::def::HirModule>> {
    let artifact_path = module_artifact_path(cache_root, module_path);
    if !artifact_path.is_file() {
        return Ok(None);
    }
    let payload = match fs::read(&artifact_path) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };
    let artifact: CachedModuleArtifact = match serde_json::from_slice(&payload) {
        Ok(artifact) => artifact,
        Err(_) => return Ok(None),
    };
    if artifact.schema != "rr-module-artifact"
        || artifact.schema_version != 2
        || artifact.compiler_version != env!("CARGO_PKG_VERSION")
        || artifact.canonical_path != module_path.to_string_lossy()
    {
        return Ok(None);
    }
    let (source_len, source_mtime_ns) = match source_file_signature(module_path) {
        Ok(sig) => sig,
        Err(_) => return Ok(None),
    };
    if artifact.source_len != source_len || artifact.source_mtime_ns != source_mtime_ns {
        return Ok(None);
    }
    let mut module = artifact.module;
    let symbols: Vec<(crate::hir::def::SymbolId, String)> = artifact
        .symbols
        .iter()
        .cloned()
        .map(|(id, name)| (crate::hir::def::SymbolId(id), name))
        .collect();
    if !lowerer.try_preload_symbols(&symbols) {
        return Ok(None);
    }
    if let Some(source_metadata) = artifact.source_metadata.as_ref() {
        if public_impl_metadata_needs_external_traits(source_metadata) {
            return Ok(None);
        }
        lowerer.preload_public_module_metadata(source_metadata)?;
    } else if hir_module_requires_source_lowering_for_metadata(&module) {
        return Ok(None);
    }
    module.id = mod_id;
    Ok(Some(module))
}
