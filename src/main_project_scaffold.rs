use RR::compiler::CliLog;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::main_project_paths::{default_module_path_for_dir, project_dir_name_from_module_path};
use super::main_project_templates::{
    build_binary_template, build_gitignore_content, build_library_template, build_lockfile_content,
    build_manifest_content,
};
use super::{report_dir_create_failure, report_file_write_failure, report_path_read_failure};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProjectKind {
    Bin,
    Lib,
}

impl ProjectKind {
    fn entry_rel_path(self) -> &'static str {
        match self {
            Self::Bin => "src/main.rr",
            Self::Lib => "src/lib.rr",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Bin => "binary",
            Self::Lib => "library",
        }
    }
}

struct NewCommandOpts {
    kind: ProjectKind,
    module_path: String,
    dir: PathBuf,
    allow_existing_dir: bool,
}

struct InitCommandOpts {
    kind: ProjectKind,
    module_path: String,
    dir: PathBuf,
}

fn parse_project_kind_flag(arg: &str) -> Option<ProjectKind> {
    match arg {
        "--bin" => Some(ProjectKind::Bin),
        "--lib" => Some(ProjectKind::Lib),
        _ => None,
    }
}

fn parse_new_command_opts(args: &[String], ui: &CliLog) -> Result<NewCommandOpts, i32> {
    let mut kind = ProjectKind::Bin;
    let mut positionals = Vec::new();

    for arg in args {
        if let Some(parsed) = parse_project_kind_flag(arg) {
            kind = parsed;
        } else if arg.starts_with('-') {
            ui.error(&format!("Unknown option for RR new: {}", arg));
            ui.warn("use RR new [--bin|--lib] <module-path|.> [dir|.]");
            return Err(1);
        } else {
            positionals.push(arg.clone());
        }
    }

    if positionals.is_empty() {
        ui.error("Missing module path for RR new");
        ui.warn("use RR new [--bin|--lib] <module-path|.> [dir|.]");
        return Err(1);
    }
    if positionals.len() > 2 {
        ui.error("RR new accepts at most two positional arguments: <module-path> [dir]");
        ui.warn("use RR new [--bin|--lib] <module-path|.> [dir|.]");
        return Err(1);
    }

    let module_path = positionals[0].trim().to_string();
    if module_path.is_empty() {
        ui.error("Module path for RR new must not be empty");
        return Err(1);
    }

    if positionals[0].trim() == "." && positionals.len() > 1 {
        ui.error("When using '.', RR new accepts it only as the sole argument or as the [dir]");
        ui.warn("use RR new . or RR new <module-path> .");
        return Err(1);
    }

    let (dir, allow_existing_dir, module_path) = if positionals[0].trim() == "." {
        let dir = match env::current_dir() {
            Ok(path) => path,
            Err(e) => {
                ui.error(&format!("Failed to determine current directory: {}", e));
                return Err(1);
            }
        };
        (dir.clone(), true, default_module_path_for_dir(&dir))
    } else if positionals.get(1).is_some_and(|arg| arg.trim() == ".") {
        let dir = match env::current_dir() {
            Ok(path) => path,
            Err(e) => {
                ui.error(&format!("Failed to determine current directory: {}", e));
                return Err(1);
            }
        };
        (dir, true, module_path)
    } else {
        (
            positionals
                .get(1)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(project_dir_name_from_module_path(&module_path))),
            false,
            module_path,
        )
    };

    Ok(NewCommandOpts {
        kind,
        module_path,
        dir,
        allow_existing_dir,
    })
}

fn parse_init_command_opts(args: &[String], ui: &CliLog) -> Result<InitCommandOpts, i32> {
    let mut kind = ProjectKind::Bin;
    let mut positionals = Vec::new();

    for arg in args {
        if let Some(parsed) = parse_project_kind_flag(arg) {
            kind = parsed;
        } else if arg.starts_with('-') {
            ui.error(&format!("Unknown option for RR init: {}", arg));
            ui.warn("use RR init [--bin|--lib] [module-path]");
            return Err(1);
        } else {
            positionals.push(arg.clone());
        }
    }

    if positionals.len() > 1 {
        ui.error("RR init accepts at most one positional argument: [module-path]");
        ui.warn("use RR init [--bin|--lib] [module-path]");
        return Err(1);
    }

    let dir = match env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            ui.error(&format!("Failed to determine current directory: {}", e));
            return Err(1);
        }
    };

    let module_path = positionals
        .pop()
        .unwrap_or_else(|| default_module_path_for_dir(&dir));

    Ok(InitCommandOpts {
        kind,
        module_path,
        dir,
    })
}

fn ensure_build_gitignore_entry(ui: &CliLog, dir: &Path) -> Result<(), i32> {
    let gitignore = dir.join(".gitignore");
    let required_line = "Build/";

    if !gitignore.exists() {
        if let Err(e) = fs::write(&gitignore, build_gitignore_content()) {
            report_file_write_failure(ui, &gitignore, &e, ".gitignore");
            return Err(1);
        }
        return Ok(());
    }

    let current = match fs::read_to_string(&gitignore) {
        Ok(content) => content,
        Err(e) => {
            report_path_read_failure(ui, &gitignore, &e, ".gitignore");
            return Err(1);
        }
    };
    if current.lines().any(|line| line.trim() == required_line) {
        return Ok(());
    }

    let mut file = match fs::OpenOptions::new().append(true).open(&gitignore) {
        Ok(file) => file,
        Err(e) => {
            report_file_write_failure(ui, &gitignore, &e, ".gitignore");
            return Err(1);
        }
    };
    let needs_newline = !current.is_empty() && !current.ends_with('\n');
    let payload = if needs_newline {
        format!("\n{required_line}\n")
    } else {
        format!("{required_line}\n")
    };
    if let Err(e) = file.write_all(payload.as_bytes()) {
        report_file_write_failure(ui, &gitignore, &e, ".gitignore");
        return Err(1);
    }

    Ok(())
}

fn scaffold_project(
    ui: &CliLog,
    dir: &Path,
    module_path: &str,
    kind: ProjectKind,
    allow_existing_dir: bool,
) -> Result<(), i32> {
    if !allow_existing_dir && dir.exists() {
        ui.error(&format!(
            "destination '{}' already exists",
            dir.to_string_lossy()
        ));
        ui.warn(
            "choose a different directory, or use RR init inside an existing project directory",
        );
        return Err(1);
    }

    if !dir.exists()
        && let Err(e) = fs::create_dir_all(dir)
    {
        report_dir_create_failure(ui, dir, &e, "project directory");
        return Err(1);
    }

    let manifest = dir.join("rr.mod");
    if manifest.exists() {
        ui.error(&format!(
            "manifest already exists at '{}'",
            manifest.to_string_lossy()
        ));
        ui.warn(
            "RR init expects an existing directory without rr.mod; use RR new for a fresh project",
        );
        return Err(1);
    }

    let src_dir = dir.join("src");
    if let Err(e) = fs::create_dir_all(&src_dir) {
        report_dir_create_failure(ui, &src_dir, &e, "source directory");
        return Err(1);
    }

    let build_dir = dir.join("Build");
    if let Err(e) = fs::create_dir_all(&build_dir) {
        report_dir_create_failure(ui, &build_dir, &e, "Build directory");
        return Err(1);
    }

    let entry_path = dir.join(kind.entry_rel_path());
    let entry_parent = entry_path.parent().unwrap_or(dir);
    if let Err(e) = fs::create_dir_all(entry_parent) {
        report_dir_create_failure(ui, entry_parent, &e, "entry source directory");
        return Err(1);
    }

    if let Err(e) = fs::write(&manifest, build_manifest_content(module_path)) {
        report_file_write_failure(ui, &manifest, &e, "rr.mod manifest");
        return Err(1);
    }

    let lockfile = dir.join("rr.lock");
    if !lockfile.exists()
        && let Err(e) = fs::write(&lockfile, build_lockfile_content())
    {
        report_file_write_failure(ui, &lockfile, &e, "rr.lock manifest");
        return Err(1);
    }

    if !entry_path.exists() {
        let template = match kind {
            ProjectKind::Bin => build_binary_template(module_path),
            ProjectKind::Lib => build_library_template(module_path),
        };
        if let Err(e) = fs::write(&entry_path, template) {
            report_file_write_failure(ui, &entry_path, &e, "entry source file");
            return Err(1);
        }
    }

    ensure_build_gitignore_entry(ui, dir)?;

    ui.success(&format!(
        "Initialized {} RR project at {}",
        kind.description(),
        dir.display()
    ));
    ui.success(&format!("Manifest: {}", manifest.display()));
    ui.success(&format!("Entry: {}", entry_path.display()));
    Ok(())
}

pub(crate) fn cmd_new(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let opts = match parse_new_command_opts(args, &ui) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    match scaffold_project(
        &ui,
        &opts.dir,
        &opts.module_path,
        opts.kind,
        opts.allow_existing_dir,
    ) {
        Ok(()) => 0,
        Err(code) => code,
    }
}

pub(crate) fn cmd_init(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let opts = match parse_init_command_opts(args, &ui) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    match scaffold_project(&ui, &opts.dir, &opts.module_path, opts.kind, true) {
        Ok(()) => 0,
        Err(code) => code,
    }
}
