use super::env::find_outermost_manifest_root;
use super::manifest::escape_toml;
use super::{InstalledModule, Manifest};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn project_dir_name_from_module_path(module_path: &str) -> String {
    module_path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or("rr-pkg")
        .to_string()
}

pub(super) fn github_repo_root(module_path: &str) -> String {
    let parts: Vec<&str> = module_path.split('/').collect();
    if parts.len() >= 3 && parts.first() == Some(&"github.com") {
        format!("{}/{}", parts[1], parts[2])
    } else {
        module_path.trim_start_matches("github.com/").to_string()
    }
}

pub(super) fn normalize_path(path: &Path) -> PathBuf {
    if let Ok(canon) = fs::canonicalize(path) {
        canon
    } else if path.is_absolute() {
        path.to_path_buf()
    } else if let Ok(cwd) = env::current_dir() {
        cwd.join(path)
    } else {
        path.to_path_buf()
    }
}

pub(super) fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}

pub(super) fn collect_publishable_files(project_root: &Path) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    collect_publishable_files_inner(project_root, project_root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_publishable_files_inner(
    root: &Path,
    dir: &Path,
    out: &mut Vec<String>,
) -> Result<(), String> {
    for entry in
        fs::read_dir(dir).map_err(|e| format!("failed to read '{}': {}", dir.display(), e))?
    {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {}", e))?;
        let path = entry.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if matches!(name, ".git" | "Build" | "target" | "vendor") {
            continue;
        }
        if path.is_dir() {
            collect_publishable_files_inner(root, &path, out)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(root)
                .ok()
                .and_then(|p| p.to_str())
                .unwrap_or(name)
                .replace('\\', "/");
            out.push(rel);
        }
    }
    Ok(())
}

pub(super) fn copy_dir_recursive(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| format!("failed to create '{}': {}", to.display(), e))?;
    for entry in
        fs::read_dir(from).map_err(|e| format!("failed to read '{}': {}", from.display(), e))?
    {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {}", e))?;
        let path = entry.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name == ".git" || name == "Build" || name == "target" || name == ".rr-meta" {
            continue;
        }
        let dest = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &dest)?;
        } else if path.is_file() {
            fs::copy(&path, &dest).map_err(|e| {
                format!(
                    "failed to copy '{}' to '{}': {}",
                    path.display(),
                    dest.display(),
                    e
                )
            })?;
        }
    }
    Ok(())
}

pub(super) fn write_lockfile<'a, I>(project_root: &Path, modules: I) -> Result<(), String>
where
    I: IntoIterator<Item = &'a InstalledModule>,
{
    let mut entries: Vec<&InstalledModule> = modules.into_iter().collect();
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    let mut out = String::new();
    out.push_str("version = 1\n");
    for module in entries {
        out.push_str("\n[[module]]\n");
        out.push_str(&format!("path = \"{}\"\n", escape_toml(&module.path)));
        out.push_str(&format!("version = \"{}\"\n", escape_toml(&module.version)));
        out.push_str(&format!("commit = \"{}\"\n", escape_toml(&module.commit)));
        out.push_str(&format!("sum = \"{}\"\n", escape_toml(&module.sum)));
        out.push_str(&format!(
            "direct = {}\n",
            if module.direct { "true" } else { "false" }
        ));
    }

    let lock_path = project_root.join("rr.lock");
    fs::write(&lock_path, out)
        .map_err(|e| format!("failed to write '{}': {}", lock_path.display(), e))
}

pub(super) fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

pub(super) fn write_meta_file(cache_dir: &Path, commit: &str, sum: &str) -> Result<(), String> {
    let meta_path = cache_dir.join(".rr-meta");
    let payload = format!("commit={commit}\nsum={sum}\n");
    fs::write(&meta_path, payload)
        .map_err(|e| format!("failed to write '{}': {}", meta_path.display(), e))
}

pub(super) fn read_meta_file(cache_dir: &Path) -> Result<(String, String), String> {
    let meta_path = cache_dir.join(".rr-meta");
    let content = fs::read_to_string(&meta_path)
        .map_err(|e| format!("failed to read '{}': {}", meta_path.display(), e))?;
    let mut commit = None;
    let mut sum = None;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("commit=") {
            commit = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("sum=") {
            sum = Some(rest.trim().to_string());
        }
    }
    Ok((
        commit.unwrap_or_default(),
        sum.unwrap_or_else(|| "fnv64:0000000000000000".to_string()),
    ))
}

pub(super) fn directory_checksum(root: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hash = 0xcbf29ce484222325_u64;
    for (rel, path) in files {
        hash = stable_hash_update(hash, rel.as_bytes());
        let content =
            fs::read(&path).map_err(|e| format!("failed to read '{}': {}", path.display(), e))?;
        hash = stable_hash_update(hash, &content);
    }
    Ok(format!("fnv64:{hash:016x}"))
}

pub(super) fn archive_checksum(path: &Path) -> Result<String, String> {
    let content =
        fs::read(path).map_err(|e| format!("failed to read '{}': {}", path.display(), e))?;
    Ok(format!(
        "fnv64:{:016x}",
        stable_hash_update(0xcbf29ce484222325_u64, &content)
    ))
}

pub(super) fn extract_registry_archive_to_temp(
    archive_path: &Path,
    prefix: &str,
) -> Result<PathBuf, String> {
    let tmp_root = unique_temp_dir(prefix);
    fs::create_dir_all(&tmp_root)
        .map_err(|e| format!("failed to create '{}': {}", tmp_root.display(), e))?;
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(archive_path)
        .arg("-C")
        .arg(&tmp_root)
        .output()
        .map_err(|e| {
            format!(
                "failed to extract archive '{}': {}",
                archive_path.display(),
                e
            )
        })?;
    if !output.status.success() {
        let _ = fs::remove_dir_all(&tmp_root);
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(tmp_root)
}

pub(super) fn collect_file_map(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut map = BTreeMap::new();
    for (rel, path) in files {
        let bytes =
            fs::read(&path).map_err(|e| format!("failed to read '{}': {}", path.display(), e))?;
        map.insert(rel, bytes);
    }
    Ok(map)
}

pub(super) fn read_manifest_from_archive(path: &Path) -> Result<Manifest, String> {
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(path)
        .arg("-O")
        .arg("rr.mod")
        .output()
        .map_err(|e| format!("failed to inspect archive '{}': {}", path.display(), e))?;
    if !output.status.success() {
        return Err(format!(
            "archive '{}' does not contain a readable rr.mod",
            path.display()
        ));
    }
    let content = String::from_utf8(output.stdout).map_err(|e| {
        format!(
            "archive '{}' rr.mod is not valid UTF-8: {}",
            path.display(),
            e
        )
    })?;
    Manifest::parse(&content)
        .map_err(|e| format!("archive '{}' has invalid rr.mod: {}", path.display(), e))
}

pub(super) fn collect_files(
    root: &Path,
    dir: &Path,
    out: &mut Vec<(String, PathBuf)>,
) -> Result<(), String> {
    for entry in
        fs::read_dir(dir).map_err(|e| format!("failed to read '{}': {}", dir.display(), e))?
    {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {}", e))?;
        let path = entry.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name == ".git" || name == "Build" || name == "target" || name == ".rr-meta" {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, out)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(root)
                .ok()
                .and_then(|rel| rel.to_str())
                .unwrap_or(name)
                .replace('\\', "/");
            out.push((rel, path));
        }
    }
    Ok(())
}

pub(super) fn stable_hash_update(mut hash: u64, bytes: &[u8]) -> u64 {
    const FNV_PRIME: u64 = 0x100000001b3;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

pub(super) fn compare_versions(lhs: &str, rhs: &str) -> Ordering {
    match (parse_version(lhs), parse_version(rhs)) {
        (Some(lhs), Some(rhs)) => lhs.cmp(&rhs),
        _ => lhs.cmp(rhs),
    }
}

fn parse_version(raw: &str) -> Option<(u64, u64, u64, String)> {
    let trimmed = raw.trim();
    let core = trimmed.strip_prefix('v')?;
    let mut parts = core.splitn(4, '.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch_and_suffix = parts.next()?;
    let (patch_str, suffix) = match patch_and_suffix.split_once('-') {
        Some((patch, suffix)) => (patch, format!("-{}", suffix)),
        None => (patch_and_suffix, String::new()),
    };
    let patch = patch_str.parse::<u64>().ok()?;
    let tail = parts
        .next()
        .map(|rest| format!("{suffix}.{rest}"))
        .unwrap_or(suffix);
    Some((major, minor, patch, tail))
}

fn version_major(raw: &str) -> Option<u64> {
    parse_version(raw).map(|(major, _, _, _)| major)
}

fn required_major_for_module_path(module_path: &str) -> Option<u64> {
    let last = module_path.rsplit('/').next()?;
    let major = last.strip_prefix('v')?.parse::<u64>().ok()?;
    (major >= 2).then_some(major)
}

pub(super) fn version_matches_module_path(version: &str, module_path: &str) -> bool {
    let Some(version_major) = version_major(version) else {
        return true;
    };
    match required_major_for_module_path(module_path) {
        Some(required) => version_major == required,
        None => version_major <= 1,
    }
}

pub(super) fn is_major_version_segment(segment: &str) -> bool {
    segment
        .strip_prefix('v')
        .and_then(|rest| rest.parse::<u64>().ok())
        .is_some_and(|major| major >= 2)
}

pub(super) fn module_path_to_rel_path(module_path: &str) -> PathBuf {
    let mut out = PathBuf::new();
    for segment in module_path.split('/') {
        out.push(segment);
    }
    out
}

pub(super) fn create_synthetic_package_entry(
    package_dir: &Path,
    import_path: &str,
    files: &[PathBuf],
) -> Result<PathBuf, String> {
    let Some(root) = find_outermost_manifest_root(package_dir) else {
        return Err(format!(
            "failed to locate package root for '{}'",
            package_dir.display()
        ));
    };
    let build_pkg_dir = root.join("Build").join("pkg");
    fs::create_dir_all(&build_pkg_dir)
        .map_err(|e| format!("failed to create '{}': {}", build_pkg_dir.display(), e))?;

    let mut hash = 0xcbf29ce484222325_u64;
    hash = stable_hash_update(hash, import_path.as_bytes());
    hash = stable_hash_update(hash, package_dir.to_string_lossy().as_bytes());
    let entry_path = build_pkg_dir.join(format!("rr_pkg_{hash:016x}.rr"));

    let mut content = String::new();
    for file in files {
        content.push_str(&format!("import \"{}\"\n", file.to_string_lossy()));
    }
    fs::write(&entry_path, content)
        .map_err(|e| format!("failed to write '{}': {}", entry_path.display(), e))?;
    Ok(normalize_path(&entry_path))
}
