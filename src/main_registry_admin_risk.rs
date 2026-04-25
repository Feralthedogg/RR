use RR::compiler::CliLog;
use std::path::PathBuf;

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
