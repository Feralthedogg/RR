use RR::compiler::{
    CliLog, CompileMode, CompileOutputOptions, CompileProfile, CompilerParallelConfig,
    CompilerParallelMode, IncrementalCompileOutput, IncrementalOptions, IncrementalSession,
    IncrementalStats, OptLevel, ParallelBackend, ParallelConfig, ParallelMode,
    compile_with_configs_incremental_with_output_options_and_compiler_parallel_and_profile,
    compile_with_configs_with_options_and_compiler_parallel_and_profile,
    default_compiler_parallel_config, default_parallel_config, default_type_config, json_escape,
    module_tree_fingerprint, module_tree_snapshot,
};
use RR::error::{RRCode, RRException, Stage};
use RR::runtime::runner::Runner;
use RR::syntax::ast::{Expr, ExprKind, Stmt, StmtKind};
use RR::syntax::parse::Parser;
use RR::typeck::{NativeBackend, TypeConfig, TypeMode};
use rustc_hash::FxHashMap;
use std::any::Any;
use std::env;
use std::fs;
use std::io::ErrorKind;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

#[path = "main_compile.rs"]
mod main_compile;
#[path = "main_pkg.rs"]
mod main_pkg;
#[path = "main_project.rs"]
mod main_project;
#[path = "main_registry.rs"]
mod main_registry;

use self::main_compile::*;
use self::main_pkg::*;
use self::main_project::*;
use self::main_registry::*;

fn main() {
    install_broken_pipe_panic_hook();
    let result = std::panic::catch_unwind(run_cli);
    match result {
        Ok(code) => {
            if code != 0 {
                std::process::exit(code);
            }
        }
        Err(payload) => {
            if panic_payload_is_broken_pipe(payload.as_ref()) {
                std::process::exit(0);
            }
            std::panic::resume_unwind(payload);
        }
    }
}

fn run_cli() -> i32 {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return 0;
    }

    match args[1].as_str() {
        "__rr_poly_isl_materialize" => {
            RR::mir::opt::poly::run_hidden_poly_cli(&args[2..]).unwrap_or(2)
        }
        "--version" | "-V" | "version" => {
            print_version();
            0
        }
        "--help" | "-h" | "help" => {
            print_usage();
            0
        }
        "new" => cmd_new(&args[2..]),
        "init" => cmd_init(&args[2..]),
        "install" => cmd_install(&args[2..]),
        "remove" => cmd_remove(&args[2..]),
        "outdated" => cmd_outdated(&args[2..]),
        "update" => cmd_update(&args[2..]),
        "publish" => cmd_publish(&args[2..]),
        "search" => cmd_search(&args[2..]),
        "registry" => cmd_registry(&args[2..]),
        "mod" => cmd_mod(&args[2..]),
        "build" => cmd_build(&args[2..]),
        "run" => cmd_run(&args[2..]),
        "watch" => cmd_watch(&args[2..]),
        _ => cmd_legacy(&args[1..]),
    }
}

fn install_broken_pipe_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if panic_payload_is_broken_pipe(info.payload()) {
            return;
        }
        default_hook(info);
    }));
}

fn panic_payload_is_broken_pipe(payload: &(dyn Any + Send)) -> bool {
    if let Some(msg) = payload.downcast_ref::<&str>() {
        msg.contains("Broken pipe")
    } else if let Some(msg) = payload.downcast_ref::<String>() {
        msg.contains("Broken pipe")
    } else {
        false
    }
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  RR --version");
    eprintln!("  RR version");
    eprintln!("  RR <input.rr> [options]");
    eprintln!("  RR new [--bin|--lib] <module-path|.> [dir|.]");
    eprintln!("  RR init [--bin|--lib] [module-path]");
    eprintln!("  RR install <github-url|module-path>[@version]");
    eprintln!("  RR remove <module-path>");
    eprintln!("  RR outdated");
    eprintln!("  RR update [module-path]");
    eprintln!(
        "  RR publish <version> [--dry-run] [--allow-dirty] [--push-tag] [--remote <name>] [--registry <dir>]"
    );
    eprintln!("  RR search <query> [--registry <dir>]");
    eprintln!("  RR registry keygen [identity] [--out-dir <dir>]");
    eprintln!(
        "  RR registry onboard [identity] [--out-dir <dir>] [--require-signed] [--require-approval] [--auto-approve] [--registry <dir>]"
    );
    eprintln!("  RR registry list [--registry <dir>]");
    eprintln!("  RR registry report [module-path] [--registry <dir>]");
    eprintln!("  RR registry diff <module-path> <from-version> <to-version> [--registry <dir>]");
    eprintln!(
        "  RR registry risk <module-path> <version> [--against <version>] [--registry <dir>]"
    );
    eprintln!("  RR registry channel show <module-path> [--registry <dir>]");
    eprintln!("  RR registry channel set <module-path> <channel> <version> [--registry <dir>]");
    eprintln!("  RR registry channel clear <module-path> <channel> [--registry <dir>]");
    eprintln!("  RR registry queue [--registry <dir>]");
    eprintln!(
        "  RR registry audit [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]"
    );
    eprintln!(
        "  RR registry audit export <file> [--format <tsv|jsonl>] [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]"
    );
    eprintln!(
        "  RR registry policy bootstrap <trusted-public-key> [--signer <identity>] [--auto-approve-signer <identity>] [--require-signed] [--require-approval] [--registry <dir>]"
    );
    eprintln!("  RR registry policy show [--registry <dir>]");
    eprintln!("  RR registry policy lint [--registry <dir>]");
    eprintln!(
        "  RR registry policy rotate-key <old-public-key> <new-public-key> [--registry <dir>]"
    );
    eprintln!("  RR registry policy apply <file> [--registry <dir>]");
    eprintln!("  RR registry info <module-path> [--registry <dir>]");
    eprintln!("  RR registry approve <module-path> <version> [--registry <dir>]");
    eprintln!("  RR registry unapprove <module-path> <version> [--registry <dir>]");
    eprintln!("  RR registry promote <module-path> <version> [--registry <dir>]");
    eprintln!("  RR registry yank <module-path> <version> [--registry <dir>]");
    eprintln!("  RR registry unyank <module-path> <version> [--registry <dir>]");
    eprintln!("  RR registry deprecate <module-path> <message> [--registry <dir>]");
    eprintln!("  RR registry undeprecate <module-path> [--registry <dir>]");
    eprintln!("  RR registry verify [module-path] [--registry <dir>]");
    eprintln!("  RR mod graph");
    eprintln!("  RR mod why <module-path>");
    eprintln!("  RR mod verify");
    eprintln!("  RR mod tidy");
    eprintln!("  RR mod vendor");
    eprintln!("  RR run [entry.rr|dir|.] [options]");
    eprintln!("  RR build [dir|file.rr] [options]");
    eprintln!("  RR watch [entry.rr|dir|.] [options]");
    eprintln!("Options:");
    eprintln!("  -o <file> / --out-dir <dir>   Output file (legacy) or build output dir");
    eprintln!("  -O0, -O1, -O2                 Optimization level (default O1)");
    eprintln!("  -o0, -o1, -o2                 (Also accepted) Optimization level");
    eprintln!("  --bin                         Scaffold a binary project for RR new/init");
    eprintln!("  --lib                         Scaffold a library project for RR new/init");
    eprintln!("  --signer <identity>           Registry policy bootstrap signer allowlist entry");
    eprintln!("  --auto-approve-signer <identity>  Registry policy bootstrap auto-approval signer");
    eprintln!("  --auto-approve               Registry onboard: auto-approve the generated signer");
    eprintln!("  --action <kind>             Registry audit action filter");
    eprintln!("  --module <path>             Registry audit module filter");
    eprintln!("  --contains <text>           Registry audit substring filter");
    eprintln!("  --format <tsv|jsonl>        Registry audit export output format");
    eprintln!("  --type-mode <strict|gradual>  Static typing mode (default strict)");
    eprintln!("  --native-backend <off|optional|required>  Native intrinsic backend mode");
    eprintln!("  --parallel-mode <off|optional|required>   Parallel execution mode");
    eprintln!("  --parallel-backend <auto|r|openmp>        Parallel backend selection");
    eprintln!("  --parallel-threads <N>                    Parallel worker threads (0=auto)");
    eprintln!("  --parallel-min-trip <N>                   Minimum trip-count for parallel path");
    eprintln!(
        "  --compiler-parallel-mode <off|auto|on>    Compiler scheduling mode (default auto)"
    );
    eprintln!(
        "  --compiler-parallel-threads <N>           Compiler worker threads (0=auto, default)"
    );
    eprintln!(
        "  --compiler-parallel-min-functions <N>     Minimum functions before compiler parallelism"
    );
    eprintln!(
        "  --compiler-parallel-min-fn-ir <N>         Minimum aggregate IR before compiler parallelism"
    );
    eprintln!(
        "  --compiler-parallel-max-jobs <N>          Maximum concurrent compiler jobs (0=threads)"
    );
    eprintln!("  --strict-let <on|off>                     Require explicit let before assignment");
    eprintln!("  --warn-implicit-decl <on|off>             Warn on legacy implicit declaration");
    eprintln!("  --incremental[=auto|off|1|1,2|1,2,3|all] Enable incremental compile phases");
    eprintln!("  --incremental-phases <...>                Same as above (separate arg form)");
    eprintln!("  --no-incremental                          Disable automatic incremental compile");
    eprintln!("  --cold                                   Bypass warm compile caches for this compile");
    eprintln!(
        "  --strict-incremental-verify               Extra validation gate for incremental mode"
    );
    eprintln!(
        "  --profile-compile                         Emit compile profile JSON for this compile"
    );
    eprintln!("  --profile-compile-out <file>              Write compile profile JSON to a file");
    eprintln!("  --compile-mode <standard|fast-dev>        Compiler pass profile selection");
    eprintln!("  --poll-ms <N>                             Watch polling interval in milliseconds");
    eprintln!("  --once                                    Run a single watch tick and exit");
    eprintln!("  --keep-r                      Keep generated .gen.R when running");
    eprintln!("  --no-runtime                  Emit helper-only R without source/native bootstrap");
    eprintln!("  --preserve-all-defs          Keep all top-level Sym_* definitions in emitted R");
    eprintln!("  --preserve-all-def           Alias for --preserve-all-defs");
}

fn print_version() {
    println!("RR Tachyon v{}", env!("CARGO_PKG_VERSION"));
}

#[derive(Clone, Debug)]
struct TargetResolutionError {
    message: String,
    help: Option<String>,
}

fn stable_hash_bytes(bytes: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET_BASIS;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn watch_output_hash(content: &str) -> u64 {
    stable_hash_bytes(content.as_bytes())
}

fn watch_output_matches_hash(path: &Path, expected_hash: Option<u64>) -> bool {
    let Some(expected_hash) = expected_hash else {
        return false;
    };
    fs::read_to_string(path)
        .map(|content| watch_output_hash(&content) == expected_hash)
        .unwrap_or(false)
}

fn apply_opt_flag(arg: &str, level: &mut OptLevel) -> bool {
    if arg == "-O0" || arg == "-o0" {
        *level = OptLevel::O0;
        true
    } else if arg == "-O1" || arg == "-o1" {
        *level = OptLevel::O1;
        true
    } else if arg == "-O2" || arg == "-O" || arg == "-o2" {
        *level = OptLevel::O2;
        true
    } else {
        false
    }
}

fn parse_nonnegative_usize(raw: &str) -> Option<usize> {
    raw.trim().parse::<usize>().ok()
}

fn parse_bool_flag(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommonCompileFlag {
    TypeMode,
    NativeBackend,
    ParallelMode,
    ParallelBackend,
    ParallelThreads,
    ParallelMinTrip,
    CompilerParallelMode,
    CompilerParallelThreads,
    CompilerParallelMinFunctions,
    CompilerParallelMinFnIr,
    CompilerParallelMaxJobs,
    StrictLet,
    WarnImplicitDecl,
}

impl CommonCompileFlag {
    fn from_arg(arg: &str) -> Option<Self> {
        match arg {
            "--type-mode" => Some(Self::TypeMode),
            "--native-backend" => Some(Self::NativeBackend),
            "--parallel-mode" => Some(Self::ParallelMode),
            "--parallel-backend" => Some(Self::ParallelBackend),
            "--parallel-threads" => Some(Self::ParallelThreads),
            "--parallel-min-trip" => Some(Self::ParallelMinTrip),
            "--compiler-parallel-mode" => Some(Self::CompilerParallelMode),
            "--compiler-parallel-threads" => Some(Self::CompilerParallelThreads),
            "--compiler-parallel-min-functions" => Some(Self::CompilerParallelMinFunctions),
            "--compiler-parallel-min-fn-ir" => Some(Self::CompilerParallelMinFnIr),
            "--compiler-parallel-max-jobs" => Some(Self::CompilerParallelMaxJobs),
            "--strict-let" => Some(Self::StrictLet),
            "--warn-implicit-decl" => Some(Self::WarnImplicitDecl),
            _ => None,
        }
    }

    fn missing_value_error(self) -> &'static str {
        match self {
            Self::TypeMode => "Missing value after --type-mode (strict|gradual)",
            Self::NativeBackend => "Missing value after --native-backend (off|optional|required)",
            Self::ParallelMode => "Missing value after --parallel-mode (off|optional|required)",
            Self::ParallelBackend => "Missing value after --parallel-backend (auto|r|openmp)",
            Self::ParallelThreads => "Missing value after --parallel-threads",
            Self::ParallelMinTrip => "Missing value after --parallel-min-trip",
            Self::CompilerParallelMode => {
                "Missing value after --compiler-parallel-mode (off|auto|on)"
            }
            Self::CompilerParallelThreads => "Missing value after --compiler-parallel-threads",
            Self::CompilerParallelMinFunctions => {
                "Missing value after --compiler-parallel-min-functions"
            }
            Self::CompilerParallelMinFnIr => "Missing value after --compiler-parallel-min-fn-ir",
            Self::CompilerParallelMaxJobs => "Missing value after --compiler-parallel-max-jobs",
            Self::StrictLet => "Missing value after --strict-let (on|off)",
            Self::WarnImplicitDecl => "Missing value after --warn-implicit-decl (on|off)",
        }
    }
}

fn next_flag_value<'a>(args: &'a [String], i: &mut usize, _ui: &CliLog) -> Result<&'a str, i32> {
    if *i + 1 >= args.len() {
        return Err(1);
    }
    *i += 1;
    Ok(&args[*i])
}

struct CommonCompileFlagState<'a> {
    opt_level: &'a mut OptLevel,
    type_cfg: &'a mut TypeConfig,
    parallel_cfg: &'a mut ParallelConfig,
    compiler_parallel_cfg: &'a mut CompilerParallelConfig,
    strict_let: &'a mut bool,
    warn_implicit_decl: &'a mut bool,
}

fn apply_common_compile_flags(
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

    let v = match next_flag_value(args, i, ui) {
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
                    ui.error("Invalid --type-mode. Use strict|gradual");
                    return Err(1);
                }
            };
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandMode {
    Legacy,
    Run,
    Build,
    Watch,
}

impl CommandMode {
    fn default_target(self) -> &'static str {
        match self {
            Self::Legacy => "",
            Self::Run | Self::Build | Self::Watch => ".",
        }
    }

    fn default_output_path(self) -> Option<String> {
        None
    }

    fn takes_output_arg(self, arg: &str) -> bool {
        match self {
            Self::Legacy => arg == "-o",
            Self::Build => arg == "--out-dir" || arg == "-o",
            Self::Run => false,
            Self::Watch => arg == "-o",
        }
    }

    fn allow_keep_r(self) -> bool {
        matches!(self, Self::Legacy | Self::Run)
    }

    fn allow_no_runtime(self) -> bool {
        matches!(self, Self::Legacy)
    }

    fn allow_legacy_mir(self) -> bool {
        matches!(self, Self::Legacy)
    }
}

#[derive(Clone, Debug)]
struct CommonOpts {
    target: String,
    output_path: Option<String>,
    keep_r: bool,
    no_runtime: bool,
    preserve_all_defs: bool,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    compiler_parallel_cfg: CompilerParallelConfig,
    strict_let: bool,
    warn_implicit_decl: bool,
    incremental: IncrementalOptions,
    cold_compile: bool,
    profile_compile: bool,
    profile_compile_out: Option<String>,
    compile_mode: CompileMode,
    compile_mode_explicit: bool,
    watch_poll_ms: u64,
    watch_once: bool,
}

impl CommonOpts {
    fn new(mode: CommandMode) -> Self {
        Self {
            target: mode.default_target().to_string(),
            output_path: mode.default_output_path(),
            keep_r: false,
            no_runtime: false,
            preserve_all_defs: false,
            opt_level: OptLevel::O1,
            type_cfg: default_type_config(),
            parallel_cfg: default_parallel_config(),
            compiler_parallel_cfg: default_compiler_parallel_config(),
            strict_let: true,
            warn_implicit_decl: false,
            incremental: IncrementalOptions::auto(),
            cold_compile: false,
            profile_compile: false,
            profile_compile_out: None,
            compile_mode: match mode {
                CommandMode::Legacy => CompileMode::Standard,
                CommandMode::Run | CommandMode::Build | CommandMode::Watch => CompileMode::FastDev,
            },
            compile_mode_explicit: false,
            watch_poll_ms: 500,
            watch_once: false,
        }
    }
}

fn parse_incremental_phases(raw: &str) -> Option<IncrementalOptions> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() || matches!(normalized.as_str(), "auto" | "on" | "true") {
        return Some(IncrementalOptions::auto());
    }
    if matches!(normalized.as_str(), "off" | "0" | "false" | "none") {
        return Some(IncrementalOptions::disabled());
    }
    if matches!(normalized.as_str(), "all" | "3") {
        return Some(IncrementalOptions::all_phases());
    }
    if matches!(normalized.as_str(), "1" | "phase1") {
        return Some(IncrementalOptions::phase1_only());
    }

    let mut options = IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: false,
        phase2: false,
        phase3: false,
        strict_verify: false,
    };
    for token in normalized.split(',') {
        if !parse_incremental_phase_token(token.trim(), &mut options) {
            return None;
        }
    }
    if !options.phase1 && !options.phase2 && !options.phase3 {
        return None;
    }
    Some(options)
}

fn parse_incremental_phase_token(token: &str, options: &mut IncrementalOptions) -> bool {
    match token {
        "1" | "phase1" => options.phase1 = true,
        "2" | "phase2" => options.phase2 = true,
        "3" | "phase3" => options.phase3 = true,
        _ => return false,
    }
    true
}

fn parse_command_opts(args: &[String], mode: CommandMode, ui: &CliLog) -> Result<CommonOpts, i32> {
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

fn write_compile_profile_artifact(
    ui: &CliLog,
    profile: &CompileProfile,
    out_path: Option<&str>,
) -> Result<(), i32> {
    let json = profile.to_json_string();
    if let Some(out_path) = out_path {
        let path = PathBuf::from(out_path);
        if let Some(parent) = path.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            report_dir_create_failure(ui, parent, &e, "compile profile output directory");
            return Err(1);
        }
        if let Err(e) = fs::write(&path, json) {
            report_file_write_failure(ui, &path, &e, "compile profile output path");
            return Err(1);
        }
        ui.success(&format!("Compile profile -> {}", path.display()));
    } else {
        eprintln!("{json}");
    }
    Ok(())
}

fn compile_profile_collection_to_json(entries: &[(String, CompileProfile)]) -> String {
    let mut out = String::from(
        "{\n  \"schema\": \"rr-compile-profile-collection\",\n  \"version\": 1,\n  \"profiles\": [\n",
    );
    for (idx, (input, profile)) in entries.iter().enumerate() {
        if idx > 0 {
            out.push_str(",\n");
        }
        out.push_str("    {\"input\": \"");
        out.push_str(&json_escape(input));
        out.push_str("\", \"profile\": ");
        out.push_str(&profile.to_json_string());
        out.push('}');
    }
    out.push_str("\n  ]\n}");
    out
}

fn write_compile_profile_collection(
    ui: &CliLog,
    entries: &[(String, CompileProfile)],
    out_path: Option<&str>,
) -> Result<(), i32> {
    let json = compile_profile_collection_to_json(entries);
    if let Some(out_path) = out_path {
        let path = PathBuf::from(out_path);
        if let Some(parent) = path.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            report_dir_create_failure(ui, parent, &e, "compile profile collection directory");
            return Err(1);
        }
        if let Err(e) = fs::write(&path, json) {
            report_file_write_failure(ui, &path, &e, "compile profile collection path");
            return Err(1);
        }
        ui.success(&format!("Compile profile -> {}", path.display()));
    } else {
        eprintln!("{json}");
    }
    Ok(())
}

fn cmd_legacy(args: &[String]) -> i32 {
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

    let output_opts = CompileOutputOptions {
        inject_runtime: !opts.no_runtime,
        preserve_all_defs: opts.preserve_all_defs,
        strict_let: opts.strict_let,
        warn_implicit_decl: opts.warn_implicit_decl,
        compile_mode: opts.compile_mode,
    };
    let mut compile_profile = opts.profile_compile.then(CompileProfile::default);
    let result = with_compile_cache_override(opts.cold_compile, || {
        if opts.incremental.enabled {
            compile_with_configs_incremental_with_output_options_and_compiler_parallel_and_profile(
                &input_path_str,
                &input,
                opts.opt_level,
                opts.type_cfg,
                opts.parallel_cfg,
                opts.compiler_parallel_cfg,
                opts.incremental,
                output_opts,
                None,
                compile_profile.as_mut(),
            )
            .map(|v| (v.r_code, v.source_map))
        } else {
            compile_with_configs_with_options_and_compiler_parallel_and_profile(
                &input_path_str,
                &input,
                opts.opt_level,
                opts.type_cfg,
                opts.parallel_cfg,
                opts.compiler_parallel_cfg,
                output_opts,
                compile_profile.as_mut(),
            )
        }
    });
    match result {
        Ok((r_code, source_map)) => {
            if let Some(profile) = compile_profile.as_ref()
                && let Err(code) = write_compile_profile_artifact(
                    &ui,
                    profile,
                    opts.profile_compile_out.as_deref(),
                )
            {
                return code;
            }
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

struct ScopedCompileCacheOverride {
    previous: Option<std::ffi::OsString>,
    temp_root: PathBuf,
}

impl Drop for ScopedCompileCacheOverride {
    fn drop(&mut self) {
        // SAFETY: The CLI runs this override synchronously around a single
        // compile invocation. We restore the previous process environment
        // immediately afterward.
        unsafe {
            if let Some(previous) = self.previous.as_ref() {
                env::set_var("RR_INCREMENTAL_CACHE_DIR", previous);
            } else {
                env::remove_var("RR_INCREMENTAL_CACHE_DIR");
            }
        }
        let _ = fs::remove_dir_all(&self.temp_root);
    }
}

fn with_compile_cache_override<T>(cold_compile: bool, f: impl FnOnce() -> T) -> T {
    if !cold_compile {
        return f();
    }
    static COLD_CACHE_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let seq = COLD_CACHE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_root = env::temp_dir().join(format!(
        "rr-cold-compile-{}-{}",
        std::process::id(),
        seq
    ));
    let _ = fs::remove_dir_all(&temp_root);
    fs::create_dir_all(&temp_root).expect("failed to create cold compile cache dir");
    let previous = env::var_os("RR_INCREMENTAL_CACHE_DIR");
    // SAFETY: The CLI applies this override only for the duration of one
    // compile call and restores the previous value immediately after.
    unsafe {
        env::set_var("RR_INCREMENTAL_CACHE_DIR", &temp_root);
    }
    let _guard = ScopedCompileCacheOverride {
        previous,
        temp_root,
    };
    f()
}
