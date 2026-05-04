use super::env::{registry_policy_override_path, registry_trust_key};
use super::git::{
    create_git_tag, ensure_git_identity, git_repo_is_dirty, git_tag_exists, push_git_tag,
};
use super::manifest::{escape_toml, parse_toml_string};
use super::primitives::*;
use super::signing::{
    registry_signature_payload, resolved_registry_trust_ed25519_keys, sign_registry_release,
    verify_registry_signer_policy,
};
use super::trust_policy::{mutate_registry_policy, parse_registry_trust_policy};
use super::util::{
    archive_checksum, collect_publishable_files, compare_versions, module_path_to_rel_path,
    normalize_path, project_dir_name_from_module_path, stable_hash_update, unique_temp_dir,
    version_matches_module_path,
};
use super::*;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

#[derive(Clone, Copy, Debug, Default)]
pub struct RegistryPolicyBootstrapOptions {
    pub require_signed: bool,
    pub require_approval: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RegistryOnboardOptions {
    pub require_signed: bool,
    pub require_approval: bool,
    pub auto_approve: bool,
}

pub fn bootstrap_registry_policy(
    trusted_public_key: &str,
    signer: Option<&str>,
    auto_approve_signer: Option<&str>,
    options: RegistryPolicyBootstrapOptions,
    registry_spec: &Path,
) -> Result<PathBuf, String> {
    let trusted_key = normalize_ed25519_public_key(trusted_public_key)?;
    mutate_registry_policy(registry_spec, "bootstrap registry policy", |policy| {
        policy.require_signed = options.require_signed;
        policy.require_approval = options.require_approval;
        policy.trusted_ed25519_keys.clear();
        policy.trusted_ed25519_keys.push(trusted_key.clone());
        policy.revoked_ed25519_keys.clear();
        policy.allowed_signers.clear();
        policy.auto_approve_signers.clear();
        if let Some(signer) = signer.filter(|value| !value.trim().is_empty()) {
            policy.allowed_signers.push(signer.trim().to_string());
        }
        if let Some(auto) = auto_approve_signer.filter(|value| !value.trim().is_empty()) {
            policy.auto_approve_signers.push(auto.trim().to_string());
        }
        Ok(())
    })
}

pub fn apply_registry_policy(source_path: &Path, registry_spec: &Path) -> Result<PathBuf, String> {
    let content = fs::read_to_string(source_path)
        .map_err(|e| format!("failed to read '{}': {}", source_path.display(), e))?;
    let policy = parse_registry_trust_policy(&content)
        .map_err(|e| format!("failed to parse '{}': {}", source_path.display(), e))?;
    mutate_registry_policy(registry_spec, "apply registry policy", move |slot| {
        *slot = policy;
        Ok(())
    })
}

pub fn onboard_registry(
    registry_spec: &Path,
    out_dir: Option<&Path>,
    identity: Option<&str>,
    options: RegistryOnboardOptions,
) -> Result<RegistryOnboardReport, String> {
    let keygen = generate_registry_keypair(out_dir, identity)?;
    let signer = identity.filter(|value| !value.trim().is_empty());
    let policy_path = bootstrap_registry_policy(
        &keygen.public_key_hex,
        signer,
        if options.auto_approve { signer } else { None },
        RegistryPolicyBootstrapOptions {
            require_signed: options.require_signed,
            require_approval: options.require_approval,
        },
        registry_spec,
    )?;
    Ok(RegistryOnboardReport {
        keygen,
        policy_path,
    })
}

pub fn approve_registry_release(
    module_path: &str,
    version: &str,
    registry_spec: &Path,
) -> Result<(), String> {
    mutate_registry_index(
        registry_spec,
        module_path,
        &format!("approve {} {}", module_path, version),
        |index| {
            let Some(entry) = index
                .releases
                .iter_mut()
                .find(|entry| entry.version == version)
            else {
                return Err(format!(
                    "registry module '{}' does not contain version '{}'",
                    module_path, version
                ));
            };
            entry.approved = true;
            Ok(())
        },
    )
}

pub fn unapprove_registry_release(
    module_path: &str,
    version: &str,
    registry_spec: &Path,
) -> Result<(), String> {
    mutate_registry_index(
        registry_spec,
        module_path,
        &format!("unapprove {} {}", module_path, version),
        |index| {
            let Some(entry) = index
                .releases
                .iter_mut()
                .find(|entry| entry.version == version)
            else {
                return Err(format!(
                    "registry module '{}' does not contain version '{}'",
                    module_path, version
                ));
            };
            entry.approved = false;
            index.channels.retain(|_, assigned| assigned != version);
            Ok(())
        },
    )
}

pub fn promote_registry_release(
    module_path: &str,
    version: &str,
    registry_spec: &Path,
) -> Result<(), String> {
    mutate_registry_index(
        registry_spec,
        module_path,
        &format!("promote registry release {} {}", module_path, version),
        |index| {
            let mut found = false;
            for entry in &mut index.releases {
                if entry.version == version {
                    if entry.yanked {
                        return Err(format!(
                            "cannot promote yanked release '{} {}'",
                            module_path, version
                        ));
                    }
                    entry.approved = true;
                    found = true;
                } else {
                    entry.approved = false;
                }
            }
            if !found {
                return Err(format!(
                    "registry module '{}' does not contain version '{}'",
                    module_path, version
                ));
            }
            Ok(())
        },
    )
}

pub fn publish_project(
    project_root: &Path,
    version: &str,
    options: &PublishOptions,
) -> Result<PublishReport, String> {
    if version.trim().is_empty() {
        return Err("publish version must be non-empty".to_string());
    }
    if !version.starts_with('v') {
        return Err("publish version must start with 'v' (for example v1.0.0)".to_string());
    }
    if !options.allow_dirty && git_repo_is_dirty(project_root)? {
        return Err("git worktree is dirty; commit or pass --allow-dirty".to_string());
    }
    if git_tag_exists(project_root, version)? {
        return Err(format!("git tag '{}' already exists", version));
    }

    let manifest = Manifest::load_from_dir(project_root)?;
    let included_files = collect_publishable_files(project_root)?;
    if included_files.is_empty() {
        return Err("nothing to publish".to_string());
    }

    let build_pkg_dir = project_root.join("Build").join("publish");
    fs::create_dir_all(&build_pkg_dir)
        .map_err(|e| format!("failed to create '{}': {}", build_pkg_dir.display(), e))?;
    let archive_path = build_pkg_dir.join(format!(
        "{}@{}.tar.gz",
        project_dir_name_from_module_path(&manifest.module_path),
        version
    ));

    if archive_path.exists() {
        fs::remove_file(&archive_path)
            .map_err(|e| format!("failed to clear '{}': {}", archive_path.display(), e))?;
    }

    let output = Command::new("tar")
        .current_dir(project_root)
        .arg("-czf")
        .arg(&archive_path)
        .args(&included_files)
        .output()
        .map_err(|e| {
            format!(
                "failed to create archive '{}': {}",
                archive_path.display(),
                e
            )
        })?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    if let Some(registry_spec) = &options.registry {
        publish_to_registry(
            project_root,
            &manifest,
            version,
            &archive_path,
            &included_files,
            options,
            registry_spec,
        )?;
    }

    let mut tag = None;
    let mut tag_pushed = false;
    if options.push_tag && !options.dry_run {
        create_git_tag(project_root, version)?;
        tag = Some(version.to_string());
        if let Some(remote) = &options.remote {
            push_git_tag(project_root, remote, version)?;
            tag_pushed = true;
        }
    }

    Ok(PublishReport {
        archive_path,
        included_files,
        dry_run: options.dry_run,
        tag,
        tag_pushed,
    })
}

fn publish_to_registry(
    project_root: &Path,
    manifest: &Manifest,
    version: &str,
    archive_path: &Path,
    included_files: &[String],
    options: &PublishOptions,
    registry_spec: &Path,
) -> Result<(), String> {
    let (write_root, is_remote) = materialize_registry_write_root(registry_spec, options.dry_run)?;
    let archive_sum = archive_checksum(archive_path)?;
    let registry_archive = registry_archive_path(&write_root, &manifest.module_path, version);

    let policy = load_registry_trust_policy(&write_root)?;
    let (archive_sig, signer) =
        sign_registry_release(&manifest.module_path, version, &archive_sum)?;
    if policy.require_signed && archive_sig.is_none() {
        return Err("registry policy requires signed releases".to_string());
    }
    if !policy.allowed_signers.is_empty() {
        verify_registry_signer_policy(&manifest.module_path, version, signer.as_deref(), &policy)?;
    }
    let approved = should_auto_approve_release(&policy, signer.as_deref());

    if !options.dry_run {
        if let Some(parent) = registry_archive.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create '{}': {}", parent.display(), e))?;
        }
        fs::copy(archive_path, &registry_archive).map_err(|e| {
            format!(
                "failed to copy '{}' to '{}': {}",
                archive_path.display(),
                registry_archive.display(),
                e
            )
        })?;
    }

    let archive_rel = registry_archive
        .strip_prefix(&write_root)
        .ok()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|| registry_archive.to_string_lossy().to_string())
        .replace('\\', "/");

    let mut index = load_registry_index(&write_root, &manifest.module_path)?;
    index.module_path = manifest.module_path.clone();
    index.description = manifest.description.clone();
    index.license = manifest.license.clone();
    index.homepage = manifest.homepage.clone();
    index.releases.retain(|entry| entry.version != version);
    index.releases.push(RegistryEntry {
        version: version.to_string(),
        archive_rel,
        archive_sum,
        file_count: included_files.len(),
        yanked: false,
        approved,
        archive_sig,
        signer,
    });
    index
        .releases
        .sort_by(|a, b| compare_versions(&a.version, &b.version));
    if !options.dry_run {
        write_registry_index(&write_root, &manifest.module_path, &index)?;
        append_registry_audit_entry(
            &write_root,
            "publish",
            &format!(
                "module={} version={} approved={} archive={}",
                manifest.module_path,
                version,
                if approved { "true" } else { "false" },
                registry_archive.display()
            ),
        )?;
    }

    if is_remote && !options.dry_run {
        run_git(Some(&write_root), &["add", "."])?;
        let _ = run_git(
            Some(&write_root),
            &[
                "commit",
                "-m",
                &format!("publish {} {}", manifest.module_path, version),
            ],
        );
        run_git(Some(&write_root), &["push", "origin", "HEAD"])?;
    }

    let _ = project_root;
    Ok(())
}

pub(in crate::pkg) fn load_registry_index(
    registry_root: &Path,
    module_path: &str,
) -> Result<RegistryIndex, String> {
    let index_path = registry_index_path(registry_root, module_path);
    if !index_path.is_file() {
        return Ok(RegistryIndex {
            module_path: module_path.to_string(),
            ..RegistryIndex::default()
        });
    }
    let content = fs::read_to_string(&index_path)
        .map_err(|e| format!("failed to read '{}': {}", index_path.display(), e))?;
    let mut index = RegistryIndex {
        module_path: module_path.to_string(),
        ..RegistryIndex::default()
    };
    let mut current: Option<RegistryEntry> = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "version = 1" {
            continue;
        }
        if trimmed == "[[release]]" {
            if let Some(entry) = current.take() {
                index.releases.push(entry);
            }
            current = Some(RegistryEntry {
                version: String::new(),
                archive_rel: String::new(),
                archive_sum: String::new(),
                file_count: 0,
                yanked: false,
                approved: true,
                archive_sig: None,
                signer: None,
            });
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            if let Some(entry) = current.as_mut() {
                match key {
                    "version" => entry.version = parse_toml_string(value)?,
                    "archive" => entry.archive_rel = parse_toml_string(value)?,
                    "sum" => entry.archive_sum = parse_toml_string(value)?,
                    "files" => entry.file_count = value.parse::<usize>().unwrap_or(0),
                    "yanked" => entry.yanked = value == "true",
                    "approved" => entry.approved = value == "true",
                    "sig" => entry.archive_sig = Some(parse_toml_string(value)?),
                    "signer" => entry.signer = Some(parse_toml_string(value)?),
                    _ => {}
                }
            } else {
                match key {
                    "module" => index.module_path = parse_toml_string(value)?,
                    "description" => index.description = Some(parse_toml_string(value)?),
                    "license" => index.license = Some(parse_toml_string(value)?),
                    "homepage" => index.homepage = Some(parse_toml_string(value)?),
                    "deprecated" => index.deprecated = Some(parse_toml_string(value)?),
                    _ if key.starts_with("channel.") => {
                        let channel = normalize_registry_channel(&key["channel.".len()..])?;
                        index.channels.insert(channel, parse_toml_string(value)?);
                    }
                    _ => {}
                }
            }
        }
    }
    if let Some(entry) = current.take() {
        index.releases.push(entry);
    }
    index.releases.retain(|entry| !entry.version.is_empty());
    Ok(index)
}

pub(in crate::pkg) fn latest_registry_version(
    registry_root: &Path,
    module_path: &str,
) -> Result<Option<String>, String> {
    let mut releases = load_registry_index(registry_root, module_path)?.releases;
    releases.retain(|entry| {
        !entry.yanked && entry.approved && version_matches_module_path(&entry.version, module_path)
    });
    releases.sort_by(|a, b| compare_versions(&a.version, &b.version));
    Ok(releases.last().map(|entry| entry.version.clone()))
}

pub(in crate::pkg) fn registry_channel_version(
    registry_root: &Path,
    module_path: &str,
    channel: &str,
) -> Result<Option<String>, String> {
    let channel = normalize_registry_channel(channel)?;
    Ok(load_registry_index(registry_root, module_path)?
        .channels
        .get(&channel)
        .cloned())
}

pub(in crate::pkg) fn load_registry_trust_policy(
    registry_root: &Path,
) -> Result<RegistryTrustPolicy, String> {
    let path = registry_policy_override_path().unwrap_or_else(|| registry_root.join("policy.toml"));
    if !path.is_file() {
        if registry_policy_override_path().is_some() {
            return Err(format!(
                "registry policy file '{}' was not found",
                path.display()
            ));
        }
        return Ok(RegistryTrustPolicy::default());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read '{}': {}", path.display(), e))?;
    parse_registry_trust_policy(&content)
        .map_err(|e| format!("failed to parse '{}': {}", path.display(), e))
}

pub(in crate::pkg) fn verify_registry_release_trust(
    module_path: &str,
    version: &str,
    archive_sum: &str,
    approved: bool,
    signature: Option<&str>,
    signer: Option<&str>,
    policy: &RegistryTrustPolicy,
) -> Result<(), String> {
    if !approved {
        return Err(format!(
            "registry release '{} {}' is pending approval",
            module_path, version
        ));
    }
    let Some(signature) = signature else {
        if policy.require_signed || !policy.allowed_signers.is_empty() {
            return Err(format!(
                "registry policy requires signed releases for '{} {}'",
                module_path, version
            ));
        }
        return Ok(());
    };

    let payload = registry_signature_payload(module_path, version, archive_sum);
    if let Some(raw_digest) = signature.strip_prefix("hmac-sha256:") {
        verify_registry_signer_policy(module_path, version, signer, policy)?;
        let Some(key) = registry_trust_key() else {
            return Err(format!(
                "registry release '{} {}' uses HMAC signing but RR_REGISTRY_TRUST_KEY is not configured",
                module_path, version
            ));
        };
        let expected = hmac_sha256_hex(&key, payload.as_bytes())?;
        if raw_digest == expected {
            return Ok(());
        }
        return Err(format!(
            "registry signature mismatch for '{} {}': expected hmac-sha256:{}, got {}",
            module_path, version, expected, signature
        ));
    }

    let Some(rest) = signature.strip_prefix("ed25519:") else {
        return Err(format!(
            "unsupported registry signature format for '{} {}': {}",
            module_path, version, signature
        ));
    };
    let Some((public_key_hex, signature_hex)) = rest.split_once(':') else {
        return Err(format!(
            "invalid ed25519 registry signature format for '{} {}': {}",
            module_path, version, signature
        ));
    };
    let trusted_keys = resolved_registry_trust_ed25519_keys(policy)?;
    if trusted_keys.is_empty() {
        return Err(format!(
            "registry release '{} {}' uses ed25519 signing but no trusted ed25519 keys are configured",
            module_path, version
        ));
    }
    if policy
        .revoked_ed25519_keys
        .iter()
        .any(|key| key == public_key_hex)
    {
        return Err(format!(
            "registry signer key is revoked for '{} {}': {}",
            module_path, version, public_key_hex
        ));
    }
    if !trusted_keys.iter().any(|key| key == public_key_hex) {
        return Err(format!(
            "registry signer key is not trusted for '{} {}': {}",
            module_path, version, public_key_hex
        ));
    }
    verify_registry_signer_policy(module_path, version, signer, policy)?;

    let public_key_bytes = hex_decode(public_key_hex)?;
    let public_key_arr: [u8; 32] = public_key_bytes.try_into().map_err(|_| {
        format!(
            "invalid ed25519 public key length for '{} {}'",
            module_path, version
        )
    })?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_arr).map_err(|e| {
        format!(
            "invalid ed25519 public key for '{} {}': {}",
            module_path, version, e
        )
    })?;

    let signature_bytes = hex_decode(signature_hex)?;
    let signature_arr: [u8; 64] = signature_bytes.try_into().map_err(|_| {
        format!(
            "invalid ed25519 signature length for '{} {}'",
            module_path, version
        )
    })?;
    let signature = Signature::from_bytes(&signature_arr);
    verifying_key
        .verify(payload.as_bytes(), &signature)
        .map_err(|e| {
            format!(
                "registry signature mismatch for '{} {}': {}",
                module_path, version, e
            )
        })
}

fn registry_archive_path(registry_root: &Path, module_path: &str, version: &str) -> PathBuf {
    registry_root
        .join("pkg")
        .join(module_path_to_rel_path(module_path))
        .join(format!(
            "{}@{}.tar.gz",
            project_dir_name_from_module_path(module_path),
            sanitize_version(version)
        ))
}

fn registry_spec_text(spec: &Path) -> String {
    spec.to_string_lossy().to_string()
}

fn is_remote_registry_spec(spec: &Path) -> bool {
    let text = registry_spec_text(spec);
    text.starts_with("http://")
        || text.starts_with("https://")
        || text.starts_with("ssh://")
        || text.starts_with("git@")
        || text.starts_with("file:")
}

fn registry_cache_root_for_spec(spec: &Path) -> PathBuf {
    let text = registry_spec_text(spec);
    let hash = stable_hash_update(0xcbf29ce484222325_u64, text.as_bytes());
    package_home().join("registry").join(format!("{hash:016x}"))
}

pub(in crate::pkg) fn materialize_registry_read_root(spec: &Path) -> Result<PathBuf, String> {
    if !is_remote_registry_spec(spec) {
        return Ok(normalize_path(spec));
    }
    let remote = registry_spec_text(spec);
    let local_root = registry_cache_root_for_spec(spec);
    if local_root.join(".git").exists() {
        let _ = run_git(Some(&local_root), &["pull", "--ff-only"])?;
    } else {
        if let Some(parent) = local_root.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create '{}': {}", parent.display(), e))?;
        }
        run_git(
            None,
            &[
                "clone",
                "--quiet",
                &remote,
                local_root.to_string_lossy().as_ref(),
            ],
        )?;
    }
    Ok(local_root)
}

pub(super) fn materialize_registry_write_root(
    spec: &Path,
    dry_run: bool,
) -> Result<(PathBuf, bool), String> {
    if !is_remote_registry_spec(spec) {
        return Ok((normalize_path(spec), false));
    }
    let remote = registry_spec_text(spec);
    let worktree = unique_temp_dir("rr-registry-publish");
    if !dry_run {
        run_git(
            None,
            &[
                "clone",
                "--quiet",
                &remote,
                worktree.to_string_lossy().as_ref(),
            ],
        )?;
        ensure_git_identity(&worktree)?;
    } else {
        fs::create_dir_all(&worktree)
            .map_err(|e| format!("failed to create '{}': {}", worktree.display(), e))?;
    }
    Ok((worktree, true))
}

fn registry_index_path(registry_root: &Path, module_path: &str) -> PathBuf {
    let mut path = registry_root
        .join("index")
        .join(module_path_to_rel_path(module_path));
    path.set_extension("toml");
    path
}

fn write_registry_index(
    registry_root: &Path,
    module_path: &str,
    index: &RegistryIndex,
) -> Result<(), String> {
    let index_path = registry_index_path(registry_root, module_path);
    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create '{}': {}", parent.display(), e))?;
    }
    let mut out = String::new();
    out.push_str("version = 1\n");
    out.push_str(&format!(
        "module = \"{}\"\n",
        escape_toml(&index.module_path)
    ));
    if let Some(description) = &index.description {
        out.push_str(&format!("description = \"{}\"\n", escape_toml(description)));
    }
    if let Some(license) = &index.license {
        out.push_str(&format!("license = \"{}\"\n", escape_toml(license)));
    }
    if let Some(homepage) = &index.homepage {
        out.push_str(&format!("homepage = \"{}\"\n", escape_toml(homepage)));
    }
    if let Some(deprecated) = &index.deprecated {
        out.push_str(&format!("deprecated = \"{}\"\n", escape_toml(deprecated)));
    }
    for (channel, version) in &index.channels {
        out.push_str(&format!(
            "channel.{} = \"{}\"\n",
            escape_toml(channel),
            escape_toml(version)
        ));
    }
    for entry in &index.releases {
        out.push_str("\n[[release]]\n");
        out.push_str(&format!("version = \"{}\"\n", escape_toml(&entry.version)));
        out.push_str(&format!(
            "archive = \"{}\"\n",
            escape_toml(&entry.archive_rel)
        ));
        out.push_str(&format!("sum = \"{}\"\n", escape_toml(&entry.archive_sum)));
        out.push_str(&format!("files = {}\n", entry.file_count));
        out.push_str(&format!(
            "yanked = {}\n",
            if entry.yanked { "true" } else { "false" }
        ));
        out.push_str(&format!(
            "approved = {}\n",
            if entry.approved { "true" } else { "false" }
        ));
        if let Some(sig) = &entry.archive_sig {
            out.push_str(&format!("sig = \"{}\"\n", escape_toml(sig)));
        }
        if let Some(signer) = &entry.signer {
            out.push_str(&format!("signer = \"{}\"\n", escape_toml(signer)));
        }
    }
    fs::write(&index_path, out)
        .map_err(|e| format!("failed to write '{}': {}", index_path.display(), e))
}

pub(super) fn load_all_registry_indices(
    registry_root: &Path,
) -> Result<Vec<(String, RegistryIndex)>, String> {
    let index_root = registry_root.join("index");
    if !index_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_registry_index_files(&index_root, &mut files)?;
    files.sort();

    let mut out = Vec::new();
    for path in files {
        let module_path = registry_module_path_from_index_path(&index_root, &path)?;
        let mut index = load_registry_index(registry_root, &module_path)?;
        if index.module_path.is_empty() {
            index.module_path = module_path.clone();
        }
        out.push((module_path, index));
    }
    Ok(out)
}

fn collect_registry_index_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        fs::read_dir(dir).map_err(|e| format!("failed to read '{}': {}", dir.display(), e))?
    {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_registry_index_files(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
            out.push(path);
        }
    }
    Ok(())
}

fn registry_module_path_from_index_path(index_root: &Path, path: &Path) -> Result<String, String> {
    let rel = path
        .strip_prefix(index_root)
        .map_err(|e| format!("failed to derive registry module path: {}", e))?;
    let mut module_path = rel.to_string_lossy().replace('\\', "/");
    if module_path.ends_with(".toml") {
        module_path.truncate(module_path.len() - ".toml".len());
    }
    Ok(module_path)
}

pub(super) fn latest_non_yanked_registry_release<'a>(
    index: &'a RegistryIndex,
    module_path: &str,
) -> Option<&'a RegistryEntry> {
    index
        .releases
        .iter()
        .filter(|entry| {
            !entry.yanked
                && entry.approved
                && version_matches_module_path(&entry.version, module_path)
        })
        .max_by(|a, b| compare_versions(&a.version, &b.version))
}

pub(super) fn default_registry_risk_baseline(
    index: &RegistryIndex,
    version: &str,
) -> Option<String> {
    let mut approved: Vec<&RegistryEntry> = index
        .releases
        .iter()
        .filter(|entry| entry.approved && entry.version != version)
        .collect();
    approved.sort_by(|a, b| compare_versions(&a.version, &b.version));
    approved.pop().map(|entry| entry.version.clone())
}

fn should_auto_approve_release(policy: &RegistryTrustPolicy, signer: Option<&str>) -> bool {
    if !policy.require_approval {
        return true;
    }
    let Some(signer) = signer.filter(|value| !value.trim().is_empty()) else {
        return false;
    };
    policy
        .auto_approve_signers
        .iter()
        .any(|value| value == signer)
}

pub(super) fn registry_query_matches(
    module_path: &str,
    index: &RegistryIndex,
    query: &str,
) -> bool {
    let query = query.to_ascii_lowercase();
    module_path.to_ascii_lowercase().contains(&query)
        || index
            .description
            .as_deref()
            .is_some_and(|text| text.to_ascii_lowercase().contains(&query))
        || index
            .license
            .as_deref()
            .is_some_and(|text| text.to_ascii_lowercase().contains(&query))
        || index
            .homepage
            .as_deref()
            .is_some_and(|text| text.to_ascii_lowercase().contains(&query))
}

pub(super) fn append_registry_audit_entry(
    registry_root: &Path,
    action: &str,
    detail: &str,
) -> Result<(), String> {
    let audit_path = registry_root.join("audit.log");
    let timestamp_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let mut existing = if audit_path.is_file() {
        fs::read_to_string(&audit_path)
            .map_err(|e| format!("failed to read '{}': {}", audit_path.display(), e))?
    } else {
        String::new()
    };
    existing.push_str(&format!(
        "{}\t{}\t{}\n",
        timestamp_secs,
        action,
        detail.replace('\n', " ")
    ));
    fs::write(&audit_path, existing)
        .map_err(|e| format!("failed to write '{}': {}", audit_path.display(), e))
}

pub(super) fn audit_entry_matches(
    entry: &RegistryAuditEntry,
    filter: &RegistryAuditFilter<'_>,
) -> bool {
    if let Some(action) = filter.action
        && entry.action != action
    {
        return false;
    }
    if let Some(module) = filter.module
        && !entry.detail.contains(&format!("module={module}"))
        && !entry.detail.contains(module)
    {
        return false;
    }
    if let Some(contains) = filter.contains
        && !entry.detail.contains(contains)
    {
        return false;
    }
    true
}

pub(super) fn mutate_registry_index(
    registry_spec: &Path,
    module_path: &str,
    commit_message: &str,
    update: impl FnOnce(&mut RegistryIndex) -> Result<(), String>,
) -> Result<(), String> {
    let (write_root, is_remote) = materialize_registry_write_root(registry_spec, false)?;
    let mut index = load_registry_index(&write_root, module_path)?;
    if index.module_path.is_empty() {
        index.module_path = module_path.to_string();
    }
    update(&mut index)?;
    write_registry_index(&write_root, module_path, &index)?;
    append_registry_audit_entry(
        &write_root,
        "registry-index",
        &format!("{} module={}", commit_message, module_path),
    )?;
    if is_remote {
        run_git(Some(&write_root), &["add", "."])?;
        let _ = run_git(Some(&write_root), &["commit", "-m", commit_message]);
        run_git(Some(&write_root), &["push", "origin", "HEAD"])?;
    }
    Ok(())
}
