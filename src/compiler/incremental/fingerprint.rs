use super::*;
pub(crate) fn collect_module_fingerprints(
    entry_path: &str,
    entry_input: &str,
) -> RR<Vec<ModuleFingerprint>> {
    let entry = normalize_module_path(Path::new(entry_path));
    let import_re = import_regex()?;

    let mut queue: Vec<PathBuf> = vec![entry.clone()];
    let mut visited = FxHashSet::default();
    let mut content_by_path: FxHashMap<PathBuf, String> = FxHashMap::default();
    content_by_path.insert(entry.clone(), entry_input.to_string());
    let mut modules = Vec::new();

    while let Some(path) = queue.pop() {
        let canonical = normalize_module_path(&path);
        if !canonical.is_absolute() {
            return Err(RRException::new(
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                format!(
                    "relative import resolution requires an absolute entry path; normalize '{}' before incremental compilation",
                    entry_path
                ),
            ));
        }
        if !visited.insert(canonical.clone()) {
            continue;
        }

        let content = if let Some(s) = content_by_path.remove(&canonical) {
            s
        } else {
            fs::read_to_string(&canonical).map_err(|e| {
                RRException::new(
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    format!(
                        "failed to load imported module '{}': {}",
                        canonical.display(),
                        e
                    ),
                )
            })?
        };

        let mut direct_imports = Vec::new();
        for cap in import_re.captures_iter(&content) {
            let Some(m) = cap.get(1) else {
                continue;
            };
            let raw_import = m.as_str();
            let resolved = crate::pkg::resolve_import_path(&canonical, raw_import)?;
            direct_imports.push(resolved.clone());
            if !visited.contains(&resolved) {
                queue.push(resolved);
            }
        }
        direct_imports.sort();
        direct_imports.dedup();

        modules.push(ModuleFingerprint {
            canonical_path: canonical.clone(),
            content_hash: stable_hash_bytes(content.as_bytes()),
            direct_imports,
            exported_symbol_fingerprint: exported_symbol_fingerprint(&content),
            function_body_fingerprint: function_body_fingerprint(&content),
        });
    }

    modules.sort_by(|a, b| a.canonical_path.cmp(&b.canonical_path));
    Ok(modules)
}

pub(crate) fn exported_symbol_fingerprint(content: &str) -> u64 {
    let mut payload = String::new();
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("export fn ") || trimmed.starts_with("fn ") {
            payload.push_str(trimmed);
            payload.push('\n');
        }
    }
    stable_hash_bytes(payload.as_bytes())
}

pub(crate) fn function_body_fingerprint(content: &str) -> u64 {
    let mut payload = String::new();
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("fn ") || trimmed.starts_with("export fn ") || !payload.is_empty() {
            payload.push_str(line);
            payload.push('\n');
        }
    }
    stable_hash_bytes(payload.as_bytes())
}
