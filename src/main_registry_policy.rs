use RR::compiler::CliLog;
use std::path::Path;

use super::require_registry_root;

pub(super) fn cmd_registry_policy_lint(ui: &CliLog, registry: Option<&Path>) -> i32 {
    match RR::pkg::lint_registry_policy(registry) {
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

pub(super) fn cmd_registry_policy_rotate_key(
    ui: &CliLog,
    registry: Option<&Path>,
    old_key: &str,
    new_key: &str,
) -> i32 {
    let registry = match require_registry_root(registry, ui, "RR registry policy rotate-key") {
        Ok(path) => path,
        Err(code) => return code,
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
