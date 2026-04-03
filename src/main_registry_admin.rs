use super::*;

pub(crate) fn cmd_registry_keygen(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let mut out_dir = None::<PathBuf>;
    let mut identity = None::<String>;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--out-dir" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --out-dir");
                    ui.warn("use RR registry keygen [identity] [--out-dir <dir>]");
                    return 1;
                }
                i += 1;
                out_dir = Some(PathBuf::from(&args[i]));
            }
            arg if arg.starts_with('-') => {
                ui.error(&format!("Unknown option: {}", arg));
                ui.warn("use RR registry keygen [identity] [--out-dir <dir>]");
                return 1;
            }
            value if identity.is_none() => identity = Some(value.to_string()),
            _ => {
                ui.error("RR registry keygen accepts at most one identity");
                ui.warn("use RR registry keygen [identity] [--out-dir <dir>]");
                return 1;
            }
        }
        i += 1;
    }

    match RR::pkg::generate_registry_keypair(out_dir.as_deref(), identity.as_deref()) {
        Ok(report) => {
            println!("public {}", report.public_key_hex);
            println!("secret {}", report.secret_key_hex);
            if let Some(identity) = report.identity {
                println!("identity {}", identity);
            }
            for path in report.written_files {
                println!("wrote {}", path.display());
            }
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}

pub(crate) fn cmd_registry_onboard(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let mut out_dir = None::<PathBuf>;
    let mut registry = None::<PathBuf>;
    let mut identity = None::<String>;
    let mut require_signed = false;
    let mut require_approval = false;
    let mut auto_approve = false;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--out-dir" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --out-dir");
                    ui.warn("use RR registry onboard [identity] [--out-dir <dir>] [--require-signed] [--require-approval] [--auto-approve] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                out_dir = Some(PathBuf::from(&args[i]));
            }
            "--registry" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --registry");
                    ui.warn("use RR registry onboard [identity] [--out-dir <dir>] [--require-signed] [--require-approval] [--auto-approve] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                registry = Some(PathBuf::from(&args[i]));
            }
            "--require-signed" => require_signed = true,
            "--require-approval" => require_approval = true,
            "--auto-approve" => auto_approve = true,
            arg if arg.starts_with('-') => {
                ui.error(&format!("Unknown option: {}", arg));
                ui.warn("use RR registry onboard [identity] [--out-dir <dir>] [--require-signed] [--require-approval] [--auto-approve] [--registry <dir>]");
                return 1;
            }
            value if identity.is_none() => identity = Some(value.to_string()),
            _ => {
                ui.error("RR registry onboard accepts at most one identity");
                ui.warn("use RR registry onboard [identity] [--out-dir <dir>] [--require-signed] [--require-approval] [--auto-approve] [--registry <dir>]");
                return 1;
            }
        }
        i += 1;
    }

    let Some(registry) = registry.as_deref() else {
        ui.error("RR registry onboard requires a registry root");
        ui.warn("pass --registry <dir-or-url> or set RR_REGISTRY_DIR");
        return 1;
    };

    match RR::pkg::onboard_registry(
        registry,
        out_dir.as_deref(),
        identity.as_deref(),
        require_signed,
        require_approval,
        auto_approve,
    ) {
        Ok(report) => {
            println!("public {}", report.keygen.public_key_hex);
            println!("secret {}", report.keygen.secret_key_hex);
            if let Some(identity) = report.keygen.identity {
                println!("identity {}", identity);
            }
            println!("policy {}", report.policy_path.display());
            for path in report.keygen.written_files {
                println!("wrote {}", path.display());
            }
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}

pub(crate) fn cmd_registry_risk(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let mut registry = None::<PathBuf>;
    let mut against = None::<String>;
    let mut positional = Vec::<String>::new();
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--registry" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --registry");
                    ui.warn("use RR registry risk <module-path> <version> [--against <version>] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                registry = Some(PathBuf::from(&args[i]));
            }
            "--against" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --against");
                    ui.warn("use RR registry risk <module-path> <version> [--against <version>] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                against = Some(args[i].clone());
            }
            arg if arg.starts_with('-') => {
                ui.error(&format!("Unknown option: {}", arg));
                ui.warn("use RR registry risk <module-path> <version> [--against <version>] [--registry <dir>]");
                return 1;
            }
            _ => positional.push(args[i].clone()),
        }
        i += 1;
    }
    if positional.len() != 2 {
        ui.error("RR registry risk expects <module-path> <version>");
        ui.warn(
            "use RR registry risk <module-path> <version> [--against <version>] [--registry <dir>]",
        );
        return 1;
    }
    match RR::pkg::registry_risk(
        registry.as_deref(),
        &positional[0],
        &positional[1],
        against.as_deref(),
    ) {
        Ok(risk) => {
            println!(
                "module={} version={} baseline={} score={} level={}",
                risk.module_path,
                risk.version,
                risk.baseline_version.as_deref().unwrap_or("-"),
                risk.score,
                risk.level
            );
            for factor in risk.factors {
                println!("factor {} +{} {}", factor.key, factor.points, factor.detail);
            }
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}

pub(crate) fn cmd_registry_audit(args: &[String]) -> i32 {
    let ui = CliLog::new();
    if matches!(args.first().map(String::as_str), Some("export")) {
        return cmd_registry_audit_export(&args[1..]);
    }
    let mut registry = None::<PathBuf>;
    let mut limit = None::<usize>;
    let mut action = None::<String>;
    let mut module = None::<String>;
    let mut contains = None::<String>;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--registry" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --registry");
                    ui.warn("use RR registry audit [--limit <n>] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                registry = Some(PathBuf::from(&args[i]));
            }
            "--limit" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --limit");
                    ui.warn("use RR registry audit [--limit <n>] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                let Ok(parsed) = args[i].parse::<usize>() else {
                    ui.error("--limit must be a positive integer");
                    return 1;
                };
                limit = Some(parsed);
            }
            "--action" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --action");
                    ui.warn("use RR registry audit [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                action = Some(args[i].clone());
            }
            "--module" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --module");
                    ui.warn("use RR registry audit [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                module = Some(args[i].clone());
            }
            "--contains" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --contains");
                    ui.warn("use RR registry audit [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                contains = Some(args[i].clone());
            }
            arg if arg.starts_with('-') => {
                ui.error(&format!("Unknown option: {}", arg));
                ui.warn("use RR registry audit [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]");
                return 1;
            }
            _ => {
                ui.error("RR registry audit does not accept positional arguments");
                ui.warn("use RR registry audit [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]");
                return 1;
            }
        }
        i += 1;
    }

    match RR::pkg::read_registry_audit_log_filtered(
        registry.as_deref(),
        limit,
        action.as_deref(),
        module.as_deref(),
        contains.as_deref(),
    ) {
        Ok(entries) => {
            if entries.is_empty() {
                ui.success("Registry audit log is empty");
                return 0;
            }
            for entry in entries {
                println!(
                    "{}\t{}\t{}",
                    entry.timestamp_secs, entry.action, entry.detail
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

pub(crate) fn cmd_registry_audit_export(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let mut registry = None::<PathBuf>;
    let mut limit = None::<usize>;
    let mut action = None::<String>;
    let mut module = None::<String>;
    let mut contains = None::<String>;
    let mut format = "tsv".to_string();
    let mut output = None::<PathBuf>;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--registry" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --registry");
                    ui.warn("use RR registry audit export <file> [--format <tsv|jsonl>] [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                registry = Some(PathBuf::from(&args[i]));
            }
            "--limit" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --limit");
                    ui.warn("use RR registry audit export <file> [--format <tsv|jsonl>] [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                let Ok(parsed) = args[i].parse::<usize>() else {
                    ui.error("--limit must be a positive integer");
                    return 1;
                };
                limit = Some(parsed);
            }
            "--action" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --action");
                    ui.warn("use RR registry audit export <file> [--format <tsv|jsonl>] [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                action = Some(args[i].clone());
            }
            "--module" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --module");
                    ui.warn("use RR registry audit export <file> [--format <tsv|jsonl>] [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                module = Some(args[i].clone());
            }
            "--contains" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --contains");
                    ui.warn("use RR registry audit export <file> [--format <tsv|jsonl>] [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                contains = Some(args[i].clone());
            }
            "--format" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --format");
                    ui.warn("use RR registry audit export <file> [--format <tsv|jsonl>] [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                format = args[i].clone();
            }
            arg if arg.starts_with('-') => {
                ui.error(&format!("Unknown option: {}", arg));
                ui.warn("use RR registry audit export <file> [--format <tsv|jsonl>] [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]");
                return 1;
            }
            value if output.is_none() => output = Some(PathBuf::from(value)),
            _ => {
                ui.error("RR registry audit export expects exactly one output file");
                ui.warn("use RR registry audit export <file> [--format <tsv|jsonl>] [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]");
                return 1;
            }
        }
        i += 1;
    }
    let Some(output) = output else {
        ui.error("Missing output file for RR registry audit export");
        ui.warn("use RR registry audit export <file> [--format <tsv|jsonl>] [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]");
        return 1;
    };

    match RR::pkg::export_registry_audit_log(
        registry.as_deref(),
        &output,
        &format,
        limit,
        action.as_deref(),
        module.as_deref(),
        contains.as_deref(),
    ) {
        Ok(count) => {
            ui.success(&format!(
                "Exported {} audit entrie(s) to {}",
                count,
                output.display()
            ));
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}

pub(crate) fn cmd_registry_policy_show(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let Ok((positional, registry)) =
        parse_registry_override_args(args, "use RR registry policy show [--registry <dir>]", &ui)
    else {
        return 1;
    };
    if !positional.is_empty() {
        ui.error("RR registry policy show does not accept positional arguments");
        ui.warn("use RR registry policy show [--registry <dir>]");
        return 1;
    }
    match RR::pkg::show_registry_policy(registry.as_deref()) {
        Ok(report) => {
            if !report.exists {
                ui.warn(&format!(
                    "No registry policy file at {}",
                    report.path.display()
                ));
                return 0;
            }
            println!("policy {}", report.path.display());
            print!("{}", report.content);
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}

pub(crate) fn cmd_registry_policy_apply(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let Ok((positional, registry)) = parse_registry_override_args(
        args,
        "use RR registry policy apply <file> [--registry <dir>]",
        &ui,
    ) else {
        return 1;
    };
    if positional.len() != 1 {
        ui.error("RR registry policy apply expects exactly one file path");
        ui.warn("use RR registry policy apply <file> [--registry <dir>]");
        return 1;
    }
    let Some(registry) = registry.as_deref() else {
        ui.error("RR registry policy apply requires a registry root");
        ui.warn("pass --registry <dir-or-url> or set RR_REGISTRY_DIR");
        return 1;
    };
    match RR::pkg::apply_registry_policy(&PathBuf::from(&positional[0]), registry) {
        Ok(path) => {
            ui.success(&format!("Applied registry policy: {}", path.display()));
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}

pub(crate) fn cmd_registry_policy_bootstrap(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let mut registry = None::<PathBuf>;
    let mut signer = None::<String>;
    let mut auto_approve_signer = None::<String>;
    let mut require_signed = false;
    let mut require_approval = false;
    let mut trusted_key = None::<String>;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--registry" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --registry");
                    ui.warn("use RR registry policy bootstrap <trusted-public-key> [--signer <identity>] [--require-signed] [--require-approval] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                registry = Some(PathBuf::from(&args[i]));
            }
            "--signer" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --signer");
                    ui.warn("use RR registry policy bootstrap <trusted-public-key> [--signer <identity>] [--auto-approve-signer <identity>] [--require-signed] [--require-approval] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                signer = Some(args[i].clone());
            }
            "--auto-approve-signer" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --auto-approve-signer");
                    ui.warn("use RR registry policy bootstrap <trusted-public-key> [--signer <identity>] [--auto-approve-signer <identity>] [--require-signed] [--require-approval] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                auto_approve_signer = Some(args[i].clone());
            }
            "--require-signed" => require_signed = true,
            "--require-approval" => require_approval = true,
            arg if arg.starts_with('-') => {
                ui.error(&format!("Unknown option: {}", arg));
                ui.warn("use RR registry policy bootstrap <trusted-public-key> [--signer <identity>] [--auto-approve-signer <identity>] [--require-signed] [--require-approval] [--registry <dir>]");
                return 1;
            }
            value if trusted_key.is_none() => trusted_key = Some(value.to_string()),
            _ => {
                ui.error("RR registry policy bootstrap expects exactly one trusted public key");
                ui.warn("use RR registry policy bootstrap <trusted-public-key> [--signer <identity>] [--auto-approve-signer <identity>] [--require-signed] [--require-approval] [--registry <dir>]");
                return 1;
            }
        }
        i += 1;
    }

    let Some(registry) = registry.as_deref() else {
        ui.error("RR registry policy bootstrap requires a registry root");
        ui.warn("pass --registry <dir-or-url> or set RR_REGISTRY_DIR");
        return 1;
    };
    let Some(trusted_key) = trusted_key else {
        ui.error("Missing trusted public key for RR registry policy bootstrap");
        ui.warn("use RR registry policy bootstrap <trusted-public-key> [--signer <identity>] [--auto-approve-signer <identity>] [--require-signed] [--require-approval] [--registry <dir>]");
        return 1;
    };

    match RR::pkg::bootstrap_registry_policy(
        &trusted_key,
        signer.as_deref(),
        auto_approve_signer.as_deref(),
        require_signed,
        require_approval,
        registry,
    ) {
        Ok(path) => {
            ui.success(&format!("Bootstrapped registry policy: {}", path.display()));
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}
