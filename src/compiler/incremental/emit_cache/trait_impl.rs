use super::*;
impl EmitFunctionCache for DiskFnEmitCache {
    fn load(&self, key: &str) -> RR<Option<(String, Vec<MapEntry>)>> {
        let (code_path, map_path) = self.paths(key);
        let meta_path = self.function_emit_meta_path(key);
        if !code_path.is_file() || !map_path.is_file() || !meta_path.is_file() {
            return Ok(None);
        }
        let code = match fs::read_to_string(&code_path) {
            Ok(code) => code,
            Err(_) => return Ok(None),
        };
        let map = match read_source_map(&map_path) {
            Ok(map) => map,
            Err(_) => return Ok(None),
        };
        let Some(meta) = read_cached_code_map_artifact_meta(&meta_path, "rr-function-emit-meta")
        else {
            return Ok(None);
        };
        if code_map_artifact_hash("rr-fn-emit-hash-v1", &code, &map) != meta.content_hash {
            return Ok(None);
        }
        Ok(Some((code, map)))
    }
    fn store(&self, key: &str, code: &str, map: &[MapEntry]) -> RR<()> {
        let (code_path, map_path) = self.paths(key);
        let meta_path = self.function_emit_meta_path(key);
        if let Some(parent) = code_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create function cache dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        fs::write(&code_path, code).map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to write function emit cache '{}': {}",
                        code_path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })?;
        write_source_map(&map_path, map)?;
        write_cached_code_map_artifact_meta(
            &meta_path,
            "rr-function-emit-meta",
            &CachedCodeMapArtifactMeta {
                content_hash: code_map_artifact_hash("rr-fn-emit-hash-v1", code, map),
            },
        )
    }
    fn load_raw_rewrite(&self, key: &str) -> RR<Option<String>> {
        let path = self.raw_rewrite_path(key);
        let meta_path = self.raw_rewrite_meta_path(key);
        if !path.is_file() || !meta_path.is_file() {
            return Ok(None);
        }
        let code = match fs::read_to_string(&path) {
            Ok(code) => code,
            Err(_) => return Ok(None),
        };
        let Some(meta) = read_cached_code_map_artifact_meta(&meta_path, "rr-raw-rewrite-meta")
        else {
            return Ok(None);
        };
        if text_artifact_hash("rr-raw-rewrite-hash-v1", &code) != meta.content_hash {
            return Ok(None);
        }
        Ok(Some(code))
    }
    fn store_raw_rewrite(&self, key: &str, code: &str) -> RR<()> {
        let path = self.raw_rewrite_path(key);
        let meta_path = self.raw_rewrite_meta_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create raw rewrite cache dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        fs::write(&path, code).map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to write raw rewrite cache '{}': {}",
                        path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })?;
        write_cached_code_map_artifact_meta(
            &meta_path,
            "rr-raw-rewrite-meta",
            &CachedCodeMapArtifactMeta {
                content_hash: text_artifact_hash("rr-raw-rewrite-hash-v1", code),
            },
        )
    }
    fn load_peephole(&self, key: &str) -> RR<Option<(String, Vec<u32>)>> {
        let (code_path, map_path) = self.peephole_paths(key);
        let meta_path = self.peephole_meta_path(key);
        if !code_path.is_file() || !map_path.is_file() || !meta_path.is_file() {
            return Ok(None);
        }
        let code = match fs::read_to_string(&code_path) {
            Ok(code) => code,
            Err(_) => return Ok(None),
        };
        let line_map = match read_line_map_cache(&map_path) {
            Ok(line_map) => line_map,
            Err(_) => return Ok(None),
        };
        let Some(meta) = read_cached_code_map_artifact_meta(&meta_path, "rr-peephole-meta") else {
            return Ok(None);
        };
        if code_line_map_artifact_hash("rr-peephole-hash-v1", &code, &line_map) != meta.content_hash
        {
            return Ok(None);
        }
        Ok(Some((code, line_map)))
    }
    fn store_peephole(&self, key: &str, code: &str, line_map: &[u32]) -> RR<()> {
        let (code_path, map_path) = self.peephole_paths(key);
        let meta_path = self.peephole_meta_path(key);
        if let Some(parent) = code_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create peephole cache dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        fs::write(&code_path, code).map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to write peephole cache '{}': {}",
                        code_path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })?;
        write_line_map_cache(&map_path, line_map)?;
        write_cached_code_map_artifact_meta(
            &meta_path,
            "rr-peephole-meta",
            &CachedCodeMapArtifactMeta {
                content_hash: code_line_map_artifact_hash("rr-peephole-hash-v1", code, line_map),
            },
        )
    }
    fn load_optimized_fragment(&self, key: &str) -> RR<Option<(String, Vec<MapEntry>)>> {
        let (code_path, map_path) = self.optimized_fragment_paths(key);
        let meta_path = self.optimized_fragment_meta_path(key);
        if !code_path.is_file() || !map_path.is_file() || !meta_path.is_file() {
            return Ok(None);
        }
        let code = match fs::read_to_string(&code_path) {
            Ok(code) => code,
            Err(_) => return Ok(None),
        };
        let map = match read_source_map(&map_path) {
            Ok(map) => map,
            Err(_) => return Ok(None),
        };
        let Some(meta) =
            read_cached_code_map_artifact_meta(&meta_path, "rr-optimized-fragment-meta")
        else {
            return Ok(None);
        };
        if code_map_artifact_hash("rr-optfrag-hash-v1", &code, &map) != meta.content_hash {
            return Ok(None);
        }
        Ok(Some((code, map)))
    }
    fn store_optimized_fragment(&self, key: &str, code: &str, map: &[MapEntry]) -> RR<()> {
        let (code_path, map_path) = self.optimized_fragment_paths(key);
        let meta_path = self.optimized_fragment_meta_path(key);
        if let Some(parent) = code_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create optimized fragment cache dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        fs::write(&code_path, code).map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to write optimized fragment cache '{}': {}",
                        code_path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })?;
        write_source_map(&map_path, map)?;
        write_cached_code_map_artifact_meta(
            &meta_path,
            "rr-optimized-fragment-meta",
            &CachedCodeMapArtifactMeta {
                content_hash: code_map_artifact_hash("rr-optfrag-hash-v1", code, map),
            },
        )
    }
    fn has_optimized_assembly_safe(&self, key: &str) -> RR<bool> {
        Ok(self.optimized_assembly_safe_path(key).is_file())
    }
    fn load_optimized_assembly_artifact(&self, key: &str) -> RR<Option<(String, Vec<MapEntry>)>> {
        let (code_path, map_path) = self.optimized_assembly_artifact_paths(key);
        let meta_path = self.optimized_assembly_meta_path(key);
        if !code_path.is_file() || !map_path.is_file() || !meta_path.is_file() {
            return Ok(None);
        }
        let code = match fs::read_to_string(&code_path) {
            Ok(code) => code,
            Err(_) => return Ok(None),
        };
        let map = match read_source_map(&map_path) {
            Ok(map) => map,
            Err(_) => return Ok(None),
        };
        let Some(meta) =
            read_cached_code_map_artifact_meta(&meta_path, "rr-optimized-assembly-meta")
        else {
            return Ok(None);
        };
        if code_map_artifact_hash("rr-optasm-hash-v1", &code, &map) != meta.content_hash {
            return Ok(None);
        }
        Ok(Some((code, map)))
    }
    fn store_optimized_assembly_artifact(&self, key: &str, code: &str, map: &[MapEntry]) -> RR<()> {
        let (code_path, map_path) = self.optimized_assembly_artifact_paths(key);
        let meta_path = self.optimized_assembly_meta_path(key);
        if let Some(parent) = code_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create optimized assembly artifact dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        fs::write(&code_path, code).map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to write optimized assembly artifact '{}': {}",
                        code_path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })?;
        write_source_map(&map_path, map)?;
        write_cached_code_map_artifact_meta(
            &meta_path,
            "rr-optimized-assembly-meta",
            &CachedCodeMapArtifactMeta {
                content_hash: code_map_artifact_hash("rr-optasm-hash-v1", code, map),
            },
        )
    }
    fn load_optimized_assembly_source_map(&self, key: &str) -> RR<Option<Vec<MapEntry>>> {
        let path = self.optimized_assembly_source_map_path(key);
        if !path.is_file() {
            return Ok(None);
        }
        match read_source_map(&path) {
            Ok(map) => Ok(Some(map)),
            Err(_) => Ok(None),
        }
    }
    fn store_optimized_assembly_source_map(&self, key: &str, map: &[MapEntry]) -> RR<()> {
        let path = self.optimized_assembly_source_map_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create optimized assembly source map dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        write_source_map(&path, map)
    }
    fn store_optimized_assembly_safe(&self, key: &str) -> RR<()> {
        let path = self.optimized_assembly_safe_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create optimized assembly cache dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        fs::write(&path, b"ok").map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to write optimized assembly cache '{}': {}",
                        path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })
    }
    fn has_optimized_raw_assembly_safe(&self, key: &str) -> RR<bool> {
        Ok(self.optimized_raw_assembly_safe_path(key).is_file())
    }
    fn store_optimized_raw_assembly_safe(&self, key: &str) -> RR<()> {
        let path = self.optimized_raw_assembly_safe_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create optimized raw assembly cache dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        fs::write(&path, b"ok").map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to write optimized raw assembly cache '{}': {}",
                        path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })
    }
    fn has_optimized_peephole_assembly_safe(&self, key: &str) -> RR<bool> {
        Ok(self.optimized_peephole_assembly_safe_path(key).is_file())
    }
    fn store_optimized_peephole_assembly_safe(&self, key: &str) -> RR<()> {
        let path = self.optimized_peephole_assembly_safe_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create optimized peephole assembly cache dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        fs::write(&path, b"ok").map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to write optimized peephole assembly cache '{}': {}",
                        path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })
    }
}
