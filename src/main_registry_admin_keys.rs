use RR::compiler::CliLog;
use std::path::PathBuf;

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
