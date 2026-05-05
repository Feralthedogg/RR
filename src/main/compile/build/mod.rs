use rr::compiler::{CliLog, CompileProfile};
use std::fs;
use std::path::{Path, PathBuf};

use super::{
    CliCompileRequest, CommandMode, CommonOpts, compile_cli_source, compile_output_options,
    default_build_output_dir, parse_command_opts, prepare_project_entry_source,
    prepare_single_file_build_source, report_dir_create_failure, report_file_write_failure,
    report_path_read_failure, resolve_project_entry_in_dir, write_compile_profile_artifact,
    write_compile_profile_collection,
};

mod files;

use self::files::{build_output_file, collect_rr_files};

struct BuildPlan {
    target: String,
    target_path: PathBuf,
    out_dir: String,
    out_root: PathBuf,
    dir_mode: bool,
    project_entry: Option<PathBuf>,
    root_abs: Option<PathBuf>,
    rr_files: Vec<PathBuf>,
}

pub(crate) fn cmd_build(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let opts = match parse_command_opts(args, CommandMode::Build, &ui) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let plan = match resolve_build_plan(&ui, &opts) {
        Ok(plan) => plan,
        Err(code) => return code,
    };

    ui.section_header("RR Build");
    ui.field_line("target", &plan.target);
    ui.field_line("output", &plan.out_dir);
    ui.field_line("opt", opts.opt_level.label());

    let mut built = 0usize;
    let mut compile_profiles: Vec<(String, CompileProfile)> = Vec::new();
    for rr in &plan.rr_files {
        let Some(profile_entry) = compile_and_write_build_file(&ui, &opts, &plan, rr) else {
            return 1;
        };
        built += 1;
        if let Some(profile) = profile_entry {
            compile_profiles.push(profile);
        }
    }

    if std::env::var_os("RR_VERBOSE_LOG").is_some() {
        ui.success(&format!(
            "Build complete: {} file(s) -> {}",
            built,
            plan.out_root.display()
        ));
    }
    if let Err(code) = write_build_profiles(&ui, &opts, &compile_profiles) {
        return code;
    }
    0
}

fn resolve_build_plan(ui: &CliLog, opts: &CommonOpts) -> Result<BuildPlan, i32> {
    let target = opts.target.clone();
    let target_path = PathBuf::from(&target);
    if !target_path.exists() {
        ui.error(&format!("build target not found: '{}'", target));
        ui.warn("pass an existing directory or .rr file; use --out-dir to choose where emitted R files go");
        return Err(1);
    }

    let out_dir = opts.output_path.clone().unwrap_or_else(|| {
        default_build_output_dir(&target_path)
            .to_string_lossy()
            .to_string()
    });

    let out_root = PathBuf::from(&out_dir);
    if let Err(e) = fs::create_dir_all(&out_root) {
        report_dir_create_failure(ui, &out_root, &e, "build --out-dir destination");
        return Err(1);
    }

    let dir_mode = target_path.is_dir();
    let project_entry = dir_mode
        .then(|| resolve_project_entry_in_dir(&target_path))
        .flatten();
    let rr_files = collect_build_inputs(
        ui,
        &target,
        &target_path,
        dir_mode,
        project_entry.as_deref(),
    )?;
    let root_abs = dir_mode
        .then(|| fs::canonicalize(&target_path).ok())
        .flatten();

    Ok(BuildPlan {
        target,
        target_path,
        out_dir,
        out_root,
        dir_mode,
        project_entry,
        root_abs,
        rr_files,
    })
}

fn collect_build_inputs(
    ui: &CliLog,
    target: &str,
    target_path: &Path,
    dir_mode: bool,
    project_entry: Option<&Path>,
) -> Result<Vec<PathBuf>, i32> {
    let mut rr_files = Vec::new();
    if let Some(entry) = project_entry {
        rr_files.push(entry.to_path_buf());
    } else if dir_mode {
        if let Err(e) = collect_rr_files(target_path, &mut rr_files) {
            ui.error(&format!("Failed while scanning '{}': {}", target, e));
            ui.warn(
                "make sure the build target directory is readable, and that RR can descend into its source tree",
            );
            return Err(1);
        }
    } else if target_path.extension().and_then(|s| s.to_str()) == Some("rr") {
        rr_files.push(target_path.to_path_buf());
    } else {
        ui.error("build target must be a directory or .rr file");
        ui.warn("use RR build <dir> to compile a project tree, or RR build path/to/file.rr for a single file");
        return Err(1);
    }

    rr_files.sort();
    if rr_files.is_empty() {
        ui.error(&format!("no .rr files found under '{}'", target));
        ui.warn(
            "add at least one .rr source file under that directory, or point RR build at a specific .rr file instead",
        );
        return Err(1);
    }
    Ok(rr_files)
}

fn prepare_build_source(
    ui: &CliLog,
    plan: &BuildPlan,
    rr_abs: &PathBuf,
) -> Option<(String, String)> {
    let rr_path_str = rr_abs.to_string_lossy().to_string();
    let raw_input = match fs::read_to_string(rr_abs) {
        Ok(s) => s,
        Err(e) => {
            report_path_read_failure(ui, rr_abs, &e, "build input");
            return None;
        }
    };
    let input = if plan.project_entry.as_ref() == Some(rr_abs) {
        match prepare_project_entry_source(rr_abs, &raw_input, "build") {
            Ok(source) => source,
            Err(err) => {
                err.display(Some(&raw_input), Some(&rr_path_str));
                return None;
            }
        }
    } else if !plan.dir_mode {
        match prepare_single_file_build_source(&raw_input) {
            Ok(source) => source,
            Err(err) => {
                err.display(Some(&raw_input), Some(&rr_path_str));
                return None;
            }
        }
    } else {
        raw_input
    };
    Some((rr_path_str, input))
}

fn compile_build_source(
    opts: &CommonOpts,
    rr_path_str: &str,
    input: &str,
    compile_profile: &mut Option<CompileProfile>,
) -> Result<String, i32> {
    let output_opts = compile_output_options(opts, true);
    compile_cli_source(CliCompileRequest {
        entry_path: rr_path_str,
        input,
        opt_level: opts.opt_level,
        type_cfg: opts.type_cfg,
        parallel_cfg: opts.parallel_cfg,
        compiler_parallel_cfg: opts.compiler_parallel_cfg,
        incremental: opts.incremental,
        output_opts,
        session: None,
        profile: compile_profile.as_mut(),
        cold_compile: opts.cold_compile,
        profile_use: opts.profile_use.as_deref(),
    })
    .map(|out| out.r_code)
    .map_err(|err| {
        err.display(Some(input), Some(rr_path_str));
        1
    })
}

fn compile_and_write_build_file(
    ui: &CliLog,
    opts: &CommonOpts,
    plan: &BuildPlan,
    rr: &PathBuf,
) -> Option<Option<(String, CompileProfile)>> {
    let rr_abs = fs::canonicalize(rr).unwrap_or_else(|_| rr.clone());
    let (rr_path_str, input) = prepare_build_source(ui, plan, &rr_abs)?;
    let mut compile_profile = opts.profile_compile.then(CompileProfile::default);
    let r_code = compile_build_source(opts, &rr_path_str, &input, &mut compile_profile).ok()?;

    let out_file = build_output_file(
        plan.dir_mode,
        rr,
        &rr_abs,
        &plan.target_path,
        plan.root_abs.as_ref(),
        &plan.out_root,
    );
    if let Some(parent) = out_file.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        report_dir_create_failure(ui, parent, &e, "build output directory");
        return None;
    }
    if let Err(e) = fs::write(&out_file, r_code) {
        report_file_write_failure(ui, &out_file, &e, "build output path");
        return None;
    }

    ui.success(&format!("Built {} -> {}", rr.display(), out_file.display()));
    Some(compile_profile.map(|profile| (rr_path_str, profile)))
}

fn write_build_profiles(
    ui: &CliLog,
    opts: &CommonOpts,
    compile_profiles: &[(String, CompileProfile)],
) -> Result<(), i32> {
    if compile_profiles.is_empty() {
        return Ok(());
    }
    if compile_profiles.len() == 1 {
        write_compile_profile_artifact(
            ui,
            &compile_profiles[0].1,
            opts.profile_compile_out.as_deref(),
        )
    } else {
        write_compile_profile_collection(ui, compile_profiles, opts.profile_compile_out.as_deref())
    }?;
    Ok(())
}
