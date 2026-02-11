mod codegen;
mod error;
mod hir;
mod mir;
mod runtime;
mod syntax;
mod utils;

use crate::runtime::runner::Runner;
use crate::syntax::parse::Parser;
use std::env;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OptLevel {
    O0,
    O1,
    O2,
}

impl OptLevel {
    fn is_optimized(self) -> bool {
        !matches!(self, Self::O0)
    }

    fn label(self) -> &'static str {
        match self {
            Self::O0 => "O0",
            Self::O1 => "O1",
            Self::O2 => "O2",
        }
    }
}

struct CliLog {
    color: bool,
    detailed: bool,
}

impl CliLog {
    fn new() -> Self {
        let is_tty = std::io::stdout().is_terminal();
        let no_color = env::var_os("NO_COLOR").is_some();
        let force_color = env::var_os("RR_FORCE_COLOR").is_some();
        let force_verbose = env::var_os("RR_VERBOSE_LOG").is_some();
        Self {
            color: ((is_tty && !no_color) || (force_color && !no_color)),
            detailed: is_tty || force_verbose,
        }
    }

    fn style(&self, code: &str, text: &str) -> String {
        if self.color {
            format!("\x1b[{}m{}\x1b[0m", code, text)
        } else {
            text.to_string()
        }
    }

    fn dim(&self, text: &str) -> String {
        self.style("2", text)
    }

    fn red_bold(&self, text: &str) -> String {
        self.style("1;91", text)
    }

    fn yellow_bold(&self, text: &str) -> String {
        self.style("1;93", text)
    }

    fn green_bold(&self, text: &str) -> String {
        self.style("1;92", text)
    }

    fn cyan_bold(&self, text: &str) -> String {
        self.style("1;96", text)
    }

    fn magenta_bold(&self, text: &str) -> String {
        self.style("1;95", text)
    }

    fn white_bold(&self, text: &str) -> String {
        self.style("1;97", text)
    }

    fn banner(&self, input: &str, level: OptLevel) {
        println!(
            "{} {}",
            self.yellow_bold("[+]"),
            self.red_bold("RR Tachyon v2.0")
        );
        println!(
            " {} {}",
            self.dim("└─"),
            self.white_bold(&format!("Input: {} ({})", input, level.label()))
        );
    }

    fn step_start(&self, idx: usize, total: usize, title: &str, detail: &str) -> Instant {
        let tag = format!("[{}/{}]", idx, total);
        println!(
            "{} {} {} {}",
            self.cyan_bold("=>"),
            self.magenta_bold(&tag),
            self.red_bold(&format!("{:<20}", title)),
            self.yellow_bold(detail)
        );
        Instant::now()
    }

    fn step_line_ok(&self, detail: &str) {
        println!("   {} {}", self.green_bold("✓"), self.white_bold(detail));
    }

    fn trace(&self, label: &str, detail: &str) {
        if self.detailed {
            println!(
                "   {} {} {}",
                self.dim("*"),
                self.dim(label),
                self.dim(detail)
            );
        }
    }

    fn pulse_success(&self, total: Duration) {
        println!(
            "{} {} {}",
            self.green_bold("✔"),
            self.green_bold("Tachyon Pulse Successful in"),
            self.green_bold(&format_duration(total))
        );
    }

    fn success(&self, msg: &str) {
        println!("{} {}", self.green_bold("✔"), self.white_bold(msg));
    }
    fn warn(&self, msg: &str) {
        eprintln!("{} {}", self.yellow_bold("!"), self.yellow_bold(msg));
    }

    fn error(&self, msg: &str) {
        eprintln!("{} {}", self.red_bold("x"), self.red_bold(msg));
    }
}

fn format_duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.2}s", d.as_secs_f64())
    }
}

fn human_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else {
        format!("{:.0}KB", (bytes as f64) / 1024.0)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    let code = match args[1].as_str() {
        "build" => cmd_build(&args[2..]),
        "run" => cmd_run(&args[2..]),
        _ => cmd_legacy(&args[1..]),
    };

    if code != 0 {
        std::process::exit(code);
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

fn cmd_legacy(args: &[String]) -> i32 {
    let ui = CliLog::new();
    let mut input_path = String::new();
    let mut output_path = None;
    let mut keep_r = false;
    let mut opt_level = OptLevel::O1;
    let mut no_runtime = false;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "-o" {
            if i + 1 < args.len() {
                output_path = Some(args[i + 1].clone());
                i += 1;
            } else {
                ui.error("Missing output file after -o");
                return 1;
            }
        } else if apply_opt_flag(arg, &mut opt_level) {
            // handled
        } else if arg == "--keep-r" {
            keep_r = true;
        } else if arg == "--no-runtime" {
            no_runtime = true;
        } else if arg == "--mir" {
            if matches!(opt_level, OptLevel::O0) {
                opt_level = OptLevel::O1;
            }
        } else if !arg.starts_with('-') {
            input_path = arg.clone();
        }
        i += 1;
    }

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

    let result = compile(&input_path, &input, opt_level, no_runtime);
    match result {
        Ok((r_code, source_map)) => {
            if let Some(out_path) = output_path {
                if let Err(e) = fs::write(&out_path, &r_code) {
                    ui.error(&format!(
                        "Failed to write output file '{}': {}",
                        out_path, e
                    ));
                    return 1;
                }
                ui.success(&format!("Compiled to {}", out_path));
                0
            } else if !no_runtime {
                Runner::run(&input_path, &input, &r_code, &source_map, None, keep_r)
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
    let mut target = ".".to_string();
    let mut keep_r = false;
    let mut opt_level = OptLevel::O1;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if apply_opt_flag(arg, &mut opt_level) {
            // handled
        } else if arg == "--keep-r" {
            keep_r = true;
        } else if !arg.starts_with('-') {
            target = arg.clone();
        }
        i += 1;
    }

    let input_path = match resolve_run_input(&target) {
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

    match compile(&input_path_str, &input, opt_level, false) {
        Ok((r_code, source_map)) => {
            Runner::run(&input_path_str, &input, &r_code, &source_map, None, keep_r)
        }
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
    let mut target = ".".to_string();
    let mut out_dir = "build".to_string();
    let mut opt_level = OptLevel::O1;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--out-dir" || arg == "-o" {
            if i + 1 < args.len() {
                out_dir = args[i + 1].clone();
                i += 1;
            } else {
                ui.error(&format!("Missing directory path after {}", arg));
                return 1;
            }
        } else if apply_opt_flag(arg, &mut opt_level) {
            // handled
        } else if !arg.starts_with('-') {
            target = arg.clone();
        }
        i += 1;
    }

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
        ui.dim("└─"),
        ui.white_bold(&format!(
            "Target: {} | Out: {} ({})",
            target,
            out_dir,
            opt_level.label()
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
        fs::canonicalize(&target_path).unwrap_or(target_path.clone())
    } else {
        PathBuf::new()
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

        let (r_code, _source_map) = match compile(&rr_path_str, &input, opt_level, true) {
            Ok(v) => v,
            Err(e) => {
                e.display(Some(&input), Some(&rr_path_str));
                return 1;
            }
        };

        let out_file = if dir_mode {
            let rel = rr_abs.strip_prefix(&root_abs).unwrap_or(&rr_abs);
            out_root.join(rel).with_extension("R")
        } else {
            let stem = rr.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
            out_root.join(format!("{}.R", stem))
        };

        if let Some(parent) = out_file.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                ui.error(&format!("Failed to create '{}': {}", parent.display(), e));
                return 1;
            }
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

fn escape_r_string(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn compile(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    _no_runtime: bool,
) -> crate::error::RR<(String, Vec<crate::codegen::mir_emit::MapEntry>)> {
    let ui = CliLog::new();
    let compile_started = Instant::now();
    let optimize = opt_level.is_optimized();
    const TOTAL_STEPS: usize = 6;
    let input_label = std::path::Path::new(entry_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(entry_path);
    ui.banner(input_label, opt_level);

    // Module Loader State
    let mut loaded_paths = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();

    // Normalize entry path
    let entry_abs =
        fs::canonicalize(entry_path).unwrap_or_else(|_| std::path::PathBuf::from(entry_path));
    let entry_str = entry_abs.to_string_lossy().to_string();

    loaded_paths.insert(entry_str.clone());
    queue.push_back((entry_str, entry_input.to_string(), 0)); // (path, content, mod_id)

    // Helper for generating IDs
    let mut next_mod_id = 1;

    // 1. & 2. Loading Loop (Parse + Lower)
    let step_load = ui.step_start(
        1,
        TOTAL_STEPS,
        "Source Analysis",
        "parse + scope resolution",
    );
    let mut hir_modules = Vec::new();
    let mut hir_lowerer = crate::hir::lower::Lowerer::new();
    let mut global_symbols = std::collections::HashMap::new();
    let mut load_errors: Vec<crate::error::RRException> = Vec::new();

    while let Some((curr_path_str, content, mod_id)) = queue.pop_front() {
        ui.trace(&format!("module#{}", mod_id), &curr_path_str);

        let mut parser = Parser::new(&content);
        let ast_prog = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                load_errors.push(e);
                continue;
            }
        };

        let (hir_mod, symbols) =
            match hir_lowerer.lower_module(ast_prog, crate::hir::def::ModuleId(mod_id as u32)) {
                Ok(v) => v,
                Err(e) => {
                    load_errors.push(e);
                    continue;
                }
            };
        for w in hir_lowerer.take_warnings() {
            ui.warn(&format!("{}: {}", curr_path_str, w));
        }
        global_symbols.extend(symbols);

        // Scan for imports
        for item in &hir_mod.items {
            if let crate::hir::def::HirItem::Import(imp) = item {
                // Resolve path (imp.module is String)
                let import_path = &imp.module;
                let curr_dir = std::path::Path::new(&curr_path_str)
                    .parent()
                    .unwrap_or(std::path::Path::new("."));
                let target = curr_dir.join(import_path);

                // Simple cycle detection / deduplication
                let target_lossy = target.to_string_lossy().to_string();
                if !loaded_paths.contains(&target_lossy) {
                    ui.trace("import", &target_lossy);
                    match fs::read_to_string(&target) {
                        Ok(content) => {
                            loaded_paths.insert(target_lossy.clone());
                            queue.push_back((target_lossy, content, next_mod_id));
                            next_mod_id += 1;
                        }
                        Err(e) => {
                            ui.error(&format!(
                                "Failed to load imported module '{}': {}",
                                target_lossy, e
                            ));
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
        hir_modules.push(hir_mod);
    }
    if !load_errors.is_empty() {
        if load_errors.len() == 1 {
            return Err(load_errors.remove(0));
        }
        return Err(crate::error::RRException::aggregate(
            "RR.ParseError",
            crate::error::RRCode::E0001,
            crate::error::Stage::Parse,
            format!("source analysis failed: {} error(s)", load_errors.len()),
            load_errors,
        ));
    }
    ui.step_line_ok(&format!(
        "Loaded {} module(s) in {}",
        hir_modules.len(),
        format_duration(step_load.elapsed())
    ));

    let hir_prog = crate::hir::def::HirProgram {
        modules: hir_modules,
    };

    // 3. Desugar
    let step_desugar = ui.step_start(
        2,
        TOTAL_STEPS,
        "Canonicalization",
        "normalize HIR structure",
    );
    let mut desugarer = crate::hir::desugar::Desugarer::new();
    let desugared_hir = desugarer.desugar_program(hir_prog)?;
    ui.step_line_ok(&format!(
        "Desugared {} module(s) in {}",
        desugared_hir.modules.len(),
        format_duration(step_desugar.elapsed())
    ));

    let mut final_output = String::new();
    let mut final_source_map = Vec::new();

    // 4. MIR Lowering
    let step_ssa = ui.step_start(
        3,
        TOTAL_STEPS,
        "SSA Graph Synthesis",
        "build dominator tree & phi nodes",
    );
    let mut all_fns = std::collections::HashMap::new();
    let mut emit_order: Vec<String> = Vec::new();
    let mut top_level_calls: Vec<String> = Vec::new();
    let mut known_fn_arities: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for module in &desugared_hir.modules {
        for item in &module.items {
            if let crate::hir::def::HirItem::Fn(f) = item {
                if let Some(name) = global_symbols.get(&f.name).cloned() {
                    known_fn_arities.insert(name, f.params.len());
                }
            }
        }
    }

    for module in desugared_hir.modules {
        let mut top_level_stmts: Vec<crate::hir::def::HirStmt> = Vec::new();

        for item in module.items {
            match item {
                crate::hir::def::HirItem::Fn(f) => {
                    let fn_name = format!("Sym_{}", f.name.0);
                    let params: Vec<String> = f
                        .params
                        .iter()
                        .map(|p| global_symbols[&p.name].clone())
                        .collect();
                    let var_names = f
                        .local_names
                        .clone()
                        .into_iter()
                        .map(|(id, name)| (id, name))
                        .collect();

                    let lowerer = crate::mir::lower_hir::MirLowerer::new(
                        fn_name.clone(),
                        params,
                        var_names,
                        &global_symbols,
                        &known_fn_arities,
                    );
                    let fn_ir = match lowerer.lower_fn(f) {
                        Ok(ir) => ir,
                        Err(e) => return Err(e),
                    };
                    all_fns.insert(fn_name.clone(), fn_ir);
                    emit_order.push(fn_name);
                }
                crate::hir::def::HirItem::Stmt(s) => top_level_stmts.push(s),
                _ => {}
            }
        }

        if !top_level_stmts.is_empty() {
            let top_fn_name = format!("Sym_top_{}", module.id.0);
            let top_fn = crate::hir::def::HirFn {
                id: crate::hir::def::FnId(1_000_000 + module.id.0),
                name: crate::hir::def::SymbolId(1_000_000 + module.id.0),
                params: Vec::new(),
                has_varargs: false,
                ret_ty: None,
                body: crate::hir::def::HirBlock {
                    stmts: top_level_stmts,
                    span: crate::utils::Span::default(),
                },
                attrs: crate::hir::def::HirFnAttrs {
                    inline_hint: crate::hir::def::InlineHint::Never,
                    tidy_safe: false,
                },
                span: crate::utils::Span::default(),
                local_names: std::collections::HashMap::new(),
                public: false,
            };
            let lowerer = crate::mir::lower_hir::MirLowerer::new(
                top_fn_name.clone(),
                Vec::new(),
                std::collections::HashMap::new(),
                &global_symbols,
                &known_fn_arities,
            );
            let fn_ir = match lowerer.lower_fn(top_fn) {
                Ok(ir) => ir,
                Err(e) => return Err(e),
            };
            all_fns.insert(top_fn_name.clone(), fn_ir);
            emit_order.push(top_fn_name.clone());
            top_level_calls.push(top_fn_name);
        }
    }
    ui.step_line_ok(&format!(
        "Synthesized {} MIR functions in {}",
        all_fns.len(),
        format_duration(step_ssa.elapsed())
    ));
    crate::mir::semantics::validate_program(&all_fns)?;
    crate::mir::semantics::validate_runtime_safety(&all_fns)?;

    // 5. Optimization & Codegen
    let tachyon = crate::mir::opt::TachyonEngine::new();
    let step_opt = ui.step_start(
        4,
        TOTAL_STEPS,
        if optimize {
            "Tachyon Optimization"
        } else {
            "Tachyon Stabilization"
        },
        if optimize {
            "execute aggressive passes"
        } else {
            "execute safe stabilization passes"
        },
    );
    for fn_ir in all_fns.values() {
        if fn_ir.unsupported_dynamic {
            if fn_ir.fallback_reasons.is_empty() {
                ui.warn(&format!(
                    "Hybrid fallback enabled for {} (dynamic feature)",
                    fn_ir.name
                ));
            } else {
                ui.warn(&format!(
                    "Hybrid fallback enabled for {}: {}",
                    fn_ir.name,
                    fn_ir.fallback_reasons.join(", ")
                ));
            }
        }
    }
    let mut pulse_stats = crate::mir::opt::TachyonPulseStats::default();
    if optimize {
        pulse_stats = tachyon.run_program_with_stats(&mut all_fns);
    } else {
        tachyon.stabilize_for_codegen(&mut all_fns);
    }
    crate::mir::semantics::validate_program(&all_fns)?;
    crate::mir::semantics::validate_runtime_safety(&all_fns)?;
    if optimize {
        ui.step_line_ok(&format!(
            "Vectorized: {} | Reduced: {} | Simplified: {} loops",
            pulse_stats.vectorized, pulse_stats.reduced, pulse_stats.simplified_loops
        ));
        ui.step_line_ok(&format!(
            "Passes: SCCP {} | GVN {} | LICM {} | BCE {} | TCO {} | DCE {}",
            pulse_stats.sccp_hits,
            pulse_stats.gvn_hits,
            pulse_stats.licm_hits,
            pulse_stats.bce_hits,
            pulse_stats.tco_hits,
            pulse_stats.dce_hits
        ));
        ui.step_line_ok(&format!(
            "Infra: Intrinsics {} | FreshAlloc {} | Simplify {} | Inline rounds {} | De-SSA {}",
            pulse_stats.intrinsics_hits,
            pulse_stats.fresh_alloc_hits,
            pulse_stats.simplify_hits,
            pulse_stats.inline_rounds,
            pulse_stats.de_ssa_hits
        ));
        ui.step_line_ok(&format!(
            "Finished in {}",
            format_duration(step_opt.elapsed())
        ));
    } else {
        ui.step_line_ok(&format!(
            "Stabilized {} MIR functions in {}",
            all_fns.len(),
            format_duration(step_opt.elapsed())
        ));
    }

    let step_emit = ui.step_start(
        5,
        TOTAL_STEPS,
        "R Code Emission",
        "reconstruct control flow",
    );
    for fn_name in &emit_order {
        if let Some(fn_ir) = all_fns.get(fn_name) {
            let (code, map) = crate::codegen::mir_emit::MirEmitter::new().emit(fn_ir)?;
            final_output.push_str(&code);
            final_output.push('\n');
            final_source_map.extend(map);
        }
    }
    ui.step_line_ok(&format!(
        "Emitted {} functions ({} debug maps) in {}",
        emit_order.len(),
        final_source_map.len(),
        format_duration(step_emit.elapsed())
    ));

    let step_runtime = ui.step_start(
        6,
        TOTAL_STEPS,
        "Runtime Injection",
        "link static analysis guards",
    );

    for call in top_level_calls {
        final_output.push_str(&format!("{}()\n", call));
    }

    // Prepend runtime so generated .R is self-contained.
    let mut with_runtime = String::new();
    with_runtime.push_str(crate::runtime::R_RUNTIME);
    if !with_runtime.ends_with('\n') {
        with_runtime.push('\n');
    }
    let source_label = std::path::Path::new(entry_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(entry_path);
    with_runtime.push_str(&format!(
        "rr_set_source(\"{}\");\n",
        escape_r_string(source_label)
    ));
    with_runtime.push_str(&final_output);
    ui.step_line_ok(&format!("Output size: {}", human_size(with_runtime.len())));
    ui.trace(
        "runtime",
        &format!("linked in {}", format_duration(step_runtime.elapsed())),
    );
    ui.pulse_success(compile_started.elapsed());

    Ok((with_runtime, final_source_map))
}
