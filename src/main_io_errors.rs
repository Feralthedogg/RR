use RR::compiler::CliLog;
use std::io::ErrorKind;
use std::path::Path;

pub(crate) fn report_path_read_failure(ui: &CliLog, path: &Path, err: &std::io::Error, role: &str) {
    ui.error(&format!("Failed to read '{}': {}", path.display(), err));
    match err.kind() {
        ErrorKind::PermissionDenied => ui.warn(&format!(
            "make the {role} readable, or adjust file and parent-directory permissions before retrying"
        )),
        ErrorKind::NotFound => ui.warn(&format!(
            "make sure the {role} exists and points to a readable .rr source file"
        )),
        _ => ui.warn(&format!(
            "check that the {role} is readable and not locked or replaced by another process"
        )),
    }
}

pub(crate) fn report_dir_create_failure(
    ui: &CliLog,
    path: &Path,
    err: &std::io::Error,
    role: &str,
) {
    ui.error(&format!("Failed to create '{}': {}", path.display(), err));
    match err.kind() {
        ErrorKind::PermissionDenied => ui.warn(&format!(
            "choose a writable {role}, or adjust parent-directory permissions before retrying"
        )),
        ErrorKind::NotFound => ui.warn(&format!(
            "create the parent directories first, or point {role} at an existing writable parent"
        )),
        _ => ui.warn(&format!(
            "choose a different {role}, or fix the destination path before retrying"
        )),
    }
}

pub(crate) fn report_file_write_failure(
    ui: &CliLog,
    path: &Path,
    err: &std::io::Error,
    role: &str,
) {
    ui.error(&format!("Failed to write '{}': {}", path.display(), err));
    match err.kind() {
        ErrorKind::PermissionDenied => ui.warn(&format!(
            "choose a writable {role}, or adjust file and parent-directory permissions before retrying"
        )),
        ErrorKind::NotFound => ui.warn(&format!(
            "create the destination directory first, or point {role} at an existing writable path"
        )),
        _ => ui.warn(&format!(
            "check that the {role} is writable and not blocked by another process"
        )),
    }
}
