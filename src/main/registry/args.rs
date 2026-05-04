use rr::compiler::CliLog;
use std::path::{Path, PathBuf};

pub(super) fn parse_registry_override_args(
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

pub(super) fn require_registry_root<'a>(
    registry: Option<&'a Path>,
    ui: &CliLog,
    command: &str,
) -> Result<&'a Path, i32> {
    let Some(registry) = registry else {
        ui.error(&format!("{command} requires a registry root"));
        ui.warn("pass --registry <dir-or-url> or set RR_REGISTRY_DIR");
        return Err(1);
    };
    Ok(registry)
}
