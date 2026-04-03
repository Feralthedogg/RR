use super::{Workspace, normalize_path};
use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

thread_local! {
    static PROJECT_ROOT_HINT: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

pub fn package_home() -> PathBuf {
    if let Some(path) = env::var_os("RRPKGHOME")
        && !path.is_empty()
    {
        return PathBuf::from(path);
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".rr")
}

pub(super) fn registry_root() -> Result<Option<PathBuf>, String> {
    let Some(raw) = env::var_os("RR_REGISTRY_DIR").filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    super::materialize_registry_read_root(&PathBuf::from(raw)).map(Some)
}

pub(super) fn registry_spec_from_override(spec: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(spec) = spec {
        return Ok(spec.to_path_buf());
    }
    env::var_os("RR_REGISTRY_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            "registry root is not configured; pass --registry <dir-or-url> or set RR_REGISTRY_DIR"
                .to_string()
        })
}

pub(super) fn registry_signing_key() -> Option<String> {
    env::var("RR_REGISTRY_SIGNING_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

pub(super) fn registry_signing_ed25519_secret() -> Option<String> {
    env::var("RR_REGISTRY_SIGNING_ED25519_SECRET")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

pub(super) fn registry_signing_identity() -> Option<String> {
    env::var("RR_REGISTRY_SIGNING_IDENTITY")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn registry_trust_key() -> Option<String> {
    env::var("RR_REGISTRY_TRUST_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(registry_signing_key)
}

pub(super) fn registry_trust_ed25519_keys() -> Vec<String> {
    env::var("RR_REGISTRY_TRUST_ED25519_KEYS")
        .ok()
        .map(|raw| {
            raw.split([',', '\n'])
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_ascii_lowercase())
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn registry_policy_override_path() -> Option<PathBuf> {
    env::var_os("RR_REGISTRY_POLICY")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

pub fn with_project_root_hint<T>(entry_path: &str, f: impl FnOnce() -> T) -> T {
    let hint = find_outermost_manifest_root(Path::new(entry_path));
    PROJECT_ROOT_HINT.with(|slot| {
        let prev = slot.replace(hint);
        let out = f();
        slot.replace(prev);
        out
    })
}

pub(super) fn current_project_root_hint() -> Option<PathBuf> {
    PROJECT_ROOT_HINT.with(|slot| slot.borrow().clone())
}

pub fn find_manifest_root(path: &Path) -> Option<PathBuf> {
    let mut cur = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };

    loop {
        if cur.join("rr.mod").is_file() {
            return Some(cur);
        }
        let Some(parent) = cur.parent() else {
            break;
        };
        cur = parent.to_path_buf();
    }
    None
}

pub(super) fn find_outermost_manifest_root(path: &Path) -> Option<PathBuf> {
    let mut found = None;
    let mut cur = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };

    loop {
        if cur.join("rr.mod").is_file() {
            found = Some(cur.clone());
        }
        let Some(parent) = cur.parent() else {
            break;
        };
        cur = parent.to_path_buf();
    }

    found
}

pub(super) fn find_workspace_root(path: &Path) -> Option<PathBuf> {
    let mut cur = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };

    loop {
        if cur.join("rr.work").is_file() {
            return Some(cur);
        }
        let Some(parent) = cur.parent() else {
            break;
        };
        cur = parent.to_path_buf();
    }
    None
}

pub(super) fn load_workspace_from(path: &Path) -> Result<Option<Workspace>, String> {
    let Some(root) = find_workspace_root(path) else {
        return Ok(None);
    };
    let work_path = root.join("rr.work");
    let content = fs::read_to_string(&work_path)
        .map_err(|e| format!("failed to read '{}': {}", work_path.display(), e))?;
    let uses = parse_workspace_uses(&content)?
        .into_iter()
        .map(|entry| normalize_path(&root.join(entry)))
        .collect();
    Ok(Some(Workspace { root, uses }))
}

fn parse_workspace_uses(content: &str) -> Result<Vec<String>, String> {
    let mut uses = Vec::new();
    let mut in_use_block = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        if in_use_block {
            if trimmed == ")" {
                in_use_block = false;
                continue;
            }
            uses.push(trimmed.to_string());
            continue;
        }
        if trimmed == "use (" {
            in_use_block = true;
        } else if let Some(rest) = trimmed.strip_prefix("use ") {
            uses.push(rest.trim().to_string());
        }
    }
    Ok(uses)
}
