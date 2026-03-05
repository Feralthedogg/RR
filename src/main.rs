use RR::compiler::{
    CliLog, OptLevel, ParallelBackend, ParallelConfig, ParallelMode, compile_with_configs,
    parallel_config_from_env, type_config_from_env,
};
use RR::runtime::runner::Runner;
use RR::typeck::{NativeBackend, TypeConfig, TypeMode};
use std::any::Any;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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
        "build" => cmd_build(&args[2..]),
        "run" => cmd_run(&args[2..]),
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
    eprintln!("  rr <input.rr> [options]");
    eprintln!("  rr run [main.rr|dir|.] [options]");
    eprintln!("  rr build [dir|file.rr] [options]");
    eprintln!("Options:");
    eprintln!("  -o <file> / --out-dir <dir>   Output file (legacy) or build output dir");
    eprintln!("  -O0, -O1, -O2                 Optimization level (default O1)");
    eprintln!("  -o0, -o1, -o2                 (Also accepted) Optimization level");
    eprintln!("  --type-mode <strict|gradual>  Static typing mode (default strict)");
    eprintln!("  --native-backend <off|optional|required>  Native intrinsic backend mode");
    eprintln!("  --parallel-mode <off|optional|required>   Parallel execution mode");
    eprintln!("  --parallel-backend <auto|r|openmp>        Parallel backend selection");
    eprintln!("  --parallel-threads <N>                    Parallel worker threads (0=auto)");
    eprintln!("  --parallel-min-trip <N>                   Minimum trip-count for parallel path");
    eprintln!("  --keep-r                      Keep generated .gen.R when running");
    eprintln!("  --no-runtime                  Compile only (legacy mode)");
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommonCompileFlag {
    TypeMode,
    NativeBackend,
    ParallelMode,
    ParallelBackend,
    ParallelThreads,
    ParallelMinTrip,
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

fn apply_common_compile_flags(
    args: &[String],
    i: &mut usize,
    opt_level: &mut OptLevel,
    type_cfg: &mut TypeConfig,
    parallel_cfg: &mut ParallelConfig,
    ui: &CliLog,
) -> Result<bool, i32> {
    let arg = &args[*i];
    if apply_opt_flag(arg, opt_level) {
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
            type_cfg.mode = match v.parse::<TypeMode>() {
                Ok(m) => m,
                Err(()) => {
                    ui.error("Invalid --type-mode. Use strict|gradual");
                    return Err(1);
                }
            };
        }
        CommonCompileFlag::NativeBackend => {
            type_cfg.native_backend = match v.parse::<NativeBackend>() {
                Ok(m) => m,
                Err(()) => {
                    ui.error("Invalid --native-backend. Use off|optional|required");
                    return Err(1);
                }
            };
        }
        CommonCompileFlag::ParallelMode => {
            parallel_cfg.mode = match v.parse::<ParallelMode>() {
                Ok(m) => m,
                Err(()) => {
                    ui.error("Invalid --parallel-mode. Use off|optional|required");
                    return Err(1);
                }
            };
        }
        CommonCompileFlag::ParallelBackend => {
            parallel_cfg.backend = match v.parse::<ParallelBackend>() {
                Ok(m) => m,
                Err(()) => {
                    ui.error("Invalid --parallel-backend. Use auto|r|openmp");
                    return Err(1);
                }
            };
        }
        CommonCompileFlag::ParallelThreads => {
            parallel_cfg.threads = match parse_nonnegative_usize(v) {
                Some(n) => n,
                None => {
                    ui.error("Invalid --parallel-threads. Use a non-negative integer.");
                    return Err(1);
                }
            };
        }
        CommonCompileFlag::ParallelMinTrip => {
            parallel_cfg.min_trip = match parse_nonnegative_usize(v) {
                Some(n) => n,
                None => {
                    ui.error("Invalid --parallel-min-trip. Use a non-negative integer.");
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
}

impl CommandMode {
    fn default_target(self) -> &'static str {
        match self {
            Self::Legacy => "",
            Self::Run | Self::Build => ".",
        }
    }

    fn default_output_path(self) -> Option<String> {
        match self {
            Self::Build => Some("build".to_string()),
            _ => None,
        }
    }

    fn takes_output_arg(self, arg: &str) -> bool {
        match self {
            Self::Legacy => arg == "-o",
            Self::Build => arg == "--out-dir" || arg == "-o",
            Self::Run => false,
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
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
}

impl CommonOpts {
    fn new(mode: CommandMode) -> Self {
        Self {
            target: mode.default_target().to_string(),
            output_path: mode.default_output_path(),
            keep_r: false,
            no_runtime: false,
            opt_level: OptLevel::O1,
            type_cfg: type_config_from_env(),
            parallel_cfg: parallel_config_from_env(),
        }
    }
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
            match apply_common_compile_flags(
                args,
                &mut i,
                &mut opts.opt_level,
                &mut opts.type_cfg,
                &mut opts.parallel_cfg,
                ui,
            ) {
                Ok(true) => {}
                Ok(false) => {
                    if mode.allow_keep_r() && arg == "--keep-r" {
                        opts.keep_r = true;
                    } else if mode.allow_no_runtime() && arg == "--no-runtime" {
                        opts.no_runtime = true;
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

    Ok(opts)
}

fn cmd_legacy(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let opts = match parse_command_opts(args, CommandMode::Legacy, &ui) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let input_path = opts.target;

    if input_path.is_empty() {
        print_usage();
        return 1;
    }
    if !input_path.ends_with(".rr") {
        ui.error("Input file must end with .rr");
        return 1;
    }

    let input = match fs::read_to_string(&input_path) {
        Ok(s) => s,
        Err(e) => {
            ui.error(&format!(
                "Failed to read input file '{}': {}",
                input_path, e
            ));
            return 1;
        }
    };

    let result = compile_with_configs(
        &input_path,
        &input,
        opts.opt_level,
        opts.type_cfg,
        opts.parallel_cfg,
    );
    match result {
        Ok((r_code, source_map)) => {
            if let Some(out_path) = opts.output_path {
                if let Err(e) = fs::write(&out_path, &r_code) {
                    ui.error(&format!(
                        "Failed to write output file '{}': {}",
                        out_path, e
                    ));
                    return 1;
                }
                ui.success(&format!("Compiled to {}", out_path));
                0
            } else if !opts.no_runtime {
                Runner::run(&input_path, &input, &r_code, &source_map, None, opts.keep_r)
            } else {
                ui.success("Compilation successful (runtime skipped)");
                0
            }
        }
        Err(e) => {
            e.display(Some(&input), Some(&input_path));
            1
        }
    }
}

fn resolve_run_input(raw: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw);
    if path.is_dir() || raw == "." {
        let entry = path.join("main.rr");
        if entry.is_file() {
            Ok(entry)
        } else {
            Err(format!("main.rr not found in '{}'", path.to_string_lossy()))
        }
    } else if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("rr") {
            Ok(path)
        } else {
            Err("run target must be a .rr file or directory".to_string())
        }
    } else {
        Err(format!("run target not found: '{}'", raw))
    }
}

fn cmd_run(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let opts = match parse_command_opts(args, CommandMode::Run, &ui) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let input_path = match resolve_run_input(&opts.target) {
        Ok(p) => p,
        Err(msg) => {
            ui.error(&msg);
            return 1;
        }
    };
    let input_path_str = input_path.to_string_lossy().to_string();
    let input = match fs::read_to_string(&input_path) {
        Ok(s) => s,
        Err(e) => {
            ui.error(&format!("Failed to read '{}': {}", input_path_str, e));
            return 1;
        }
    };

    match compile_with_configs(
        &input_path_str,
        &input,
        opts.opt_level,
        opts.type_cfg,
        opts.parallel_cfg,
    ) {
        Ok((r_code, source_map)) => Runner::run(
            &input_path_str,
            &input,
            &r_code,
            &source_map,
            None,
            opts.keep_r,
        ),
        Err(e) => {
            e.display(Some(&input), Some(&input_path_str));
            1
        }
    }
}

fn collect_rr_files(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name == "build" || name == "target" || name == ".git" {
                continue;
            }
            collect_rr_files(&path, files)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rr") {
            files.push(path);
        }
    }
    Ok(())
}

fn cmd_build(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let opts = match parse_command_opts(args, CommandMode::Build, &ui) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let target = opts.target;
    let out_dir = opts.output_path.unwrap_or_else(|| "build".to_string());

    let target_path = PathBuf::from(&target);
    if !target_path.exists() {
        ui.error(&format!("build target not found: '{}'", target));
        return 1;
    }

    let out_root = PathBuf::from(&out_dir);
    if let Err(e) = fs::create_dir_all(&out_root) {
        ui.error(&format!(
            "Failed to create output directory '{}': {}",
            out_dir, e
        ));
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
    if dir_mode {
        if let Err(e) = collect_rr_files(&target_path, &mut rr_files) {
            ui.error(&format!("Failed while scanning '{}': {}", target, e));
            return 1;
        }
    } else if target_path.extension().and_then(|s| s.to_str()) == Some("rr") {
        rr_files.push(target_path.clone());
    } else {
        ui.error("build target must be a directory or .rr file");
        return 1;
    }

    rr_files.sort();
    if rr_files.is_empty() {
        ui.error(&format!("no .rr files found under '{}'", target));
        return 1;
    }

    let root_abs = if dir_mode {
        fs::canonicalize(&target_path).ok()
    } else {
        None
    };

    let mut built = 0usize;
    for rr in rr_files {
        let rr_abs = fs::canonicalize(&rr).unwrap_or(rr.clone());
        let rr_path_str = rr_abs.to_string_lossy().to_string();
        let input = match fs::read_to_string(&rr_abs) {
            Ok(s) => s,
            Err(e) => {
                ui.error(&format!("Failed to read '{}': {}", rr_path_str, e));
                return 1;
            }
        };

        let (r_code, _source_map) = match compile_with_configs(
            &rr_path_str,
            &input,
            opts.opt_level,
            opts.type_cfg,
            opts.parallel_cfg,
        ) {
            Ok(v) => v,
            Err(e) => {
                e.display(Some(&input), Some(&rr_path_str));
                return 1;
            }
        };

        let out_file = if dir_mode {
            let rel = rr
                .strip_prefix(&target_path)
                .ok()
                .filter(|p| !p.as_os_str().is_empty())
                .map(Path::to_path_buf)
                .or_else(|| {
                    root_abs.as_ref().and_then(|root| {
                        rr_abs
                            .strip_prefix(root)
                            .ok()
                            .filter(|p| !p.as_os_str().is_empty())
                            .map(Path::to_path_buf)
                    })
                })
                .or_else(|| rr.file_name().map(PathBuf::from))
                .unwrap_or_else(|| PathBuf::from("out.rr"));
            out_root.join(rel).with_extension("R")
        } else {
            let stem = rr.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
            out_root.join(format!("{}.R", stem))
        };

        if let Some(parent) = out_file.parent()
            && let Err(e) = fs::create_dir_all(parent) {
                ui.error(&format!("Failed to create '{}': {}", parent.display(), e));
                return 1;
            }
        if let Err(e) = fs::write(&out_file, r_code) {
            ui.error(&format!("Failed to write '{}': {}", out_file.display(), e));
            return 1;
        }

        ui.success(&format!("Built {} -> {}", rr.display(), out_file.display()));
        built += 1;
    }

    ui.success(&format!(
        "Build complete: {} file(s) -> {}",
        built,
        out_root.display()
    ));
    0
}
