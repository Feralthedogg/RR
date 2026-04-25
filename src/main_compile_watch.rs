use RR::compiler::{
    CliLog, CompileProfile, IncrementalSession, module_tree_fingerprint, module_tree_snapshot,
};
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use super::super::{
    CliCompileRequest, CommandMode, compile_cli_source, compile_output_options,
    default_watch_output_file, file_name_is_main_rr, parse_command_opts,
    prepare_project_entry_source, report_dir_create_failure, report_file_write_failure,
    report_path_read_failure, resolve_command_input, watch_output_hash, watch_output_matches_hash,
    write_compile_profile_artifact,
};

#[path = "main_compile_watch_changes.rs"]
mod main_compile_watch_changes;
#[path = "main_compile_watch_state.rs"]
mod main_compile_watch_state;

use self::main_compile_watch_changes::summarize_watch_changes;
use self::main_compile_watch_state::WatchState;

pub(crate) fn cmd_watch(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let mut opts = match parse_command_opts(args, CommandMode::Watch, &ui) {
        Ok(v) => v,
        Err(code) => return code,
    };

    if opts.incremental.enabled && !opts.incremental.auto {
        opts.incremental.phase3 = true;
    }

    let input_path = match resolve_command_input(&opts.target, "watch") {
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
    let out_file = if let Some(out) = opts.output_path.clone() {
        PathBuf::from(out)
    } else {
        default_watch_output_file(&input_path)
    };
    if let Some(parent) = out_file.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        report_dir_create_failure(&ui, parent, &e, "watch output directory");
        return 1;
    }

    ui.success(&format!(
        "Watching {} (poll={}ms)",
        input_path.display(),
        opts.watch_poll_ms
    ));

    let mut session = IncrementalSession::default();
    let mut state = WatchState::default();
    loop {
        let raw_input = match fs::read_to_string(&input_path) {
            Ok(s) => s,
            Err(e) => {
                report_path_read_failure(&ui, &input_path, &e, "watch input");
                return 1;
            }
        };
        let input = if file_name_is_main_rr(&input_path) {
            match prepare_project_entry_source(&input_path, &raw_input, "watch") {
                Ok(source) => source,
                Err(err) => {
                    err.display(Some(&raw_input), Some(&input_path_str));
                    if opts.watch_once {
                        return 1;
                    }
                    thread::sleep(Duration::from_millis(opts.watch_poll_ms));
                    continue;
                }
            }
        } else {
            raw_input
        };
        let snapshot = match module_tree_snapshot(&input_path_str, &input) {
            Ok(snapshot) => snapshot,
            Err(e) => {
                e.display(Some(&input), Some(&input_path_str));
                if opts.watch_once {
                    return 1;
                }
                thread::sleep(Duration::from_millis(opts.watch_poll_ms));
                continue;
            }
        };
        let fingerprint = match module_tree_fingerprint(&input_path_str, &input) {
            Ok(fp) => fp,
            Err(e) => {
                e.display(Some(&input), Some(&input_path_str));
                if opts.watch_once {
                    return 1;
                }
                thread::sleep(Duration::from_millis(opts.watch_poll_ms));
                continue;
            }
        };
        let output_current = watch_output_matches_hash(&out_file, state.output_hash());
        if state.unchanged_and_output_current(fingerprint, output_current) {
            if opts.watch_once {
                return 0;
            }
            if state.should_report_idle_wait() {
                ui.success("unchanged module tree; waiting for changes");
            }
            thread::sleep(Duration::from_millis(opts.watch_poll_ms));
            continue;
        }
        state.resume_after_rebuild_candidate();
        if state.should_announce_change(fingerprint) {
            if let Some(prev) = &state.last_successful_snapshot
                && let Some(summary) = summarize_watch_changes(prev, &snapshot)
            {
                ui.success(&format!("change detected in {summary}"));
            }
        }
        if state.output_was_modified_or_missing(fingerprint, output_current) {
            ui.success("watch output missing or changed; restoring");
        }

        let output_opts = compile_output_options(&opts, true);
        let mut compile_profile = opts.profile_compile.then(CompileProfile::default);
        let watch_result = compile_cli_source(CliCompileRequest {
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
        });

        match watch_result {
            Ok(out) => {
                let output_hash = watch_output_hash(&out.r_code);
                if let Err(e) = fs::write(&out_file, out.r_code.as_bytes()) {
                    report_file_write_failure(&ui, &out_file, &e, "watch output path");
                    return 1;
                }
                state.record_success(snapshot, fingerprint, output_hash);
                if out.stats.phase3_memory_hit || out.stats.phase1_artifact_hit {
                    ui.success(&format!(
                        "cache hit (phase1_hit={}, phase3_hit={}) -> {}",
                        out.stats.phase1_artifact_hit,
                        out.stats.phase3_memory_hit,
                        out_file.display()
                    ));
                } else {
                    ui.success(&format!(
                        "rebuilt (phase2 hits={}, misses={}{}) -> {}",
                        out.stats.phase2_emit_hits,
                        out.stats.phase2_emit_misses,
                        if out.stats.miss_reasons.is_empty() {
                            String::new()
                        } else {
                            format!(", reasons={}", out.stats.miss_reasons.join(","))
                        },
                        out_file.display()
                    ));
                }
                if let Some(profile) = compile_profile.as_ref()
                    && let Err(code) = write_compile_profile_artifact(
                        &ui,
                        profile,
                        opts.profile_compile_out.as_deref(),
                    )
                {
                    return code;
                }
            }
            Err(e) => {
                e.display(Some(&input), Some(&input_path_str));
                if opts.watch_once {
                    return 1;
                }
            }
        }

        if opts.watch_once {
            return 0;
        }
        thread::sleep(Duration::from_millis(opts.watch_poll_ms));
    }
}
