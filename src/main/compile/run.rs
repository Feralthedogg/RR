use rr::compiler::{CliLog, CompileProfile, IncrementalSession};
use rr::runtime::runner::Runner;
use std::fs;

use super::{
    CliCompileRequest, CommandMode, compile_cli_source, compile_output_options, parse_command_opts,
    prepare_project_entry_source, report_path_read_failure, resolve_command_input,
    write_compile_profile_artifact,
};

pub(crate) fn cmd_run(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let opts = match parse_command_opts(args, CommandMode::Run, &ui) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let input_path = match resolve_command_input(&opts.target, "run") {
        Ok(p) => p,
        Err(err) => {
            ui.error(&err.message);
            if let Some(help) = err.help {
                ui.warn(&help);
            }
            return 1;
        }
    };
    let input_path_str = input_path.to_string_lossy().to_string();
    let raw_input = match fs::read_to_string(&input_path) {
        Ok(s) => s,
        Err(e) => {
            report_path_read_failure(&ui, &input_path, &e, "run input");
            return 1;
        }
    };
    let input = match prepare_project_entry_source(&input_path, &raw_input, "run") {
        Ok(source) => source,
        Err(err) => {
            err.display(Some(&raw_input), Some(&input_path_str));
            return 1;
        }
    };

    let output_opts = compile_output_options(&opts, true);
    let mut compile_profile = opts.profile_compile.then(CompileProfile::default);
    let mut session = IncrementalSession::default();
    let result = compile_cli_source(CliCompileRequest {
        entry_path: &input_path_str,
        input: &input,
        opt_level: opts.opt_level,
        type_cfg: opts.type_cfg,
        parallel_cfg: opts.parallel_cfg,
        compiler_parallel_cfg: opts.compiler_parallel_cfg,
        incremental: opts.incremental,
        output_opts,
        session: Some(&mut session),
        profile: compile_profile.as_mut(),
        cold_compile: opts.cold_compile,
        profile_use: opts.profile_use.as_deref(),
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
            Runner::run(
                &input_path_str,
                &input,
                &r_code,
                &source_map,
                None,
                opts.keep_r,
            )
        }
        Err(e) => {
            e.display(Some(&input), Some(&input_path_str));
            1
        }
    }
}
