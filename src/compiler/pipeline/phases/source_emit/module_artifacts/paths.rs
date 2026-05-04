use super::*;
use std::time::{Duration, UNIX_EPOCH};

pub(crate) fn duration_from_nanos(ns: u128) -> Duration {
    Duration::from_nanos(ns.min(u64::MAX as u128) as u64)
}

pub(crate) fn module_artifact_cache_root(entry_path: &str) -> PathBuf {
    if let Some(v) = std::env::var_os("RR_INCREMENTAL_CACHE_DIR")
        && !v.is_empty()
    {
        return PathBuf::from(v).join("modules");
    }

    let normalized_entry = normalize_module_path(Path::new(entry_path));
    let mut cur = normalized_entry
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    loop {
        let managed_root = cur.join("rr.mod").is_file()
            || cur.join("src").join("main.rr").is_file()
            || cur.join("src").join("lib.rr").is_file();
        let legacy_root = cur.file_name().and_then(|name| name.to_str()) != Some("src")
            && cur.join("main.rr").is_file();
        if managed_root || legacy_root {
            return cur.join("Build").join("incremental").join("modules");
        }
        let Some(parent) = cur.parent() else {
            return normalized_entry
                .parent()
                .unwrap_or(Path::new("."))
                .join(".rr-cache")
                .join("modules");
        };
        cur = parent.to_path_buf();
    }
}

pub(crate) fn module_artifact_path(cache_root: &Path, canonical_path: &Path) -> PathBuf {
    let path_hash = stable_hash_bytes(canonical_path.to_string_lossy().as_bytes());
    cache_root.join(format!("{:016x}.json", path_hash))
}

pub(crate) fn source_file_signature(path: &Path) -> std::io::Result<(u64, u128)> {
    let meta = fs::metadata(path)?;
    let modified = meta
        .modified()
        .ok()
        .and_then(|ts| ts.duration_since(UNIX_EPOCH).ok())
        .map(|dur| dur.as_nanos())
        .unwrap_or(0);
    Ok((meta.len(), modified))
}
