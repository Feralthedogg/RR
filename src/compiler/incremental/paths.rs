use super::*;
pub(crate) fn cache_root_for_entry(entry_path: &str) -> PathBuf {
    if let Some(v) = env::var_os("RR_INCREMENTAL_CACHE_DIR")
        && !v.is_empty()
    {
        return PathBuf::from(v);
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
            return cur.join("Build").join("incremental");
        }
        let Some(parent) = cur.parent() else {
            break;
        };
        cur = parent.to_path_buf();
    }
    normalized_entry
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Build")
        .join("incremental")
}

pub(crate) fn artifact_paths(cache_root: &Path, key: &str) -> (PathBuf, PathBuf) {
    let artifact_dir = cache_root.join("artifacts");
    (
        artifact_dir.join(format!("{}.R", key)),
        artifact_dir.join(format!("{}.map", key)),
    )
}

pub(crate) fn verify_strict_artifact_match(
    tier: StrictArtifactTier,
    cache_root: &Path,
    expected: &CachedArtifact,
    built: &CachedArtifact,
) -> RR<()> {
    if expected.r_code != built.r_code {
        return Err(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "strict incremental verification failed: {} R output mismatch",
                    tier.label()
                ),
            )
            .note(format!("incremental cache root: {}", cache_root.display()))
            .help("rerun with --no-incremental to bypass cached artifacts for this compile")
            .fix(format!(
                "clear the incremental cache at `{}` and rebuild if you expect the cache to be stale or corrupted",
                cache_root.display()
            )),
        );
    }
    if expected.source_map != built.source_map {
        return Err(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "strict incremental verification failed: {} source map mismatch",
                    tier.label()
                ),
            )
            .note(format!("incremental cache root: {}", cache_root.display()))
            .help("rerun with --no-incremental to bypass cached artifacts for this compile")
            .fix(format!(
                "clear the incremental cache at `{}` and rebuild if you expect the cache to be stale or corrupted",
                cache_root.display()
            )),
        );
    }
    Ok(())
}

pub(crate) fn attach_incremental_cache_recovery_guidance(
    mut err: RRException,
    cache_root: Option<&Path>,
) -> RRException {
    err = err.help("rerun with --no-incremental to bypass cached artifacts for this compile");
    if let Some(cache_root) = cache_root {
        err = err
            .note(format!("incremental cache root: {}", cache_root.display()))
            .fix(format!(
                "clear the incremental cache at `{}` and rebuild if you expect the cache to be stale, malformed, or inaccessible",
                cache_root.display()
            ));
    }
    err
}

pub(crate) fn incremental_cache_root_for_path(path: &Path) -> Option<&Path> {
    cache_root_for_artifact_path(path)
}
