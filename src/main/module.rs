use rr::compiler::CliLog;
use std::env;
use std::path::PathBuf;

fn resolve_mod_project_root(ui: &CliLog, subcommand: &str) -> Result<PathBuf, i32> {
    let cwd = match env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            ui.error(&format!("Failed to determine current directory: {}", e));
            return Err(1);
        }
    };
    let Some(project_root) = rr::pkg::find_manifest_root(&cwd) else {
        ui.error(&format!(
            "RR mod {subcommand} requires an rr.mod manifest in the current directory or a parent directory"
        ));
        ui.warn(&format!(
            "run RR init first, then retry RR mod {subcommand} from inside that project"
        ));
        return Err(1);
    };
    Ok(project_root)
}

pub(crate) fn cmd_mod(args: &[String]) -> i32 {
    let ui = CliLog::new();
    match args {
        [subcommand] if subcommand == "graph" => {
            let project_root = match resolve_mod_project_root(&ui, "graph") {
                Ok(root) => root,
                Err(code) => return code,
            };
            match rr::pkg::graph_project_dependencies(&project_root) {
                Ok(edges) => {
                    for (from, to) in edges {
                        println!("{from} {to}");
                    }
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand, target] if subcommand == "why" => {
            let project_root = match resolve_mod_project_root(&ui, "why") {
                Ok(root) => root,
                Err(code) => return code,
            };
            match rr::pkg::why_project_dependency(&project_root, target) {
                Ok(path) => {
                    for (idx, node) in path.iter().enumerate() {
                        if idx == 0 {
                            println!("{node}");
                        } else {
                            println!("-> {node}");
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
        [subcommand] if subcommand == "verify" => {
            let project_root = match resolve_mod_project_root(&ui, "verify") {
                Ok(root) => root,
                Err(code) => return code,
            };
            match rr::pkg::verify_project_dependencies(&project_root) {
                Ok(report) => {
                    if report.mismatches.is_empty() {
                        ui.success(&format!("Verified {} module(s)", report.checked));
                        return 0;
                    }
                    for mismatch in report.mismatches {
                        ui.error(&format!(
                            "{} expected={} actual={} root={}",
                            mismatch.path,
                            mismatch.expected_sum,
                            mismatch.actual_sum,
                            mismatch.source_root.display()
                        ));
                    }
                    1
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand] if subcommand == "tidy" => {
            let project_root = match resolve_mod_project_root(&ui, "tidy") {
                Ok(root) => root,
                Err(code) => return code,
            };
            match rr::pkg::tidy_project(&project_root) {
                Ok((added, removed, total)) => {
                    ui.success(&format!(
                        "Tidied manifest: added {}, removed {}",
                        added, removed
                    ));
                    ui.success(&format!("Lock entries: {}", total));
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        [subcommand] if subcommand == "vendor" => {
            let project_root = match resolve_mod_project_root(&ui, "vendor") {
                Ok(root) => root,
                Err(code) => return code,
            };
            match rr::pkg::vendor_project_dependencies(&project_root) {
                Ok(count) => {
                    ui.success(&format!("Vendored {} module(s)", count));
                    ui.success(&format!(
                        "Vendor dir: {}",
                        project_root.join("vendor").display()
                    ));
                    0
                }
                Err(message) => {
                    ui.error(&message);
                    1
                }
            }
        }
        _ => {
            ui.error("RR mod expects a supported subcommand");
            ui.warn("use RR mod graph, RR mod why, RR mod verify, RR mod tidy, or RR mod vendor");
            1
        }
    }
}
