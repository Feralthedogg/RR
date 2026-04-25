use RR::compiler::CliLog;
use std::env;
use std::path::PathBuf;

fn current_project_root(ui: &CliLog, command: &str) -> Result<PathBuf, i32> {
    let cwd = match env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            ui.error(&format!("Failed to determine current directory: {}", e));
            return Err(1);
        }
    };
    let Some(project_root) = RR::pkg::find_manifest_root(&cwd) else {
        ui.error(&format!(
            "RR {command} requires an rr.mod manifest in the current directory or a parent directory"
        ));
        ui.warn(&format!(
            "run RR init first, then retry RR {command} from inside that project"
        ));
        return Err(1);
    };
    Ok(project_root)
}

pub(crate) fn cmd_install(args: &[String]) -> i32 {
    let ui = CliLog::new();
    if args.len() != 1 {
        ui.error("RR install expects exactly one module spec");
        ui.warn("use RR install <github-url|module-path>[@version]");
        return 1;
    }

    let project_root = match current_project_root(&ui, "install") {
        Ok(root) => root,
        Err(code) => return code,
    };

    match RR::pkg::install_dependency_in_project(&project_root, &args[0]) {
        Ok(report) => {
            ui.success(&format!(
                "Installed {} {}",
                report.direct_module.path, report.direct_module.version
            ));
            ui.success(&format!("Project: {}", report.project_root.display()));
            ui.success(&format!("Lock entries: {}", report.all_modules.len()));
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}

pub(crate) fn cmd_remove(args: &[String]) -> i32 {
    let ui = CliLog::new();
    if args.len() != 1 {
        ui.error("RR remove expects exactly one module path");
        ui.warn("use RR remove <module-path>");
        return 1;
    }

    let project_root = match current_project_root(&ui, "remove") {
        Ok(root) => root,
        Err(code) => return code,
    };

    match RR::pkg::remove_dependency_from_project(&project_root, &args[0]) {
        Ok((removed_require, removed_replace, remaining)) => {
            ui.success(&format!(
                "Removed {}{}{}",
                args[0],
                if removed_require { " require" } else { "" },
                if removed_replace { " replace" } else { "" }
            ));
            ui.success(&format!("Lock entries: {}", remaining));
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}

pub(crate) fn cmd_outdated(args: &[String]) -> i32 {
    let ui = CliLog::new();
    if !args.is_empty() {
        ui.error("RR outdated does not accept positional arguments");
        ui.warn("use RR outdated");
        return 1;
    }
    let project_root = match current_project_root(&ui, "outdated") {
        Ok(root) => root,
        Err(code) => return code,
    };
    match RR::pkg::outdated_direct_dependencies(&project_root) {
        Ok(entries) => {
            if entries.is_empty() {
                ui.success("No direct dependencies are recorded in rr.mod");
                return 0;
            }
            for entry in entries {
                let latest = entry.latest_version.unwrap_or_else(|| "-".to_string());
                ui.success(&format!(
                    "{} current={} latest={} status={}",
                    entry.path, entry.current_version, latest, entry.status
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

pub(crate) fn cmd_update(args: &[String]) -> i32 {
    let ui = CliLog::new();
    if args.len() > 1 {
        ui.error("RR update accepts at most one module path");
        ui.warn("use RR update [module-path]");
        return 1;
    }
    let project_root = match current_project_root(&ui, "update") {
        Ok(root) => root,
        Err(code) => return code,
    };
    match RR::pkg::update_project_dependencies(&project_root, args.first().map(String::as_str)) {
        Ok(modules) => {
            ui.success(&format!(
                "Updated dependency graph: {} locked module(s)",
                modules.len()
            ));
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}

pub(crate) fn cmd_publish(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let mut version = None::<String>;
    let mut options = RR::pkg::PublishOptions::default();
    let mut i = 0usize;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--dry-run" => options.dry_run = true,
            "--allow-dirty" => options.allow_dirty = true,
            "--push-tag" => options.push_tag = true,
            "--registry" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --registry");
                    ui.warn("use RR publish <version> [--dry-run] [--allow-dirty] [--push-tag] [--remote <name>] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                options.registry = Some(PathBuf::from(&args[i]));
            }
            "--remote" => {
                if i + 1 >= args.len() {
                    ui.error("Missing value after --remote");
                    ui.warn("use RR publish <version> [--dry-run] [--allow-dirty] [--push-tag] [--remote <name>] [--registry <dir>]");
                    return 1;
                }
                i += 1;
                options.remote = Some(args[i].clone());
            }
            _ if arg.starts_with('-') => {
                ui.error(&format!("Unknown option for RR publish: {}", arg));
                ui.warn("use RR publish <version> [--dry-run] [--allow-dirty] [--push-tag] [--remote <name>] [--registry <dir>]");
                return 1;
            }
            _ if version.is_none() => version = Some(arg.clone()),
            _ => {
                ui.error("RR publish expects exactly one version argument");
                ui.warn("use RR publish <version> [--dry-run] [--allow-dirty] [--push-tag] [--remote <name>] [--registry <dir>]");
                return 1;
            }
        }
        i += 1;
    }

    let Some(version) = version else {
        ui.error("Missing version for RR publish");
        ui.warn(
            "use RR publish <version> [--dry-run] [--allow-dirty] [--push-tag] [--remote <name>] [--registry <dir>]",
        );
        return 1;
    };

    let project_root = match current_project_root(&ui, "publish") {
        Ok(root) => root,
        Err(code) => return code,
    };

    match RR::pkg::publish_project(&project_root, &version, &options) {
        Ok(report) => {
            if report.dry_run {
                ui.success(&format!(
                    "Publish dry-run OK: {} file(s) -> {}",
                    report.included_files.len(),
                    report.archive_path.display()
                ));
            } else {
                ui.success(&format!(
                    "Published archive: {} ({} file(s))",
                    report.archive_path.display(),
                    report.included_files.len()
                ));
            }
            if let Some(tag) = report.tag {
                if report.tag_pushed {
                    ui.success(&format!("Pushed git tag {}", tag));
                } else {
                    ui.success(&format!("Prepared git tag {}", tag));
                }
            }
            0
        }
        Err(message) => {
            ui.error(&message);
            1
        }
    }
}
