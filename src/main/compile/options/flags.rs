use rr::compiler::{
    CliLog, CompilerParallelConfig, CompilerParallelMode, NativeBackend, OptLevel, ParallelBackend,
    ParallelConfig, ParallelMode, TypeConfig, TypeMode,
};
use std::env;

use super::flag_defs::{
    CommonCompileFlag, apply_opt_flag, next_flag_value, parse_bool_flag, parse_nonnegative_usize,
};

pub(super) struct CommonCompileFlagState<'a> {
    pub(super) opt_level: &'a mut OptLevel,
    pub(super) type_cfg: &'a mut TypeConfig,
    pub(super) parallel_cfg: &'a mut ParallelConfig,
    pub(super) compiler_parallel_cfg: &'a mut CompilerParallelConfig,
    pub(super) strict_let: &'a mut bool,
    pub(super) warn_implicit_decl: &'a mut bool,
}

pub(super) fn apply_common_compile_flags(
    args: &[String],
    i: &mut usize,
    state: &mut CommonCompileFlagState<'_>,
    ui: &CliLog,
) -> Result<bool, i32> {
    let arg = &args[*i];
    if apply_opt_flag(arg, state.opt_level) {
        return Ok(true);
    }
    let Some(flag) = CommonCompileFlag::from_arg(arg) else {
        return Ok(false);
    };

    let v = match next_flag_value(args, i) {
        Ok(value) => value,
        Err(code) => {
            ui.error(flag.missing_value_error());
            return Err(code);
        }
    };

    match flag {
        CommonCompileFlag::TypeMode => {
            state.type_cfg.mode = match v.parse::<TypeMode>() {
                Ok(m) => m,
                Err(()) => {
                    ui.error("Invalid --type-mode. Use strict");
                    return Err(1);
                }
            };
            if state.type_cfg.mode == TypeMode::Gradual
                && env::var_os("RR_ALLOW_GRADUAL_TYPE_MODE").is_none()
            {
                ui.error("--type-mode gradual was removed from the stable RR 2.0 CLI");
                ui.warn(
                    "use strict mode, or set RR_ALLOW_GRADUAL_TYPE_MODE=1 for temporary compatibility",
                );
                return Err(1);
            }
        }
        CommonCompileFlag::NativeBackend => {
            state.type_cfg.native_backend = match v.parse::<NativeBackend>() {
                Ok(m) => m,
                Err(()) => {
                    ui.error("Invalid --native-backend. Use off|optional|required");
                    return Err(1);
                }
            };
        }
        CommonCompileFlag::ParallelMode => {
            state.parallel_cfg.mode = match v.parse::<ParallelMode>() {
                Ok(m) => m,
                Err(()) => {
                    ui.error("Invalid --parallel-mode. Use off|optional|required");
                    return Err(1);
                }
            };
        }
        CommonCompileFlag::ParallelBackend => {
            state.parallel_cfg.backend = match v.parse::<ParallelBackend>() {
                Ok(m) => m,
                Err(()) => {
                    ui.error("Invalid --parallel-backend. Use auto|r|openmp");
                    return Err(1);
                }
            };
        }
        CommonCompileFlag::ParallelThreads => {
            state.parallel_cfg.threads = match parse_nonnegative_usize(v) {
                Some(n) => n,
                None => {
                    ui.error("Invalid --parallel-threads. Use a non-negative integer.");
                    return Err(1);
                }
            };
        }
        CommonCompileFlag::ParallelMinTrip => {
            state.parallel_cfg.min_trip = match parse_nonnegative_usize(v) {
                Some(n) => n,
                None => {
                    ui.error("Invalid --parallel-min-trip. Use a non-negative integer.");
                    return Err(1);
                }
            };
        }
        CommonCompileFlag::CompilerParallelMode => {
            state.compiler_parallel_cfg.mode = match v.parse::<CompilerParallelMode>() {
                Ok(m) => m,
                Err(()) => {
                    ui.error("Invalid --compiler-parallel-mode. Use off|auto|on");
                    return Err(1);
                }
            };
        }
        CommonCompileFlag::CompilerParallelThreads => {
            state.compiler_parallel_cfg.threads = match parse_nonnegative_usize(v) {
                Some(n) => n,
                None => {
                    ui.error("Invalid --compiler-parallel-threads. Use a non-negative integer.");
                    return Err(1);
                }
            };
        }
        CommonCompileFlag::CompilerParallelMinFunctions => {
            state.compiler_parallel_cfg.min_functions = match parse_nonnegative_usize(v) {
                Some(n) => n,
                None => {
                    ui.error(
                        "Invalid --compiler-parallel-min-functions. Use a non-negative integer.",
                    );
                    return Err(1);
                }
            };
        }
        CommonCompileFlag::CompilerParallelMinFnIr => {
            state.compiler_parallel_cfg.min_fn_ir = match parse_nonnegative_usize(v) {
                Some(n) => n,
                None => {
                    ui.error("Invalid --compiler-parallel-min-fn-ir. Use a non-negative integer.");
                    return Err(1);
                }
            };
        }
        CommonCompileFlag::CompilerParallelMaxJobs => {
            state.compiler_parallel_cfg.max_jobs = match parse_nonnegative_usize(v) {
                Some(n) => n,
                None => {
                    ui.error("Invalid --compiler-parallel-max-jobs. Use a non-negative integer.");
                    return Err(1);
                }
            };
        }
        CommonCompileFlag::StrictLet => {
            *state.strict_let = match parse_bool_flag(v) {
                Some(value) => value,
                None => {
                    ui.error("Invalid --strict-let. Use on|off.");
                    return Err(1);
                }
            };
            if !*state.strict_let && env::var_os("RR_ALLOW_LEGACY_IMPLICIT_DECL").is_none() {
                ui.error("--strict-let off was removed from the stable RR 2.0 CLI");
                ui.warn(
                    "add explicit `let` bindings, or set RR_ALLOW_LEGACY_IMPLICIT_DECL=1 while migrating",
                );
                return Err(1);
            }
        }
        CommonCompileFlag::WarnImplicitDecl => {
            *state.warn_implicit_decl = match parse_bool_flag(v) {
                Some(value) => value,
                None => {
                    ui.error("Invalid --warn-implicit-decl. Use on|off.");
                    return Err(1);
                }
            };
        }
    }
    Ok(true)
}
