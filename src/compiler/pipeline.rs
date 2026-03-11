use crate::codegen::mir_emit::MapEntry;
use crate::error::{InternalCompilerError, Stage};
use crate::syntax::parse::Parser;
use crate::typeck::{NativeBackend, TypeConfig, TypeMode};
use rustc_hash::{FxHashMap, FxHashSet};
use std::env;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
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
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::R => "r",
            Self::OpenMp => "openmp",
        }
    }
}

impl std::str::FromStr for ParallelMode {
    type Err = ();

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        match v.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "optional" => Ok(Self::Optional),
            "required" => Ok(Self::Required),
            _ => Err(()),
        }
    }
}

impl std::str::FromStr for ParallelBackend {
    type Err = ();

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        match v.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "r" => Ok(Self::R),
            "openmp" => Ok(Self::OpenMp),
            _ => Err(()),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompileOutputOptions {
    pub inject_runtime: bool,
}

impl Default for CompileOutputOptions {
    fn default() -> Self {
        Self {
            inject_runtime: true,
        }
    }
}

pub struct CliLog {
    color: bool,
    quiet: bool,
    detailed: bool,
    slow_step_ms: usize,
    slow_step_repeat_ms: usize,
}

struct CliStep {
    started: Instant,
    stop_tx: Option<mpsc::Sender<()>>,
    watcher: Option<thread::JoinHandle<()>>,
}

impl CliStep {
    fn new(started: Instant) -> Self {
        Self {
            started,
            stop_tx: None,
            watcher: None,
        }
    }

    fn with_watcher(
        started: Instant,
        stop_tx: mpsc::Sender<()>,
        watcher: thread::JoinHandle<()>,
    ) -> Self {
        Self {
            started,
            stop_tx: Some(stop_tx),
            watcher: Some(watcher),
        }
    }

    fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }
}

impl Drop for CliStep {
    fn drop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.watcher.take() {
            let _ = handle.join();
        }
    }
}

pub fn type_config_from_env() -> TypeConfig {
    let mode = env::var("RR_TYPE_MODE")
        .ok()
        .and_then(|v| v.parse::<TypeMode>().ok())
        .unwrap_or(TypeMode::Strict);
    let native_backend = env::var("RR_NATIVE_BACKEND")
        .ok()
        .and_then(|v| v.parse::<NativeBackend>().ok())
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

fn env_truthy(key: &str) -> bool {
    env::var(key)
        .ok()
        .map(|raw| {
            matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

pub fn parallel_config_from_env() -> ParallelConfig {
    let mode = env::var("RR_PARALLEL_MODE")
        .ok()
        .and_then(|v| v.parse::<ParallelMode>().ok())
        .unwrap_or(ParallelMode::Off);
    let backend = env::var("RR_PARALLEL_BACKEND")
        .ok()
        .and_then(|v| v.parse::<ParallelBackend>().ok())
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

impl Default for CliLog {
    fn default() -> Self {
        Self::new()
    }
}

impl CliLog {
    pub fn new() -> Self {
        let is_tty = std::io::stdout().is_terminal();
        let no_color = env::var_os("NO_COLOR").is_some();
        let force_color = env::var_os("RR_FORCE_COLOR").is_some();
        let force_verbose = env::var_os("RR_VERBOSE_LOG").is_some();
        let quiet = env_truthy("RR_QUIET_LOG");
        let slow_step_ms = parse_nonnegative_usize_env("RR_SLOW_STEP_MS").unwrap_or(3000);
        let slow_step_repeat_ms = parse_nonnegative_usize_env("RR_SLOW_STEP_REPEAT_MS")
            .unwrap_or(6000)
            .max(200);
        Self {
            color: (force_color || is_tty) && !no_color,
            quiet,
            detailed: !quiet && (is_tty || force_verbose),
            slow_step_ms,
            slow_step_repeat_ms,
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
        if self.quiet {
            return;
        }
        println!(
            "{} {}",
            self.yellow_bold("[+]"),
            self.red_bold(&format!("RR Tachyon v{}", env!("CARGO_PKG_VERSION")))
        );
        println!(
            " {} {}",
            self.dim("|-"),
            self.white_bold(&format!("Input: {} ({})", input, level.label()))
        );
    }

    fn slow_step_hint(title: &str) -> &'static str {
        match title {
            "Source Analysis" => "module import graph and parse recovery are in progress.",
            "Canonicalization" => {
                "desugaring and HIR normalization may be processing large bodies."
            }
            "SSA Graph Synthesis" => {
                "lowering + phi placement is still running on large control-flow graphs."
            }
            "Tachyon Optimization" => {
                "pass fixed-point search is active; final pass/budget stats print when this step completes."
            }
            "R Code Emission" => "function emission and source-map stitching are still running.",
            "Runtime Injection" => {
                "runtime prelude merge and final output assembly are in progress."
            }
            _ => "",
        }
    }

    fn slow_step_verbose_hint(title: &str) -> &'static str {
        match title {
            "Tachyon Optimization" => {
                "enable RR_VERBOSE_LOG=1 and RR_VECTORIZE_TRACE=1 for pass-level trace details."
            }
            "SSA Graph Synthesis" => {
                "large CFG lowering can dominate here; keep RR_VERBOSE_LOG=1 to see module/function progress."
            }
            "R Code Emission" => {
                "this can spike on very large generated functions; cache-aware emission stats print at completion."
            }
            _ => "set RR_VERBOSE_LOG=1 for additional progress details.",
        }
    }

    fn step_start(&self, idx: usize, total: usize, title: &str, detail: &str) -> CliStep {
        if self.quiet {
            return CliStep::new(Instant::now());
        }
        let tag = format!("[{}/{}]", idx, total);
        println!(
            "{} {} {} {}",
            self.cyan_bold("=>"),
            self.magenta_bold(&tag),
            self.red_bold(&format!("{:<20}", title)),
            self.yellow_bold(detail)
        );
        let started = Instant::now();
        if !self.detailed || self.slow_step_ms == 0 {
            return CliStep::new(started);
        }

        let slow_after = Duration::from_millis(self.slow_step_ms as u64);
        let repeat_after = Duration::from_millis(self.slow_step_repeat_ms as u64);
        let slow_prefix = format!(
            "   {} {}",
            self.yellow_bold("[slow]"),
            self.yellow_bold(title)
        );
        let slow_detail = format!(
            "   {} {}",
            self.dim("*"),
            self.dim(&format!("detail: {}", detail))
        );
        let hint = Self::slow_step_hint(title).to_string();
        let verbose_hint = Self::slow_step_verbose_hint(title).to_string();
        let slow_hint = if hint.is_empty() {
            None
        } else {
            Some(format!(
                "   {} {}",
                self.dim("*"),
                self.dim(&format!("hint: {}", hint))
            ))
        };
        let slow_extra_prefix = format!("   {} {}", self.dim("*"), self.dim("extra:"));

        let (tx, rx) = mpsc::channel::<()>();
        let watcher = thread::spawn(move || {
            let mut next_wait = slow_after;
            let mut printed_context = false;
            let mut timeout_count = 0usize;
            loop {
                match rx.recv_timeout(next_wait) {
                    Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        timeout_count += 1;
                        println!(
                            "{} still running ({})",
                            slow_prefix,
                            format_duration(started.elapsed())
                        );
                        if !printed_context {
                            println!("{}", slow_detail);
                            if let Some(h) = &slow_hint {
                                println!("{}", h);
                            }
                            printed_context = true;
                        } else {
                            println!(
                                "{} {} (repeat #{})",
                                slow_extra_prefix, verbose_hint, timeout_count
                            );
                        }
                        next_wait = repeat_after;
                    }
                }
            }
        });
        CliStep::with_watcher(started, tx, watcher)
    }

    fn step_line_ok(&self, detail: &str) {
        if self.quiet {
            return;
        }
        println!("   {} {}", self.green_bold("[ok]"), self.white_bold(detail));
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
        if self.quiet {
            return;
        }
        println!(
            "{} {} {}",
            self.green_bold("[ok]"),
            self.green_bold("Tachyon Pulse Successful in"),
            self.green_bold(&format_duration(total))
        );
    }

    pub fn success(&self, msg: &str) {
        if self.quiet {
            return;
        }
        println!("{} {}", self.green_bold("[ok]"), self.white_bold(msg));
    }
    pub fn warn(&self, msg: &str) {
        if self.quiet {
            return;
        }
        eprintln!("{} {}", self.yellow_bold("!"), self.yellow_bold(msg));
    }

    pub fn error(&self, msg: &str) {
        if self.quiet {
            return;
        }
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

pub(crate) trait EmitFunctionCache {
    fn load(&mut self, key: &str) -> crate::error::RR<Option<(String, Vec<MapEntry>)>>;
    fn store(&mut self, key: &str, code: &str, map: &[MapEntry]) -> crate::error::RR<()>;
}

fn stable_hash_bytes(bytes: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET_BASIS;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

pub(crate) fn fn_emit_cache_salt() -> u64 {
    stable_hash_bytes(include_str!("pipeline.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/mir_emit.rs").as_bytes())
        ^ stable_hash_bytes(env!("RR_COMPILER_BUILD_HASH").as_bytes())
}

pub(crate) fn compile_output_cache_salt() -> u64 {
    fn_emit_cache_salt()
        ^ stable_hash_bytes(crate::runtime::R_RUNTIME.as_bytes())
        ^ stable_hash_bytes(include_str!("../runtime/subset.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../runtime/source.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("r_peephole.rs").as_bytes())
}

fn fn_emit_cache_key(
    fn_ir: &crate::mir::def::FnIR,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
) -> String {
    let payload = format!(
        "rr-fn-emit-v2|{}|{}|{}|{}|{}|{}|{}|{}|{:?}",
        fn_ir.name,
        opt_level.label(),
        type_cfg.mode.as_str(),
        type_cfg.native_backend.as_str(),
        parallel_cfg.mode.as_str(),
        parallel_cfg.backend.as_str(),
        parallel_cfg.threads,
        fn_emit_cache_salt(),
        fn_ir
    );
    format!("{:016x}", stable_hash_bytes(payload.as_bytes()))
}

pub(crate) struct SourceAnalysisOutput {
    desugared_hir: crate::hir::def::HirProgram,
    global_symbols: FxHashMap<crate::hir::def::SymbolId, String>,
}

pub(crate) struct MirSynthesisOutput {
    all_fns: FxHashMap<String, crate::mir::def::FnIR>,
    emit_order: Vec<String>,
    emit_roots: Vec<String>,
    top_level_calls: Vec<String>,
}

fn called_user_fns(
    fn_ir: &crate::mir::def::FnIR,
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    for value in &fn_ir.values {
        match &value.kind {
            crate::mir::def::ValueKind::Call { callee, .. } => {
                let canonical = callee.strip_suffix("_fresh").unwrap_or(callee);
                if all_fns.contains_key(canonical) {
                    out.insert(canonical.to_string());
                }
            }
            crate::mir::def::ValueKind::Load { var } => {
                if all_fns.contains_key(var) {
                    out.insert(var.clone());
                }
            }
            crate::mir::def::ValueKind::RSymbol { name } => {
                if all_fns.contains_key(name) {
                    out.insert(name.clone());
                }
            }
            _ => {}
        }
    }
    out
}

fn reachable_emit_order(
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
    emit_order: &[String],
    emit_roots: &[String],
) -> Vec<String> {
    if emit_roots.is_empty() {
        return emit_order.to_vec();
    }

    let mut reachable = FxHashSet::default();
    let mut worklist: Vec<String> = emit_roots
        .iter()
        .filter(|name| all_fns.contains_key(name.as_str()))
        .cloned()
        .collect();

    while let Some(name) = worklist.pop() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        let Some(fn_ir) = all_fns.get(&name) else {
            continue;
        };
        for callee in called_user_fns(fn_ir, all_fns) {
            if !reachable.contains(&callee) {
                worklist.push(callee);
            }
        }
    }

    if reachable.iter().all(|name| name.starts_with("Sym_top_")) {
        return emit_order.to_vec();
    }

    emit_order
        .iter()
        .filter(|name| reachable.contains(name.as_str()))
        .cloned()
        .collect()
}

pub(crate) fn run_source_analysis_and_canonicalization(
    ui: &CliLog,
    entry_path: &str,
    entry_input: &str,
    total_steps: usize,
) -> crate::error::RR<SourceAnalysisOutput> {
    // Module Loader State
    let mut loaded_paths: FxHashSet<PathBuf> = FxHashSet::default();
    let mut queue = std::collections::VecDeque::new();

    // Normalize entry path
    let entry_abs = normalize_module_path(Path::new(entry_path));
    loaded_paths.insert(entry_abs.clone());
    queue.push_back((entry_abs, entry_input.to_string(), 0)); // (path, content, mod_id)

    // Helper for generating IDs
    let mut next_mod_id = 1;

    let step_load = ui.step_start(
        1,
        total_steps,
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

    let step_desugar = ui.step_start(
        2,
        total_steps,
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

    Ok(SourceAnalysisOutput {
        desugared_hir,
        global_symbols,
    })
}

fn emit_r_functions(
    ui: &CliLog,
    total_steps: usize,
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
    emit_order: &[String],
) -> crate::error::RR<(String, Vec<crate::codegen::mir_emit::MapEntry>)> {
    let (out, map, _, _) = emit_r_functions_cached(
        ui,
        total_steps,
        all_fns,
        emit_order,
        OptLevel::O0,
        TypeConfig::default(),
        ParallelConfig::default(),
        None,
    )?;
    Ok((out, map))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_r_functions_cached(
    ui: &CliLog,
    total_steps: usize,
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
    emit_order: &[String],
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    mut cache: Option<&mut dyn EmitFunctionCache>,
) -> crate::error::RR<(String, Vec<MapEntry>, usize, usize)> {
    let step_emit = ui.step_start(
        5,
        total_steps,
        "R Code Emission",
        "reconstruct control flow",
    );
    let mut final_output = String::new();
    let mut final_source_map = Vec::new();
    let mut cache_hits = 0usize;
    let mut cache_misses = 0usize;

    for fn_name in emit_order {
        let Some(fn_ir) = all_fns.get(fn_name) else {
            return Err(crate::error::InternalCompilerError::new(
                crate::error::Stage::Codegen,
                format!(
                    "emit order references missing function '{}': MIR synthesis invariant violated",
                    fn_name
                ),
            )
            .into_exception());
        };

        let key = fn_emit_cache_key(fn_ir, opt_level, type_cfg, parallel_cfg);
        let maybe_hit = if let Some(ref mut c) = cache {
            c.load(&key)?
        } else {
            None
        };

        let (code, map) = if let Some((code, map)) = maybe_hit {
            cache_hits += 1;
            (code, map)
        } else {
            let (code, map) = crate::codegen::mir_emit::MirEmitter::new().emit(fn_ir)?;
            if let Some(ref mut c) = cache {
                c.store(&key, &code, &map)?;
            }
            cache_misses += 1;
            (code, map)
        };
        final_output.push_str(&code);
        final_output.push('\n');
        final_source_map.extend(map);
    }
    ui.step_line_ok(&format!(
        "Emitted {} functions ({} debug maps) in {}",
        emit_order.len(),
        final_source_map.len(),
        format_duration(step_emit.elapsed())
    ));

    let direct_builtin_call_map =
        matches!(type_cfg.native_backend, crate::typeck::NativeBackend::Off)
            && matches!(parallel_cfg.mode, ParallelMode::Off);
    let final_output =
        crate::compiler::r_peephole::optimize_emitted_r(&final_output, direct_builtin_call_map);

    Ok((final_output, final_source_map, cache_hits, cache_misses))
}

pub(crate) fn run_mir_synthesis(
    ui: &CliLog,
    total_steps: usize,
    desugared_hir: crate::hir::def::HirProgram,
    global_symbols: &FxHashMap<crate::hir::def::SymbolId, String>,
    type_cfg: TypeConfig,
) -> crate::error::RR<MirSynthesisOutput> {
    let step_ssa = ui.step_start(
        3,
        total_steps,
        "SSA Graph Synthesis",
        "build dominator tree & phi nodes",
    );
    let mut all_fns = FxHashMap::default();
    let mut emit_order: Vec<String> = Vec::new();
    let mut emit_roots: Vec<String> = Vec::new();
    let mut top_level_calls: Vec<String> = Vec::new();
    let mut known_fn_arities: FxHashMap<String, usize> = FxHashMap::default();

    for module in &desugared_hir.modules {
        for item in &module.items {
            if let crate::hir::def::HirItem::Fn(f) = item
                && let Some(name) = global_symbols.get(&f.name).cloned()
            {
                known_fn_arities.insert(name, f.params.len());
            }
        }
    }

    for module in desugared_hir.modules {
        let mut top_level_stmts: Vec<crate::hir::def::HirStmt> = Vec::new();

        for item in module.items {
            match item {
                crate::hir::def::HirItem::Fn(f) => {
                    let fn_name = format!("Sym_{}", f.name.0);
                    let is_public = f.public;
                    let mut params = Vec::with_capacity(f.params.len());
                    for p in &f.params {
                        let Some(name) = global_symbols.get(&p.name).cloned() else {
                            return Err(InternalCompilerError::new(
                                Stage::Mir,
                                format!(
                                    "missing parameter symbol during MIR lowering pipeline: {:?}",
                                    p.name
                                ),
                            )
                            .into_exception());
                        };
                        params.push(name);
                    }
                    let var_names = f.local_names.clone().into_iter().collect();

                    let lowerer = crate::mir::lower_hir::MirLowerer::new(
                        fn_name.clone(),
                        params,
                        var_names,
                        global_symbols,
                        &known_fn_arities,
                    );
                    let fn_ir = lowerer.lower_fn(f)?;
                    all_fns.insert(fn_name.clone(), fn_ir);
                    if is_public {
                        emit_roots.push(fn_name.clone());
                    }
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
                global_symbols,
                &known_fn_arities,
            );
            let fn_ir = lowerer.lower_fn(top_fn)?;
            all_fns.insert(top_fn_name.clone(), fn_ir);
            emit_order.push(top_fn_name.clone());
            emit_roots.push(top_fn_name.clone());
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

    Ok(MirSynthesisOutput {
        all_fns,
        emit_order,
        emit_roots,
        top_level_calls,
    })
}

pub(crate) fn run_tachyon_phase(
    ui: &CliLog,
    total_steps: usize,
    optimize: bool,
    all_fns: &mut FxHashMap<String, crate::mir::def::FnIR>,
) -> crate::error::RR<()> {
    let tachyon = crate::mir::opt::TachyonEngine::new();
    let step_opt = ui.step_start(
        4,
        total_steps,
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
    let mut fallback_msgs = Vec::new();
    let mut opaque_msgs = Vec::new();
    for fn_ir in all_fns.values() {
        if fn_ir.unsupported_dynamic {
            let msg = if fn_ir.fallback_reasons.is_empty() {
                format!(
                    "Hybrid fallback enabled for {} (dynamic feature)",
                    fn_ir.name
                )
            } else {
                format!(
                    "Hybrid fallback enabled for {}: {}",
                    fn_ir.name,
                    fn_ir.fallback_reasons.join(", ")
                )
            };
            fallback_msgs.push(msg);
        }
        if fn_ir.opaque_interop {
            let msg = if fn_ir.opaque_reasons.is_empty() {
                format!(
                    "Opaque interop enabled for {} (package/runtime call)",
                    fn_ir.name
                )
            } else {
                format!(
                    "Opaque interop enabled for {}: {}",
                    fn_ir.name,
                    fn_ir.opaque_reasons.join(", ")
                )
            };
            opaque_msgs.push(msg);
        }
    }
    fallback_msgs.sort();
    for msg in fallback_msgs {
        ui.warn(&msg);
    }
    opaque_msgs.sort();
    for msg in opaque_msgs {
        ui.warn(&msg);
    }
    let mut pulse_stats = crate::mir::opt::TachyonPulseStats::default();
    if optimize {
        if ui.detailed && ui.slow_step_ms > 0 {
            let slow_after = Duration::from_millis(ui.slow_step_ms as u64);
            let repeat_after = Duration::from_millis(ui.slow_step_repeat_ms as u64);
            let mut last_report = Duration::ZERO;
            let mut last_marker: Option<(crate::mir::opt::TachyonProgressTier, usize)> = None;
            let mut progress_cb = |event: crate::mir::opt::TachyonProgress| {
                let elapsed = step_opt.elapsed();
                if elapsed < slow_after {
                    return;
                }
                if elapsed.saturating_sub(last_report) < repeat_after {
                    return;
                }
                let marker = (event.tier, event.completed);
                if last_marker == Some(marker) {
                    return;
                }
                last_marker = Some(marker);
                last_report = elapsed;
                ui.trace(
                    "progress",
                    &format!(
                        "tier={} {}/{} fn={} elapsed={}",
                        event.tier.label(),
                        event.completed,
                        event.total,
                        event.function,
                        format_duration(elapsed)
                    ),
                );
            };
            pulse_stats = tachyon.run_program_with_stats_progress(all_fns, &mut progress_cb);
        } else {
            pulse_stats = tachyon.run_program_with_stats(all_fns);
        }
    } else {
        tachyon.stabilize_for_codegen(all_fns);
    }
    crate::mir::semantics::validate_program(all_fns)?;
    crate::mir::semantics::validate_runtime_safety(all_fns)?;
    if optimize {
        ui.step_line_ok(&format!(
            "Vectorized: {} | Reduced: {} | Simplified: {} loops",
            pulse_stats.vectorized, pulse_stats.reduced, pulse_stats.simplified_loops
        ));
        if pulse_stats.vector_loops_seen > 0 && pulse_stats.vector_skipped > 0 {
            ui.step_line_ok(&format!(
                "VecSkip: {}/{} (no-iv {} | bound {} | cfg {} | indirect {} | store {} | no-pattern {})",
                pulse_stats.vector_skipped,
                pulse_stats.vector_loops_seen,
                pulse_stats.vector_skip_no_iv,
                pulse_stats.vector_skip_non_canonical_bound,
                pulse_stats.vector_skip_unsupported_cfg_shape,
                pulse_stats.vector_skip_indirect_index_access,
                pulse_stats.vector_skip_store_effects,
                pulse_stats.vector_skip_no_supported_pattern
            ));
        }
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

    Ok(())
}

pub(crate) fn inject_runtime_prelude(
    entry_path: &str,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    mut final_output: String,
    top_level_calls: &[String],
) -> String {
    for call in top_level_calls {
        final_output.push_str(&format!("{}()\n", call));
    }

    // Prepend runtime so generated .R is self-contained.
    let mut with_runtime = String::new();
    let runtime_roots = runtime_roots_for_output(&final_output, true);
    with_runtime.push_str(&crate::runtime::render_runtime_subset(&runtime_roots));
    if !with_runtime.ends_with('\n') {
        with_runtime.push('\n');
    }
    append_runtime_configuration(
        &mut with_runtime,
        entry_path,
        type_cfg,
        parallel_cfg,
        true,
        &runtime_roots,
    );
    with_runtime.push_str(&final_output);
    with_runtime
}

fn runtime_roots_for_output(
    final_output: &str,
    include_source_bootstrap: bool,
) -> FxHashSet<String> {
    let _ = include_source_bootstrap;
    crate::runtime::referenced_runtime_symbols(final_output)
}

fn roots_need_strict_index_config(roots: &FxHashSet<String>) -> bool {
    roots.iter().any(|name| {
        matches!(
            name.as_str(),
            "rr_index1_read"
                | "rr_index1_read_strict"
                | "rr_index1_read_vec"
                | "rr_index1_read_idx"
                | "rr_index_vec_floor"
                | "rr_gather"
        )
    })
}

fn roots_need_native_parallel_config(roots: &FxHashSet<String>) -> bool {
    roots.iter().any(|name| {
        name.starts_with("rr_parallel_")
            || name.starts_with("rr_native_")
            || name.starts_with("rr_intrinsic_")
            || name.starts_with("rr_call_map_")
            || matches!(
                name.as_str(),
                "rr_parallel_typed_vec_call" | "rr_vector_scalar_fallback_enabled"
            )
    })
}

fn append_runtime_configuration(
    out: &mut String,
    entry_path: &str,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    include_source_bootstrap: bool,
    runtime_roots: &FxHashSet<String>,
) {
    if include_source_bootstrap {
        let source_label = Path::new(entry_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(entry_path);
        out.push_str(&format!(
            ".rr_env$file <- \"{}\";\n",
            escape_r_string(source_label)
        ));
        let native_roots = compile_time_native_roots(entry_path);
        if !native_roots.is_empty() {
            out.push_str(".rr_env$native_anchor_roots <- unique(vapply(c(");
            for (idx, root) in native_roots.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                out.push('"');
                out.push_str(&escape_r_string(root));
                out.push('"');
            }
            out.push_str(
                "), function(p) normalizePath(as.character(p), winslash = \"/\", mustWork = FALSE), character(1)));\n",
            );
        }
    }
    if roots_need_strict_index_config(runtime_roots) {
        out.push_str(
            ".rr_env$strict_index_read <- identical(Sys.getenv(\"RR_STRICT_INDEX_READ\", \"0\"), \"1\");\n",
        );
    }
    if roots_need_native_parallel_config(runtime_roots) {
        out.push_str(&format!(
            ".rr_env$native_backend <- \"{}\";\n",
            type_cfg.native_backend.as_str()
        ));
        out.push_str(&format!(
            ".rr_env$parallel_mode <- \"{}\";\n",
            parallel_cfg.mode.as_str()
        ));
        out.push_str(&format!(
            ".rr_env$parallel_backend <- \"{}\";\n",
            parallel_cfg.backend.as_str()
        ));
        out.push_str(&format!(
            ".rr_env$parallel_threads <- as.integer({});\n",
            parallel_cfg.threads
        ));
        out.push_str(&format!(
            ".rr_env$parallel_min_trip <- as.integer({});\n",
            parallel_cfg.min_trip
        ));
        out.push_str(
            ".rr_env$vector_fallback_base_trip <- suppressWarnings(as.integer(Sys.getenv(\"RR_VECTOR_FALLBACK_BASE_TRIP\", \"12\")));\n",
        );
        out.push_str(
            "if (is.na(.rr_env$vector_fallback_base_trip) || .rr_env$vector_fallback_base_trip < 0L) .rr_env$vector_fallback_base_trip <- 12L;\n",
        );
        out.push_str(
            ".rr_env$vector_fallback_helper_scale <- suppressWarnings(as.integer(Sys.getenv(\"RR_VECTOR_FALLBACK_HELPER_SCALE\", \"4\")));\n",
        );
        out.push_str(
            "if (is.na(.rr_env$vector_fallback_helper_scale) || .rr_env$vector_fallback_helper_scale < 0L) .rr_env$vector_fallback_helper_scale <- 4L;\n",
        );
        out.push_str(
            ".rr_env$native_autobuild <- tolower(Sys.getenv(\"RR_NATIVE_AUTOBUILD\", \"1\")) %in% c(\"1\", \"true\", \"yes\", \"on\");\n",
        );
        out.push_str(".rr_env$native_lib <- \"\";\n");
        out.push_str(".rr_env$native_loaded <- FALSE;\n");
        if !include_source_bootstrap {
            out.push_str(".rr_env$native_anchor_roots <- character(0);\n");
        }
    }
}

fn compile_time_native_roots(entry_path: &str) -> Vec<String> {
    let entry = Path::new(entry_path);
    let canonical = fs::canonicalize(entry).unwrap_or_else(|_| entry.to_path_buf());
    let Some(entry_dir) = canonical.parent().map(Path::to_path_buf) else {
        return Vec::new();
    };
    let mut cur = entry_dir.clone();
    loop {
        let probe = cur.join("Cargo.toml");
        if probe.is_file() {
            return vec![cur.to_string_lossy().replace('\\', "/")];
        }
        let Some(parent) = cur.parent() else {
            break;
        };
        if parent == cur {
            break;
        }
        cur = parent.to_path_buf();
    }
    vec![entry_dir.to_string_lossy().replace('\\', "/")]
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
    compile_with_configs_with_options(
        entry_path,
        entry_input,
        opt_level,
        type_cfg,
        parallel_cfg,
        CompileOutputOptions::default(),
    )
}

pub fn compile_with_configs_with_options(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    output_opts: CompileOutputOptions,
) -> crate::error::RR<(String, Vec<crate::codegen::mir_emit::MapEntry>)> {
    let (code, map, _, _) = compile_with_configs_using_emit_cache(
        entry_path,
        entry_input,
        opt_level,
        type_cfg,
        parallel_cfg,
        None,
        output_opts,
    )?;
    Ok((code, map))
}

pub(crate) fn compile_with_configs_using_emit_cache(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    cache: Option<&mut dyn EmitFunctionCache>,
    output_opts: CompileOutputOptions,
) -> crate::error::RR<(String, Vec<MapEntry>, usize, usize)> {
    let ui = CliLog::new();
    let compile_started = Instant::now();
    let optimize = opt_level.is_optimized();
    const TOTAL_STEPS: usize = 6;
    let input_label = std::path::Path::new(entry_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(entry_path);
    ui.banner(input_label, opt_level);

    let SourceAnalysisOutput {
        desugared_hir,
        global_symbols,
    } = run_source_analysis_and_canonicalization(&ui, entry_path, entry_input, TOTAL_STEPS)?;

    let MirSynthesisOutput {
        mut all_fns,
        emit_order,
        emit_roots,
        top_level_calls,
    } = run_mir_synthesis(&ui, TOTAL_STEPS, desugared_hir, &global_symbols, type_cfg)?;

    run_tachyon_phase(&ui, TOTAL_STEPS, optimize, &mut all_fns)?;
    verify_emittable_program(&all_fns)?;
    let emit_order = reachable_emit_order(&all_fns, &emit_order, &emit_roots);

    let (final_output, final_source_map, emit_cache_hits, emit_cache_misses) =
        emit_r_functions_cached(
            &ui,
            TOTAL_STEPS,
            &all_fns,
            &emit_order,
            opt_level,
            type_cfg,
            parallel_cfg,
            cache,
        )?;

    let final_code = if output_opts.inject_runtime {
        let step_runtime = ui.step_start(
            6,
            TOTAL_STEPS,
            "Runtime Injection",
            "link static analysis guards",
        );
        let with_runtime = inject_runtime_prelude(
            entry_path,
            type_cfg,
            parallel_cfg,
            final_output,
            &top_level_calls,
        );
        ui.step_line_ok(&format!("Output size: {}", human_size(with_runtime.len())));
        ui.trace(
            "runtime",
            &format!("linked in {}", format_duration(step_runtime.elapsed())),
        );
        with_runtime
    } else {
        let step_runtime = ui.step_start(
            6,
            TOTAL_STEPS,
            "Runtime Injection",
            "helper-only (--no-runtime)",
        );
        let mut without_runtime = String::new();
        let runtime_roots = runtime_roots_for_output(&final_output, false);
        without_runtime.push_str(&crate::runtime::render_runtime_subset(&runtime_roots));
        if !without_runtime.ends_with('\n') {
            without_runtime.push('\n');
        }
        append_runtime_configuration(
            &mut without_runtime,
            entry_path,
            type_cfg,
            parallel_cfg,
            false,
            &runtime_roots,
        );
        without_runtime.push_str(&final_output);
        for call in &top_level_calls {
            without_runtime.push_str(&format!("{}()\n", call));
        }
        ui.step_line_ok(&format!(
            "Output size: {}",
            human_size(without_runtime.len())
        ));
        ui.trace(
            "runtime",
            &format!("helper-only in {}", format_duration(step_runtime.elapsed())),
        );
        without_runtime
    };
    ui.pulse_success(compile_started.elapsed());

    Ok((
        final_code,
        final_source_map,
        emit_cache_hits,
        emit_cache_misses,
    ))
}

fn verify_emittable_program(
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
) -> crate::error::RR<()> {
    let mut names: Vec<&String> = all_fns.keys().collect();
    names.sort();
    for name in names {
        let Some(fn_ir) = all_fns.get(name) else {
            continue;
        };
        crate::mir::verify::verify_emittable_ir(fn_ir).map_err(|e| {
            crate::error::InternalCompilerError::new(
                crate::error::Stage::Codegen,
                format!(
                    "emittable MIR verification failed for function '{}': {}",
                    fn_ir.name, e
                ),
            )
            .into_exception()
        })?;
    }
    Ok(())
}
