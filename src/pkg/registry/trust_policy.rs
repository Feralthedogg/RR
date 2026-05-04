use super::git::run_git;
use super::manifest::{escape_toml, parse_toml_string};
use super::publishing::{append_registry_audit_entry, materialize_registry_write_root};
use super::*;

pub(super) fn parse_registry_trust_policy(content: &str) -> Result<RegistryTrustPolicy, String> {
    let mut policy = RegistryTrustPolicy::default();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "version" => {}
            "require_signed" => policy.require_signed = value == "true",
            "require_approval" => policy.require_approval = value == "true",
            "trusted_ed25519" => policy
                .trusted_ed25519_keys
                .push(parse_toml_string(value)?.to_ascii_lowercase()),
            "revoked_ed25519" => policy
                .revoked_ed25519_keys
                .push(parse_toml_string(value)?.to_ascii_lowercase()),
            "allowed_signer" => policy.allowed_signers.push(parse_toml_string(value)?),
            "auto_approve_signer" => policy.auto_approve_signers.push(parse_toml_string(value)?),
            _ => {}
        }
    }
    Ok(policy)
}

fn render_registry_trust_policy(policy: &RegistryTrustPolicy) -> String {
    let mut out = String::new();
    out.push_str("version = 1\n");
    out.push_str(&format!(
        "require_signed = {}\n",
        if policy.require_signed {
            "true"
        } else {
            "false"
        }
    ));
    out.push_str(&format!(
        "require_approval = {}\n",
        if policy.require_approval {
            "true"
        } else {
            "false"
        }
    ));

    let mut trusted = policy.trusted_ed25519_keys.clone();
    trusted.sort();
    trusted.dedup();
    for key in trusted {
        out.push_str(&format!("trusted_ed25519 = \"{}\"\n", escape_toml(&key)));
    }

    let mut revoked = policy.revoked_ed25519_keys.clone();
    revoked.sort();
    revoked.dedup();
    for key in revoked {
        out.push_str(&format!("revoked_ed25519 = \"{}\"\n", escape_toml(&key)));
    }

    let mut signers = policy.allowed_signers.clone();
    signers.sort();
    signers.dedup();
    for signer in signers {
        out.push_str(&format!("allowed_signer = \"{}\"\n", escape_toml(&signer)));
    }

    let mut auto_signers = policy.auto_approve_signers.clone();
    auto_signers.sort();
    auto_signers.dedup();
    for signer in auto_signers {
        out.push_str(&format!(
            "auto_approve_signer = \"{}\"\n",
            escape_toml(&signer)
        ));
    }

    out
}

pub(super) fn mutate_registry_policy(
    registry_spec: &Path,
    commit_message: &str,
    update: impl FnOnce(&mut RegistryTrustPolicy) -> Result<(), String>,
) -> Result<PathBuf, String> {
    let (write_root, is_remote) = materialize_registry_write_root(registry_spec, false)?;
    let policy_path = write_root.join("policy.toml");
    let mut policy = if policy_path.is_file() {
        let content = fs::read_to_string(&policy_path)
            .map_err(|e| format!("failed to read '{}': {}", policy_path.display(), e))?;
        parse_registry_trust_policy(&content)
            .map_err(|e| format!("failed to parse '{}': {}", policy_path.display(), e))?
    } else {
        RegistryTrustPolicy::default()
    };

    update(&mut policy)?;
    fs::write(&policy_path, render_registry_trust_policy(&policy))
        .map_err(|e| format!("failed to write '{}': {}", policy_path.display(), e))?;
    append_registry_audit_entry(
        &write_root,
        "registry-policy",
        &format!("{} path={}", commit_message, policy_path.display()),
    )?;

    if is_remote {
        run_git(Some(&write_root), &["add", "."])?;
        let _ = run_git(Some(&write_root), &["commit", "-m", commit_message]);
        run_git(Some(&write_root), &["push", "origin", "HEAD"])?;
    }

    Ok(policy_path)
}
