use crate::syntax::parse::Parser;
use crate::typeck::{NativeBackend, TypeConfig, TypeMode};
use rustc_hash::{FxHashMap, FxHashSet};
use std::env;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptLevel {
    O0,
    O1,
    O2,
}

impl OptLevel {
    fn is_optimized(self) -> bool {
        !matches!(self, Self::O0)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::O0 => "O0",
            Self::O1 => "O1",
            Self::O2 => "O2",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParallelMode {
    Off,
    Optional,
    Required,
}

impl ParallelMode {
    pub fn from_str(v: &str) -> Option<Self> {
        match v.trim().to_ascii_lowercase().as_str() {
            "off" => Some(Self::Off),
            "optional" => Some(Self::Optional),
            "required" => Some(Self::Required),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Optional => "optional",
            Self::Required => "required",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParallelBackend {
    Auto,
    R,
    OpenMp,
}

impl ParallelBackend {
    pub fn from_str(v: &str) -> Option<Self> {
        match v.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "r" => Some(Self::R),
            "openmp" => Some(Self::OpenMp),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::R => "r",
            Self::OpenMp => "openmp",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ParallelConfig {
    pub mode: ParallelMode,
    pub backend: ParallelBackend,
    pub threads: usize,
    pub min_trip: usize,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            mode: ParallelMode::Off,
            backend: ParallelBackend::Auto,
            threads: 0,
            min_trip: 4096,
        }
    }
}

pub struct CliLog {
    color: bool,
    detailed: bool,
}

fn type_config_from_env() -> TypeConfig {
    let mode = env::var("RR_TYPE_MODE")
        .ok()
        .and_then(|v| TypeMode::from_str(&v))
        .unwrap_or(TypeMode::Strict);
    let native_backend = env::var("RR_NATIVE_BACKEND")
        .ok()
        .and_then(|v| NativeBackend::from_str(&v))
        .unwrap_or(NativeBackend::Off);
    TypeConfig {
        mode,
        native_backend,
    }
}

fn parse_nonnegative_usize_env(key: &str) -> Option<usize> {
    env::var(key)
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
}

fn parallel_config_from_env() -> ParallelConfig {
    let mode = env::var("RR_PARALLEL_MODE")
        .ok()
        .and_then(|v| ParallelMode::from_str(&v))
        .unwrap_or(ParallelMode::Off);
    let backend = env::var("RR_PARALLEL_BACKEND")
        .ok()
        .and_then(|v| ParallelBackend::from_str(&v))
        .unwrap_or(ParallelBackend::Auto);
    let threads = parse_nonnegative_usize_env("RR_PARALLEL_THREADS").unwrap_or(0);
    let min_trip = parse_nonnegative_usize_env("RR_PARALLEL_MIN_TRIP").unwrap_or(4096);
    ParallelConfig {
        mode,
        backend,
        threads,
        min_trip,
    }
}

impl CliLog {
    pub fn new() -> Self {
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

    pub fn dim(&self, text: &str) -> String {
        self.style("2", text)
    }

    pub fn red_bold(&self, text: &str) -> String {
        self.style("1;91", text)
    }

    pub fn yellow_bold(&self, text: &str) -> String {
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

    pub fn white_bold(&self, text: &str) -> String {
        self.style("1;97", text)
    }

    fn banner(&self, input: &str, level: OptLevel) {
        println!(
            "{} {}",
            self.yellow_bold("[+]"),
            self.red_bold("RR Tachyon v3.0")
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

    pub fn success(&self, msg: &str) {
        println!("{} {}", self.green_bold("✔"), self.white_bold(msg));
    }
    pub fn warn(&self, msg: &str) {
        eprintln!("{} {}", self.yellow_bold("!"), self.yellow_bold(msg));
    }

    pub fn error(&self, msg: &str) {
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

fn escape_r_string(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn normalize_module_path(path: &Path) -> PathBuf {
    if let Ok(canon) = fs::canonicalize(path) {
        return canon;
    }
    if path.is_absolute() {
        path.to_path_buf()
    } else if let Ok(cwd) = env::current_dir() {
        cwd.join(path)
    } else {
        path.to_path_buf()
    }
}

pub fn compile(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
) -> crate::error::RR<(String, Vec<crate::codegen::mir_emit::MapEntry>)> {
    compile_with_configs(
        entry_path,
        entry_input,
        opt_level,
        type_config_from_env(),
        parallel_config_from_env(),
    )
}

pub fn compile_with_config(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
) -> crate::error::RR<(String, Vec<crate::codegen::mir_emit::MapEntry>)> {
    compile_with_configs(
        entry_path,
        entry_input,
        opt_level,
        type_cfg,
        parallel_config_from_env(),
    )
}

pub fn compile_with_configs(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
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
    let mut loaded_paths: FxHashSet<PathBuf> = FxHashSet::default();
    let mut queue = std::collections::VecDeque::new();

    // Normalize entry path
    let entry_abs = normalize_module_path(Path::new(entry_path));
    loaded_paths.insert(entry_abs.clone());
    queue.push_back((entry_abs, entry_input.to_string(), 0)); // (path, content, mod_id)

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
    let mut global_symbols = FxHashMap::default();
    let mut load_errors: Vec<crate::error::RRException> = Vec::new();

    while let Some((curr_path, content, mod_id)) = queue.pop_front() {
        let curr_path_str = curr_path.to_string_lossy().to_string();
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
                let curr_dir = curr_path.parent().unwrap_or(Path::new("."));
                let target = normalize_module_path(&curr_dir.join(import_path));

                // Simple cycle detection / deduplication
                if !loaded_paths.contains(&target) {
                    let target_lossy = target.to_string_lossy().to_string();
                    ui.trace("import", &target_lossy);
                    match fs::read_to_string(&target) {
                        Ok(content) => {
                            loaded_paths.insert(target.clone());
                            queue.push_back((target, content, next_mod_id));
                            next_mod_id += 1;
                        }
                        Err(e) => {
                            return Err(crate::error::RRException::new(
                                "RR.ParseError",
                                crate::error::RRCode::E0001,
                                crate::error::Stage::Parse,
                                format!("failed to load imported module '{}': {}", target_lossy, e),
                            ));
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
    let mut all_fns = FxHashMap::default();
    let mut emit_order: Vec<String> = Vec::new();
    let mut top_level_calls: Vec<String> = Vec::new();
    let mut known_fn_arities: FxHashMap<String, usize> = FxHashMap::default();

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
                local_names: FxHashMap::default(),
                public: false,
            };
            let lowerer = crate::mir::lower_hir::MirLowerer::new(
                top_fn_name.clone(),
                Vec::new(),
                FxHashMap::default(),
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

    crate::typeck::solver::analyze_program(&mut all_fns, type_cfg)?;
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
            "Budget: IR {}/{} | MaxFn {}/{} | AlwaysFns {} | OptimizedFns {} | SkippedFns {}{}",
            pulse_stats.total_program_ir,
            pulse_stats.full_opt_ir_limit,
            pulse_stats.max_function_ir,
            pulse_stats.full_opt_fn_limit,
            pulse_stats.always_tier_functions,
            pulse_stats.optimized_functions,
            pulse_stats.skipped_functions,
            if pulse_stats.selective_budget_mode {
                " | selective"
            } else {
                ""
            }
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
    with_runtime.push_str(&format!(
        "rr_set_type_mode(\"{}\");\n",
        type_cfg.mode.as_str()
    ));
    with_runtime.push_str(&format!(
        "rr_set_native_backend(\"{}\");\n",
        type_cfg.native_backend.as_str()
    ));
    with_runtime.push_str(&format!(
        "rr_set_parallel_mode(\"{}\");\n",
        parallel_cfg.mode.as_str()
    ));
    with_runtime.push_str(&format!(
        "rr_set_parallel_backend(\"{}\");\n",
        parallel_cfg.backend.as_str()
    ));
    with_runtime.push_str(&format!(
        "rr_set_parallel_threads({});\n",
        parallel_cfg.threads
    ));
    with_runtime.push_str(&format!(
        "rr_set_parallel_min_trip({});\n",
        parallel_cfg.min_trip
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
