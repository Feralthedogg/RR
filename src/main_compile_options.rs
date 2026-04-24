use RR::compiler::{CliLog, CompileMode, IncrementalOptions, OptLevel};

#[path = "main_compile_flags.rs"]
mod main_compile_flags;
#[path = "main_compile_incremental.rs"]
mod main_compile_incremental;
#[path = "main_compile_option_types.rs"]
mod main_compile_option_types;

use self::main_compile_flags::{CommonCompileFlagState, apply_common_compile_flags};
use self::main_compile_incremental::parse_incremental_phases;
pub(super) use self::main_compile_option_types::{CommandMode, CommonOpts};

pub(super) fn parse_command_opts(
    args: &[String],
    mode: CommandMode,
    ui: &CliLog,
) -> Result<CommonOpts, i32> {
    let mut opts = CommonOpts::new(mode);
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if mode.takes_output_arg(arg) {
            if i + 1 >= args.len() {
                if matches!(mode, CommandMode::Legacy) {
                    ui.error("Missing output file after -o");
                } else {
                    ui.error(&format!("Missing directory path after {}", arg));
                }
                return Err(1);
            }
            opts.output_path = Some(args[i + 1].clone());
            i += 1;
        } else {
            let mut compile_flag_state = CommonCompileFlagState {
                opt_level: &mut opts.opt_level,
                type_cfg: &mut opts.type_cfg,
                parallel_cfg: &mut opts.parallel_cfg,
                compiler_parallel_cfg: &mut opts.compiler_parallel_cfg,
                strict_let: &mut opts.strict_let,
                warn_implicit_decl: &mut opts.warn_implicit_decl,
            };
            match apply_common_compile_flags(args, &mut i, &mut compile_flag_state, ui) {
                Ok(true) => {}
                Ok(false) => {
                    if mode.allow_keep_r() && arg == "--keep-r" {
                        opts.keep_r = true;
                    } else if mode.allow_no_runtime() && arg == "--no-runtime" {
                        opts.no_runtime = true;
                    } else if arg == "--preserve-all-defs" || arg == "--preserve-all-def" {
                        opts.preserve_all_defs = true;
                    } else if arg == "--no-incremental" {
                        opts.incremental = IncrementalOptions::disabled();
                    } else if arg == "--cold" || arg == "--cold-compile" {
                        opts.cold_compile = true;
                    } else if arg == "--incremental" {
                        opts.incremental = IncrementalOptions::auto();
                    } else if let Some(raw) = arg.strip_prefix("--incremental=") {
                        let Some(parsed) = parse_incremental_phases(raw) else {
                            ui.error(
                                "Invalid --incremental value. Use auto|off|1|1,2|1,2,3|all|phase1,phase2,phase3",
                            );
                            return Err(1);
                        };
                        opts.incremental = parsed;
                    } else if arg == "--incremental-phases" {
                        if i + 1 >= args.len() {
                            ui.error(
                                "Missing value after --incremental-phases (e.g. 1,2,3 or off)",
                            );
                            return Err(1);
                        }
                        i += 1;
                        let Some(parsed) = parse_incremental_phases(&args[i]) else {
                            ui.error(
                                "Invalid --incremental-phases value. Use auto|off|1|1,2|1,2,3|all",
                            );
                            return Err(1);
                        };
                        opts.incremental = parsed;
                    } else if arg == "--strict-incremental-verify" {
                        if !opts.incremental.enabled {
                            opts.incremental = IncrementalOptions::auto();
                        } else {
                            opts.incremental.enabled = true;
                        }
                        if !opts.incremental.auto
                            && !opts.incremental.phase1
                            && !opts.incremental.phase2
                            && !opts.incremental.phase3
                        {
                            opts.incremental.phase1 = true;
                        }
                        opts.incremental.strict_verify = true;
                    } else if arg == "--profile-compile" {
                        opts.profile_compile = true;
                    } else if arg == "--profile-compile-out" {
                        if i + 1 >= args.len() {
                            ui.error("Missing value after --profile-compile-out");
                            return Err(1);
                        }
                        i += 1;
                        opts.profile_compile = true;
                        opts.profile_compile_out = Some(args[i].clone());
                    } else if arg == "--compile-mode" {
                        if i + 1 >= args.len() {
                            ui.error("Missing value after --compile-mode");
                            return Err(1);
                        }
                        i += 1;
                        opts.compile_mode = match args[i].trim().to_ascii_lowercase().as_str() {
                            "standard" | "release" => CompileMode::Standard,
                            "fast-dev" | "fast" | "dev" => CompileMode::FastDev,
                            _ => {
                                ui.error("Invalid --compile-mode. Use standard or fast-dev.");
                                return Err(1);
                            }
                        };
                        opts.compile_mode_explicit = true;
                    } else if matches!(mode, CommandMode::Watch) && arg == "--once" {
                        opts.watch_once = true;
                    } else if matches!(mode, CommandMode::Watch) && arg == "--poll-ms" {
                        if i + 1 >= args.len() {
                            ui.error("Missing value after --poll-ms");
                            return Err(1);
                        }
                        i += 1;
                        let Ok(ms) = args[i].trim().parse::<u64>() else {
                            ui.error("Invalid --poll-ms. Use a positive integer.");
                            return Err(1);
                        };
                        if ms == 0 {
                            ui.error("--poll-ms must be >= 1");
                            return Err(1);
                        }
                        opts.watch_poll_ms = ms;
                    } else if mode.allow_legacy_mir() && arg == "--mir" {
                        if matches!(opts.opt_level, OptLevel::O0) {
                            opts.opt_level = OptLevel::O1;
                        }
                    } else if !arg.starts_with('-') {
                        opts.target = arg.clone();
                    } else {
                        ui.error(&format!("Unknown option: {}", arg));
                        return Err(1);
                    }
                }
                Err(code) => return Err(code),
            }
        }
        i += 1;
    }

    if matches!(mode, CommandMode::Build)
        && !opts.compile_mode_explicit
        && matches!(opts.opt_level, OptLevel::O2)
    {
        opts.compile_mode = CompileMode::Standard;
    }

    Ok(opts)
}
