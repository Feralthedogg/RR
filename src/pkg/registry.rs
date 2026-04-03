use super::env::{
    registry_policy_override_path, registry_signing_ed25519_secret, registry_signing_identity,
    registry_signing_key, registry_spec_from_override, registry_trust_ed25519_keys,
    registry_trust_key,
};
use super::git::{
    create_git_tag, ensure_git_identity, git_repo_is_dirty, git_tag_exists, push_git_tag,
};
use super::manifest::escape_toml;
use super::util::{
    collect_file_map, collect_publishable_files, escape_json, extract_registry_archive_to_temp,
    module_path_to_rel_path, project_dir_name_from_module_path, read_manifest_from_archive,
    stable_hash_update,
};
use super::*;

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

pub fn bootstrap_registry_policy(
    trusted_public_key: &str,
    signer: Option<&str>,
    auto_approve_signer: Option<&str>,
    require_signed: bool,
    require_approval: bool,
    registry_spec: &Path,
) -> Result<PathBuf, String> {
    let trusted_key = normalize_ed25519_public_key(trusted_public_key)?;
    mutate_registry_policy(registry_spec, "bootstrap registry policy", |policy| {
        policy.require_signed = require_signed;
        policy.require_approval = require_approval;
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
    require_signed: bool,
    require_approval: bool,
    auto_approve: bool,
) -> Result<RegistryOnboardReport, String> {
    let keygen = generate_registry_keypair(out_dir, identity)?;
    let signer = identity.filter(|value| !value.trim().is_empty());
    let policy_path = bootstrap_registry_policy(
        &keygen.public_key_hex,
        signer,
        if auto_approve { signer } else { None },
        require_signed,
        require_approval,
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

pub(super) fn load_registry_index(
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

pub(super) fn latest_registry_version(
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

pub(super) fn registry_channel_version(
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

pub(super) fn load_registry_trust_policy(
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

pub(super) fn verify_registry_release_trust(
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

pub(super) fn materialize_registry_read_root(spec: &Path) -> Result<PathBuf, String> {
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

fn materialize_registry_write_root(spec: &Path, dry_run: bool) -> Result<(PathBuf, bool), String> {
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

fn load_all_registry_indices(registry_root: &Path) -> Result<Vec<(String, RegistryIndex)>, String> {
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

fn latest_non_yanked_registry_release<'a>(
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

fn default_registry_risk_baseline(index: &RegistryIndex, version: &str) -> Option<String> {
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

fn registry_query_matches(module_path: &str, index: &RegistryIndex, query: &str) -> bool {
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

fn append_registry_audit_entry(
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

fn audit_entry_matches(entry: &RegistryAuditEntry, filter: &RegistryAuditFilter<'_>) -> bool {
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

fn mutate_registry_index(
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

fn sign_registry_release(
    module_path: &str,
    version: &str,
    archive_sum: &str,
) -> Result<(Option<String>, Option<String>), String> {
    let payload = registry_signature_payload(module_path, version, archive_sum);
    if let Some(secret) = registry_signing_ed25519_secret() {
        let signing_key = load_ed25519_signing_key(&secret)?;
        let public_key = hex_encode(signing_key.verifying_key().as_bytes());
        let signature = signing_key.sign(payload.as_bytes());
        let sig = format!("ed25519:{public_key}:{}", hex_encode(&signature.to_bytes()));
        return Ok((Some(sig), registry_signing_identity()));
    }
    if let Some(key) = registry_signing_key() {
        let digest = hmac_sha256_hex(&key, payload.as_bytes())?;
        return Ok((
            Some(format!("hmac-sha256:{digest}")),
            registry_signing_identity(),
        ));
    }
    Ok((None, None))
}

fn verify_registry_signer_policy(
    module_path: &str,
    version: &str,
    signer: Option<&str>,
    policy: &RegistryTrustPolicy,
) -> Result<(), String> {
    if policy.allowed_signers.is_empty() {
        return Ok(());
    }
    let Some(signer) = signer.filter(|value| !value.trim().is_empty()) else {
        return Err(format!(
            "registry policy requires an allowed signer for '{} {}'",
            module_path, version
        ));
    };
    if policy
        .allowed_signers
        .iter()
        .any(|allowed| allowed == signer)
    {
        Ok(())
    } else {
        Err(format!(
            "registry signer '{}' is not allowed for '{} {}'",
            signer, module_path, version
        ))
    }
}

fn resolved_registry_trust_ed25519_keys(
    policy: &RegistryTrustPolicy,
) -> Result<Vec<String>, String> {
    let mut keys = registry_trust_ed25519_keys();
    for key in &policy.trusted_ed25519_keys {
        if !keys.iter().any(|entry| entry == key) {
            keys.push(key.clone());
        }
    }
    if let Some(secret) = registry_signing_ed25519_secret() {
        let signing_key = load_ed25519_signing_key(&secret)?;
        let public = hex_encode(signing_key.verifying_key().as_bytes());
        if !keys.iter().any(|entry| entry == &public) {
            keys.push(public);
        }
    }
    Ok(keys)
}

fn load_ed25519_signing_key(secret_hex: &str) -> Result<SigningKey, String> {
    let secret_bytes = hex_decode(secret_hex)?;
    let secret_arr: [u8; 32] = secret_bytes
        .try_into()
        .map_err(|_| "RR_REGISTRY_SIGNING_ED25519_SECRET must be 32 bytes of hex".to_string())?;
    Ok(SigningKey::from_bytes(&secret_arr))
}

fn registry_signature_payload(module_path: &str, version: &str, archive_sum: &str) -> String {
    format!("module={module_path}\nversion={version}\nsum={archive_sum}\n")
}

fn parse_registry_trust_policy(content: &str) -> Result<RegistryTrustPolicy, String> {
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

fn mutate_registry_policy(
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

fn validate_unique_hex_keys(
    values: &[String],
    label: &str,
    warnings: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    let mut seen = std::collections::BTreeSet::new();
    for value in values {
        match normalize_ed25519_public_key(value) {
            Ok(normalized) => {
                if !seen.insert(normalized.clone()) {
                    warnings.push(format!("duplicate {} entry: {}", label, normalized));
                }
            }
            Err(message) => {
                errors.push(format!("invalid {} entry '{}': {}", label, value, message))
            }
        }
    }
}

fn validate_unique_strings(values: &[String], label: &str, warnings: &mut Vec<String>) {
    let mut seen = std::collections::BTreeSet::new();
    for value in values {
        let normalized = value.trim();
        if !seen.insert(normalized.to_string()) {
            warnings.push(format!("duplicate {} entry: {}", label, normalized));
        }
    }
}

fn normalize_ed25519_public_key(raw: &str) -> Result<String, String> {
    let decoded = hex_decode(raw)?;
    let arr: [u8; 32] = decoded
        .try_into()
        .map_err(|_| "ed25519 public key must be 32 bytes of hex".to_string())?;
    Ok(hex_encode(&arr))
}

fn normalize_registry_channel(raw: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err("channel name must be non-empty".to_string());
    }
    if value == "latest" {
        return Err("channel name 'latest' is reserved".to_string());
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(format!(
            "invalid channel name '{}': use only letters, digits, '.', '_' or '-'",
            value
        ));
    }
    Ok(value.to_string())
}

fn hmac_sha256_hex(key: &str, payload: &[u8]) -> Result<String, String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes())
        .map_err(|e| format!("failed to initialize registry signing key: {}", e))?;
    mac.update(payload);
    let bytes = mac.finalize().into_bytes();
    Ok(hex_encode(bytes.as_slice()))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn hex_decode(raw: &str) -> Result<Vec<u8>, String> {
    let raw = raw.trim();
    if !raw.len().is_multiple_of(2) {
        return Err(format!("invalid hex string length: {}", raw.len()));
    }
    let mut out = Vec::with_capacity(raw.len() / 2);
    let bytes = raw.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() {
        let hi = decode_hex_nibble(bytes[idx])?;
        let lo = decode_hex_nibble(bytes[idx + 1])?;
        out.push((hi << 4) | lo);
        idx += 2;
    }
    Ok(out)
}

fn decode_hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid hex digit '{}'", byte as char)),
    }
}

fn signature_scheme_name(signature: &str) -> Option<String> {
    signature.split(':').next().map(ToOwned::to_owned)
}
