use rr::compiler::CliLog;
use std::path::Path;

use super::require_registry_root;

pub(super) fn cmd_registry_channel_show(
    ui: &CliLog,
    registry: Option<&Path>,
    module_path: &str,
) -> i32 {
    match rr::pkg::registry_module_info(module_path, registry) {
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

pub(super) fn cmd_registry_channel_set(
    ui: &CliLog,
    registry: Option<&Path>,
    module_path: &str,
    channel: &str,
    version: &str,
) -> i32 {
    let registry = match require_registry_root(registry, ui, "RR registry channel set") {
        Ok(path) => path,
        Err(code) => return code,
    };
    match rr::pkg::set_registry_channel(module_path, channel, version, registry) {
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

pub(super) fn cmd_registry_channel_clear(
    ui: &CliLog,
    registry: Option<&Path>,
    module_path: &str,
    channel: &str,
) -> i32 {
    let registry = match require_registry_root(registry, ui, "RR registry channel clear") {
        Ok(path) => path,
        Err(code) => return code,
    };
    match rr::pkg::clear_registry_channel(module_path, channel, registry) {
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
