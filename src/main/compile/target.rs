use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(crate) struct TargetResolutionError {
    pub(crate) message: String,
    pub(crate) help: Option<String>,
}

fn managed_entry_path(dir: &Path) -> PathBuf {
    dir.join("src").join("main.rr")
}

pub(crate) fn resolve_project_entry_in_dir(dir: &Path) -> Option<PathBuf> {
    let managed_entry = managed_entry_path(dir);
    if managed_entry.is_file() {
        return Some(fs::canonicalize(&managed_entry).unwrap_or(managed_entry));
    }
    None
}

pub(crate) fn file_name_is_main_rr(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some("main.rr")
}

pub(crate) fn resolve_command_input(
    raw: &str,
    command: &str,
) -> Result<PathBuf, TargetResolutionError> {
    let path = PathBuf::from(raw);
    if path.is_dir() || raw == "." {
        if let Some(entry) = resolve_project_entry_in_dir(&path) {
            Ok(entry)
        } else {
            Err(TargetResolutionError {
                message: format!("src/main.rr not found in '{}'", path.to_string_lossy()),
                help: Some(format!(
                    "add src/main.rr for a managed project, or run RR {command} with an explicit .rr file path"
                )),
            })
        }
    } else if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("rr") {
            Ok(fs::canonicalize(&path).unwrap_or(path))
        } else {
            Err(TargetResolutionError {
                message: format!("{command} target must be a .rr file or directory"),
                help: Some(format!(
                    "pass a .rr file directly, or point RR {command} at a directory containing src/main.rr"
                )),
            })
        }
    } else {
        Err(TargetResolutionError {
            message: format!("{command} target not found: '{}'", raw),
            help: Some(format!(
                "use RR {command} . inside a project directory, or pass an existing .rr file path"
            )),
        })
    }
}
