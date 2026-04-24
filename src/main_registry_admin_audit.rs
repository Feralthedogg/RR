use RR::compiler::CliLog;
use std::path::PathBuf;

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
