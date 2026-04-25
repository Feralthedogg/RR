use RR::compiler::CliLog;
use std::path::Path;

use super::require_registry_root;

pub(super) fn cmd_registry_approve(
    ui: &CliLog,
    registry: Option<&Path>,
    module_path: &str,
    version: &str,
) -> i32 {
    let registry = match require_registry_root(registry, ui, "RR registry approve") {
        Ok(path) => path,
        Err(code) => return code,
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

pub(super) fn cmd_registry_unapprove(
    ui: &CliLog,
    registry: Option<&Path>,
    module_path: &str,
    version: &str,
) -> i32 {
    let registry = match require_registry_root(registry, ui, "RR registry unapprove") {
        Ok(path) => path,
        Err(code) => return code,
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

pub(super) fn cmd_registry_promote(
    ui: &CliLog,
    registry: Option<&Path>,
    module_path: &str,
    version: &str,
) -> i32 {
    let registry = match require_registry_root(registry, ui, "RR registry promote") {
        Ok(path) => path,
        Err(code) => return code,
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

pub(super) fn cmd_registry_yank(
    ui: &CliLog,
    registry: Option<&Path>,
    module_path: &str,
    version: &str,
) -> i32 {
    let registry = match require_registry_root(registry, ui, "RR registry yank") {
        Ok(path) => path,
        Err(code) => return code,
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

pub(super) fn cmd_registry_unyank(
    ui: &CliLog,
    registry: Option<&Path>,
    module_path: &str,
    version: &str,
) -> i32 {
    let registry = match require_registry_root(registry, ui, "RR registry unyank") {
        Ok(path) => path,
        Err(code) => return code,
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

pub(super) fn cmd_registry_deprecate(
    ui: &CliLog,
    registry: Option<&Path>,
    module_path: &str,
    message: &[String],
) -> i32 {
    let registry = match require_registry_root(registry, ui, "RR registry deprecate") {
        Ok(path) => path,
        Err(code) => return code,
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

pub(super) fn cmd_registry_undeprecate(
    ui: &CliLog,
    registry: Option<&Path>,
    module_path: &str,
) -> i32 {
    let registry = match require_registry_root(registry, ui, "RR registry undeprecate") {
        Ok(path) => path,
        Err(code) => return code,
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
