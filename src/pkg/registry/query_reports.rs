use super::env::{registry_policy_override_path, registry_spec_from_override};
use super::primitives::*;
use super::publishing::{
    audit_entry_matches, default_registry_risk_baseline, latest_non_yanked_registry_release,
    load_all_registry_indices, mutate_registry_index, registry_query_matches,
};
use super::trust_policy::{mutate_registry_policy, parse_registry_trust_policy};
use super::util::{
    collect_file_map, escape_json, extract_registry_archive_to_temp, read_manifest_from_archive,
};
use super::*;
use ed25519_dalek::SigningKey;
use getrandom::getrandom;

pub fn search_registry_modules(
    query: &str,
    registry_spec: Option<&Path>,
) -> Result<Vec<RegistrySearchResult>, String> {
    let registry_root = registry_spec_from_override(registry_spec)?;
    let indices = load_all_registry_indices(&registry_root)?;
    let mut matches = Vec::new();
    for (module_path, index) in indices {
        if !registry_query_matches(&module_path, &index, query) {
            continue;
        }
        let latest_version = latest_non_yanked_registry_release(&index, &module_path)
            .map(|entry| entry.version.clone());
        let yanked_count = index.releases.iter().filter(|entry| entry.yanked).count();
        let pending_count = index
            .releases
            .iter()
            .filter(|entry| !entry.approved)
            .count();
        matches.push(RegistrySearchResult {
            path: module_path,
            latest_version,
            description: index.description.clone(),
            license: index.license.clone(),
            deprecated: index.deprecated.clone(),
            release_count: index.releases.len(),
            yanked_count,
            pending_count,
        });
    }
    matches.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(matches)
}

pub fn list_registry_modules(
    registry_spec: Option<&Path>,
) -> Result<Vec<RegistrySearchResult>, String> {
    let registry_root = registry_spec_from_override(registry_spec)?;
    let indices = load_all_registry_indices(&registry_root)?;
    let mut modules = Vec::new();
    for (module_path, index) in indices {
        let latest_version = latest_non_yanked_registry_release(&index, &module_path)
            .map(|entry| entry.version.clone());
        let yanked_count = index.releases.iter().filter(|entry| entry.yanked).count();
        let pending_count = index
            .releases
            .iter()
            .filter(|entry| !entry.approved)
            .count();
        modules.push(RegistrySearchResult {
            path: module_path,
            latest_version,
            description: index.description.clone(),
            license: index.license.clone(),
            deprecated: index.deprecated.clone(),
            release_count: index.releases.len(),
            yanked_count,
            pending_count,
        });
    }
    modules.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(modules)
}

pub fn set_registry_channel(
    module_path: &str,
    channel: &str,
    version: &str,
    registry_spec: &Path,
) -> Result<(), String> {
    let channel = normalize_registry_channel(channel)?;
    mutate_registry_index(
        registry_spec,
        module_path,
        &format!("set registry channel {} {}", module_path, channel),
        |index| {
            let Some(entry) = index.releases.iter().find(|entry| entry.version == version) else {
                return Err(format!(
                    "registry module '{}' does not contain version '{}'",
                    module_path, version
                ));
            };
            if entry.yanked {
                return Err(format!(
                    "cannot assign channel '{}' to yanked release '{} {}'",
                    channel, module_path, version
                ));
            }
            if !entry.approved {
                return Err(format!(
                    "cannot assign channel '{}' to unapproved release '{} {}'",
                    channel, module_path, version
                ));
            }
            index.channels.insert(channel.clone(), version.to_string());
            Ok(())
        },
    )
}

pub fn clear_registry_channel(
    module_path: &str,
    channel: &str,
    registry_spec: &Path,
) -> Result<(), String> {
    let channel = normalize_registry_channel(channel)?;
    mutate_registry_index(
        registry_spec,
        module_path,
        &format!("clear registry channel {} {}", module_path, channel),
        |index| {
            if index.channels.remove(&channel).is_none() {
                return Err(format!(
                    "registry module '{}' does not define channel '{}'",
                    module_path, channel
                ));
            }
            Ok(())
        },
    )
}

pub fn list_registry_queue(registry_spec: Option<&Path>) -> Result<Vec<RegistryQueueItem>, String> {
    let registry_root = registry_spec_from_override(registry_spec)?;
    let indices = load_all_registry_indices(&registry_root)?;
    let mut queue = Vec::new();
    for (module_path, index) in indices {
        for entry in index.releases {
            if entry.approved {
                continue;
            }
            queue.push(RegistryQueueItem {
                path: module_path.clone(),
                version: entry.version,
                yanked: entry.yanked,
                signed: entry.archive_sig.is_some(),
                signer: entry.signer,
            });
        }
    }
    queue.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| compare_versions(&a.version, &b.version))
    });
    Ok(queue)
}

pub fn read_registry_audit_log(
    registry_spec: Option<&Path>,
    limit: Option<usize>,
) -> Result<Vec<RegistryAuditEntry>, String> {
    read_registry_audit_log_filtered(registry_spec, limit, None, None, None)
}

pub fn read_registry_audit_log_filtered(
    registry_spec: Option<&Path>,
    limit: Option<usize>,
    action: Option<&str>,
    module: Option<&str>,
    contains: Option<&str>,
) -> Result<Vec<RegistryAuditEntry>, String> {
    let registry_root = registry_spec_from_override(registry_spec)?;
    let audit_path = registry_root.join("audit.log");
    if !audit_path.is_file() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&audit_path)
        .map_err(|e| format!("failed to read '{}': {}", audit_path.display(), e))?;
    let filter = RegistryAuditFilter {
        action,
        module,
        contains,
    };
    let mut entries = Vec::new();
    for line in content.lines() {
        let mut parts = line.splitn(3, '\t');
        let timestamp_secs = parts
            .next()
            .and_then(|raw| raw.parse::<u64>().ok())
            .unwrap_or(0);
        let action = parts.next().unwrap_or_default().to_string();
        let detail = parts.next().unwrap_or_default().to_string();
        let entry = RegistryAuditEntry {
            timestamp_secs,
            action,
            detail,
        };
        if audit_entry_matches(&entry, &filter) {
            entries.push(entry);
        }
    }
    if let Some(limit) = limit
        && entries.len() > limit
    {
        entries = entries.split_off(entries.len() - limit);
    }
    Ok(entries)
}

pub fn export_registry_audit_log(
    registry_spec: Option<&Path>,
    output_path: &Path,
    format: &str,
    limit: Option<usize>,
    action: Option<&str>,
    module: Option<&str>,
    contains: Option<&str>,
) -> Result<usize, String> {
    let entries = read_registry_audit_log_filtered(registry_spec, limit, action, module, contains)?;
    let mut rendered = String::new();
    match format {
        "tsv" => {
            for entry in &entries {
                rendered.push_str(&format!(
                    "{}\t{}\t{}\n",
                    entry.timestamp_secs,
                    entry.action,
                    entry.detail.replace('\n', " ")
                ));
            }
        }
        "jsonl" => {
            for entry in &entries {
                rendered.push_str(&format!(
                    "{{\"timestamp_secs\":{},\"action\":\"{}\",\"detail\":\"{}\"}}\n",
                    entry.timestamp_secs,
                    escape_json(&entry.action),
                    escape_json(&entry.detail)
                ));
            }
        }
        other => {
            return Err(format!(
                "unsupported audit export format '{}'; expected 'tsv' or 'jsonl'",
                other
            ));
        }
    }
    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create '{}': {}", parent.display(), e))?;
    }
    fs::write(output_path, rendered)
        .map_err(|e| format!("failed to write '{}': {}", output_path.display(), e))?;
    Ok(entries.len())
}

pub fn registry_report(
    registry_spec: Option<&Path>,
    module_path: Option<&str>,
) -> Result<RegistryReport, String> {
    let registry_root = registry_spec_from_override(registry_spec)?;
    let modules: Vec<(String, RegistryIndex)> = if let Some(module_path) = module_path {
        vec![(
            module_path.to_string(),
            load_registry_index(&registry_root, module_path)?,
        )]
    } else {
        load_all_registry_indices(&registry_root)?
    };

    let mut report_modules = Vec::new();
    let mut total_release_count = 0usize;
    let mut total_channel_count = 0usize;
    let mut total_approved = 0usize;
    let mut total_pending = 0usize;
    let mut total_yanked = 0usize;
    let mut total_signed = 0usize;
    let mut deprecated_module_count = 0usize;

    for (module_path, index) in modules {
        let release_count = index.releases.len();
        let channel_count = index.channels.len();
        let approved_count = index.releases.iter().filter(|entry| entry.approved).count();
        let pending_count = index
            .releases
            .iter()
            .filter(|entry| !entry.approved)
            .count();
        let yanked_count = index.releases.iter().filter(|entry| entry.yanked).count();
        let signed_count = index
            .releases
            .iter()
            .filter(|entry| entry.archive_sig.is_some())
            .count();
        let latest_version = latest_non_yanked_registry_release(&index, &module_path)
            .map(|entry| entry.version.clone());
        let deprecated = index.deprecated.is_some();

        total_release_count += release_count;
        total_channel_count += channel_count;
        total_approved += approved_count;
        total_pending += pending_count;
        total_yanked += yanked_count;
        total_signed += signed_count;
        if deprecated {
            deprecated_module_count += 1;
        }

        report_modules.push(RegistryReportModule {
            path: module_path,
            latest_version,
            channel_count,
            release_count,
            approved_count,
            pending_count,
            yanked_count,
            signed_count,
            deprecated,
        });
    }

    report_modules.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(RegistryReport {
        module_count: report_modules.len(),
        channel_count: total_channel_count,
        release_count: total_release_count,
        approved_count: total_approved,
        pending_count: total_pending,
        yanked_count: total_yanked,
        signed_count: total_signed,
        deprecated_module_count,
        modules: report_modules,
    })
}

pub fn registry_diff(
    registry_spec: Option<&Path>,
    module_path: &str,
    from_version: &str,
    to_version: &str,
) -> Result<RegistryDiffReport, String> {
    let registry_root = registry_spec_from_override(registry_spec)?;
    let index = load_registry_index(&registry_root, module_path)?;
    registry_diff_from_index(
        &registry_root,
        &index,
        module_path,
        from_version,
        to_version,
    )
}

pub fn registry_risk(
    registry_spec: Option<&Path>,
    module_path: &str,
    version: &str,
    against: Option<&str>,
) -> Result<RegistryRiskReport, String> {
    let registry_root = registry_spec_from_override(registry_spec)?;
    let index = load_registry_index(&registry_root, module_path)?;
    let Some(release) = index.releases.iter().find(|entry| entry.version == version) else {
        return Err(format!(
            "registry module '{}' does not contain version '{}'",
            module_path, version
        ));
    };

    let mut factors = Vec::new();
    if release.yanked {
        factors.push(RegistryRiskFactor {
            key: "yanked".to_string(),
            points: 6,
            detail: "release is yanked".to_string(),
        });
    }
    if !release.approved {
        factors.push(RegistryRiskFactor {
            key: "pending-approval".to_string(),
            points: 4,
            detail: "release is pending approval".to_string(),
        });
    }
    if release.archive_sig.is_none() {
        factors.push(RegistryRiskFactor {
            key: "unsigned".to_string(),
            points: 3,
            detail: "release is unsigned".to_string(),
        });
    }
    if let Some(message) = &index.deprecated {
        factors.push(RegistryRiskFactor {
            key: "deprecated-module".to_string(),
            points: 2,
            detail: format!("module is deprecated: {}", message),
        });
    }

    let baseline_version = against
        .map(ToOwned::to_owned)
        .or_else(|| default_registry_risk_baseline(&index, version));
    if let Some(baseline_version) = &baseline_version {
        let diff = registry_diff_from_index(
            &registry_root,
            &index,
            module_path,
            baseline_version,
            version,
        )?;
        let changed_total =
            diff.added_files.len() + diff.removed_files.len() + diff.changed_files.len();
        if changed_total >= 10 {
            factors.push(RegistryRiskFactor {
                key: "large-change".to_string(),
                points: 3,
                detail: format!("{} files differ from {}", changed_total, baseline_version),
            });
        } else if changed_total >= 2 {
            factors.push(RegistryRiskFactor {
                key: "multi-file-change".to_string(),
                points: 1,
                detail: format!("{} files differ from {}", changed_total, baseline_version),
            });
        }
        if diff.from_signer != diff.to_signer {
            factors.push(RegistryRiskFactor {
                key: "signer-change".to_string(),
                points: 2,
                detail: format!(
                    "signer changed from {} to {}",
                    diff.from_signer.as_deref().unwrap_or("none"),
                    diff.to_signer.as_deref().unwrap_or("none")
                ),
            });
        }
        if diff.from_signed != diff.to_signed {
            factors.push(RegistryRiskFactor {
                key: "signature-state-change".to_string(),
                points: 2,
                detail: format!(
                    "signature state changed from {} to {}",
                    if diff.from_signed {
                        "signed"
                    } else {
                        "unsigned"
                    },
                    if diff.to_signed { "signed" } else { "unsigned" }
                ),
            });
        }
    }

    let score = factors.iter().map(|factor| factor.points).sum();
    let level = if score >= 8 {
        "high"
    } else if score >= 4 {
        "medium"
    } else {
        "low"
    };

    Ok(RegistryRiskReport {
        module_path: module_path.to_string(),
        version: version.to_string(),
        baseline_version,
        score,
        level: level.to_string(),
        factors,
    })
}

fn registry_diff_from_index(
    registry_root: &Path,
    index: &RegistryIndex,
    module_path: &str,
    from_version: &str,
    to_version: &str,
) -> Result<RegistryDiffReport, String> {
    let from_entry = index
        .releases
        .iter()
        .find(|entry| entry.version == from_version)
        .ok_or_else(|| {
            format!(
                "registry module '{}' does not contain version '{}'",
                module_path, from_version
            )
        })?;
    let to_entry = index
        .releases
        .iter()
        .find(|entry| entry.version == to_version)
        .ok_or_else(|| {
            format!(
                "registry module '{}' does not contain version '{}'",
                module_path, to_version
            )
        })?;

    let from_archive = registry_root.join(&from_entry.archive_rel);
    let to_archive = registry_root.join(&to_entry.archive_rel);
    if !from_archive.is_file() {
        return Err(format!(
            "registry archive '{}' is missing",
            from_archive.display()
        ));
    }
    if !to_archive.is_file() {
        return Err(format!(
            "registry archive '{}' is missing",
            to_archive.display()
        ));
    }

    let from_root = extract_registry_archive_to_temp(&from_archive, "rr-registry-diff-from")?;
    let to_root = extract_registry_archive_to_temp(&to_archive, "rr-registry-diff-to")?;
    let from_files = collect_file_map(&from_root)?;
    let to_files = collect_file_map(&to_root)?;
    let _ = fs::remove_dir_all(&from_root);
    let _ = fs::remove_dir_all(&to_root);

    let mut added_files = Vec::new();
    let mut removed_files = Vec::new();
    let mut changed_files = Vec::new();

    for path in from_files.keys() {
        if !to_files.contains_key(path) {
            removed_files.push(path.clone());
        }
    }
    for path in to_files.keys() {
        match from_files.get(path) {
            None => added_files.push(path.clone()),
            Some(old) if old != &to_files[path] => changed_files.push(path.clone()),
            Some(_) => {}
        }
    }

    added_files.sort();
    removed_files.sort();
    changed_files.sort();

    Ok(RegistryDiffReport {
        module_path: module_path.to_string(),
        from_version: from_version.to_string(),
        to_version: to_version.to_string(),
        from_approved: from_entry.approved,
        to_approved: to_entry.approved,
        from_yanked: from_entry.yanked,
        to_yanked: to_entry.yanked,
        from_signed: from_entry.archive_sig.is_some(),
        to_signed: to_entry.archive_sig.is_some(),
        from_signer: from_entry.signer.clone(),
        to_signer: to_entry.signer.clone(),
        added_files,
        removed_files,
        changed_files,
    })
}

pub fn show_registry_policy(
    registry_spec: Option<&Path>,
) -> Result<RegistryPolicyShowReport, String> {
    let registry_root = registry_spec_from_override(registry_spec)?;
    let path = registry_policy_override_path().unwrap_or_else(|| registry_root.join("policy.toml"));
    let exists = path.is_file();
    let content = if exists {
        fs::read_to_string(&path)
            .map_err(|e| format!("failed to read '{}': {}", path.display(), e))?
    } else {
        String::new()
    };
    Ok(RegistryPolicyShowReport {
        path,
        exists,
        content,
    })
}

pub fn registry_module_info(
    module_path: &str,
    registry_spec: Option<&Path>,
) -> Result<RegistryInfo, String> {
    let registry_root = registry_spec_from_override(registry_spec)?;
    let index = load_registry_index(&registry_root, module_path)?;
    let mut releases = index
        .releases
        .into_iter()
        .map(|entry| RegistryReleaseInfo {
            version: entry.version,
            archive_rel: entry.archive_rel,
            archive_sum: entry.archive_sum,
            file_count: entry.file_count,
            yanked: entry.yanked,
            approved: entry.approved,
            signed: entry.archive_sig.is_some(),
            signer: entry.signer,
            signature_scheme: entry.archive_sig.as_deref().and_then(signature_scheme_name),
        })
        .collect::<Vec<_>>();
    releases.sort_by(|a, b| compare_versions(&a.version, &b.version));
    Ok(RegistryInfo {
        path: module_path.to_string(),
        description: index.description,
        license: index.license,
        homepage: index.homepage,
        deprecated: index.deprecated,
        channels: index.channels,
        releases,
    })
}

pub fn yank_registry_release(
    module_path: &str,
    version: &str,
    registry_spec: &Path,
) -> Result<(), String> {
    mutate_registry_index(
        registry_spec,
        module_path,
        &format!("yank registry release {} {}", module_path, version),
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
            entry.yanked = true;
            index.channels.retain(|_, assigned| assigned != version);
            Ok(())
        },
    )
}

pub fn unyank_registry_release(
    module_path: &str,
    version: &str,
    registry_spec: &Path,
) -> Result<(), String> {
    mutate_registry_index(
        registry_spec,
        module_path,
        &format!("unyank registry release {} {}", module_path, version),
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
            entry.yanked = false;
            Ok(())
        },
    )
}

pub fn deprecate_registry_module(
    module_path: &str,
    message: &str,
    registry_spec: &Path,
) -> Result<(), String> {
    mutate_registry_index(
        registry_spec,
        module_path,
        &format!("deprecate registry module {}", module_path),
        |index| {
            index.deprecated = Some(message.to_string());
            Ok(())
        },
    )
}

pub fn undeprecate_registry_module(module_path: &str, registry_spec: &Path) -> Result<(), String> {
    mutate_registry_index(
        registry_spec,
        module_path,
        &format!("undeprecate registry module {}", module_path),
        |index| {
            index.deprecated = None;
            Ok(())
        },
    )
}

pub fn verify_registry(
    registry_spec: Option<&Path>,
    only_module: Option<&str>,
) -> Result<RegistryVerifyReport, String> {
    let registry_root = registry_spec_from_override(registry_spec)?;
    let modules: Vec<(String, RegistryIndex)> = if let Some(module_path) = only_module {
        vec![(
            module_path.to_string(),
            load_registry_index(&registry_root, module_path)?,
        )]
    } else {
        load_all_registry_indices(&registry_root)?
    };

    let mut issues = Vec::new();
    let mut checked_releases = 0usize;
    for (module_path, index) in &modules {
        for entry in &index.releases {
            checked_releases += 1;
            let archive_path = registry_root.join(&entry.archive_rel);
            if !archive_path.is_file() {
                issues.push(RegistryVerifyIssue {
                    path: module_path.clone(),
                    version: entry.version.clone(),
                    archive_path: archive_path.clone(),
                    message: "archive is missing".to_string(),
                });
                continue;
            }

            match archive_checksum(&archive_path) {
                Ok(sum) if sum != entry.archive_sum => {
                    issues.push(RegistryVerifyIssue {
                        path: module_path.clone(),
                        version: entry.version.clone(),
                        archive_path: archive_path.clone(),
                        message: format!(
                            "checksum mismatch: expected {}, got {}",
                            entry.archive_sum, sum
                        ),
                    });
                    continue;
                }
                Err(err) => {
                    issues.push(RegistryVerifyIssue {
                        path: module_path.clone(),
                        version: entry.version.clone(),
                        archive_path: archive_path.clone(),
                        message: err,
                    });
                    continue;
                }
                Ok(_) => {}
            }

            match read_manifest_from_archive(&archive_path) {
                Ok(manifest) if manifest.module_path != *module_path => {
                    issues.push(RegistryVerifyIssue {
                        path: module_path.clone(),
                        version: entry.version.clone(),
                        archive_path: archive_path.clone(),
                        message: format!("archive declares module '{}'", manifest.module_path),
                    });
                }
                Err(err) => issues.push(RegistryVerifyIssue {
                    path: module_path.clone(),
                    version: entry.version.clone(),
                    archive_path: archive_path.clone(),
                    message: err,
                }),
                Ok(_) => {}
            }

            if let Ok(policy) = load_registry_trust_policy(&registry_root)
                && let Err(err) = verify_registry_release_trust(
                    module_path,
                    &entry.version,
                    &entry.archive_sum,
                    entry.approved,
                    entry.archive_sig.as_deref(),
                    entry.signer.as_deref(),
                    &policy,
                )
            {
                issues.push(RegistryVerifyIssue {
                    path: module_path.clone(),
                    version: entry.version.clone(),
                    archive_path: archive_path.clone(),
                    message: err,
                });
            }
        }
    }

    Ok(RegistryVerifyReport {
        checked_modules: modules.len(),
        checked_releases,
        issues,
    })
}

pub fn generate_registry_keypair(
    out_dir: Option<&Path>,
    identity: Option<&str>,
) -> Result<RegistryKeygenReport, String> {
    let mut secret = [0u8; 32];
    getrandom(&mut secret).map_err(|e| format!("failed to generate registry keypair: {}", e))?;
    let signing_key = SigningKey::from_bytes(&secret);
    let secret_key_hex = hex_encode(&secret);
    let public_key_hex = hex_encode(signing_key.verifying_key().as_bytes());

    let mut written_files = Vec::new();
    if let Some(out_dir) = out_dir {
        fs::create_dir_all(out_dir)
            .map_err(|e| format!("failed to create '{}': {}", out_dir.display(), e))?;

        let secret_path = out_dir.join("registry-ed25519-secret.key");
        fs::write(&secret_path, format!("{}\n", secret_key_hex))
            .map_err(|e| format!("failed to write '{}': {}", secret_path.display(), e))?;
        written_files.push(secret_path);

        let public_path = out_dir.join("registry-ed25519-public.key");
        fs::write(&public_path, format!("{}\n", public_key_hex))
            .map_err(|e| format!("failed to write '{}': {}", public_path.display(), e))?;
        written_files.push(public_path);

        let mut env_file = String::new();
        env_file.push_str(&format!(
            "export RR_REGISTRY_SIGNING_ED25519_SECRET={}\n",
            secret_key_hex
        ));
        if let Some(identity) = identity {
            env_file.push_str(&format!(
                "export RR_REGISTRY_SIGNING_IDENTITY={}\n",
                identity
            ));
        }
        env_file.push_str(&format!(
            "export RR_REGISTRY_TRUST_ED25519_KEYS={}\n",
            public_key_hex
        ));
        let env_path = out_dir.join("registry-signing.env");
        fs::write(&env_path, env_file)
            .map_err(|e| format!("failed to write '{}': {}", env_path.display(), e))?;
        written_files.push(env_path);
    }

    Ok(RegistryKeygenReport {
        public_key_hex,
        secret_key_hex,
        identity: identity.map(ToOwned::to_owned),
        written_files,
    })
}

pub fn lint_registry_policy(
    registry_spec: Option<&Path>,
) -> Result<RegistryPolicyLintReport, String> {
    let registry_root = registry_spec_from_override(registry_spec)?;
    let path = registry_policy_override_path().unwrap_or_else(|| registry_root.join("policy.toml"));
    let exists = path.is_file();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let policy = if exists {
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("failed to read '{}': {}", path.display(), e))?;
        match parse_registry_trust_policy(&content) {
            Ok(policy) => policy,
            Err(err) => {
                errors.push(err);
                RegistryTrustPolicy::default()
            }
        }
    } else {
        warnings.push("policy.toml does not exist".to_string());
        RegistryTrustPolicy::default()
    };

    validate_unique_hex_keys(
        &policy.trusted_ed25519_keys,
        "trusted_ed25519",
        &mut warnings,
        &mut errors,
    );
    validate_unique_hex_keys(
        &policy.revoked_ed25519_keys,
        "revoked_ed25519",
        &mut warnings,
        &mut errors,
    );
    validate_unique_strings(&policy.allowed_signers, "allowed_signer", &mut warnings);
    validate_unique_strings(
        &policy.auto_approve_signers,
        "auto_approve_signer",
        &mut warnings,
    );

    for revoked in &policy.revoked_ed25519_keys {
        if let Ok(normalized) = normalize_ed25519_public_key(revoked)
            && policy
                .trusted_ed25519_keys
                .iter()
                .any(|key| key == &normalized)
        {
            errors.push(format!(
                "key {} appears in both trusted_ed25519 and revoked_ed25519",
                normalized
            ));
        }
    }

    Ok(RegistryPolicyLintReport {
        path,
        exists,
        require_signed: policy.require_signed,
        require_approval: policy.require_approval,
        trusted_count: policy.trusted_ed25519_keys.len(),
        revoked_count: policy.revoked_ed25519_keys.len(),
        allowed_signer_count: policy.allowed_signers.len(),
        auto_approve_signer_count: policy.auto_approve_signers.len(),
        warnings,
        errors,
    })
}

pub fn rotate_registry_policy_key(
    old_public_key: &str,
    new_public_key: &str,
    registry_spec: &Path,
) -> Result<PathBuf, String> {
    let old_key = normalize_ed25519_public_key(old_public_key)?;
    let new_key = normalize_ed25519_public_key(new_public_key)?;
    if old_key == new_key {
        return Err("old and new public keys are identical".to_string());
    }
    mutate_registry_policy(registry_spec, "rotate registry signing key", |policy| {
        if !policy
            .trusted_ed25519_keys
            .iter()
            .any(|key| key == &old_key)
        {
            return Err(format!("old public key '{}' is not trusted", old_key));
        }
        if !policy
            .trusted_ed25519_keys
            .iter()
            .any(|key| key == &new_key)
        {
            policy.trusted_ed25519_keys.push(new_key.clone());
        }
        policy.trusted_ed25519_keys.retain(|key| key != &old_key);
        if !policy
            .revoked_ed25519_keys
            .iter()
            .any(|key| key == &old_key)
        {
            policy.revoked_ed25519_keys.push(old_key.clone());
        }
        Ok(())
    })
}
