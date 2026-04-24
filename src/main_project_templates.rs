use super::main_project_paths::project_dir_name_from_module_path;

pub(super) fn build_gitignore_content() -> &'static str {
    "Build/\n"
}

pub(super) fn build_manifest_content(module_path: &str) -> String {
    format!("module {module_path}\n\nrr {}\n", rr_language_line())
}

pub(super) fn build_lockfile_content() -> &'static str {
    "version = 1\n"
}

pub(super) fn build_binary_template(module_path: &str) -> String {
    let project_name = project_dir_name_from_module_path(module_path);
    format!(
        "fn main() {{\n  print(\"Hello from {project_name}\")\n}}\n\n/*\nmain <- function() {{\n  print(\"Hello from {project_name}\")\n}}\n*/\n"
    )
}

pub(super) fn build_library_template(module_path: &str) -> String {
    let project_name = project_dir_name_from_module_path(module_path);
    format!("export fn hello() {{\n  return \"Hello from {project_name}\"\n}}\n")
}

fn rr_language_line() -> String {
    let mut parts = env!("CARGO_PKG_VERSION").split('.');
    let major = parts.next().unwrap_or("0");
    let minor = parts.next().unwrap_or("0");
    format!("{major}.{minor}")
}
