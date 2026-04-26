use RR::compiler::{CliLog, CompileProfile};
use std::fs;
use std::path::PathBuf;

use super::super::{
    CliCompileRequest, CommandMode, compile_cli_source, compile_output_options,
    default_build_output_dir, parse_command_opts, prepare_project_entry_source,
    prepare_single_file_build_source, report_dir_create_failure, report_file_write_failure,
    report_path_read_failure, resolve_project_entry_in_dir, write_compile_profile_artifact,
    write_compile_profile_collection,
};

#[path = "main_compile_build_files.rs"]
mod main_compile_build_files;

use self::main_compile_build_files::{build_output_file, collect_rr_files};

pub(crate) fn cmd_build(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let opts = match parse_command_opts(args, CommandMode::Build, &ui) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let target = opts.target.clone();
    let target_path = PathBuf::from(&target);
    if !target_path.exists() {
        ui.error(&format!("build target not found: '{}'", target));
        ui.warn("pass an existing directory or .rr file; use --out-dir to choose where emitted R files go");
        return 1;
    }

    let out_dir = opts.output_path.clone().unwrap_or_else(|| {
        default_build_output_dir(&target_path)
            .to_string_lossy()
            .to_string()
    });

    let out_root = PathBuf::from(&out_dir);
    if let Err(e) = fs::create_dir_all(&out_root) {
        report_dir_create_failure(&ui, &out_root, &e, "build --out-dir destination");
        return 1;
    }
    println!("{} {}", ui.yellow_bold("[+]"), ui.red_bold("RR Build"));
    println!(
        " {} {}",
        ui.dim("|-"),
        ui.white_bold(&format!(
            "Target: {} | Out: {} ({})",
            target,
            out_dir,
            opts.opt_level.label()
        ))
    );

    let mut rr_files = Vec::new();
    let dir_mode = target_path.is_dir();
    let project_entry = if dir_mode {
        resolve_project_entry_in_dir(&target_path)
    } else {
        None
    };
    if let Some(entry) = &project_entry {
        rr_files.push(entry.clone());
    } else if dir_mode {
        if let Err(e) = collect_rr_files(&target_path, &mut rr_files) {
            ui.error(&format!("Failed while scanning '{}': {}", target, e));
            ui.warn(
                "make sure the build target directory is readable, and that RR can descend into its source tree",
            );
            return 1;
        }
    } else if target_path.extension().and_then(|s| s.to_str()) == Some("rr") {
        rr_files.push(target_path.clone());
    } else {
        ui.error("build target must be a directory or .rr file");
        ui.warn("use RR build <dir> to compile a project tree, or RR build path/to/file.rr for a single file");
        return 1;
    }

    rr_files.sort();
    if rr_files.is_empty() {
        ui.error(&format!("no .rr files found under '{}'", target));
        ui.warn(
            "add at least one .rr source file under that directory, or point RR build at a specific .rr file instead",
        );
        return 1;
    }

    let root_abs = if dir_mode {
        fs::canonicalize(&target_path).ok()
    } else {
        None
    };

    let mut built = 0usize;
    let mut compile_profiles: Vec<(String, CompileProfile)> = Vec::new();
    for rr in rr_files {
        let rr_abs = fs::canonicalize(&rr).unwrap_or(rr.clone());
        let rr_path_str = rr_abs.to_string_lossy().to_string();
        let raw_input = match fs::read_to_string(&rr_abs) {
            Ok(s) => s,
            Err(e) => {
                report_path_read_failure(&ui, &rr_abs, &e, "build input");
                return 1;
            }
        };
        let input = if project_entry.as_ref() == Some(&rr_abs) {
            match prepare_project_entry_source(&rr_abs, &raw_input, "build") {
                Ok(source) => source,
                Err(err) => {
                    err.display(Some(&raw_input), Some(&rr_path_str));
                    return 1;
                }
            }
        } else if !dir_mode {
            match prepare_single_file_build_source(&raw_input) {
                Ok(source) => source,
                Err(err) => {
                    err.display(Some(&raw_input), Some(&rr_path_str));
                    return 1;
                }
            }
        } else {
            raw_input
        };

        let output_opts = compile_output_options(&opts, true);
        let mut compile_profile = opts.profile_compile.then(CompileProfile::default);
        let build_out = compile_cli_source(CliCompileRequest {
            entry_path: &rr_path_str,
            input: &input,
            opt_level: opts.opt_level,
            type_cfg: opts.type_cfg,
            parallel_cfg: opts.parallel_cfg,
            compiler_parallel_cfg: opts.compiler_parallel_cfg,
            incremental: opts.incremental,
            output_opts,
            session: None,
            profile: compile_profile.as_mut(),
            cold_compile: opts.cold_compile,
        });

        let r_code = match build_out {
            Ok(out) => out.r_code,
            Err(e) => {
                e.display(Some(&input), Some(&rr_path_str));
                return 1;
            }
        };

        let out_file = build_output_file(
            dir_mode,
            &rr,
            &rr_abs,
            &target_path,
            root_abs.as_ref(),
            &out_root,
        );

        if let Some(parent) = out_file.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            report_dir_create_failure(&ui, parent, &e, "build output directory");
            return 1;
        }
        if let Err(e) = fs::write(&out_file, r_code) {
            report_file_write_failure(&ui, &out_file, &e, "build output path");
            return 1;
        }

        ui.success(&format!("Built {} -> {}", rr.display(), out_file.display()));
        built += 1;
        if let Some(profile) = compile_profile.take() {
            compile_profiles.push((rr_path_str, profile));
        }
    }

    ui.success(&format!(
        "Build complete: {} file(s) -> {}",
        built,
        out_root.display()
    ));
    if !compile_profiles.is_empty()
        && let Err(code) = if compile_profiles.len() == 1 {
            write_compile_profile_artifact(
                &ui,
                &compile_profiles[0].1,
                opts.profile_compile_out.as_deref(),
            )
        } else {
            write_compile_profile_collection(
                &ui,
                &compile_profiles,
                opts.profile_compile_out.as_deref(),
            )
        }
    {
        return code;
    }
    0
}
