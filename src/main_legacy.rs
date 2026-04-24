use super::{
    CliCompileRequest, CommandMode, compile_cli_source, compile_output_options, parse_command_opts,
    print_usage, report_file_write_failure, report_path_read_failure,
    write_compile_profile_artifact,
};
use RR::compiler::{CliLog, CompileProfile};
use RR::runtime::runner::Runner;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn cmd_legacy(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let opts = match parse_command_opts(args, CommandMode::Legacy, &ui) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let input_path = PathBuf::from(&opts.target);

    if opts.target.is_empty() {
        print_usage();
        return 1;
    }
    if input_path.extension().and_then(|s| s.to_str()) != Some("rr") {
        ui.error("Input file must end with .rr");
        return 1;
    }

    let input = match fs::read_to_string(&input_path) {
        Ok(s) => s,
        Err(e) => {
            report_path_read_failure(&ui, &input_path, &e, "input path");
            return 1;
        }
    };
    let input_path = fs::canonicalize(&input_path).unwrap_or(input_path);
    let input_path_str = input_path.to_string_lossy().to_string();

    let output_opts = compile_output_options(&opts, !opts.no_runtime);
    let mut compile_profile = opts.profile_compile.then(CompileProfile::default);
    let result = compile_cli_source(CliCompileRequest {
        entry_path: &input_path_str,
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
    match result {
        Ok(out) => {
            if let Some(profile) = compile_profile.as_ref()
                && let Err(code) = write_compile_profile_artifact(
                    &ui,
                    profile,
                    opts.profile_compile_out.as_deref(),
                )
            {
                return code;
            }
            let r_code = out.r_code;
            let source_map = out.source_map;
            if let Some(out_path) = opts.output_path {
                if let Err(e) = fs::write(&out_path, &r_code) {
                    report_file_write_failure(&ui, Path::new(&out_path), &e, "legacy output path");
                    return 1;
                }
                ui.success(&format!("Compiled to {}", out_path));
                0
            } else if !opts.no_runtime {
                Runner::run(
                    &input_path_str,
                    &input,
                    &r_code,
                    &source_map,
                    None,
                    opts.keep_r,
                )
            } else {
                ui.success("Compilation successful (helper-only emission)");
                0
            }
        }
        Err(e) => {
            e.display(Some(&input), Some(&input_path_str));
            1
        }
    }
}
