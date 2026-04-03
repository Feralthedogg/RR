mod env;
mod git;
mod manifest;
mod project;
mod registry;
#[cfg(test)]
mod tests;
mod types;
mod util;

use crate::error::{RR, RRCode, RRException, Stage};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use getrandom::getrandom;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub use env::{find_manifest_root, package_home, with_project_root_hint};
pub use project::*;
pub use registry::*;
pub use types::*;

use self::env::registry_root;
use self::git::{run_git, run_git_with_env};
use self::manifest::parse_toml_string;
use self::project::normalize_replace_target;
use self::registry::{
    latest_registry_version, load_registry_index, load_registry_trust_policy,
    materialize_registry_read_root, registry_channel_version, verify_registry_release_trust,
};
use self::util::{
    archive_checksum, compare_versions, copy_dir_recursive, directory_checksum, github_repo_root,
    normalize_path, read_meta_file, unique_temp_dir, version_matches_module_path, write_meta_file,
};

fn parse_module_request(spec: &str) -> Result<ModuleRequest, String> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err("missing module spec".to_string());
    }

    let (raw_source, explicit_version) = match trimmed.rsplit_once('@') {
        Some((lhs, rhs)) if !rhs.contains('/') && !rhs.is_empty() => {
            (lhs.to_string(), Some(rhs.to_string()))
        }
        _ => (trimmed.to_string(), None),
    };

    if let Some(path) = raw_source
        .strip_prefix("https://github.com/")
        .or_else(|| raw_source.strip_prefix("http://github.com/"))
    {
        parse_github_source(path, explicit_version)
    } else if let Some(path) = raw_source.strip_prefix("ssh://git@github.com/") {
        parse_github_source(path, explicit_version).map(|mut req| {
            req.source = ModuleSource::Git {
                repo_url: format!(
                    "ssh://git@github.com/{}",
                    github_repo_root(&req.module_path)
                ),
            };
            req
        })
    } else if let Some(path) = raw_source.strip_prefix("git@github.com:") {
        parse_github_source(path, explicit_version).map(|mut req| {
            req.source = ModuleSource::Git {
                repo_url: format!("git@github.com:{}", github_repo_root(&req.module_path)),
            };
            req
        })
    } else if let Some(path) = raw_source.strip_prefix("github.com/") {
        parse_github_source(path, explicit_version).map(|mut req| {
            req.module_path = format!(
                "github.com/{}",
                req.module_path.trim_start_matches("github.com/")
            );
            req
        })
    } else if raw_source.contains('/') {
        let Some(registry_root) = registry_root()? else {
            return Err(format!(
                "unsupported module source '{}'; set RR_REGISTRY_DIR for registry-backed modules or use a GitHub source",
                raw_source
            ));
        };
        Ok(ModuleRequest {
            module_path: raw_source,
            source: ModuleSource::Registry { registry_root },
            subdir: PathBuf::new(),
            requested: explicit_version
                .map(|version| {
                    if version == "latest" {
                        RequestedVersion::Latest
                    } else if version.starts_with('v') {
                        RequestedVersion::Exact(version)
                    } else {
                        RequestedVersion::Channel(version)
                    }
                })
                .unwrap_or(RequestedVersion::Latest),
        })
    } else {
        Err(format!(
            "unsupported module source '{}'; use a github.com module path, a GitHub URL, or a registry-backed module path",
            raw_source
        ))
    }
}

fn parse_github_source(
    path: &str,
    explicit_version: Option<String>,
) -> Result<ModuleRequest, String> {
    let cleaned = path.trim_matches('/').trim_end_matches(".git");
    let parts: Vec<&str> = cleaned.split('/').filter(|part| !part.is_empty()).collect();
    if parts.len() < 2 {
        return Err(format!("invalid GitHub module path '{}'", path));
    }

    let owner = parts[0];
    let repo = parts[1];
    let mut subdir = PathBuf::new();
    let mut implied_version: Option<String> = None;

    if parts.get(2) == Some(&"tree") {
        if parts.len() < 4 {
            return Err(format!("invalid GitHub tree URL '{}'", path));
        }
        implied_version = Some(parts[3].to_string());
        for part in &parts[4..] {
            subdir.push(part);
        }
    } else {
        for part in &parts[2..] {
            subdir.push(part);
        }
    }

    let mut module_path = format!("github.com/{owner}/{repo}");
    if !subdir.as_os_str().is_empty() {
        module_path.push('/');
        module_path.push_str(&subdir.to_string_lossy());
    }

    Ok(ModuleRequest {
        module_path,
        source: ModuleSource::Git {
            repo_url: format!("https://github.com/{owner}/{repo}"),
        },
        subdir,
        requested: explicit_version
            .or(implied_version)
            .map(|version| {
                if version == "latest" {
                    RequestedVersion::Latest
                } else {
                    RequestedVersion::Exact(version)
                }
            })
            .unwrap_or(RequestedVersion::Latest),
    })
}

fn install_single_remote_module(
    request: &ModuleRequest,
    direct: bool,
    state: &mut InstallState,
) -> Result<InstalledModule, String> {
    let resolved = resolve_version(request)?;
    if let Some(existing) = state.installed.get_mut(&request.module_path)
        && compare_versions(&existing.version, &resolved.version) != Ordering::Less
    {
        if direct {
            existing.direct = true;
        }
        return Ok(existing.clone());
    }

    let cache_dir = module_cache_dir(&state.package_home, &request.module_path, &resolved.version);
    let installed = if cache_dir.join("rr.mod").is_file() {
        load_cached_module(&cache_dir, &request.module_path, &resolved.version, direct)?
    } else {
        match &request.source {
            ModuleSource::Git { .. } => {
                checkout_and_cache_module(request, &resolved, &cache_dir, direct)?
            }
            ModuleSource::Registry { registry_root } => {
                install_registry_module(request, registry_root, &resolved, &cache_dir, direct)?
            }
        }
    };

    state
        .installed
        .insert(request.module_path.clone(), installed.clone());

    Ok(state
        .installed
        .get(&request.module_path)
        .cloned()
        .unwrap_or(installed))
}

fn install_registry_module(
    request: &ModuleRequest,
    registry_root: &Path,
    resolved: &ResolvedVersion,
    cache_dir: &Path,
    direct: bool,
) -> Result<InstalledModule, String> {
    let index = load_registry_index(registry_root, &request.module_path)?;
    let Some(entry) = index
        .releases
        .into_iter()
        .find(|entry| entry.version == resolved.version)
    else {
        return Err(format!(
            "registry '{}' does not contain module '{}' at version '{}'",
            registry_root.display(),
            request.module_path,
            resolved.version
        ));
    };
    if !entry.approved {
        return Err(format!(
            "registry module '{}' version '{}' is pending approval",
            request.module_path, resolved.version
        ));
    }
    let archive_path = registry_root.join(&entry.archive_rel);
    if !archive_path.is_file() {
        return Err(format!(
            "registry archive '{}' is missing",
            archive_path.display()
        ));
    }
    let actual_archive_sum = archive_checksum(&archive_path)?;
    if actual_archive_sum != entry.archive_sum {
        return Err(format!(
            "registry archive checksum mismatch for '{}': expected {}, got {}",
            request.module_path, entry.archive_sum, actual_archive_sum
        ));
    }
    let policy = load_registry_trust_policy(registry_root)?;
    verify_registry_release_trust(
        &request.module_path,
        &resolved.version,
        &entry.archive_sum,
        entry.approved,
        entry.archive_sig.as_deref(),
        entry.signer.as_deref(),
        &policy,
    )?;

    let tmp_root = unique_temp_dir("rr-registry");
    fs::create_dir_all(&tmp_root)
        .map_err(|e| format!("failed to create '{}': {}", tmp_root.display(), e))?;
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(&archive_path)
        .arg("-C")
        .arg(&tmp_root)
        .output()
        .map_err(|e| {
            format!(
                "failed to extract registry archive '{}': {}",
                archive_path.display(),
                e
            )
        })?;
    if !output.status.success() {
        let _ = fs::remove_dir_all(&tmp_root);
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    if cache_dir.exists() {
        fs::remove_dir_all(cache_dir)
            .map_err(|e| format!("failed to clear '{}': {}", cache_dir.display(), e))?;
    }
    let parent = cache_dir.parent().ok_or_else(|| {
        format!(
            "failed to determine cache directory parent for '{}'",
            cache_dir.display()
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("failed to create '{}': {}", parent.display(), e))?;
    copy_dir_recursive(&tmp_root, cache_dir)?;
    let _ = fs::remove_dir_all(&tmp_root);

    let manifest = Manifest::load_from_dir(cache_dir)?;
    if manifest.module_path != request.module_path {
        return Err(format!(
            "registry package '{}' declares module '{}'",
            request.module_path, manifest.module_path
        ));
    }

    let sum = directory_checksum(cache_dir)?;
    write_meta_file(cache_dir, "registry", &sum)?;

    Ok(InstalledModule {
        path: request.module_path.clone(),
        version: resolved.version.clone(),
        commit: "registry".to_string(),
        sum,
        direct,
    })
}

fn resolve_manifest_dependencies(
    project_root: &Path,
    manifest: &Manifest,
) -> Result<Vec<InstalledModule>, String> {
    let mut selected = BTreeMap::<String, SelectedModule>::new();
    let mut queue = Vec::<String>::new();
    let mut state = InstallState {
        package_home: package_home(),
        installed: BTreeMap::new(),
    };
    let mut processed_versions = BTreeMap::<String, String>::new();

    for (path, version) in &manifest.requires {
        selected.insert(
            path.clone(),
            SelectedModule {
                version: version.clone(),
                direct: true,
            },
        );
        queue.push(path.clone());
    }

    while let Some(module_path) = queue.pop() {
        let Some(selection) = selected.get(&module_path).cloned() else {
            continue;
        };
        if processed_versions
            .get(&module_path)
            .is_some_and(|seen| seen == &selection.version)
        {
            continue;
        }

        let loaded = load_selected_module(
            project_root,
            manifest,
            &module_path,
            &selection.version,
            selection.direct,
            &mut state,
        )?;

        let effective_version = loaded.installed.version.clone();
        processed_versions.insert(module_path.clone(), effective_version.clone());
        if let Some(current) = selected.get_mut(&module_path) {
            current.version = effective_version.clone();
            current.direct |= selection.direct;
        }

        for (dep_path, dep_version) in loaded.manifest.requires {
            let should_enqueue = match selected.get_mut(&dep_path) {
                Some(existing) => {
                    if compare_versions(&existing.version, &dep_version) == Ordering::Less {
                        existing.version = dep_version.clone();
                        true
                    } else {
                        false
                    }
                }
                None => {
                    selected.insert(
                        dep_path.clone(),
                        SelectedModule {
                            version: dep_version.clone(),
                            direct: false,
                        },
                    );
                    true
                }
            };
            if should_enqueue {
                queue.push(dep_path);
            }
        }
    }

    let mut final_modules = Vec::new();
    for (path, selection) in selected {
        let loaded = load_selected_module(
            project_root,
            manifest,
            &path,
            &selection.version,
            selection.direct,
            &mut state,
        )?;
        let mut installed = loaded.installed;
        installed.direct = selection.direct;
        final_modules.push(installed);
    }
    final_modules.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(final_modules)
}

fn load_selected_module(
    project_root: &Path,
    root_manifest: &Manifest,
    module_path: &str,
    version: &str,
    direct: bool,
    state: &mut InstallState,
) -> Result<LoadedModule, String> {
    if let Some(target) = root_manifest.replaces.get(module_path) {
        let local_root = normalize_replace_target(project_root, target);
        let manifest = Manifest::load_from_dir(&local_root)?;
        if manifest.module_path != module_path {
            return Err(format!(
                "replace target '{}' declares module '{}', expected '{}'",
                local_root.display(),
                manifest.module_path,
                module_path
            ));
        }
        let installed = InstalledModule {
            path: module_path.to_string(),
            version: version.to_string(),
            commit: "replace".to_string(),
            sum: directory_checksum(&local_root)?,
            direct,
        };
        return Ok(LoadedModule {
            installed,
            manifest,
            root: local_root,
        });
    }

    let request = parse_module_request(&format!("{module_path}@{version}"))?;
    let installed = install_single_remote_module(&request, direct, state)?;
    let root = module_cache_dir(&state.package_home, module_path, &installed.version);
    let manifest = Manifest::load_from_dir(&root)?;
    Ok(LoadedModule {
        installed,
        manifest,
        root,
    })
}

fn resolve_version(request: &ModuleRequest) -> Result<ResolvedVersion, String> {
    match &request.requested {
        RequestedVersion::Exact(version) => {
            if !version_matches_module_path(version, &request.module_path) {
                return Err(format!(
                    "version '{}' does not match module path '{}' major-version rule",
                    version, request.module_path
                ));
            }
            Ok(ResolvedVersion {
                version: version.clone(),
                checkout_ref: Some(version.clone()),
            })
        }
        RequestedVersion::Channel(channel) => match &request.source {
            ModuleSource::Registry { registry_root } => {
                let Some(version) =
                    registry_channel_version(registry_root, &request.module_path, channel)?
                else {
                    return Err(format!(
                        "registry module '{}' does not define channel '{}'",
                        request.module_path, channel
                    ));
                };
                Ok(ResolvedVersion {
                    version: version.clone(),
                    checkout_ref: Some(version),
                })
            }
            ModuleSource::Git { .. } => Err(format!(
                "channel selector '@{}' is only supported for registry-backed modules",
                channel
            )),
        },
        RequestedVersion::Latest => {
            if let Some(tag) = latest_available_version(request)? {
                Ok(ResolvedVersion {
                    version: tag.clone(),
                    checkout_ref: Some(tag),
                })
            } else {
                Ok(ResolvedVersion {
                    version: "latest".to_string(),
                    checkout_ref: None,
                })
            }
        }
    }
}

fn latest_available_version(request: &ModuleRequest) -> Result<Option<String>, String> {
    match &request.source {
        ModuleSource::Git { repo_url } => highest_remote_tag(repo_url, &request.module_path),
        ModuleSource::Registry { registry_root } => {
            latest_registry_version(registry_root, &request.module_path)
        }
    }
}

fn highest_remote_tag(repo_url: &str, module_path: &str) -> Result<Option<String>, String> {
    let output = run_git(None, &["ls-remote", "--tags", "--refs", repo_url])?;
    let mut best: Option<String> = None;
    for line in output.lines() {
        let Some((_, reference)) = line.split_once('\t') else {
            continue;
        };
        let Some(tag) = reference.strip_prefix("refs/tags/") else {
            continue;
        };
        if !tag.starts_with('v') {
            continue;
        }
        if !version_matches_module_path(tag, module_path) {
            continue;
        }
        match &best {
            Some(current) if compare_versions(tag, current) != Ordering::Greater => {}
            _ => best = Some(tag.to_string()),
        }
    }
    Ok(best)
}

fn checkout_and_cache_module(
    request: &ModuleRequest,
    resolved: &ResolvedVersion,
    cache_dir: &Path,
    direct: bool,
) -> Result<InstalledModule, String> {
    let ModuleSource::Git { repo_url } = &request.source else {
        return Err("internal error: checkout_and_cache_module requires a git source".to_string());
    };
    let tmp_root = unique_temp_dir("rr-pkg");
    run_git(
        None,
        &[
            "clone",
            "--quiet",
            repo_url,
            tmp_root.to_string_lossy().as_ref(),
        ],
    )?;

    if let Some(checkout_ref) = &resolved.checkout_ref {
        run_git(
            Some(&tmp_root),
            &["checkout", "--quiet", checkout_ref.as_str()],
        )?;
    }

    let commit = run_git(Some(&tmp_root), &["rev-parse", "HEAD"])?
        .trim()
        .to_string();

    let version = if resolved.version == "latest" {
        pseudo_version_for_checkout(&tmp_root, &commit)?
    } else {
        resolved.version.clone()
    };

    let module_root = tmp_root.join(&request.subdir);
    let manifest = Manifest::load_from_dir(&module_root)?;
    if manifest.module_path != request.module_path {
        let _ = fs::remove_dir_all(&tmp_root);
        return Err(format!(
            "module path mismatch: requested '{}', but dependency declares '{}'",
            request.module_path, manifest.module_path
        ));
    }

    if !module_root.join("src").join("lib.rr").is_file() {
        let _ = fs::remove_dir_all(&tmp_root);
        return Err(format!(
            "installed module '{}' does not contain src/lib.rr",
            request.module_path
        ));
    }

    if cache_dir.exists() {
        fs::remove_dir_all(cache_dir)
            .map_err(|e| format!("failed to clear '{}': {}", cache_dir.display(), e))?;
    }
    let parent = cache_dir.parent().ok_or_else(|| {
        format!(
            "failed to determine cache directory parent for '{}'",
            cache_dir.display()
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("failed to create '{}': {}", parent.display(), e))?;
    copy_dir_recursive(&module_root, cache_dir)?;

    let sum = directory_checksum(cache_dir)?;
    write_meta_file(cache_dir, &commit, &sum)?;
    let _ = fs::remove_dir_all(&tmp_root);

    Ok(InstalledModule {
        path: request.module_path.clone(),
        version,
        commit,
        sum,
        direct,
    })
}

fn load_cached_module(
    cache_dir: &Path,
    module_path: &str,
    version: &str,
    direct: bool,
) -> Result<InstalledModule, String> {
    let (commit, sum) = read_meta_file(cache_dir)?;
    Ok(InstalledModule {
        path: module_path.to_string(),
        version: version.to_string(),
        commit,
        sum,
        direct,
    })
}

fn pseudo_version_for_checkout(repo_root: &Path, commit: &str) -> Result<String, String> {
    let timestamp = run_git_with_env(
        Some(repo_root),
        &[("TZ", "UTC")],
        &[
            "show",
            "-s",
            "--date=format-local:%Y%m%d%H%M%S",
            "--format=%cd",
            "HEAD",
        ],
    )?
    .trim()
    .to_string();
    let short = commit.chars().take(12).collect::<String>();
    Ok(format!("v0.0.0-{timestamp}-{short}"))
}

fn module_cache_dir(package_home: &Path, module_path: &str, version: &str) -> PathBuf {
    let mut out = package_home.join("pkg").join("mod");
    let mut parts = module_path.split('/').peekable();
    while let Some(part) = parts.next() {
        if parts.peek().is_some() {
            out.push(part);
        } else {
            out.push(format!("{}@{}", part, sanitize_version(version)));
        }
    }
    out
}

fn sanitize_version(version: &str) -> String {
    version.replace('/', "_")
}
