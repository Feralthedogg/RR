use super::*;
#[path = "main_registry_admin.rs"]
mod main_registry_admin;

pub(crate) use self::main_registry_admin::*;

fn parse_registry_override_args(
    args: &[String],
    usage: &str,
    ui: &CliLog,
) -> Result<(Vec<String>, Option<PathBuf>), i32> {
    let mut positional = Vec::new();
    let mut registry = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--registry" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --registry");
                    ui.warn(usage);
                    return Err(1);
                }
                i += 1;
                registry = Some(PathBuf::from(&args[i]));
            }
            arg if arg.starts_with('-') => {
                ui.error(&format!("Unknown option: {}", arg));
                ui.warn(usage);
                return Err(1);
            }
            _ => positional.push(args[i].clone()),
        }
        i += 1;
    }
    Ok((positional, registry))
}

pub(crate) fn cmd_search(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let Ok((positional, registry)) =
        parse_registry_override_args(args, "use RR search <query> [--registry <dir>]", &ui)
    else {
        return 1;
    };
    if positional.len() != 1 {
        ui.error("RR search expects exactly one query");
        ui.warn("use RR search <query> [--registry <dir>]");
        return 1;
    }
    match RR::pkg::search_registry_modules(&positional[0], registry.as_deref()) {
        Ok(results) => {
            if results.is_empty() {
                ui.success("No registry modules matched the query");
                return 0;
            }
            for result in results {
                let latest = result.latest_version.unwrap_or_else(|| "-".to_string());
                let description = result
                    .description
                    .as_deref()
                    .filter(|text| !text.is_empty())
                    .unwrap_or("-");
                let license = result
                    .license
                    .as_deref()
                    .filter(|text| !text.is_empty())
                    .unwrap_or("-");
                let deprecated = result
                    .deprecated
                    .as_deref()
                    .filter(|text| !text.is_empty())
                    .unwrap_or("-");
                ui.success(&format!(
                    "{} latest={} releases={} yanked={} license={} deprecated={} desc={}",
                    result.path,
                    latest,
                    result.release_count,
                    result.yanked_count,
                    license,
                    deprecated,
                    description
                ));
            }
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}

fn print_registry_search_result(result: RR::pkg::RegistrySearchResult) {
    let latest = result.latest_version.unwrap_or_else(|| "-".to_string());
    let description = result
        .description
        .as_deref()
        .filter(|text| !text.is_empty())
        .unwrap_or("-");
    let license = result
        .license
        .as_deref()
        .filter(|text| !text.is_empty())
        .unwrap_or("-");
    let deprecated = result
        .deprecated
        .as_deref()
        .filter(|text| !text.is_empty())
        .unwrap_or("-");
    println!(
        "{} latest={} releases={} pending={} yanked={} license={} deprecated={} desc={}",
        result.path,
        latest,
        result.release_count,
        result.pending_count,
        result.yanked_count,
        license,
        deprecated,
        description
    );
}

pub(crate) fn cmd_registry(args: &[String]) -> i32 {
    let ui = CliLog::new();
    if matches!(args.first().map(String::as_str), Some("keygen")) {
        return cmd_registry_keygen(&args[1..]);
    }
    if matches!(args.first().map(String::as_str), Some("onboard")) {
        return cmd_registry_onboard(&args[1..]);
    }
    if matches!(args.first().map(String::as_str), Some("risk")) {
        return cmd_registry_risk(&args[1..]);
    }
    if matches!(args.first().map(String::as_str), Some("audit")) {
        return cmd_registry_audit(&args[1..]);
    }
    if matches!(args.first().map(String::as_str), Some("policy"))
        && matches!(args.get(1).map(String::as_str), Some("bootstrap"))
    {
        return cmd_registry_policy_bootstrap(&args[2..]);
    }
    if matches!(args.first().map(String::as_str), Some("policy"))
        && matches!(args.get(1).map(String::as_str), Some("show"))
    {
        return cmd_registry_policy_show(&args[2..]);
    }
    if matches!(args.first().map(String::as_str), Some("policy"))
        && matches!(args.get(1).map(String::as_str), Some("apply"))
    {
        return cmd_registry_policy_apply(&args[2..]);
    }
    let Ok((positional, registry)) = parse_registry_override_args(
        args,
        "use RR registry keygen [identity] [--out-dir <dir>], RR registry onboard [identity] [--out-dir <dir>] [--require-signed] [--require-approval] [--auto-approve] [--registry <dir>], RR registry list [--registry <dir>], RR registry report [module-path] [--registry <dir>], RR registry diff <module-path> <from-version> <to-version> [--registry <dir>], RR registry risk <module-path> <version> [--against <version>] [--registry <dir>], RR registry channel show <module-path> [--registry <dir>], RR registry channel set <module-path> <channel> <version> [--registry <dir>], RR registry channel clear <module-path> <channel> [--registry <dir>], RR registry queue [--registry <dir>], RR registry audit [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>], RR registry audit export <file> [--format <tsv|jsonl>] [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>], RR registry policy bootstrap <trusted-public-key> [--signer <identity>] [--auto-approve-signer <identity>] [--require-signed] [--require-approval] [--registry <dir>], RR registry policy show [--registry <dir>], RR registry policy lint [--registry <dir>], RR registry policy rotate-key <old-public-key> <new-public-key> [--registry <dir>], RR registry policy apply <file> [--registry <dir>], RR registry info <module-path> [--registry <dir>], RR registry approve <module-path> <version> [--registry <dir>], RR registry unapprove <module-path> <version> [--registry <dir>], RR registry promote <module-path> <version> [--registry <dir>], RR registry yank <module-path> <version> [--registry <dir>], RR registry unyank <module-path> <version> [--registry <dir>], RR registry deprecate <module-path> <message> [--registry <dir>], RR registry undeprecate <module-path> [--registry <dir>], or RR registry verify [module-path] [--registry <dir>]",
        &ui,
    ) else {
        return 1;
    };

    match positional.as_slice() {
        [subcommand] if subcommand == "list" => {
            match RR::pkg::list_registry_modules(registry.as_deref()) {
                Ok(results) => {
                    if results.is_empty() {
                        ui.success("Registry is empty");
                        return 0;
                    }
                    for result in results {
                        print_registry_search_result(result);
                    }
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand] if subcommand == "report" => {
            match RR::pkg::registry_report(registry.as_deref(), None) {
                Ok(report) => {
                    println!(
                        "modules={} channels={} releases={} approved={} pending={} yanked={} signed={} deprecated={}",
                        report.module_count,
                        report.channel_count,
                        report.release_count,
                        report.approved_count,
                        report.pending_count,
                        report.yanked_count,
                        report.signed_count,
                        report.deprecated_module_count
                    );
                    for module in report.modules {
                        println!(
                            "{} latest={} channels={} releases={} approved={} pending={} yanked={} signed={} deprecated={}",
                            module.path,
                            module.latest_version.unwrap_or_else(|| "-".to_string()),
                            module.channel_count,
                            module.release_count,
                            module.approved_count,
                            module.pending_count,
                            module.yanked_count,
                            module.signed_count,
                            if module.deprecated { "true" } else { "false" }
                        );
                    }
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand, module_path] if subcommand == "report" => {
            match RR::pkg::registry_report(registry.as_deref(), Some(module_path)) {
                Ok(report) => {
                    println!(
                        "modules={} channels={} releases={} approved={} pending={} yanked={} signed={} deprecated={}",
                        report.module_count,
                        report.channel_count,
                        report.release_count,
                        report.approved_count,
                        report.pending_count,
                        report.yanked_count,
                        report.signed_count,
                        report.deprecated_module_count
                    );
                    for module in report.modules {
                        println!(
                            "{} latest={} channels={} releases={} approved={} pending={} yanked={} signed={} deprecated={}",
                            module.path,
                            module.latest_version.unwrap_or_else(|| "-".to_string()),
                            module.channel_count,
                            module.release_count,
                            module.approved_count,
                            module.pending_count,
                            module.yanked_count,
                            module.signed_count,
                            if module.deprecated { "true" } else { "false" }
                        );
                    }
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand, module_path, from_version, to_version] if subcommand == "diff" => {
            match RR::pkg::registry_diff(registry.as_deref(), module_path, from_version, to_version)
            {
                Ok(diff) => {
                    println!(
                        "module={} from={} to={}",
                        diff.module_path, diff.from_version, diff.to_version
                    );
                    println!(
                        "meta approved={}=>{} yanked={}=>{} signed={}=>{} signer={}=>{}",
                        if diff.from_approved { "true" } else { "false" },
                        if diff.to_approved { "true" } else { "false" },
                        if diff.from_yanked { "true" } else { "false" },
                        if diff.to_yanked { "true" } else { "false" },
                        if diff.from_signed { "true" } else { "false" },
                        if diff.to_signed { "true" } else { "false" },
                        diff.from_signer.as_deref().unwrap_or("-"),
                        diff.to_signer.as_deref().unwrap_or("-")
                    );
                    println!(
                        "files added={} removed={} changed={}",
                        diff.added_files.len(),
                        diff.removed_files.len(),
                        diff.changed_files.len()
                    );
                    for path in diff.added_files {
                        println!("+ {}", path);
                    }
                    for path in diff.removed_files {
                        println!("- {}", path);
                    }
                    for path in diff.changed_files {
                        println!("~ {}", path);
                    }
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [group, subcommand, module_path] if group == "channel" && subcommand == "show" => {
            match RR::pkg::registry_module_info(module_path, registry.as_deref()) {
                Ok(info) => {
                    if info.channels.is_empty() {
                        ui.success("Registry module has no channel assignments");
                        return 0;
                    }
                    for (channel, version) in info.channels {
                        println!("{} {}", channel, version);
                    }
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [group, subcommand, module_path, channel, version]
            if group == "channel" && subcommand == "set" =>
        {
            let Some(registry) = registry.as_deref() else {
                ui.error("RR registry channel set requires a registry root");
                ui.warn("pass --registry <dir-or-url> or set RR_REGISTRY_DIR");
                return 1;
            };
            match RR::pkg::set_registry_channel(module_path, channel, version, registry) {
                Ok(()) => {
                    ui.success(&format!(
                        "Set channel {} {} {}",
                        module_path, channel, version
                    ));
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [group, subcommand, module_path, channel]
            if group == "channel" && subcommand == "clear" =>
        {
            let Some(registry) = registry.as_deref() else {
                ui.error("RR registry channel clear requires a registry root");
                ui.warn("pass --registry <dir-or-url> or set RR_REGISTRY_DIR");
                return 1;
            };
            match RR::pkg::clear_registry_channel(module_path, channel, registry) {
                Ok(()) => {
                    ui.success(&format!("Cleared channel {} {}", module_path, channel));
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand] if subcommand == "queue" => {
            match RR::pkg::list_registry_queue(registry.as_deref()) {
                Ok(items) => {
                    if items.is_empty() {
                        ui.success("Registry approval queue is empty");
                        return 0;
                    }
                    for item in items {
                        println!(
                            "{} {} yanked={} signed={} signer={}",
                            item.path,
                            item.version,
                            if item.yanked { "true" } else { "false" },
                            if item.signed { "true" } else { "false" },
                            item.signer.as_deref().unwrap_or("-")
                        );
                    }
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [group, subcommand] if group == "policy" && subcommand == "lint" => {
            match RR::pkg::lint_registry_policy(registry.as_deref()) {
                Ok(report) => {
                    if !report.exists {
                        ui.warn(&format!(
                            "No registry policy file at {}",
                            report.path.display()
                        ));
                        return 0;
                    }
                    println!("policy {}", report.path.display());
                    println!(
                        "require_signed={} require_approval={} trusted={} revoked={} allowed_signers={}",
                        if report.require_signed {
                            "true"
                        } else {
                            "false"
                        },
                        if report.require_approval {
                            "true"
                        } else {
                            "false"
                        },
                        report.trusted_count,
                        report.revoked_count,
                        report.allowed_signer_count
                    );
                    println!("auto_approve_signers={}", report.auto_approve_signer_count);
                    for warning in report.warnings {
                        ui.warn(&warning);
                    }
                    if report.errors.is_empty() {
                        ui.success("Registry policy lint OK");
                        0
                    } else {
                        for error in report.errors {
                            ui.error(&error);
                        }
                        1
                    }
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [group, subcommand, old_key, new_key]
            if group == "policy" && subcommand == "rotate-key" =>
        {
            let Some(registry) = registry.as_deref() else {
                ui.error("RR registry policy rotate-key requires a registry root");
                ui.warn("pass --registry <dir-or-url> or set RR_REGISTRY_DIR");
                return 1;
            };
            match RR::pkg::rotate_registry_policy_key(old_key, new_key, registry) {
                Ok(path) => {
                    ui.success(&format!("Updated registry policy: {}", path.display()));
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand, module_path, version] if subcommand == "approve" => {
            let Some(registry) = registry.as_deref() else {
                ui.error("RR registry approve requires a registry root");
                ui.warn("pass --registry <dir-or-url> or set RR_REGISTRY_DIR");
                return 1;
            };
            match RR::pkg::approve_registry_release(module_path, version, registry) {
                Ok(()) => {
                    ui.success(&format!("Approved {} {}", module_path, version));
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand, module_path, version] if subcommand == "unapprove" => {
            let Some(registry) = registry.as_deref() else {
                ui.error("RR registry unapprove requires a registry root");
                ui.warn("pass --registry <dir-or-url> or set RR_REGISTRY_DIR");
                return 1;
            };
            match RR::pkg::unapprove_registry_release(module_path, version, registry) {
                Ok(()) => {
                    ui.success(&format!("Unapproved {} {}", module_path, version));
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand, module_path, version] if subcommand == "promote" => {
            let Some(registry) = registry.as_deref() else {
                ui.error("RR registry promote requires a registry root");
                ui.warn("pass --registry <dir-or-url> or set RR_REGISTRY_DIR");
                return 1;
            };
            match RR::pkg::promote_registry_release(module_path, version, registry) {
                Ok(()) => {
                    ui.success(&format!("Promoted {} {}", module_path, version));
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand, module_path] if subcommand == "info" => {
            match RR::pkg::registry_module_info(module_path, registry.as_deref()) {
                Ok(info) => {
                    println!("module {}", info.path);
                    if let Some(description) = info.description.filter(|value| !value.is_empty()) {
                        println!("description {}", description);
                    }
                    if let Some(license) = info.license.filter(|value| !value.is_empty()) {
                        println!("license {}", license);
                    }
                    if let Some(homepage) = info.homepage.filter(|value| !value.is_empty()) {
                        println!("homepage {}", homepage);
                    }
                    if let Some(deprecated) = info.deprecated.filter(|value| !value.is_empty()) {
                        println!("deprecated {}", deprecated);
                    }
                    for (channel, version) in info.channels {
                        println!("channel {} {}", channel, version);
                    }
                    for release in info.releases {
                        let signer = release
                            .signer
                            .as_deref()
                            .filter(|value| !value.is_empty())
                            .unwrap_or("-");
                        let scheme = release
                            .signature_scheme
                            .as_deref()
                            .filter(|value| !value.is_empty())
                            .unwrap_or("-");
                        println!(
                            "release {} files={} yanked={} approved={} signed={} scheme={} signer={} archive={} sum={}",
                            release.version,
                            release.file_count,
                            if release.yanked { "true" } else { "false" },
                            if release.approved { "true" } else { "false" },
                            if release.signed { "true" } else { "false" },
                            scheme,
                            signer,
                            release.archive_rel,
                            release.archive_sum
                        );
                    }
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand] if subcommand == "verify" => {
            match RR::pkg::verify_registry(registry.as_deref(), None) {
                Ok(report) => {
                    if report.issues.is_empty() {
                        ui.success(&format!(
                            "Verified registry: {} module(s), {} release(s)",
                            report.checked_modules, report.checked_releases
                        ));
                        return 0;
                    }
                    for issue in report.issues {
                        ui.error(&format!(
                            "{} {} archive={} {}",
                            issue.path,
                            issue.version,
                            issue.archive_path.display(),
                            issue.message
                        ));
                    }
                    1
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand, module_path] if subcommand == "verify" => {
            match RR::pkg::verify_registry(registry.as_deref(), Some(module_path)) {
                Ok(report) => {
                    if report.issues.is_empty() {
                        ui.success(&format!(
                            "Verified registry module {}: {} release(s)",
                            module_path, report.checked_releases
                        ));
                        return 0;
                    }
                    for issue in report.issues {
                        ui.error(&format!(
                            "{} {} archive={} {}",
                            issue.path,
                            issue.version,
                            issue.archive_path.display(),
                            issue.message
                        ));
                    }
                    1
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand, module_path, version] if subcommand == "yank" => {
            let Some(registry) = registry.as_deref() else {
                ui.error("RR registry yank requires a registry root");
                ui.warn("pass --registry <dir-or-url> or set RR_REGISTRY_DIR");
                return 1;
            };
            match RR::pkg::yank_registry_release(module_path, version, registry) {
                Ok(()) => {
                    ui.success(&format!("Yanked {} {}", module_path, version));
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand, module_path, version] if subcommand == "unyank" => {
            let Some(registry) = registry.as_deref() else {
                ui.error("RR registry unyank requires a registry root");
                ui.warn("pass --registry <dir-or-url> or set RR_REGISTRY_DIR");
                return 1;
            };
            match RR::pkg::unyank_registry_release(module_path, version, registry) {
                Ok(()) => {
                    ui.success(&format!("Unyanked {} {}", module_path, version));
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand, module_path, message @ ..] if subcommand == "deprecate" => {
            let Some(registry) = registry.as_deref() else {
                ui.error("RR registry deprecate requires a registry root");
                ui.warn("pass --registry <dir-or-url> or set RR_REGISTRY_DIR");
                return 1;
            };
            let message = message.join(" ");
            match RR::pkg::deprecate_registry_module(module_path, &message, registry) {
                Ok(()) => {
                    ui.success(&format!("Deprecated {}: {}", module_path, message));
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand, module_path] if subcommand == "undeprecate" => {
            let Some(registry) = registry.as_deref() else {
                ui.error("RR registry undeprecate requires a registry root");
                ui.warn("pass --registry <dir-or-url> or set RR_REGISTRY_DIR");
                return 1;
            };
            match RR::pkg::undeprecate_registry_module(module_path, registry) {
                Ok(()) => {
                    ui.success(&format!("Cleared deprecation for {}", module_path));
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        _ => {
            ui.error("RR registry expects a supported subcommand");
            ui.warn("use RR registry keygen [identity] [--out-dir <dir>], RR registry onboard [identity] [--out-dir <dir>] [--require-signed] [--require-approval] [--auto-approve] [--registry <dir>], RR registry list [--registry <dir>], RR registry report [module-path] [--registry <dir>], RR registry diff <module-path> <from-version> <to-version> [--registry <dir>], RR registry risk <module-path> <version> [--against <version>] [--registry <dir>], RR registry channel show <module-path> [--registry <dir>], RR registry channel set <module-path> <channel> <version> [--registry <dir>], RR registry channel clear <module-path> <channel> [--registry <dir>], RR registry queue [--registry <dir>], RR registry audit [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>], RR registry audit export <file> [--format <tsv|jsonl>] [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>], RR registry policy bootstrap <trusted-public-key> [--signer <identity>] [--auto-approve-signer <identity>] [--require-signed] [--require-approval] [--registry <dir>], RR registry policy show [--registry <dir>], RR registry policy lint [--registry <dir>], RR registry policy rotate-key <old-public-key> <new-public-key> [--registry <dir>], RR registry policy apply <file> [--registry <dir>], RR registry info <module-path> [--registry <dir>], RR registry approve <module-path> <version> [--registry <dir>], RR registry unapprove <module-path> <version> [--registry <dir>], RR registry promote <module-path> <version> [--registry <dir>], RR registry yank <module-path> <version> [--registry <dir>], RR registry unyank <module-path> <version> [--registry <dir>], RR registry deprecate <module-path> <message> [--registry <dir>], RR registry undeprecate <module-path> [--registry <dir>], or RR registry verify [module-path] [--registry <dir>]");
            1
        }
    }
}
