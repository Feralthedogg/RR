use RR::compiler::CliLog;
use std::path::PathBuf;

use super::super::parse_registry_override_args;

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
