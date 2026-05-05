use super::*;
pub(crate) fn load_artifact(cache_root: &Path, key: &str) -> RR<Option<CachedArtifact>> {
    let (code_path, map_path) = artifact_paths(cache_root, key);
    if !code_path.is_file() || !map_path.is_file() {
        return Ok(None);
    }
    let r_code = fs::read_to_string(&code_path).map_err(|e| {
        attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to read incremental artifact '{}': {}",
                    code_path.display(),
                    e
                ),
            ),
            Some(cache_root),
        )
    })?;
    let source_map = read_source_map(&map_path)?;
    Ok(Some(CachedArtifact { r_code, source_map }))
}

pub(crate) fn store_artifact(cache_root: &Path, key: &str, artifact: &CachedArtifact) -> RR<()> {
    let (code_path, map_path) = artifact_paths(cache_root, key);
    if let Some(parent) = code_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to create cache directory '{}': {}",
                        parent.display(),
                        e
                    ),
                ),
                Some(cache_root),
            )
        })?;
    }
    fs::write(&code_path, &artifact.r_code).map_err(|e| {
        attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to write incremental artifact '{}': {}",
                    code_path.display(),
                    e
                ),
            ),
            Some(cache_root),
        )
    })?;
    write_source_map(&map_path, &artifact.source_map)
}

pub(crate) fn render_source_map_cache_contents(map: &[MapEntry]) -> String {
    let mut out = String::new();
    for entry in map {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            entry.r_line,
            entry.rr_span.start_byte,
            entry.rr_span.end_byte,
            entry.rr_span.start_line,
            entry.rr_span.start_col,
            entry.rr_span.end_line,
            entry.rr_span.end_col
        ));
    }
    out
}

pub(crate) fn code_map_artifact_hash(kind: &str, code: &str, map: &[MapEntry]) -> u64 {
    let map_contents = render_source_map_cache_contents(map);
    let mut payload = String::new();
    payload.push_str(kind);
    payload.push('|');
    payload.push_str(code);
    payload.push('|');
    payload.push_str(&map_contents);
    stable_hash_bytes(payload.as_bytes())
}

pub(crate) fn render_line_map_cache_contents(map: &[u32]) -> String {
    let mut out = String::new();
    for line in map {
        out.push_str(&line.to_string());
        out.push('\n');
    }
    out
}

pub(crate) fn code_line_map_artifact_hash(kind: &str, code: &str, line_map: &[u32]) -> u64 {
    let line_map_contents = render_line_map_cache_contents(line_map);
    let mut payload = String::new();
    payload.push_str(kind);
    payload.push('|');
    payload.push_str(code);
    payload.push('|');
    payload.push_str(&line_map_contents);
    stable_hash_bytes(payload.as_bytes())
}

pub(crate) fn text_artifact_hash(kind: &str, text: &str) -> u64 {
    let mut payload = String::new();
    payload.push_str(kind);
    payload.push('|');
    payload.push_str(text);
    stable_hash_bytes(payload.as_bytes())
}

pub(crate) fn write_cached_code_map_artifact_meta(
    path: &Path,
    schema: &str,
    meta: &CachedCodeMapArtifactMeta,
) -> RR<()> {
    let payload = format!(
        "schema={}\nversion={}\nhash={:016x}\n",
        schema,
        env!("CARGO_PKG_VERSION"),
        meta.content_hash
    );
    fs::write(path, payload).map_err(|e| {
        attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to write code-map artifact metadata '{}': {}",
                    path.display(),
                    e
                ),
            ),
            incremental_cache_root_for_path(path),
        )
    })
}

pub(crate) fn read_cached_code_map_artifact_meta(
    path: &Path,
    expected_schema: &str,
) -> Option<CachedCodeMapArtifactMeta> {
    let content = fs::read_to_string(path).ok()?;
    let mut schema = None;
    let mut version = None;
    let mut hash = None;
    for line in content.lines() {
        let (key, value) = line.split_once('=')?;
        match key {
            "schema" => schema = Some(value),
            "version" => version = Some(value),
            "hash" => hash = u64::from_str_radix(value, 16).ok(),
            _ => {}
        }
    }
    if schema != Some(expected_schema) || version != Some(env!("CARGO_PKG_VERSION")) {
        return None;
    }
    Some(CachedCodeMapArtifactMeta {
        content_hash: hash?,
    })
}

pub(crate) fn write_source_map(path: &Path, map: &[MapEntry]) -> RR<()> {
    let out = render_source_map_cache_contents(map);
    fs::write(path, out).map_err(|e| {
        attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to write incremental source map '{}': {}",
                    path.display(),
                    e
                ),
            ),
            incremental_cache_root_for_path(path),
        )
    })
}

pub(crate) fn write_line_map_cache(path: &Path, map: &[u32]) -> RR<()> {
    let out = render_line_map_cache_contents(map);
    fs::write(path, out).map_err(|e| {
        attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to write peephole line map '{}': {}",
                    path.display(),
                    e
                ),
            ),
            incremental_cache_root_for_path(path),
        )
    })
}

pub(crate) fn read_source_map(path: &Path) -> RR<Vec<MapEntry>> {
    let content = fs::read_to_string(path).map_err(|e| {
        attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to read incremental source map '{}': {}",
                    path.display(),
                    e
                ),
            ),
            incremental_cache_root_for_path(path),
        )
    })?;
    let mut out = Vec::new();
    for (line_no, line) in content.lines().enumerate() {
        if let Some(entry) = parse_source_map_entry(line) {
            out.push(entry);
        } else if !line.trim().is_empty() {
            let err = RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to parse incremental source map '{}': malformed entry at line {}",
                    path.display(),
                    line_no + 1
                ),
            )
            .note("the cached source map entry is malformed or was written by an incompatible compiler shape");
            return Err(attach_incremental_cache_recovery_guidance(
                err,
                incremental_cache_root_for_path(path),
            ));
        }
    }
    Ok(out)
}

pub(crate) fn read_line_map_cache(path: &Path) -> RR<Vec<u32>> {
    let content = fs::read_to_string(path).map_err(|e| {
        attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to read peephole line map '{}': {}",
                    path.display(),
                    e
                ),
            ),
            incremental_cache_root_for_path(path),
        )
    })?;
    let mut out = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parsed = trimmed.parse::<u32>().map_err(|_| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to parse peephole line map '{}': malformed entry at line {}",
                        path.display(),
                        idx + 1
                    ),
                ),
                incremental_cache_root_for_path(path),
            )
        })?;
        out.push(parsed);
    }
    Ok(out)
}

pub(crate) fn cache_root_for_artifact_path(path: &Path) -> Option<&Path> {
    path.ancestors()
        .find(|ancestor| ancestor.file_name().and_then(|name| name.to_str()) == Some(".rr-cache"))
}

pub(crate) fn parse_source_map_entry(line: &str) -> Option<MapEntry> {
    if line.trim().is_empty() {
        return None;
    }
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() != 7 {
        return None;
    }
    let parsed = (
        parts[0].parse::<u32>(),
        parts[1].parse::<usize>(),
        parts[2].parse::<usize>(),
        parts[3].parse::<u32>(),
        parts[4].parse::<u32>(),
        parts[5].parse::<u32>(),
        parts[6].parse::<u32>(),
    );
    let (
        Ok(r_line),
        Ok(start_byte),
        Ok(end_byte),
        Ok(start_line),
        Ok(start_col),
        Ok(end_line),
        Ok(end_col),
    ) = parsed
    else {
        return None;
    };
    Some(MapEntry {
        r_line,
        rr_span: Span {
            start_byte,
            end_byte,
            start_line,
            start_col,
            end_line,
            end_col,
        },
    })
}

pub(crate) fn stable_hash_bytes(bytes: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET_BASIS;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
