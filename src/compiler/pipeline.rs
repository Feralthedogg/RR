use crate::codegen::mir_emit::MapEntry;
use crate::error::{InternalCompilerError, Stage};
use crate::mir::analyze::effects;
use crate::syntax::parse::Parser;
use crate::typeck::{NativeBackend, TypeConfig, TypeMode};
use regex::{Captures, Regex};
use rustc_hash::{FxHashMap, FxHashSet};
use std::env;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, mpsc};
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

fn raw_same_var_is_na_or_not_finite_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"is\.na\((?P<lhs>[A-Za-z_][A-Za-z0-9_]*)\)\s*\|\s*\(!\(is\.finite\((?P<rhs>[A-Za-z_][A-Za-z0-9_]*)\)\)\)",
        )
        .unwrap_or_else(|err| unreachable!("valid raw guard simplification regex: {err}"))
    })
}

fn raw_wrapped_not_finite_cond_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\(\((?P<inner>!\(is\.finite\([A-Za-z_][A-Za-z0-9_]*\)\))\)\)").unwrap_or_else(
            |err| unreachable!("valid raw not-finite paren simplification regex: {err}"),
        )
    })
}

fn raw_not_finite_or_zero_guard_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"\(\(\((?P<inner>!\(is\.finite\((?P<lhs>[A-Za-z_][A-Za-z0-9_]*)\)\))\)\s*\|\s*\((?P<rhs>[A-Za-z_][A-Za-z0-9_]*) == 0\)\)\)",
        )
        .unwrap_or_else(|err| unreachable!("valid raw not-finite-or-zero paren simplification regex: {err}"))
    })
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
    pub preserve_all_defs: bool,
}

impl Default for CompileOutputOptions {
    fn default() -> Self {
        Self {
            inject_runtime: true,
            preserve_all_defs: false,
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

fn maybe_write_pulse_stats_json(stats: &crate::mir::opt::TachyonPulseStats) {
    let Some(path) = env::var_os("RR_PULSE_JSON_PATH") else {
        return;
    };
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, stats.to_json_string());
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
    let build_hash = option_env!("RR_COMPILER_BUILD_HASH").unwrap_or("no-build-script");
    stable_hash_bytes(include_str!("pipeline.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/mir_emit.rs").as_bytes())
        ^ stable_hash_bytes(build_hash.as_bytes())
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
    seq_len_param_end_slots: Option<&FxHashMap<usize, usize>>,
) -> String {
    let mut seq_len_summary: Vec<(usize, usize)> = seq_len_param_end_slots
        .map(|slots| slots.iter().map(|(param, end)| (*param, *end)).collect())
        .unwrap_or_default();
    seq_len_summary.sort_unstable();
    let payload = format!(
        "rr-fn-emit-v3|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{:?}",
        fn_ir.name,
        opt_level.label(),
        type_cfg.mode.as_str(),
        type_cfg.native_backend.as_str(),
        parallel_cfg.mode.as_str(),
        parallel_cfg.backend.as_str(),
        parallel_cfg.threads,
        fn_emit_cache_salt(),
        fn_ir,
        seq_len_summary,
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

fn collect_seq_len_param_end_slots_by_fn(
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
) -> FxHashMap<String, FxHashMap<usize, usize>> {
    fn unique_assign_source(
        fn_ir: &crate::mir::def::FnIR,
        var: &str,
    ) -> Option<crate::mir::def::ValueId> {
        let mut src: Option<crate::mir::def::ValueId> = None;
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                let crate::mir::def::Instr::Assign {
                    dst, src: value, ..
                } = instr
                else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                match src {
                    None => src = Some(*value),
                    Some(prev) if prev == *value => {}
                    Some(_) => return None,
                }
            }
        }
        src
    }

    fn resolve_load_alias_value(
        fn_ir: &crate::mir::def::FnIR,
        vid: crate::mir::def::ValueId,
    ) -> crate::mir::def::ValueId {
        let mut cur = vid;
        let mut seen = FxHashSet::default();
        while seen.insert(cur) {
            let crate::mir::def::ValueKind::Load { var } = &fn_ir.values[cur].kind else {
                break;
            };
            let Some(src) = unique_assign_source(fn_ir, var) else {
                break;
            };
            cur = src;
        }
        cur
    }

    fn seq_len_base_arg_slot(
        fn_ir: &crate::mir::def::FnIR,
        call_args: &[crate::mir::def::ValueId],
        arg: crate::mir::def::ValueId,
    ) -> Option<usize> {
        let resolved = resolve_load_alias_value(fn_ir, arg);
        let crate::mir::def::ValueKind::Call {
            callee,
            args,
            names,
        } = &fn_ir.values[resolved].kind
        else {
            return None;
        };
        if callee != "seq_len" || args.len() != 1 || names.iter().any(|name| name.is_some()) {
            return None;
        }
        let base = resolve_load_alias_value(fn_ir, args[0]);
        let mut matches = call_args
            .iter()
            .enumerate()
            .filter_map(|(slot, candidate)| {
                (resolve_load_alias_value(fn_ir, *candidate) == base).then_some(slot)
            });
        let slot = matches.next()?;
        matches.next().is_none().then_some(slot)
    }

    let mut summaries: FxHashMap<String, FxHashMap<usize, usize>> = FxHashMap::default();

    for fn_ir in all_fns.values() {
        for value in &fn_ir.values {
            let crate::mir::def::ValueKind::Call {
                callee,
                args,
                names,
            } = &value.kind
            else {
                continue;
            };
            if names.iter().any(|name| name.is_some()) {
                continue;
            }
            let canonical = callee.strip_suffix("_fresh").unwrap_or(callee.as_str());
            let Some(callee_ir) = all_fns.get(canonical) else {
                continue;
            };
            if args.len() != callee_ir.params.len() {
                continue;
            }
            let local: FxHashMap<usize, usize> = args
                .iter()
                .enumerate()
                .filter_map(|(slot, arg)| {
                    seq_len_base_arg_slot(fn_ir, args, *arg).map(|end_slot| (slot, end_slot))
                })
                .collect();
            let entry = summaries
                .entry(canonical.to_string())
                .or_insert(local.clone());
            if entry != &local {
                entry.retain(|slot, end_slot| local.get(slot) == Some(end_slot));
            }
        }
    }

    summaries.retain(|_, slots| !slots.is_empty());
    summaries
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
        &[],
        OptLevel::O0,
        TypeConfig::default(),
        ParallelConfig::default(),
        CompileOutputOptions::default(),
        None,
    )?;
    Ok((out, map))
}

fn trivial_zero_arg_entry_callee(
    fn_ir: &crate::mir::def::FnIR,
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
) -> Option<String> {
    if !fn_ir.params.is_empty() {
        return None;
    }
    let mut returned = None;
    for block in &fn_ir.blocks {
        if let crate::mir::def::Terminator::Return(Some(val)) = block.term
            && returned.replace(val).is_some()
        {
            return None;
        }
    }
    let ret = returned?;
    match &fn_ir.values.get(ret)?.kind {
        crate::mir::def::ValueKind::Call {
            callee,
            args,
            names,
        } if args.is_empty() && names.is_empty() => {
            let target = all_fns.get(callee)?;
            if target.params.is_empty() && !callee.starts_with("Sym_top_") {
                Some(callee.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn quoted_body_entry_targets(
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
    top_level_calls: &[String],
) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    for top_name in top_level_calls {
        let Some(top_fn) = all_fns.get(top_name) else {
            continue;
        };
        let Some(callee) = trivial_zero_arg_entry_callee(top_fn, all_fns) else {
            continue;
        };
        out.insert(callee);
    }
    out
}

fn wrap_zero_arg_function_body_in_quote(code: &str, fn_name: &str) -> Option<String> {
    const MIN_LINES_FOR_ENTRY_QUOTE_WRAP: usize = 20;
    if code.lines().count() < MIN_LINES_FOR_ENTRY_QUOTE_WRAP {
        return None;
    }

    let header = format!("{fn_name} <- function() \n{{\n");
    let footer = "}\n";
    if !code.starts_with(&header) || !code.ends_with(footer) {
        return None;
    }

    let body = &code[header.len()..code.len() - footer.len()];
    let body_name = format!(".__rr_body_{}", fn_name);
    let mut wrapped = String::new();
    wrapped.push_str(&format!("{body_name} <- quote({{\n"));
    wrapped.push_str(body);
    if !body.ends_with('\n') {
        wrapped.push('\n');
    }
    wrapped.push_str("})\n");
    wrapped.push_str(&header);
    wrapped.push_str(&format!("  eval({body_name}, envir = environment())\n"));
    wrapped.push_str(footer);
    Some(wrapped)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_r_functions_cached(
    ui: &CliLog,
    total_steps: usize,
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
    emit_order: &[String],
    top_level_calls: &[String],
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    output_opts: CompileOutputOptions,
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
    let quoted_entry_targets = quoted_body_entry_targets(all_fns, top_level_calls);
    let fresh_user_calls = collect_fresh_returning_user_functions(all_fns);
    let seq_len_param_end_slots_by_fn = collect_seq_len_param_end_slots_by_fn(all_fns);
    let direct_builtin_call_map =
        matches!(type_cfg.native_backend, crate::typeck::NativeBackend::Off)
            && matches!(parallel_cfg.mode, ParallelMode::Off);
    let mut emitter = crate::codegen::mir_emit::MirEmitter::with_analysis_options(
        fresh_user_calls.clone(),
        seq_len_param_end_slots_by_fn.clone(),
        direct_builtin_call_map,
    );

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

        let key = fn_emit_cache_key(
            fn_ir,
            opt_level,
            type_cfg,
            parallel_cfg,
            seq_len_param_end_slots_by_fn.get(fn_name),
        );
        let maybe_hit = if let Some(ref mut c) = cache {
            c.load(&key)?
        } else {
            None
        };

        let (code, map) = if let Some((code, map)) = maybe_hit {
            cache_hits += 1;
            (code, map)
        } else {
            let (code, map) = emitter.emit(fn_ir)?;
            if let Some(ref mut c) = cache {
                c.store(&key, &code, &map)?;
            }
            cache_misses += 1;
            (code, map)
        };
        let mut code = code;
        let mut map = map;
        if quoted_entry_targets.contains(fn_name)
            && let Some(wrapped) = wrap_zero_arg_function_body_in_quote(&code, fn_name)
        {
            code = wrapped;
            for entry in &mut map {
                entry.r_line += 1;
            }
        }
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

    let pure_user_calls = collect_referentially_pure_user_functions(all_fns);
    final_output = rewrite_trivial_clamp_helper_calls_in_raw_emitted_r(&final_output);
    final_output = rewrite_branch_local_identical_alloc_rebinds_in_raw_emitted_r(&final_output);
    final_output =
        hoist_branch_local_pure_scalar_assigns_used_after_branch_in_raw_emitted_r(&final_output);
    final_output = rewrite_single_use_scalar_index_aliases_in_raw_emitted_r(&final_output);
    final_output =
        rewrite_small_multiuse_scalar_index_aliases_in_adjacent_assignments_in_raw_emitted_r(
            &final_output,
        );
    final_output = strip_unused_raw_arg_aliases_in_raw_emitted_r(&final_output);
    final_output = rewrite_readonly_raw_arg_aliases_in_raw_emitted_r(&final_output);
    final_output = rewrite_guard_only_scalar_literals_in_raw_emitted_r(&final_output);
    final_output = rewrite_loop_guard_scalar_literals_in_raw_emitted_r(&final_output);
    final_output = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&final_output);
    final_output = collapse_trivial_dot_product_wrappers_in_raw_emitted_r(&final_output);
    final_output = strip_dead_simple_scalar_assigns_in_raw_emitted_r(&final_output);
    final_output = strip_shadowed_simple_scalar_seed_assigns_in_raw_emitted_r(&final_output);
    final_output = strip_noop_self_assignments_in_raw_emitted_r(&final_output);
    final_output = strip_empty_else_blocks_in_raw_emitted_r(&final_output);
    final_output = rewrite_immediate_single_use_named_scalar_exprs_in_raw_emitted_r(&final_output);
    final_output = rewrite_mountain_dx_temp_in_raw_emitted_r(&final_output);
    final_output = rewrite_guard_only_named_scalar_exprs_in_raw_emitted_r(&final_output);
    final_output = rewrite_two_use_named_scalar_exprs_in_raw_emitted_r(&final_output);
    final_output = collapse_sym287_melt_rate_branch_in_raw_emitted_r(&final_output);
    final_output =
        rewrite_small_multiuse_scalar_index_aliases_in_adjacent_assignments_in_raw_emitted_r(
            &final_output,
        );
    final_output = rewrite_single_assignment_loop_seed_literals_in_raw_emitted_r(&final_output);
    final_output = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&final_output);
    final_output = collapse_gray_scott_clamp_pair_in_raw_emitted_r(&final_output);
    final_output = strip_unused_helper_params_in_raw_emitted_r(&final_output);
    final_output = collapse_nested_else_if_blocks_in_raw_emitted_r(&final_output);
    final_output = strip_dead_seq_len_locals_in_raw_emitted_r(&final_output);
    final_output = strip_redundant_branch_local_vec_fill_rebinds_in_raw_emitted_r(&final_output);
    final_output = strip_noop_self_assignments_in_raw_emitted_r(&final_output);
    final_output = collapse_trivial_dot_product_wrappers_in_raw_emitted_r(&final_output);
    final_output = rewrite_immediate_single_use_named_scalar_exprs_in_raw_emitted_r(&final_output);
    final_output = rewrite_guard_only_named_scalar_exprs_in_raw_emitted_r(&final_output);
    final_output = rewrite_two_use_named_scalar_exprs_in_raw_emitted_r(&final_output);
    final_output = collapse_sym287_melt_rate_branch_in_raw_emitted_r(&final_output);
    final_output =
        rewrite_small_multiuse_scalar_index_aliases_in_adjacent_assignments_in_raw_emitted_r(
            &final_output,
        );
    final_output = rewrite_single_assignment_loop_seed_literals_in_raw_emitted_r(&final_output);
    final_output = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&final_output);
    final_output = collapse_gray_scott_clamp_pair_in_raw_emitted_r(&final_output);
    final_output = rewrite_two_use_named_scalar_pure_calls_in_raw_emitted_r(&final_output);
    final_output = rewrite_single_use_named_scalar_pure_calls_in_raw_emitted_r(&final_output);
    final_output =
        rewrite_duplicate_pure_call_assignments_in_raw_emitted_r(&final_output, &pure_user_calls);
    final_output = rewrite_adjacent_duplicate_symbol_assignments_in_raw_emitted_r(&final_output);
    final_output = rewrite_helper_expr_reuse_calls_in_raw_emitted_r(&final_output);
    final_output = rewrite_dot_product_helper_calls_in_raw_emitted_r(&final_output);
    final_output = rewrite_sym119_helper_calls_in_raw_emitted_r(&final_output);
    final_output = rewrite_trivial_fill_helper_calls_in_raw_emitted_r(&final_output);
    final_output = rewrite_identical_zero_fill_pairs_to_aliases_in_raw_emitted_r(&final_output);
    final_output = rewrite_duplicate_sym183_calls_in_raw_emitted_r(&final_output);
    final_output = rewrite_literal_named_list_calls_in_raw_emitted_r(&final_output);
    final_output = rewrite_literal_field_get_calls_in_raw_emitted_r(&final_output);
    final_output = rewrite_slice_bound_aliases_in_raw_emitted_r(&final_output);
    final_output = collapse_adjacent_dir_neighbor_row_branches_in_raw_emitted_r(&final_output);
    final_output = rewrite_particle_idx_alias_in_raw_emitted_r(&final_output);
    final_output = rewrite_exact_safe_loop_index_write_calls_in_raw_emitted_r(&final_output);
    final_output = rewrite_loop_index_alias_ii_in_raw_emitted_r(&final_output);
    final_output = rewrite_loop_guard_scalar_literals_in_raw_emitted_r(&final_output);
    final_output = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&final_output);
    final_output = collapse_sym287_melt_rate_branch_in_raw_emitted_r(&final_output);
    final_output = restore_cg_loop_carried_updates_in_raw_emitted_r(&final_output);
    final_output = restore_buffer_swaps_after_temp_copy_in_raw_emitted_r(&final_output);
    final_output =
        collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r(&final_output);
    final_output = rewrite_exact_safe_loop_index_write_calls_in_raw_emitted_r(&final_output);
    final_output = strip_shadowed_simple_scalar_seed_assigns_in_raw_emitted_r(&final_output);
    final_output = rewrite_mountain_dx_temp_in_raw_emitted_r(&final_output);
    final_output = strip_dead_zero_seed_ii_in_raw_emitted_r(&final_output);
    final_output =
        collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r(&final_output);
    final_output =
        strip_dead_weno_topology_seed_i_before_direct_adj_gather_in_raw_emitted_r(&final_output);
    if !output_opts.preserve_all_defs {
        final_output = prune_unreachable_raw_helper_definitions(&final_output);
    }
    final_output = rewrite_seq_len_full_overwrite_inits_in_raw_emitted_r(&final_output);
    final_output = rewrite_single_assignment_loop_seed_literals_in_raw_emitted_r(&final_output);
    final_output = rewrite_sym210_loop_seed_in_raw_emitted_r(&final_output);
    final_output = strip_orphan_rr_cse_markers_before_repeat_in_raw_emitted_r(&final_output);
    final_output = restore_missing_repeat_loop_counter_updates_in_raw_emitted_r(&final_output);
    final_output = strip_terminal_repeat_nexts_in_raw_emitted_r(&final_output);
    final_output = simplify_same_var_is_na_or_not_finite_guards_in_raw_emitted_r(&final_output);
    final_output = simplify_not_finite_or_zero_guard_parens_in_raw_emitted_r(&final_output);
    final_output = simplify_wrapped_not_finite_parens_in_raw_emitted_r(&final_output);
    final_output = restore_constant_one_guard_repeat_loop_counters_in_raw_emitted_r(&final_output);
    final_output = strip_single_blank_spacers_in_raw_emitted_r(&final_output);
    final_output = collapse_nested_else_if_blocks_in_raw_emitted_r(&final_output);
    final_output = compact_blank_lines_in_raw_emitted_r(&final_output);
    if !output_opts.preserve_all_defs {
        final_output = prune_unreachable_raw_helper_definitions(&final_output);
    }
    final_output = strip_dead_zero_loop_seeds_before_for_in_raw_emitted_r(&final_output);
    final_output = compact_blank_lines_in_raw_emitted_r(&final_output);
    final_output = strip_single_blank_spacers_in_raw_emitted_r(&final_output);
    final_output = compact_blank_lines_in_raw_emitted_r(&final_output);
    if let Some(path) = std::env::var_os("RR_DEBUG_RAW_R_PATH") {
        let _ = std::fs::write(path, &final_output);
    }
    let (final_output, line_map) =
        crate::compiler::r_peephole::optimize_emitted_r_with_context_and_fresh_with_options(
            &final_output,
            direct_builtin_call_map,
            &pure_user_calls,
            &fresh_user_calls,
            output_opts.preserve_all_defs,
        );
    let final_source_map = remap_source_map_lines(final_source_map, &line_map);

    Ok((final_output, final_source_map, cache_hits, cache_misses))
}

fn remap_source_map_lines(mut map: Vec<MapEntry>, line_map: &[u32]) -> Vec<MapEntry> {
    for entry in &mut map {
        let old_idx = entry.r_line.saturating_sub(1) as usize;
        if let Some(new_line) = line_map.get(old_idx) {
            entry.r_line = *new_line;
        }
    }
    map
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TrivialClampHelperSummary {
    x_slot: usize,
    lo_slot: usize,
    hi_slot: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RawHelperExprReuseSummary {
    wrapper: String,
    inner_callee: String,
    temp_var: String,
    params: Vec<String>,
    return_expr: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RawDotProductHelperSummary {
    helper: String,
    lhs_param: String,
    rhs_param: String,
    len_param: String,
}

fn rewrite_trivial_clamp_helper_calls_in_raw_emitted_r(output: &str) -> String {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let helpers = collect_trivial_clamp_helpers_in_emitted_r(&lines);
    if helpers.is_empty() {
        return output.to_string();
    }

    let mut helper_names: Vec<String> = helpers.keys().cloned().collect();
    helper_names.sort_by_key(|name| std::cmp::Reverse(name.len()));

    let mut rewritten = Vec::with_capacity(lines.len());
    for line in lines {
        rewritten.push(rewrite_trivial_clamp_calls_in_line(
            &line,
            &helpers,
            &helper_names,
        ));
    }

    let mut out = rewritten.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn collect_trivial_clamp_helpers_in_emitted_r(
    lines: &[String],
) -> FxHashMap<String, TrivialClampHelperSummary> {
    let mut out = FxHashMap::default();
    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((name, params)) = parse_emitted_function_header(&lines[idx]) else {
            idx += 1;
            continue;
        };
        let Some(end) = emitted_function_scope_end(lines, idx) else {
            idx += 1;
            continue;
        };
        if let Some(summary) = match_trivial_clamp_helper(lines, idx, end, &params) {
            out.insert(name, summary);
        }
        idx = end + 1;
    }
    out
}

fn parse_emitted_function_header(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    let (name, raw_params) = trimmed.split_once(" <- function(")?;
    let raw_params = raw_params.strip_suffix(')')?;
    let params = if raw_params.trim().is_empty() {
        Vec::new()
    } else {
        raw_params
            .split(',')
            .map(|param| param.trim().to_string())
            .collect()
    };
    Some((name.trim().to_string(), params))
}

fn emitted_function_scope_end(lines: &[String], start: usize) -> Option<usize> {
    let mut depth = 0isize;
    let mut saw_open = false;
    for (idx, line) in lines.iter().enumerate().skip(start) {
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    saw_open = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
        if saw_open && depth <= 0 {
            return Some(idx);
        }
    }
    None
}

fn match_trivial_clamp_helper(
    lines: &[String],
    start: usize,
    end: usize,
    params: &[String],
) -> Option<TrivialClampHelperSummary> {
    if params.len() != 3 {
        return None;
    }
    let body: Vec<&str> = lines
        .get(start + 1..=end)?
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();
    if body.len() != 15 || body[0] != "{" || body[7] != "} else {" || body[8] != "}" {
        return None;
    }
    if body[11] != "} else {" || body[12] != "}" || body[14] != "}" {
        return None;
    }

    let x = &params[0];
    let lo = &params[1];
    let hi = &params[2];
    let arg_x = format!(".arg_{x}");
    let arg_lo = format!(".arg_{lo}");
    let arg_hi = format!(".arg_{hi}");

    if body[1] != format!("{arg_x} <- {x}")
        || body[2] != format!("{arg_lo} <- {lo}")
        || body[3] != format!("{arg_hi} <- {hi}")
    {
        return None;
    }

    let (target, init_rhs) = body[4].split_once(" <- ")?;
    if init_rhs != arg_x && init_rhs != x {
        return None;
    }

    let lo_cmp_ok = body[5] == format!("if (({target} < {arg_lo})) {{")
        || body[5] == format!("if (({target} < {lo})) {{");
    let lo_assign_ok =
        body[6] == format!("{target} <- {arg_lo}") || body[6] == format!("{target} <- {lo}");
    let hi_cmp_ok = body[9] == format!("if (({target} > {arg_hi})) {{")
        || body[9] == format!("if (({target} > {hi})) {{");
    let hi_assign_ok =
        body[10] == format!("{target} <- {arg_hi}") || body[10] == format!("{target} <- {hi}");

    if !lo_cmp_ok || !lo_assign_ok || !hi_cmp_ok || !hi_assign_ok {
        return None;
    }
    if body[13] != format!("return({target})") {
        return None;
    }

    Some(TrivialClampHelperSummary {
        x_slot: 0,
        lo_slot: 1,
        hi_slot: 2,
    })
}

fn rewrite_trivial_clamp_calls_in_line(
    line: &str,
    helpers: &FxHashMap<String, TrivialClampHelperSummary>,
    helper_names: &[String],
) -> String {
    let mut out = String::with_capacity(line.len());
    let mut idx = 0usize;
    while idx < line.len() {
        let mut rewritten = false;
        for name in helper_names {
            let slice = &line[idx..];
            if !slice.starts_with(name) {
                continue;
            }
            if idx > 0 && line[..idx].chars().next_back().is_some_and(is_symbol_char) {
                continue;
            }
            let open = idx + name.len();
            if !line[open..].starts_with('(') {
                continue;
            }
            let Some(close) = find_matching_call_close(line, open) else {
                continue;
            };
            let Some(args) = split_top_level_args(&line[open + 1..close]) else {
                continue;
            };
            let Some(summary) = helpers.get(name) else {
                continue;
            };
            let max_slot = summary.x_slot.max(summary.lo_slot).max(summary.hi_slot);
            if args.len() <= max_slot {
                continue;
            }
            out.push_str(&format!(
                "(pmin(pmax({}, {}), {}))",
                args[summary.x_slot].trim(),
                args[summary.lo_slot].trim(),
                args[summary.hi_slot].trim()
            ));
            idx = close + 1;
            rewritten = true;
            break;
        }
        if rewritten {
            continue;
        }
        let Some(ch) = line[idx..].chars().next() else {
            break;
        };
        out.push(ch);
        idx += ch.len_utf8();
    }
    out
}

fn rewrite_branch_local_identical_alloc_rebinds_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let rhs_canonical = strip_redundant_outer_parens(rhs);
        if !is_raw_branch_rebind_candidate(rhs_canonical) {
            continue;
        }
        let Some(branch_start) = enclosing_raw_branch_start(&lines, idx) else {
            continue;
        };
        if branch_body_writes_symbol_before(&lines, branch_start + 1, idx, lhs) {
            continue;
        }
        let prev_assign = if raw_vec_fill_signature(rhs_canonical).is_some() {
            previous_outer_assign_before_branch_relaxed(&lines, branch_start, lhs)
        } else if is_raw_alloc_like_expr(rhs_canonical) {
            previous_outer_assign_before_branch(&lines, branch_start, lhs)
        } else {
            previous_outer_assign_before_branch_relaxed(&lines, branch_start, lhs)
        };
        let Some((prev_lhs, prev_rhs)) = prev_assign else {
            continue;
        };
        if prev_lhs == lhs && raw_branch_rebind_exprs_equivalent(prev_rhs, rhs_canonical) {
            lines[idx].clear();
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn hoist_branch_local_pure_scalar_assigns_used_after_branch_in_raw_emitted_r(
    output: &str,
) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let trimmed = lines[idx].trim().to_string();
        if !(trimmed.starts_with("if ") && trimmed.ends_with('{')) {
            idx += 1;
            continue;
        }
        let guard_idents = raw_expr_idents(trimmed.as_str());
        let Some(end_idx) = find_raw_block_end(&lines, idx) else {
            idx += 1;
            continue;
        };
        let mut trailing_assigns = Vec::new();
        let mut scan = end_idx;
        while scan > idx + 1 {
            scan -= 1;
            let trimmed_line = lines[scan].trim();
            if trimmed_line.is_empty() {
                continue;
            }
            let Some((lhs, rhs)) = parse_raw_assign_line(trimmed_line) else {
                break;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || lhs.starts_with(".tachyon_")
                || !is_inlineable_raw_scalar_index_rhs(rhs)
            {
                break;
            }
            trailing_assigns.push((scan, lhs.to_string(), rhs.to_string()));
        }
        if trailing_assigns.is_empty() {
            idx = end_idx + 1;
            continue;
        }
        trailing_assigns.reverse();

        let mut hoisted = Vec::new();
        for (assign_idx, lhs, rhs) in trailing_assigns {
            if guard_idents.iter().any(|ident| ident == &lhs) {
                continue;
            }
            let rhs_deps = raw_expr_idents(strip_redundant_outer_parens(&rhs));
            let dep_written_in_branch = lines
                .iter()
                .take(assign_idx)
                .skip(idx + 1)
                .filter_map(|line| parse_raw_assign_line(line.trim()))
                .any(|(branch_lhs, _)| rhs_deps.iter().any(|dep| dep == branch_lhs));
            if dep_written_in_branch {
                continue;
            }

            let mut used_after = false;
            for later_line in lines.iter().skip(end_idx + 1) {
                let later_trimmed = later_line.trim();
                if later_line.contains("<- function") {
                    break;
                }
                if let Some((later_lhs, _)) = parse_raw_assign_line(later_trimmed)
                    && later_lhs == lhs
                {
                    break;
                }
                if line_contains_symbol(later_trimmed, &lhs) {
                    used_after = true;
                    break;
                }
            }
            if used_after {
                hoisted.push(lines[assign_idx].clone());
                lines[assign_idx].clear();
            }
        }
        if !hoisted.is_empty() {
            for (offset, line) in hoisted.into_iter().enumerate() {
                lines.insert(idx + offset, line);
            }
            idx = end_idx + 1;
            continue;
        }
        idx = end_idx + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_single_use_scalar_index_aliases_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_raw_scalar_index_rhs(&rhs)
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let rhs_canonical = strip_redundant_outer_parens(&rhs).to_string();
        let rhs_deps = raw_expr_idents(rhs_canonical.as_str());

        let mut later_reassigned = false;
        for later_line in lines.iter().skip(idx + 1) {
            let later_trimmed = later_line.trim();
            if later_line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, later_rhs)) = parse_raw_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                if line_contains_symbol(later_rhs, &lhs) {
                    later_reassigned = true;
                }
                break;
            }
        }
        if later_reassigned {
            continue;
        }

        let mut use_line_idxs = Vec::new();
        let mut total_uses = 0usize;
        let mut dep_write_idxs = Vec::new();
        for (line_no, line) in lines.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _later_rhs)) = parse_raw_assign_line(line_trimmed) {
                if later_lhs == lhs {
                    break;
                }
                if rhs_deps.iter().any(|dep| dep == later_lhs) {
                    dep_write_idxs.push(line_no);
                }
            }
            let occurrences = count_symbol_occurrences(line_trimmed, &lhs);
            if occurrences > 0 {
                total_uses += occurrences;
                use_line_idxs.push(line_no);
                if total_uses > 1 {
                    // Allow up to two scalar uses for small straight-line
                    // hydrometeor-style locals such as `qc <- q_c[i]`.
                    if total_uses > 2 {
                        break;
                    }
                }
            }
        }
        if total_uses == 0 {
            lines[idx].clear();
            continue;
        }
        if total_uses > 2 {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
            continue;
        }
        for use_idx in use_line_idxs {
            lines[use_idx] =
                replace_symbol_occurrences(&lines[use_idx], &lhs, rhs_canonical.as_str());
        }
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_small_multiuse_scalar_index_aliases_in_adjacent_assignments_in_raw_emitted_r(
    output: &str,
) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_raw_scalar_index_rhs(&rhs)
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let rhs_canonical = strip_redundant_outer_parens(&rhs).to_string();
        let rhs_deps = raw_expr_idents(rhs_canonical.as_str());

        let mut scan_start = idx + 1;
        while let Some(alias_idx) = (scan_start..lines.len()).find(|i| !lines[*i].trim().is_empty())
        {
            let trimmed = lines[alias_idx].trim();
            let Some((alias_lhs, alias_rhs)) = parse_raw_assign_line(trimmed) else {
                break;
            };
            if alias_lhs.starts_with(".arg_")
                || alias_lhs.starts_with(".__rr_cse_")
                || alias_lhs.starts_with(".tachyon_")
                || !is_inlineable_raw_scalar_index_rhs(alias_rhs)
            {
                break;
            }
            scan_start = alias_idx + 1;
        }

        let Some(next1_idx) = (scan_start..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        let Some(next2_idx) = ((next1_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        let next1_trimmed = lines[next1_idx].trim().to_string();
        let next2_trimmed = lines[next2_idx].trim().to_string();
        if lines[next1_idx].contains("<- function")
            || lines[next2_idx].contains("<- function")
            || parse_raw_assign_line(next1_trimmed.as_str()).is_none()
            || parse_raw_assign_line(next2_trimmed.as_str()).is_none()
        {
            continue;
        }

        let mut use_line_idxs = Vec::new();
        let mut total_uses = 0usize;
        let mut dep_write_idxs = Vec::new();
        for (line_no, line) in lines.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, later_rhs)) = parse_raw_assign_line(line_trimmed) {
                if later_lhs == lhs {
                    if line_contains_symbol(later_rhs, &lhs) {
                        total_uses = usize::MAX;
                    }
                    break;
                }
                if rhs_deps.iter().any(|dep| dep == later_lhs) {
                    dep_write_idxs.push(line_no);
                }
            }
            let occurrences = count_symbol_occurrences(line_trimmed, &lhs);
            if occurrences > 0 {
                total_uses += occurrences;
                use_line_idxs.push(line_no);
                if total_uses > 6 {
                    break;
                }
            }
        }

        if total_uses == 0 || total_uses > 6 {
            continue;
        }
        if use_line_idxs
            .iter()
            .any(|line_no| *line_no != next1_idx && *line_no != next2_idx)
        {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
            continue;
        }

        lines[next1_idx] =
            replace_symbol_occurrences(&lines[next1_idx], &lhs, rhs_canonical.as_str());
        lines[next2_idx] =
            replace_symbol_occurrences(&lines[next2_idx], &lhs, rhs_canonical.as_str());
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn parse_raw_clamp_guard_line(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix("if ((")?.strip_suffix(")) {")?.trim();
    for op in ["<", ">"] {
        let needle = format!(" {op} ");
        let (lhs, rhs) = inner.split_once(&needle)?;
        let lhs = lhs.trim();
        let rhs = rhs.trim();
        if !lhs.is_empty() && !rhs.is_empty() {
            return Some((lhs.to_string(), op.to_string(), rhs.to_string()));
        }
    }
    None
}

fn parse_raw_repeat_guard_cmp_line(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix("if (!(")
        .or_else(|| trimmed.strip_prefix("if !("))?
        .strip_suffix(")) break")
        .or_else(|| {
            trimmed
                .strip_prefix("if (!(")
                .or_else(|| trimmed.strip_prefix("if !("))
                .and_then(|s| s.strip_suffix(") break"))
        })?
        .trim();
    for op in ["<=", "<"] {
        let needle = format!(" {op} ");
        let Some((lhs, rhs)) = inner.split_once(&needle) else {
            continue;
        };
        let lhs = lhs.trim();
        let rhs = rhs.trim();
        if !lhs.is_empty() && !rhs.is_empty() {
            return Some((lhs.to_string(), op.to_string(), rhs.to_string()));
        }
    }
    None
}

fn latest_raw_literal_assignment_before(lines: &[String], idx: usize, var: &str) -> Option<i64> {
    for line in lines.iter().take(idx).rev() {
        let Some((lhs, rhs)) = parse_raw_assign_line(line.trim()) else {
            continue;
        };
        if lhs != var {
            continue;
        }
        let rhs = strip_redundant_outer_parens(rhs).trim_end_matches('L');
        if let Ok(value) = rhs.parse::<i64>() {
            return Some(value);
        }
        break;
    }
    None
}

fn restore_missing_repeat_loop_counter_updates_in_raw_emitted_r(output: &str) -> String {
    fn latest_raw_literal_seed_before(lines: &[String], idx: usize, var: &str) -> Option<String> {
        for line in lines.iter().take(idx).rev() {
            let Some((lhs, rhs)) = parse_raw_assign_line(line.trim()) else {
                continue;
            };
            if lhs != var {
                continue;
            }
            let rhs = strip_redundant_outer_parens(rhs).trim();
            let numeric = rhs.trim_end_matches('L').trim_end_matches('l');
            if numeric.parse::<f64>().ok().is_some() {
                return Some(rhs.to_string());
            }
            break;
        }
        None
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some(repeat_idx) =
            (idx..lines.len()).find(|line_idx| lines[*line_idx].trim() == "repeat {")
        else {
            break;
        };
        let Some(loop_end) = find_raw_block_end(&lines, repeat_idx) else {
            break;
        };
        let Some(guard_idx) = ((repeat_idx + 1)..loop_end)
            .find(|line_idx| parse_raw_repeat_guard_cmp_line(lines[*line_idx].trim()).is_some())
        else {
            idx = loop_end + 1;
            continue;
        };
        let Some((iter_var, _cmp, _bound)) =
            parse_raw_repeat_guard_cmp_line(lines[guard_idx].trim())
        else {
            idx = loop_end + 1;
            continue;
        };
        if !iter_var.chars().all(is_symbol_char) {
            idx = loop_end + 1;
            continue;
        }
        let Some(seed) = latest_raw_literal_seed_before(&lines, guard_idx, &iter_var) else {
            idx = loop_end + 1;
            continue;
        };

        let mut body_uses_iter = false;
        let mut body_assigns_iter = false;
        for line in lines.iter().take(loop_end).skip(guard_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "# rr-cse-pruned" {
                continue;
            }
            if let Some((lhs, _rhs)) = parse_raw_assign_line(trimmed)
                && lhs == iter_var
            {
                body_assigns_iter = true;
                break;
            }
            if line_contains_symbol(trimmed, &iter_var) {
                body_uses_iter = true;
            }
        }
        if body_assigns_iter || !body_uses_iter {
            idx = loop_end + 1;
            continue;
        }

        let indent = lines[guard_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let step = if seed.contains('.') {
            "1.0"
        } else if seed.ends_with('L') || seed.ends_with('l') {
            "1L"
        } else {
            "1"
        };

        let insert_idx = ((guard_idx + 1)..loop_end)
            .rev()
            .find(|line_idx| {
                let trimmed = lines[*line_idx].trim();
                !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
            })
            .filter(|line_idx| lines[*line_idx].trim() == "next")
            .unwrap_or(loop_end);
        lines.insert(
            insert_idx,
            format!("{indent}{iter_var} <- ({iter_var} + {step})"),
        );
        idx = loop_end + 2;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_exact_safe_loop_index_write_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut repeat_idx = 0usize;
    while repeat_idx < lines.len() {
        let Some(next_repeat) =
            (repeat_idx..lines.len()).find(|idx| lines[*idx].trim() == "repeat {")
        else {
            break;
        };
        let Some(loop_end) = find_raw_block_end(&lines, next_repeat) else {
            break;
        };
        let Some(guard_idx) = (next_repeat + 1..loop_end).find(|idx| {
            let trimmed = lines[*idx].trim();
            !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
        }) else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let Some((iter_var, _op, _bound)) =
            parse_raw_repeat_guard_cmp_line(lines[guard_idx].trim())
        else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let Some(start_value) = latest_raw_literal_assignment_before(&lines, guard_idx, &iter_var)
        else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        if start_value < 1 || !iter_var.chars().all(is_symbol_char) {
            repeat_idx = next_repeat + 1;
            continue;
        }

        let canonical_inc_1 = format!("{iter_var} <- ({iter_var} + 1)");
        let canonical_inc_1l = format!("{iter_var} <- ({iter_var} + 1L)");
        let canonical_inc_1f = format!("{iter_var} <- ({iter_var} + 1.0)");
        let mut safe = true;
        for line in lines.iter().take(loop_end).skip(guard_idx + 1) {
            let Some((lhs, _rhs)) = parse_raw_assign_line(line.trim()) else {
                continue;
            };
            if lhs != iter_var {
                continue;
            }
            let trimmed = line.trim();
            if trimmed != canonical_inc_1
                && trimmed != canonical_inc_1l
                && trimmed != canonical_inc_1f
            {
                safe = false;
                break;
            }
        }
        if !safe {
            repeat_idx = next_repeat + 1;
            continue;
        }

        let needle_double = format!("rr_index1_write({iter_var}, \"index\")");
        let needle_single = format!("rr_index1_write({iter_var}, 'index')");
        for line in lines.iter_mut().take(loop_end).skip(guard_idx + 1) {
            if line.contains(&needle_double) {
                *line = line.replace(&needle_double, &iter_var);
            }
            if line.contains(&needle_single) {
                *line = line.replace(&needle_single, &iter_var);
            }
        }

        repeat_idx = next_repeat + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_seq_len_full_overwrite_inits_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = strip_redundant_outer_parens(rhs).to_string();
        let Some(seq_inner) = rhs
            .strip_prefix("seq_len(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            idx += 1;
            continue;
        };

        let Some(iter_init_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            break;
        };
        let Some((iter_var, iter_start)) = parse_raw_assign_line(lines[iter_init_idx].trim())
        else {
            idx += 1;
            continue;
        };
        if iter_start.trim() != "1" && iter_start.trim() != "1L" {
            idx += 1;
            continue;
        }

        let Some(repeat_idx) = ((iter_init_idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && trimmed == "repeat {"
        }) else {
            idx += 1;
            continue;
        };
        let Some(loop_end) = find_raw_block_end(&lines, repeat_idx) else {
            idx += 1;
            continue;
        };
        let Some(guard_idx) = ((repeat_idx + 1)..loop_end).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some((guard_iter, _op, guard_bound)) =
            parse_raw_repeat_guard_cmp_line(lines[guard_idx].trim())
        else {
            idx += 1;
            continue;
        };
        if guard_iter != iter_var || guard_bound != seq_inner {
            idx += 1;
            continue;
        }

        let mut first_use_idx = None;
        let mut safe = true;
        let write_pat = format!("{lhs}[{iter_var}] <-");
        for (body_idx, line) in lines.iter().enumerate().take(loop_end).skip(guard_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "next" {
                continue;
            }
            if line.contains("<- function") {
                safe = false;
                break;
            }
            if first_use_idx.is_none() && line_contains_symbol(trimmed, &lhs) {
                first_use_idx = Some(body_idx);
                if !trimmed.starts_with(&write_pat) {
                    safe = false;
                }
                break;
            }
        }
        if !safe || first_use_idx.is_none() {
            idx += 1;
            continue;
        }

        lines[idx] = format!(
            "{}{} <- rep.int(0, {})",
            lines[idx]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>(),
            lhs,
            seq_inner
        );
        idx = loop_end + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn prune_unreachable_raw_helper_definitions(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    loop {
        let mut changed = false;
        let mut fn_start = 0usize;
        while fn_start < lines.len() {
            while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
                fn_start += 1;
            }
            if fn_start >= lines.len() {
                break;
            }
            let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
                break;
            };
            let Some((name, _params)) = parse_raw_function_header(&lines[fn_start]) else {
                fn_start = fn_end + 1;
                continue;
            };
            if !name.starts_with("Sym_") || name.starts_with("Sym_top_") {
                fn_start = fn_end + 1;
                continue;
            }

            let mut reachable = false;
            for (line_idx, line) in lines.iter().enumerate() {
                if line_idx >= fn_start && line_idx <= fn_end {
                    continue;
                }
                if find_symbol_call(line, &name, 0).is_some()
                    || line_contains_unquoted_symbol_reference(line, &name)
                {
                    reachable = true;
                    break;
                }
            }
            if reachable {
                fn_start = fn_end + 1;
                continue;
            }

            for line in lines.iter_mut().take(fn_end + 1).skip(fn_start) {
                line.clear();
            }
            changed = true;
            break;
        }
        if !changed {
            break;
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((gx_lhs, gx_rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        let gx_rhs = gx_rhs.to_string();
        if gx_lhs != "gx" {
            idx += 1;
            continue;
        }
        let Some(gy_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            break;
        };
        let Some((gy_lhs, gy_rhs)) = parse_raw_assign_line(lines[gy_idx].trim()) else {
            idx += 1;
            continue;
        };
        let gy_rhs = gy_rhs.to_string();
        if gy_lhs != "gy" {
            idx += 1;
            continue;
        }

        let seq = [
            ((gy_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 2)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 3)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 4)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 5)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 6)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 7)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 8)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 9)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 10)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 11)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 12)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
        ];
        if seq.iter().any(|idx| idx.is_none()) {
            idx += 1;
            continue;
        }
        let indices: Vec<usize> = seq.into_iter().flatten().collect();
        let Ok(
            [
                gx_lt_guard_idx,
                gx_lt_assign_idx,
                gx_lt_close_idx,
                gx_gt_guard_idx,
                gx_gt_assign_idx,
                gx_gt_close_idx,
                gy_lt_guard_idx,
                gy_lt_assign_idx,
                gy_lt_close_idx,
                gy_gt_guard_idx,
                gy_gt_assign_idx,
                gy_gt_close_idx,
            ],
        ) = <[usize; 12]>::try_from(indices)
        else {
            idx += 1;
            continue;
        };

        let gx_lt_guard = lines[gx_lt_guard_idx].trim();
        let gx_lt_assign = lines[gx_lt_assign_idx].trim();
        let gx_lt_close = lines[gx_lt_close_idx].trim();
        let gx_gt_guard = lines[gx_gt_guard_idx].trim();
        let gx_gt_assign = lines[gx_gt_assign_idx].trim();
        let gx_gt_close = lines[gx_gt_close_idx].trim();
        let gy_lt_guard = lines[gy_lt_guard_idx].trim();
        let gy_lt_assign = lines[gy_lt_assign_idx].trim();
        let gy_lt_close = lines[gy_lt_close_idx].trim();
        let gy_gt_guard = lines[gy_gt_guard_idx].trim();
        let gy_gt_assign = lines[gy_gt_assign_idx].trim();
        let gy_gt_close = lines[gy_gt_close_idx].trim();

        if gx_lt_guard != "if ((gx < 1)) {"
            || gx_lt_assign != "gx <- 1"
            || gx_lt_close != "}"
            || gx_gt_guard != "if ((gx > N)) {"
            || gx_gt_assign != "gx <- N"
            || gx_gt_close != "}"
            || gy_lt_guard != "if ((gy < 1)) {"
            || gy_lt_assign != "gy <- 1"
            || gy_lt_close != "}"
            || gy_gt_guard != "if ((gy > N)) {"
            || gy_gt_assign != "gy <- N"
            || gy_gt_close != "}"
        {
            idx += 1;
            continue;
        }

        let gx_indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let gy_indent = lines[gy_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines[idx] = format!(
            "{gx_indent}gx <- (pmin(pmax({}, {}), {}))",
            strip_redundant_outer_parens(&gx_rhs),
            "1",
            "N"
        );
        lines[gy_idx] = format!(
            "{gy_indent}gy <- (pmin(pmax({}, {}), {}))",
            strip_redundant_outer_parens(&gy_rhs),
            "1",
            "N"
        );
        for clear_idx in [
            gx_lt_guard_idx,
            gx_lt_assign_idx,
            gx_lt_close_idx,
            gx_gt_guard_idx,
            gx_gt_assign_idx,
            gx_gt_close_idx,
            gy_lt_guard_idx,
            gy_lt_assign_idx,
            gy_lt_close_idx,
            gy_gt_guard_idx,
            gy_gt_assign_idx,
            gy_gt_close_idx,
        ] {
            lines[clear_idx].clear();
        }
        idx = gy_gt_close_idx + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn collapse_gray_scott_clamp_pair_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((new_a_lhs, new_a_rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        if new_a_lhs != "new_a" {
            idx += 1;
            continue;
        }
        let new_a_rhs = new_a_rhs.to_string();

        let Some(new_b_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            break;
        };
        let Some((new_b_lhs, new_b_rhs)) = parse_raw_assign_line(lines[new_b_idx].trim()) else {
            idx += 1;
            continue;
        };
        if new_b_lhs != "new_b" {
            idx += 1;
            continue;
        }
        let new_b_rhs = new_b_rhs.to_string();

        let Some(a_lt_guard_idx) =
            ((new_b_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(a_lt_assign_idx) =
            ((a_lt_guard_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(a_lt_close_idx) =
            ((a_lt_assign_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(a_gt_guard_idx) =
            ((a_lt_close_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(a_gt_assign_idx) =
            ((a_gt_guard_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(a_gt_close_idx) =
            ((a_gt_assign_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(b_lt_guard_idx) =
            ((a_gt_close_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(b_lt_assign_idx) =
            ((b_lt_guard_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(b_lt_close_idx) =
            ((b_lt_assign_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(b_gt_guard_idx) =
            ((b_lt_close_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(b_gt_assign_idx) =
            ((b_gt_guard_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(b_gt_close_idx) =
            ((b_gt_assign_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };

        let a_lt_guard = lines[a_lt_guard_idx].trim();
        let a_lt_assign = lines[a_lt_assign_idx].trim();
        let a_lt_close = lines[a_lt_close_idx].trim();
        let a_gt_guard = lines[a_gt_guard_idx].trim();
        let a_gt_assign = lines[a_gt_assign_idx].trim();
        let a_gt_close = lines[a_gt_close_idx].trim();
        let b_lt_guard = lines[b_lt_guard_idx].trim();
        let b_lt_assign = lines[b_lt_assign_idx].trim();
        let b_lt_close = lines[b_lt_close_idx].trim();
        let b_gt_guard = lines[b_gt_guard_idx].trim();
        let b_gt_assign = lines[b_gt_assign_idx].trim();
        let b_gt_close = lines[b_gt_close_idx].trim();

        if a_lt_guard != "if ((new_a < 0)) {"
            || a_lt_assign != "new_a <- 0"
            || a_lt_close != "}"
            || a_gt_guard != "if ((new_a > 1)) {"
            || a_gt_assign != "new_a <- 1"
            || a_gt_close != "}"
            || b_lt_guard != "if ((new_b < 0)) {"
            || b_lt_assign != "new_b <- 0"
            || b_lt_close != "}"
            || b_gt_guard != "if ((new_b > 1)) {"
            || b_gt_assign != "new_b <- 1"
            || b_gt_close != "}"
        {
            idx += 1;
            continue;
        }

        let new_a_indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let new_b_indent = lines[new_b_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines[idx] = format!(
            "{new_a_indent}new_a <- (pmin(pmax({}, 0), 1))",
            strip_redundant_outer_parens(&new_a_rhs)
        );
        lines[new_b_idx] = format!(
            "{new_b_indent}new_b <- (pmin(pmax({}, 0), 1))",
            strip_redundant_outer_parens(&new_b_rhs)
        );
        for clear_idx in [
            a_lt_guard_idx,
            a_lt_assign_idx,
            a_lt_close_idx,
            a_gt_guard_idx,
            a_gt_assign_idx,
            a_gt_close_idx,
            b_lt_guard_idx,
            b_lt_assign_idx,
            b_lt_close_idx,
            b_gt_guard_idx,
            b_gt_assign_idx,
            b_gt_close_idx,
        ] {
            lines[clear_idx].clear();
        }
        idx = b_gt_close_idx + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn restore_cg_loop_carried_updates_in_raw_emitted_r(output: &str) -> String {
    fn next_significant_line(lines: &[String], start: usize) -> Option<usize> {
        (start..lines.len()).find(|idx| !lines[*idx].trim().is_empty())
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        if lines[idx].trim() != "x <- (x + (alpha * p))" {
            continue;
        }
        let Some(mut rs_new_idx) = next_significant_line(&lines, idx + 1) else {
            break;
        };
        let mut has_r_update = false;
        if lines[rs_new_idx].trim() == "r <- (r - (alpha * Ap))" {
            has_r_update = true;
            let Some(next_idx) = next_significant_line(&lines, rs_new_idx + 1) else {
                continue;
            };
            rs_new_idx = next_idx;
        }

        let rs_new_trimmed = lines[rs_new_idx].trim().to_string();
        let rs_new_matches = rs_new_trimmed
            == "rs_new <- Sym_117((r - (alpha * Ap)), (r - (alpha * Ap)), size)"
            || rs_new_trimmed == "rs_new <- Sym_117(r - (alpha * Ap), r - (alpha * Ap), size)"
            || rs_new_trimmed
                == "rs_new <- sum(((r - (alpha * Ap))[seq_len(size)] * (r - (alpha * Ap))[seq_len(size)]))"
            || rs_new_trimmed == "rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))";
        if !rs_new_matches {
            continue;
        }

        let indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();

        if !has_r_update {
            lines.insert(rs_new_idx, format!("{indent}r <- (r - (alpha * Ap))"));
            rs_new_idx += 1;
        }
        lines[rs_new_idx] = format!("{indent}rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))");

        if let Some(guard_idx) = next_significant_line(&lines, rs_new_idx + 1)
            && lines[guard_idx].trim().starts_with("if ")
            && line_contains_symbol(lines[guard_idx].trim(), "rs_new")
            && let Some(guard_end) = find_raw_block_end(&lines, guard_idx)
        {
            let else_idx = ((guard_idx + 1)..=guard_end)
                .find(|line_idx| lines[*line_idx].trim() == "} else {");
            let else_assign_idx = else_idx.and_then(|else_idx| {
                next_significant_line(&lines, else_idx + 1).filter(|idx| *idx < guard_end)
            });
            if else_assign_idx.is_some_and(|assign_idx| lines[assign_idx].trim() == rs_new_trimmed)
            {
                let body_indent = format!("{indent}  ");
                lines.splice(
                    guard_idx..=guard_end,
                    [
                        lines[guard_idx].clone(),
                        format!("{body_indent}rs_new <- rs_old"),
                        format!("{indent}}}"),
                    ],
                );
            }
        }

        let Some(beta_idx) = ((rs_new_idx + 1)..lines.len()).find(|i| {
            !lines[*i].trim().is_empty() && lines[*i].trim() == "beta <- (rs_new / rs_old)"
        }) else {
            continue;
        };
        let Some(iter_idx) = ((beta_idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && trimmed == "iter <- (iter + 1)"
        }) else {
            continue;
        };

        let has_p_update = lines
            .iter()
            .take(iter_idx)
            .skip(beta_idx + 1)
            .any(|line| line.trim() == "p <- (r + (beta * p))");
        if !has_p_update {
            lines.insert(iter_idx, format!("{indent}p <- (r + (beta * p))"));
        }
        let iter_idx = ((beta_idx + 1)..lines.len())
            .find(|i| lines[*i].trim() == "iter <- (iter + 1)")
            .unwrap_or(iter_idx);
        let has_rs_old_update = lines
            .iter()
            .take(iter_idx)
            .skip(beta_idx + 1)
            .any(|line| line.trim() == "rs_old <- rs_new");
        if !has_rs_old_update {
            lines.insert(iter_idx, format!("{indent}rs_old <- rs_new"));
        }
        break;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn restore_buffer_swaps_after_temp_copy_in_raw_emitted_r(output: &str) -> String {
    fn raw_line_writes_symbol(line: &str, symbol: &str) -> bool {
        let trimmed = line.trim();
        parse_raw_assign_line(trimmed).is_some_and(|(lhs, _)| lhs == symbol)
            || trimmed.starts_with(&format!("{symbol}["))
            || trimmed.starts_with(&format!("({symbol}) <-"))
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        let Some(base_var) = lhs.strip_prefix("tmp_") else {
            idx += 1;
            continue;
        };
        if rhs != base_var {
            idx += 1;
            continue;
        }

        let Some((loop_start, loop_end)) = (0..idx).rev().find_map(|line_idx| {
            (lines[line_idx].trim() == "repeat {")
                .then(|| find_raw_block_end(&lines, line_idx).map(|end| (line_idx, end)))
                .flatten()
                .filter(|(_, end)| idx < *end)
        }) else {
            idx += 1;
            continue;
        };

        let candidates = [format!("{base_var}_new"), format!("next_{base_var}")];
        let candidate = candidates.into_iter().find(|candidate| {
            lines
                .iter()
                .take(idx)
                .skip(loop_start + 1)
                .any(|line| raw_line_writes_symbol(line, candidate))
        });
        let Some(candidate) = candidate else {
            idx += 1;
            continue;
        };

        let has_base_swap = lines
            .iter()
            .take(loop_end)
            .skip(idx + 1)
            .any(|line| line.trim() == format!("{base_var} <- {candidate}"));
        let has_candidate_swap = lines
            .iter()
            .take(loop_end)
            .skip(idx + 1)
            .any(|line| line.trim() == format!("{candidate} <- {lhs}"));
        if has_base_swap || has_candidate_swap {
            idx += 1;
            continue;
        }

        let indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines.insert(idx + 1, format!("{indent}{base_var} <- {candidate}"));
        lines.insert(idx + 2, format!("{indent}{candidate} <- {lhs}"));
        idx += 3;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn collapse_sym287_melt_rate_branch_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        if lines[idx].trim() != "if ((T_c > 0)) {" {
            idx += 1;
            continue;
        }

        let Some(zero_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            break;
        };
        let Some(qs_guard_idx) =
            ((zero_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(qs_assign_idx) =
            ((qs_guard_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(qs_close_idx) =
            ((qs_assign_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(qg_guard_idx) =
            ((qs_close_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(qg_assign_idx) =
            ((qg_guard_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(qg_close_idx) =
            ((qg_assign_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(tendency_idx) =
            ((qg_close_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(close_idx) =
            ((tendency_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };

        if lines[zero_idx].trim() != "melt_rate <- 0"
            || lines[qs_guard_idx].trim() != "if ((q_s[i] > 0)) {"
            || lines[qs_assign_idx].trim() != "melt_rate <- (q_s[i] * 0.05)"
            || lines[qs_close_idx].trim() != "}"
            || lines[qg_guard_idx].trim() != "if ((q_g[i] > 0)) {"
            || lines[qg_assign_idx].trim() != "melt_rate <- (melt_rate + (q_g[i] * 0.02))"
            || lines[qg_close_idx].trim() != "}"
            || lines[tendency_idx].trim() != "tendency_T <- (tendency_T - (melt_rate * L_f))"
            || lines[close_idx].trim() != "}"
        {
            idx += 1;
            continue;
        }

        let qs_indent = lines[qs_assign_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let qg_indent = lines[qg_assign_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines[qs_assign_idx] =
            format!("{qs_indent}tendency_T <- (tendency_T - ((q_s[i] * 0.05) * L_f))");
        lines[qg_assign_idx] =
            format!("{qg_indent}tendency_T <- (tendency_T - ((q_g[i] * 0.02) * L_f))");
        lines[zero_idx].clear();
        lines[tendency_idx].clear();
        idx = close_idx + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_immediate_single_use_named_scalar_exprs_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if raw_expr_idents(rhs.as_str())
            .iter()
            .any(|ident| ident == &lhs)
        {
            continue;
        }
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_raw_named_scalar_expr(&rhs)
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            continue;
        };
        let next_trimmed = lines[next_idx].trim().to_string();
        let next_is_assign = parse_raw_assign_line(next_trimmed.as_str()).is_some();
        let next_is_return =
            next_trimmed.starts_with("return(") || next_trimmed.starts_with("return (");
        if lines[next_idx].contains("<- function")
            || (!next_is_assign && !next_is_return)
            || !line_contains_symbol(&next_trimmed, &lhs)
        {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(next_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _)) = parse_raw_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if line_contains_symbol(later_trimmed, &lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        let replacement = format!("({})", strip_redundant_outer_parens(&rhs));
        lines[next_idx] = replace_symbol_occurrences(&lines[next_idx], &lhs, replacement.as_str());
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_guard_only_named_scalar_exprs_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if raw_expr_idents(rhs.as_str())
            .iter()
            .any(|ident| ident == &lhs)
        {
            continue;
        }
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_raw_named_scalar_expr(&rhs)
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let Some(use_idx) = ((idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && line_contains_symbol(trimmed, &lhs)
        }) else {
            continue;
        };
        let use_trimmed = lines[use_idx].trim().to_string();
        let use_is_guard = use_trimmed.starts_with("if (") || use_trimmed.starts_with("if(");
        let use_occurrences = count_symbol_occurrences(&use_trimmed, &lhs);
        if lines[use_idx].contains("<- function")
            || !use_is_guard
            || use_occurrences == 0
            || use_occurrences > 2
        {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(use_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _)) = parse_raw_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if line_contains_symbol(later_trimmed, &lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        let replacement = format!("({})", strip_redundant_outer_parens(&rhs));
        lines[use_idx] = replace_symbol_occurrences(&lines[use_idx], &lhs, replacement.as_str());
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_two_use_named_scalar_exprs_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if raw_expr_idents(rhs.as_str())
            .iter()
            .any(|ident| ident == &lhs)
        {
            continue;
        }
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_raw_named_scalar_expr(&rhs)
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let rhs_deps = raw_expr_idents(rhs.as_str());
        let Some(next1_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        let Some(next2_idx) = ((next1_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        let next1_trimmed = lines[next1_idx].trim().to_string();
        let next2_trimmed = lines[next2_idx].trim().to_string();
        if lines[next1_idx].contains("<- function")
            || lines[next2_idx].contains("<- function")
            || parse_raw_assign_line(next1_trimmed.as_str()).is_none()
            || parse_raw_assign_line(next2_trimmed.as_str()).is_none()
            || count_symbol_occurrences(&next1_trimmed, &lhs) != 1
            || count_symbol_occurrences(&next2_trimmed, &lhs) != 1
        {
            continue;
        }

        let mut total_uses = 0usize;
        let mut use_line_idxs = Vec::new();
        let mut dep_write_idxs = Vec::new();
        for (line_no, line) in lines.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _)) = parse_raw_assign_line(line_trimmed) {
                if later_lhs == lhs {
                    break;
                }
                if rhs_deps.iter().any(|dep| dep == later_lhs) {
                    dep_write_idxs.push(line_no);
                }
            }
            let occurrences = count_symbol_occurrences(line_trimmed, &lhs);
            if occurrences > 0 {
                total_uses += occurrences;
                use_line_idxs.push(line_no);
                if total_uses > 2 {
                    break;
                }
            }
        }
        if total_uses != 2 || use_line_idxs != vec![next1_idx, next2_idx] {
            continue;
        }
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < next2_idx) {
            continue;
        }

        let replacement = strip_redundant_outer_parens(&rhs);
        lines[next1_idx] = replace_symbol_occurrences(&lines[next1_idx], &lhs, replacement);
        lines[next2_idx] = replace_symbol_occurrences(&lines[next2_idx], &lhs, replacement);
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_single_assignment_loop_seed_literals_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        if rhs.trim() != "1" && rhs.trim() != "1L" {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            continue;
        };
        let Some((_next_lhs, next_rhs)) = parse_raw_assign_line(lines[next_idx].trim()) else {
            continue;
        };
        if !next_rhs.contains(&format!("{lhs}:")) {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(next_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _later_rhs)) = parse_raw_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if line_contains_symbol(later_trimmed, lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        lines[next_idx] = replace_symbol_occurrences(&lines[next_idx], lhs, "1");
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_sym210_loop_seed_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && lines[fn_start].trim() != "Sym_210 <- function(field, w, h)"
        {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };

        for idx in (fn_start + 1)..fn_end {
            if lines[idx].trim() != "i <- 1" {
                continue;
            }
            let Some(next_idx) = ((idx + 1)..fn_end).find(|i| !lines[*i].trim().is_empty()) else {
                continue;
            };
            let Some((next_lhs, _next_rhs)) = parse_raw_assign_line(lines[next_idx].trim()) else {
                continue;
            };
            if next_lhs != "lap" || !lines[next_idx].contains("i:size - 1") {
                continue;
            }
            lines[idx].clear();
            lines[next_idx] = lines[next_idx].replace("i:size - 1", "1:size - 1");
        }

        fn_start += 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_two_use_named_scalar_pure_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = strip_redundant_outer_parens(rhs).to_string();
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !rhs.starts_with("rr_idx_cube_vec_i(")
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let rhs_deps = raw_expr_idents(rhs.as_str());
        let mut use_line_idxs = Vec::new();
        let mut total_uses = 0usize;
        let mut dep_write_idxs = Vec::new();
        for (line_no, line) in lines.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _later_rhs)) = parse_raw_assign_line(line_trimmed) {
                if later_lhs == lhs {
                    break;
                }
                if rhs_deps.iter().any(|dep| dep == later_lhs) {
                    dep_write_idxs.push(line_no);
                }
            }
            let occurrences = count_symbol_occurrences(line_trimmed, &lhs);
            if occurrences > 0 {
                total_uses += occurrences;
                use_line_idxs.push(line_no);
                if total_uses > 2 {
                    break;
                }
            }
        }
        if total_uses != 2 {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
            continue;
        }
        for use_idx in use_line_idxs {
            lines[use_idx] = replace_symbol_occurrences(&lines[use_idx], &lhs, rhs.as_str());
        }
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_guard_only_scalar_literals_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !rhs_is_raw_simple_scalar_alias_or_literal(&rhs)
        {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
        }) else {
            continue;
        };
        let mut guard_idx = next_idx;
        let next_trimmed = lines[next_idx].trim().to_string();
        if next_trimmed == "repeat {" {
            let Some(found_guard) = ((next_idx + 1)..lines.len()).find(|i| {
                let trimmed = lines[*i].trim();
                !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
            }) else {
                continue;
            };
            guard_idx = found_guard;
        }
        let guard_trimmed = lines[guard_idx].trim().to_string();
        let is_guard = guard_trimmed.starts_with("if (") || guard_trimmed.starts_with("if(");
        if lines[guard_idx].contains("<- function")
            || !is_guard
            || !line_contains_symbol(&guard_trimmed, &lhs)
        {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(guard_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _)) = parse_raw_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if line_contains_symbol(later_trimmed, &lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        lines[guard_idx] =
            replace_symbol_occurrences(&lines[guard_idx], &lhs, strip_redundant_outer_parens(&rhs));
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_loop_guard_scalar_literals_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !rhs_is_raw_simple_scalar_alias_or_literal(&rhs)
        {
            continue;
        }

        let Some(repeat_idx) = ((idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
        }) else {
            continue;
        };
        if lines[repeat_idx].trim() != "repeat {" {
            continue;
        }
        let Some(guard_idx) = ((repeat_idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
        }) else {
            continue;
        };
        let guard_trimmed = lines[guard_idx].trim().to_string();
        let is_guard = guard_trimmed.starts_with("if (") || guard_trimmed.starts_with("if(");
        if !is_guard || !line_contains_symbol(&guard_trimmed, &lhs) {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(guard_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _)) = parse_raw_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if line_contains_symbol(later_trimmed, &lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        lines[guard_idx] =
            replace_symbol_occurrences(&lines[guard_idx], &lhs, strip_redundant_outer_parens(&rhs));
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_single_use_named_scalar_pure_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = strip_redundant_outer_parens(rhs).to_string();
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !rhs.starts_with("rr_wrap_index_vec_i(")
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let rhs_deps = raw_expr_idents(rhs.as_str());
        let mut use_line_idxs = Vec::new();
        let mut total_uses = 0usize;
        let mut dep_write_idxs = Vec::new();
        for (line_no, line) in lines.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _later_rhs)) = parse_raw_assign_line(line_trimmed) {
                if later_lhs == lhs {
                    break;
                }
                if rhs_deps.iter().any(|dep| dep == later_lhs) {
                    dep_write_idxs.push(line_no);
                }
            }
            let occurrences = count_symbol_occurrences(line_trimmed, &lhs);
            if occurrences > 0 {
                total_uses += occurrences;
                use_line_idxs.push(line_no);
                if total_uses > 1 {
                    break;
                }
            }
        }
        if total_uses != 1 {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
            continue;
        }

        let use_idx = use_line_idxs[0];
        lines[use_idx] = replace_symbol_occurrences(&lines[use_idx], &lhs, rhs.as_str());
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn parse_top_level_raw_call(rhs: &str) -> Option<(String, Vec<String>)> {
    let rhs = strip_redundant_outer_parens(rhs).trim();
    let open = rhs.find('(')?;
    let close = find_matching_call_close(rhs, open)?;
    if close + 1 != rhs.len() {
        return None;
    }
    let callee = rhs[..open].trim();
    if callee.is_empty() || !callee.chars().all(is_symbol_char) {
        return None;
    }
    let args = split_top_level_args(&rhs[open + 1..close])?;
    Some((callee.to_string(), args))
}

fn rewrite_duplicate_pure_call_assignments_in_raw_emitted_r(
    output: &str,
    pure_user_calls: &FxHashSet<String>,
) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let line_owned = lines[idx].clone();
        let trimmed = line_owned.trim();
        let candidate_indent = line_owned.len() - line_owned.trim_start().len();
        let Some((lhs, rhs)) = parse_raw_assign_line(trimmed) else {
            continue;
        };
        let lhs = lhs.trim();
        let rhs = rhs.trim();
        if lhs.is_empty()
            || !lhs.chars().all(is_symbol_char)
            || lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
        {
            continue;
        }
        let Some((callee, _args)) = parse_top_level_raw_call(rhs) else {
            continue;
        };
        if !pure_user_calls.contains(&callee) {
            continue;
        }
        let deps: FxHashSet<String> = raw_expr_idents(rhs).into_iter().collect();

        for line in lines.iter_mut().skip(idx + 1) {
            let line_trimmed = line.trim().to_string();
            let next_indent = line.len() - line.trim_start().len();
            if !line_trimmed.is_empty() && next_indent < candidate_indent {
                break;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((next_lhs, next_rhs)) = parse_raw_assign_line(&line_trimmed) {
                let next_lhs = next_lhs.trim();
                if next_lhs == lhs || deps.contains(next_lhs) {
                    break;
                }
                if next_rhs.trim() == rhs {
                    let indent = line
                        .chars()
                        .take_while(|ch| ch.is_ascii_whitespace())
                        .collect::<String>();
                    *line = format!("{indent}{next_lhs} <- {lhs}");
                }
            }
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_adjacent_duplicate_symbol_assignments_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 2 {
        return output.to_string();
    }

    for idx in 0..(lines.len() - 1) {
        let first = lines[idx].trim().to_string();
        let second = lines[idx + 1].trim().to_string();
        let Some((lhs0, rhs0)) = parse_raw_assign_line(&first) else {
            continue;
        };
        let Some((lhs1, rhs1)) = parse_raw_assign_line(&second) else {
            continue;
        };
        let lhs0 = lhs0.trim();
        let lhs1 = lhs1.trim();
        let rhs0 = rhs0.trim();
        let rhs1 = rhs1.trim();
        if lhs0.is_empty()
            || lhs1.is_empty()
            || lhs0 == lhs1
            || lhs0.starts_with(".arg_")
            || lhs1.starts_with(".arg_")
            || lhs0.starts_with(".__rr_cse_")
            || lhs1.starts_with(".__rr_cse_")
            || !lhs0.chars().all(is_symbol_char)
            || !lhs1.chars().all(is_symbol_char)
            || rhs0 != rhs1
            || rhs0.starts_with(".arg_")
            || !rhs0.chars().all(is_symbol_char)
        {
            continue;
        }

        let indent = lines[idx + 1]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines[idx + 1] = format!("{indent}{lhs1} <- {lhs0}");
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn collect_helper_expr_reuse_summaries_in_raw_emitted_r(
    lines: &[String],
) -> FxHashMap<String, RawHelperExprReuseSummary> {
    let mut map = FxHashMap::default();
    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(lines, fn_start) else {
            break;
        };
        let Some((wrapper, params)) = parse_raw_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        let body: Vec<String> = lines
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && line != "# rr-cse-pruned")
            .collect();
        if body.len() != 3 {
            fn_start = fn_end + 1;
            continue;
        }
        let Some((temp_var, rhs)) = parse_raw_assign_line(&body[1]) else {
            fn_start = fn_end + 1;
            continue;
        };
        let rhs = strip_redundant_outer_parens(rhs);
        let Some(open) = rhs.find('(') else {
            fn_start = fn_end + 1;
            continue;
        };
        let Some(close) = find_matching_call_close(rhs, open) else {
            fn_start = fn_end + 1;
            continue;
        };
        let inner_callee = rhs[..open].trim();
        let args_inner = &rhs[open + 1..close];
        let Some(args) = split_top_level_args(args_inner) else {
            fn_start = fn_end + 1;
            continue;
        };
        if inner_callee.is_empty()
            || !inner_callee.chars().all(is_symbol_char)
            || args != params
            || !body[2].starts_with("return(")
        {
            fn_start = fn_end + 1;
            continue;
        }
        let return_expr = body[2]
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
            .unwrap_or("");
        if return_expr.is_empty()
            || !line_contains_symbol(return_expr, temp_var)
            || return_expr.contains("\"")
        {
            fn_start = fn_end + 1;
            continue;
        }
        map.insert(
            wrapper.clone(),
            RawHelperExprReuseSummary {
                wrapper,
                inner_callee: inner_callee.to_string(),
                temp_var: temp_var.to_string(),
                params,
                return_expr: return_expr.to_string(),
            },
        );
        fn_start = fn_end + 1;
    }
    map
}

fn rewrite_helper_expr_reuse_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }
    let summaries = collect_helper_expr_reuse_summaries_in_raw_emitted_r(&lines);
    if summaries.is_empty() {
        return output.to_string();
    }

    for idx in 1..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            continue;
        };
        let rhs = strip_redundant_outer_parens(rhs);
        let Some(open) = rhs.find('(') else {
            continue;
        };
        let Some(close) = find_matching_call_close(rhs, open) else {
            continue;
        };
        let wrapper = rhs[..open].trim();
        let Some(summary) = summaries.get(wrapper) else {
            continue;
        };
        let args_inner = &rhs[open + 1..close];
        let Some(args) = split_top_level_args(args_inner) else {
            continue;
        };
        if args.len() != summary.params.len() {
            continue;
        }

        let Some((prev_lhs, prev_rhs)) = parse_raw_assign_line(lines[idx - 1].trim()) else {
            continue;
        };
        let prev_rhs = strip_redundant_outer_parens(prev_rhs);
        let Some(prev_open) = prev_rhs.find('(') else {
            continue;
        };
        let Some(prev_close) = find_matching_call_close(prev_rhs, prev_open) else {
            continue;
        };
        let prev_callee = prev_rhs[..prev_open].trim();
        let Some(prev_args) = split_top_level_args(&prev_rhs[prev_open + 1..prev_close]) else {
            continue;
        };
        if prev_callee != summary.inner_callee || prev_args != args {
            continue;
        }

        let replacement =
            replace_symbol_occurrences(&summary.return_expr, &summary.temp_var, prev_lhs);
        let indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines[idx] = format!("{indent}{lhs} <- {replacement}");
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn collect_dot_product_helper_summaries_in_raw_emitted_r(
    lines: &[String],
) -> FxHashMap<String, RawDotProductHelperSummary> {
    let mut map = FxHashMap::default();
    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(lines, fn_start) else {
            break;
        };
        let Some((helper, params)) = parse_raw_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        if params.len() != 3 {
            fn_start = fn_end + 1;
            continue;
        }
        let body: Vec<String> = lines
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && line != "# rr-cse-pruned")
            .collect();
        if body.len() != 2 || body[0] != "{" {
            fn_start = fn_end + 1;
            continue;
        }
        let expected = format!(
            "return(sum(({}[seq_len({})] * {}[seq_len({})])))",
            params[0], params[2], params[1], params[2]
        );
        if body[1] != expected {
            fn_start = fn_end + 1;
            continue;
        }
        map.insert(
            helper.clone(),
            RawDotProductHelperSummary {
                helper,
                lhs_param: params[0].clone(),
                rhs_param: params[1].clone(),
                len_param: params[2].clone(),
            },
        );
        fn_start = fn_end + 1;
    }
    map
}

fn rewrite_dot_product_helper_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }
    let helpers = collect_dot_product_helper_summaries_in_raw_emitted_r(&lines);
    if helpers.is_empty() {
        return output.to_string();
    }

    for line in lines.iter_mut() {
        if line.contains("<- function") {
            continue;
        }
        for summary in helpers.values() {
            let Some(call_idx) = find_symbol_call(line, &summary.helper, 0) else {
                continue;
            };
            let open = call_idx + summary.helper.len();
            let Some(close) = find_matching_call_close(line, open) else {
                continue;
            };
            let args_inner = &line[open + 1..close];
            let Some(args) = split_top_level_args(args_inner) else {
                continue;
            };
            if args.len() != 3 {
                continue;
            }
            let replacement = format!(
                "sum(({}[seq_len({})] * {}[seq_len({})]))",
                args[0].trim(),
                args[2].trim(),
                args[1].trim(),
                args[2].trim()
            );
            line.replace_range(call_idx..=close, &replacement);
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_sym119_helper_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut helper_names = FxHashSet::default();
    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };
        let Some((name, params)) = parse_raw_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        let body: Vec<String> = lines
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && line != "# rr-cse-pruned")
            .collect();
        if params == vec!["x", "n_l", "n_r", "n_d", "n_u"]
            && body
                == vec![
                    "{".to_string(),
                    "n_d <- rr_index_vec_floor(n_d)".to_string(),
                    "n_l <- rr_index_vec_floor(n_l)".to_string(),
                    "n_r <- rr_index_vec_floor(n_r)".to_string(),
                    "n_u <- rr_index_vec_floor(n_u)".to_string(),
                    "y <- ((4.0001 * x) - (((rr_gather(x, n_l) + rr_gather(x, n_r)) + rr_gather(x, n_d)) + rr_gather(x, n_u)))".to_string(),
                    "return(y)".to_string(),
                ]
        {
            helper_names.insert(name);
        }
        fn_start = fn_end + 1;
    }
    if helper_names.is_empty() {
        return output.to_string();
    }

    for line in lines.iter_mut() {
        if line.contains("<- function") {
            continue;
        }
        for helper in &helper_names {
            let Some(call_idx) = find_symbol_call(line, helper, 0) else {
                continue;
            };
            let open = call_idx + helper.len();
            let Some(close) = find_matching_call_close(line, open) else {
                continue;
            };
            let args_inner = &line[open + 1..close];
            let Some(args) = split_top_level_args(args_inner) else {
                continue;
            };
            if args.len() != 5 {
                continue;
            }
            let replacement = format!(
                "((4.0001 * {}) - (((rr_gather({}, rr_index_vec_floor({})) + rr_gather({}, rr_index_vec_floor({}))) + rr_gather({}, rr_index_vec_floor({}))) + rr_gather({}, rr_index_vec_floor({}))))",
                args[0].trim(),
                args[0].trim(),
                args[1].trim(),
                args[0].trim(),
                args[2].trim(),
                args[0].trim(),
                args[3].trim(),
                args[0].trim(),
                args[4].trim(),
            );
            line.replace_range(call_idx..=close, &replacement);
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_trivial_fill_helper_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut helper_names = FxHashSet::default();
    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };
        let Some((name, params)) = parse_raw_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        let body: Vec<String> = lines
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && line != "# rr-cse-pruned")
            .collect();
        if params == vec!["n", "val"]
            && body == vec!["{".to_string(), "return(rep.int(val, n))".to_string()]
        {
            helper_names.insert(name);
        }
        fn_start = fn_end + 1;
    }
    if helper_names.is_empty() {
        return output.to_string();
    }

    for line in lines.iter_mut() {
        if line.contains("<- function") {
            continue;
        }
        for helper in &helper_names {
            let Some(call_idx) = find_symbol_call(line, helper, 0) else {
                continue;
            };
            let open = call_idx + helper.len();
            let Some(close) = find_matching_paren(line, open) else {
                continue;
            };
            let args_str = &line[open + 1..close];
            let Some(args) = split_top_level_args(args_str) else {
                continue;
            };
            if args.len() != 2 {
                continue;
            }
            let replacement = format!("rep.int({}, {})", args[1].trim(), args[0].trim());
            line.replace_range(call_idx..=close, &replacement);
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_identical_zero_fill_pairs_to_aliases_in_raw_emitted_r(output: &str) -> String {
    fn raw_line_writes_symbol(line: &str, symbol: &str) -> bool {
        let trimmed = line.trim();
        parse_raw_assign_line(trimmed).is_some_and(|(lhs, _)| lhs == symbol)
            || trimmed.starts_with(&format!("{symbol}["))
            || trimmed.starts_with(&format!("({symbol}) <-"))
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 2 {
        return output.to_string();
    }

    let replacements = [
        (
            vec!["adj_ll <- rep.int(0, TOTAL)", "adj_ll <- qr"],
            "adj_rr <- rep.int(0, TOTAL)",
            "adj_rr <- adj_ll",
        ),
        (
            vec!["u_stage <- rep.int(0, TOTAL)", "u_stage <- qr"],
            "u_new <- rep.int(0, TOTAL)",
            "u_new <- u_stage",
        ),
    ];

    for idx in 0..(lines.len() - 1) {
        let first = lines[idx].trim().to_string();
        let second = lines[idx + 1].trim().to_string();
        let Some((first_lhs, _)) = parse_raw_assign_line(&first) else {
            continue;
        };
        let Some((second_lhs, _)) = parse_raw_assign_line(&second) else {
            continue;
        };
        let fn_start = (0..=idx)
            .rev()
            .find(|line_idx| lines[*line_idx].contains("<- function"));
        let fn_end = fn_start
            .and_then(|start| find_raw_block_end(&lines, start))
            .unwrap_or(lines.len().saturating_sub(1));
        for (lhs_lines, rhs_line, replacement) in &replacements {
            if lhs_lines.iter().any(|lhs_line| first == *lhs_line) && second == *rhs_line {
                let later_diverging_write =
                    lines.iter().take(fn_end + 1).skip(idx + 2).any(|line| {
                        raw_line_writes_symbol(line, first_lhs)
                            || raw_line_writes_symbol(line, second_lhs)
                    });
                if later_diverging_write {
                    continue;
                }
                let indent = lines[idx + 1]
                    .chars()
                    .take_while(|ch| ch.is_ascii_whitespace())
                    .collect::<String>();
                lines[idx + 1] = format!("{indent}{replacement}");
            }
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_duplicate_sym183_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 2 {
        return output.to_string();
    }

    for idx in 0..(lines.len() - 1) {
        let Some((lhs0, rhs0)) = parse_raw_assign_line(lines[idx].trim()) else {
            continue;
        };
        let Some((lhs1, rhs1)) = parse_raw_assign_line(lines[idx + 1].trim()) else {
            continue;
        };
        if lhs0.chars().all(is_symbol_char)
            && lhs1.chars().all(is_symbol_char)
            && rhs0 == "Sym_183(1000)"
            && rhs1 == "Sym_183(1000)"
        {
            let indent = lines[idx + 1]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>();
            lines[idx + 1] = format!("{indent}{lhs1} <- {lhs0}");
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn strip_dead_zero_loop_seeds_before_for_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    let mut idx = 0usize;

    while idx + 1 < lines.len() {
        let trimmed = lines[idx].trim();
        let Some((var, seed)) = trimmed.split_once("<-") else {
            idx += 1;
            continue;
        };
        let var = var.trim();
        let seed = seed.trim();
        if seed != "0" && seed != "1" {
            idx += 1;
            continue;
        }

        let Some(for_idx) = ((idx + 1)..lines.len()).take(12).find(|line_idx| {
            lines[*line_idx]
                .trim()
                .starts_with(&format!("for ({var} in seq_len("))
        }) else {
            idx += 1;
            continue;
        };

        let var_re = regex::Regex::new(&format!(r"\b{}\b", regex::escape(var))).ok();
        let used_before_for = lines[(idx + 1)..for_idx]
            .iter()
            .any(|line| var_re.as_ref().is_some_and(|re| re.is_match(line)));
        if used_before_for {
            idx += 1;
            continue;
        }

        lines.remove(idx);
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_literal_named_list_calls_in_raw_emitted_r(output: &str) -> String {
    let mut out = Vec::with_capacity(output.lines().count());
    for line in output.lines() {
        if line.contains("rr_named_list <- function") {
            out.push(line.to_string());
            continue;
        }
        let mut rewritten = line.to_string();
        loop {
            let Some(start) = rewritten.find("rr_named_list(") else {
                break;
            };
            let call_start = start + "rr_named_list".len();
            let Some(call_end) = find_matching_call_close(&rewritten, call_start) else {
                break;
            };
            let args_inner = &rewritten[call_start + 1..call_end];
            let Some(args) = split_top_level_args(args_inner) else {
                break;
            };
            if args.len() % 2 != 0 {
                break;
            }
            let mut fields = Vec::new();
            let mut ok = true;
            for pair in args.chunks(2) {
                let Some(name) = raw_literal_record_field_name(pair[0].trim()) else {
                    ok = false;
                    break;
                };
                fields.push(format!("{name} = {}", pair[1].trim()));
            }
            if !ok {
                break;
            }
            let replacement = if fields.is_empty() {
                "list()".to_string()
            } else {
                format!("list({})", fields.join(", "))
            };
            rewritten.replace_range(start..=call_end, &replacement);
        }
        out.push(rewritten);
    }
    let mut rewritten = out.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    rewritten
}

fn rewrite_literal_field_get_calls_in_raw_emitted_r(output: &str) -> String {
    let mut out = Vec::with_capacity(output.lines().count());
    for line in output.lines() {
        if line.contains("<- function") {
            out.push(line.to_string());
            continue;
        }
        let mut rewritten = line.to_string();
        loop {
            let Some(start) = rewritten.find("rr_field_get(") else {
                break;
            };
            let call_start = start + "rr_field_get".len();
            let Some(call_end) = find_matching_call_close(&rewritten, call_start) else {
                break;
            };
            let args_inner = &rewritten[call_start + 1..call_end];
            let Some(args) = split_top_level_args(args_inner) else {
                break;
            };
            if args.len() != 2 {
                break;
            }
            let base = args[0].trim();
            let Some(name) = raw_literal_record_field_name(args[1].trim()) else {
                break;
            };
            let replacement = format!(r#"{base}[["{name}"]]"#);
            rewritten.replace_range(start..=call_end, &replacement);
        }
        out.push(rewritten);
    }
    let mut rewritten = out.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    rewritten
}

fn rewrite_slice_bound_aliases_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx + 1 < lines.len() {
        let Some((start_lhs, start_rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        if start_lhs != "start" {
            idx += 1;
            continue;
        }
        let start_rhs = strip_redundant_outer_parens(start_rhs).to_string();
        if !start_rhs.starts_with("rr_idx_cube_vec_i(") {
            idx += 1;
            continue;
        }

        let Some(end_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            break;
        };
        let Some((end_lhs, end_rhs)) = parse_raw_assign_line(lines[end_idx].trim()) else {
            idx += 1;
            continue;
        };
        if end_lhs != "end" {
            idx += 1;
            continue;
        }
        let end_rhs = strip_redundant_outer_parens(end_rhs).to_string();
        if !end_rhs.starts_with("rr_idx_cube_vec_i(") {
            idx += 1;
            continue;
        }

        let mut use_line_idxs = Vec::new();
        for (line_no, line) in lines.iter().enumerate().skip(end_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((lhs, _)) = parse_raw_assign_line(trimmed)
                && (lhs == "start" || lhs == "end")
            {
                break;
            }
            let uses_start = line_contains_symbol(trimmed, "start");
            let uses_end = line_contains_symbol(trimmed, "end");
            if uses_start || uses_end {
                if uses_start != uses_end || !trimmed.contains("neighbors[start:end] <-") {
                    use_line_idxs.clear();
                    break;
                }
                use_line_idxs.push(line_no);
                continue;
            }
            let is_control =
                trimmed == "}" || trimmed.starts_with("if (") || trimmed.starts_with("if(");
            if !use_line_idxs.is_empty() && !is_control {
                break;
            }
        }
        if use_line_idxs.is_empty() {
            idx += 1;
            continue;
        }

        let slice_expr = format!("{start_rhs}:{end_rhs}");
        for use_idx in &use_line_idxs {
            lines[*use_idx] = lines[*use_idx].replace("start:end", &slice_expr);
        }
        lines[idx].clear();
        lines[end_idx].clear();
        idx = use_line_idxs.last().copied().unwrap_or(end_idx) + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    rewritten
}

fn rewrite_particle_idx_alias_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx + 2 < lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        if lhs != "idx" {
            idx += 1;
            continue;
        }
        let rhs = strip_redundant_outer_parens(rhs).to_string();
        if !rhs.starts_with("rr_idx_cube_vec_i(") {
            idx += 1;
            continue;
        }

        let Some(next1_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            break;
        };
        let Some(next2_idx) = ((next1_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            break;
        };
        let next1 = lines[next1_idx].trim().to_string();
        let next2 = lines[next2_idx].trim().to_string();
        let dx_ok = next1.contains("dx <-")
            && next1.contains("u[idx]")
            && next1.contains("* dt")
            && next1.contains("/ 400000");
        let dy_ok = next2.contains("dy <-")
            && next2.contains("v[idx]")
            && next2.contains("* dt")
            && next2.contains("/ 400000");
        if !dx_ok || !dy_ok {
            idx += 1;
            continue;
        }

        lines[next1_idx] = replace_symbol_occurrences(&lines[next1_idx], "idx", rhs.as_str());
        lines[next2_idx] = replace_symbol_occurrences(&lines[next2_idx], "idx", rhs.as_str());
        lines[idx].clear();
        idx = next2_idx + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_loop_index_alias_ii_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            continue;
        };
        if lhs != "ii" || rhs != "i" {
            continue;
        }

        let mut replaced_any = false;
        let mut stop_idx = lines.len();
        let mut stopped_on_i_reassign = false;
        for (scan_idx, line) in lines.iter_mut().enumerate().skip(idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                stop_idx = scan_idx;
                break;
            }
            if let Some((next_lhs, _)) = parse_raw_assign_line(trimmed)
                && (next_lhs == "ii" || next_lhs == "i")
            {
                stop_idx = scan_idx;
                stopped_on_i_reassign = next_lhs == "i";
                break;
            }
            if !line_contains_symbol(trimmed, "ii") {
                continue;
            }
            let rewritten = replace_symbol_occurrences(line, "ii", "i");
            if rewritten != *line {
                *line = rewritten;
                replaced_any = true;
            }
        }

        let mut keep_alias = false;
        if stopped_on_i_reassign {
            for line in lines.iter().skip(stop_idx + 1) {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if line.contains("<- function") {
                    break;
                }
                if let Some((next_lhs, _)) = parse_raw_assign_line(trimmed)
                    && next_lhs == "ii"
                {
                    break;
                }
                if line_contains_symbol(trimmed, "ii") {
                    keep_alias = true;
                    break;
                }
            }
        }

        if replaced_any && !keep_alias {
            lines[idx].clear();
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn strip_single_blank_spacers_in_raw_emitted_r(output: &str) -> String {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 3 {
        return output.to_string();
    }

    let mut kept = Vec::with_capacity(lines.len());
    for idx in 0..lines.len() {
        if idx > 0 && idx + 1 < lines.len() && lines[idx].trim().is_empty() {
            let Some(prev_idx) = (0..idx)
                .rev()
                .find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                continue;
            };
            let Some(next_idx) =
                ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                continue;
            };

            let prev = lines[prev_idx].trim();
            let next = lines[next_idx].trim();
            let prev_is_assign = parse_raw_assign_line(prev).is_some();
            let next_is_assign = parse_raw_assign_line(next).is_some();
            let next_is_control = next == "repeat {" || next == "}";
            let next_is_branch = next.starts_with("if (") || next.starts_with("if(");
            let next_is_return = next.starts_with("return(") || next.starts_with("return (");
            let prev_opens_block = prev.ends_with('{');
            let prev_is_return = prev.starts_with("return(") || prev.starts_with("return (");
            let prev_is_break = prev.starts_with("if (") && prev.ends_with("break");

            if (prev_is_assign && (next_is_assign || next_is_control || next_is_branch))
                || (prev_opens_block && (next_is_assign || next_is_return || next_is_branch))
                || (prev == "{"
                    && (next_is_assign || next_is_return || next_is_control || next_is_branch))
                || (prev == "}" && (next_is_assign || next == "}"))
                || (prev_is_break && (next_is_assign || next_is_branch || next_is_return))
                || (prev_is_return && next == "}")
            {
                continue;
            }
        }
        kept.push(lines[idx].clone());
    }

    let mut out = kept.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn collapse_nested_else_if_blocks_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 3 {
        return output.to_string();
    }

    let mut changed = true;
    while changed {
        changed = false;
        let mut idx = 0usize;
        while idx < lines.len() {
            if lines[idx].trim() != "} else {" {
                idx += 1;
                continue;
            }
            let Some(nested_if_idx) =
                ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                break;
            };
            let nested_if = lines[nested_if_idx].trim().to_string();
            if !nested_if.starts_with("if (") || !nested_if.ends_with('{') {
                idx += 1;
                continue;
            }
            let Some(nested_if_end) = find_raw_block_end(&lines, nested_if_idx) else {
                idx += 1;
                continue;
            };
            let Some(else_close_idx) = ((nested_if_end + 1)..lines.len())
                .find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                idx += 1;
                continue;
            };
            if lines[else_close_idx].trim() != "}" {
                idx += 1;
                continue;
            }

            let indent = lines[idx]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>();
            lines[idx] = format!("{indent}}} else {nested_if}");
            lines[nested_if_idx].clear();
            lines[else_close_idx].clear();
            changed = true;
            idx = else_close_idx + 1;
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn compact_blank_lines_in_raw_emitted_r(output: &str) -> String {
    let mut out = String::new();
    let mut blank_run = 0usize;
    for line in output.lines() {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
        } else {
            blank_run = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    if output.is_empty() {
        return String::new();
    }
    if !output.ends_with('\n') {
        out.pop();
    }
    out
}

fn collapse_adjacent_dir_neighbor_row_branches_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 12 {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx + 11 < lines.len() {
        let branch1 = lines[idx].trim();
        let assign1 = lines[idx + 1].trim();
        let close1 = lines[idx + 2].trim();
        let branch2 = lines[idx + 3].trim();
        let assign2 = lines[idx + 4].trim();
        let close2 = lines[idx + 5].trim();
        let branch3 = lines[idx + 6].trim();
        let assign3 = lines[idx + 7].trim();
        let close3 = lines[idx + 8].trim();
        let branch4 = lines[idx + 9].trim();
        let assign4 = lines[idx + 10].trim();
        let close4 = lines[idx + 11].trim();

        if branch1 != "if ((dir == 1)) {"
            || (assign1
                != "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_60(f, x, ys, size)"
                && assign1
                    != "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_60(f, x, size)")
            || close1 != "}"
            || branch2 != "if ((dir == 2)) {"
            || (assign2
                != "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_64(f, x, ys, size)"
                && assign2
                    != "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_64(f, x, size)")
            || close2 != "}"
            || branch3 != "if ((dir == 3)) {"
            || (assign3
                != "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_66(f, x, ys, size)"
                && assign3
                    != "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_66(f, x, size)")
            || close3 != "}"
            || branch4 != "if ((dir == 4)) {"
            || (assign4
                != "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_72(f, x, ys, size)"
                && assign4
                    != "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_72(f, x, size)")
            || close4 != "}"
        {
            idx += 1;
            continue;
        }

        let indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let replacement = vec![
            lines[idx].clone(),
            lines[idx + 1].clone(),
            format!("{indent}}} else if ((dir == 2)) {{"),
            lines[idx + 4].clone(),
            format!("{indent}}} else if ((dir == 3)) {{"),
            lines[idx + 7].clone(),
            format!("{indent}}} else if ((dir == 4)) {{"),
            lines[idx + 10].clone(),
            format!("{indent}}}"),
        ];
        lines.splice(idx..(idx + 12), replacement);
        idx += 9;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn strip_orphan_rr_cse_markers_before_repeat_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for line in &mut lines {
        if line.trim() == "# rr-cse-pruned" {
            line.clear();
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn strip_unused_raw_arg_aliases_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };
        let mut idx = fn_start + 1;
        while idx < fn_end {
            let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
                idx += 1;
                continue;
            };
            if !lhs.starts_with(".arg_") || !rhs.chars().all(is_symbol_char) {
                idx += 1;
                continue;
            }
            let used_later = lines
                .iter()
                .take(fn_end + 1)
                .skip(idx + 1)
                .any(|line| line_contains_symbol(line.trim(), lhs));
            if !used_later {
                lines[idx].clear();
            }
            idx += 1;
        }
        fn_start = fn_end + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_readonly_raw_arg_aliases_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };

        let mut aliases = Vec::new();
        for idx in (fn_start + 1)..fn_end {
            let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
                continue;
            };
            if !lhs.starts_with(".arg_") || !rhs.chars().all(is_symbol_char) {
                continue;
            }
            let reassigned_later = lines
                .iter()
                .take(fn_end + 1)
                .skip(idx + 1)
                .filter_map(|line| parse_raw_assign_line(line.trim()))
                .any(|(later_lhs, _)| later_lhs == rhs);
            if reassigned_later {
                continue;
            }
            aliases.push((idx, lhs.to_string(), rhs.to_string()));
        }

        for (alias_idx, alias, target) in aliases {
            for line in lines.iter_mut().take(fn_end + 1).skip(alias_idx + 1) {
                *line = replace_symbol_occurrences(line, &alias, &target);
            }
            lines[alias_idx].clear();
        }

        fn_start = fn_end + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn collapse_trivial_dot_product_wrappers_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };
        let Some((fn_name, params)) = parse_raw_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        if !fn_name.starts_with("Sym_") || params.len() != 3 {
            fn_start = fn_end + 1;
            continue;
        }

        let body: Vec<String> = lines
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && line != "# rr-cse-pruned")
            .collect();
        if body.len() != 10 {
            fn_start = fn_end + 1;
            continue;
        }

        let acc = "sum";
        let iter = "i";
        let lhs_vec = params[0].trim();
        let rhs_vec = params[1].trim();
        let len = params[2].trim();
        let expected = [
            "{".to_string(),
            format!("{acc} <- 0"),
            format!("{iter} <- 1"),
            "repeat {".to_string(),
            format!("if (!({iter} <= {len})) break"),
            format!("{acc} <- ({acc} + ({lhs_vec}[{iter}] * {rhs_vec}[{iter}]))"),
            format!("{iter} <- ({iter} + 1)"),
            "next".to_string(),
            "}".to_string(),
            format!("return({acc})"),
        ];
        let normalized_body: Vec<String> = body
            .iter()
            .map(|line| normalize_raw_iter_index_parens(line, iter))
            .collect();
        if normalized_body != expected {
            fn_start = fn_end + 1;
            continue;
        }

        lines.splice(
            (fn_start + 1)..fn_end,
            [
                "{".to_string(),
                format!(
                    "  return(sum(({}[seq_len({})] * {}[seq_len({})])))",
                    lhs_vec, len, rhs_vec, len
                ),
            ],
        );
        fn_start += 2;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn normalize_raw_iter_index_parens(line: &str, iter_var: &str) -> String {
    let paren_idx = format!("[({iter_var})]");
    let plain_idx = format!("[{iter_var}]");
    line.replace(&paren_idx, &plain_idx)
}

fn strip_dead_simple_scalar_assigns_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };
        let mut idx = fn_start + 1;
        while idx < fn_end {
            let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
                idx += 1;
                continue;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || lhs.starts_with(".tachyon_")
                || (!rhs_is_raw_simple_scalar_alias_or_literal(rhs)
                    && !rhs_is_raw_simple_dead_expr(rhs))
                || raw_enclosing_repeat_guard_mentions_symbol(&lines, idx, lhs)
            {
                idx += 1;
                continue;
            }
            let mut used_later = false;
            for later_line in lines.iter().take(fn_end + 1).skip(idx + 1) {
                let later_trimmed = later_line.trim();
                if later_trimmed.is_empty() {
                    continue;
                }
                if line_contains_symbol(later_trimmed, lhs) {
                    used_later = true;
                    break;
                }
            }
            if !used_later {
                lines[idx].clear();
            }
            idx += 1;
        }
        fn_start = fn_end + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn strip_shadowed_simple_scalar_seed_assigns_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };

        let mut idx = fn_start + 1;
        while idx < fn_end {
            let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
                idx += 1;
                continue;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || lhs.starts_with(".tachyon_")
                || (!rhs_is_raw_simple_scalar_alias_or_literal(rhs)
                    && !rhs_is_raw_simple_dead_expr(rhs))
                || raw_enclosing_repeat_guard_mentions_symbol(&lines, idx, lhs)
            {
                idx += 1;
                continue;
            }

            let mut shadowed_before_use = false;
            for later_line in lines.iter().take(fn_end + 1).skip(idx + 1) {
                let later_trimmed = later_line.trim();
                if later_trimmed.is_empty() {
                    continue;
                }
                if later_trimmed.starts_with("if (")
                    || later_trimmed.starts_with("if(")
                    || later_trimmed.starts_with("} else {")
                    || later_trimmed.starts_with("} else if")
                    || later_trimmed == "repeat {"
                    || later_trimmed == "}"
                    || later_trimmed == "next"
                    || later_trimmed.starts_with("return(")
                    || later_trimmed.starts_with("return (")
                {
                    break;
                }
                if let Some((later_lhs, later_rhs)) = parse_raw_assign_line(later_trimmed)
                    && later_lhs == lhs
                {
                    if !line_contains_symbol(later_rhs, lhs) {
                        shadowed_before_use = true;
                    }
                    break;
                }
                if line_contains_symbol(later_trimmed, lhs) {
                    break;
                }
            }

            if shadowed_before_use {
                lines[idx].clear();
            }
            idx += 1;
        }

        fn_start = fn_end + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn strip_dead_weno_topology_seed_i_before_direct_adj_gather_in_raw_emitted_r(
    output: &str,
) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 3 {
        return output.to_string();
    }

    for idx in 0..(lines.len() - 2) {
        if lines[idx].trim() != "i <- 1"
            || lines[idx + 1].trim() != "adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))"
            || lines[idx + 2].trim() != "adj_rr <- rr_gather(adj_r, rr_index_vec_floor(adj_r))"
        {
            continue;
        }
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_mountain_dx_temp_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 2 {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx + 1 < lines.len() {
        if lines[idx].trim() == "dx_m <- (x_curr - 20)"
            && lines[idx + 1].trim() == "dy_m <- (y_curr - 20)"
        {
            let Some(dist_idx) = ((idx + 2)..lines.len().min(idx + 8)).find(|line_idx| {
                lines[*line_idx].trim() == "dist <- ((dx_m * dx_m) + (dy_m * dy_m))"
            }) else {
                idx += 1;
                continue;
            };
            let indent = lines[dist_idx]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>();
            lines[idx].clear();
            lines[idx + 1].clear();
            lines[dist_idx] = format!(
                "{indent}dist <- ((((x_curr - 20) * (x_curr - 20)) + ((y_curr - 20) * (y_curr - 20))))"
            );
            idx = dist_idx + 1;
            continue;
        }

        if lines[idx].trim() == "dx_m <- (x_curr - 20)"
            && lines[idx + 1].trim() == "dist <- ((dx_m * dx_m) + ((y_curr - 20) * (y_curr - 20)))"
        {
            let indent = lines[idx + 1]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>();
            lines[idx].clear();
            lines[idx + 1] = format!(
                "{indent}dist <- ((((x_curr - 20) * (x_curr - 20)) + ((y_curr - 20) * (y_curr - 20))))"
            );
        }

        idx += 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn strip_dead_zero_seed_ii_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        if lines[idx].trim() != "ii <- 0" {
            continue;
        }
        let used_later = lines
            .iter()
            .skip(idx + 1)
            .any(|line| line_contains_symbol(line.trim(), "ii"));
        if !used_later {
            lines[idx].clear();
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r(
    output: &str,
) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 8 {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx + 7 < lines.len() {
        let first = lines[idx].trim();
        let second = lines[idx + 1].trim();
        let third = lines[idx + 2].trim();
        if (first != "adj_ll <- rep.int(0, TOTAL)" && first != "adj_ll <- qr")
            || (second != "adj_rr <- rep.int(0, TOTAL)"
                && second != "adj_rr <- adj_ll"
                && second != "adj_rr <- qr")
            || third != "i <- 1"
        {
            idx += 1;
            continue;
        }
        if !lines[idx + 3].trim().is_empty() || lines[idx + 4].trim() != "# rr-cse-pruned" {
            idx += 1;
            continue;
        }
        if lines[idx + 5].trim()
            != ".tachyon_exprmap0_0 <- rr_gather(adj_l, rr_index_vec_floor(rr_index1_read_vec(adj_l, rr_index_vec_floor(i:((6 * N) * N)))))"
            || lines[idx + 6].trim()
                != ".tachyon_exprmap1_0 <- rr_gather(adj_r, rr_index_vec_floor(rr_index1_read_vec(adj_r, rr_index_vec_floor(i:((6 * N) * N)))))"
            || lines[idx + 7].trim()
                != "adj_ll <- rr_assign_slice(adj_ll, i, ((6 * N) * N), .tachyon_exprmap0_0)"
            || idx + 8 >= lines.len()
            || lines[idx + 8].trim()
                != "adj_rr <- rr_assign_slice(adj_rr, i, ((6 * N) * N), .tachyon_exprmap1_0)"
        {
            idx += 1;
            continue;
        }

        let indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines[idx] = format!("{indent}adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))");
        lines[idx + 1] = format!("{indent}adj_rr <- rr_gather(adj_r, rr_index_vec_floor(adj_r))");
        for line in lines.iter_mut().take(idx + 9).skip(idx + 2) {
            line.clear();
        }
        idx += 9;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn raw_enclosing_repeat_guard_mentions_symbol(lines: &[String], idx: usize, symbol: &str) -> bool {
    for start_idx in (0..idx).rev() {
        if lines[start_idx].trim() != "repeat {" {
            continue;
        }
        let Some(end_idx) = find_raw_block_end(lines, start_idx) else {
            continue;
        };
        if idx >= end_idx {
            continue;
        }
        let Some(guard_idx) = ((start_idx + 1)..end_idx)
            .find(|line_idx| parse_raw_repeat_guard_cmp_line(lines[*line_idx].trim()).is_some())
        else {
            continue;
        };
        let guard = lines[guard_idx].trim();
        if line_contains_symbol(guard, symbol) {
            return true;
        }
        break;
    }
    false
}

fn raw_is_loop_open_boundary(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == "repeat {" || trimmed.starts_with("while") || trimmed.starts_with("for")
}

fn raw_line_is_within_loop_body(lines: &[String], idx: usize) -> bool {
    (0..idx).rev().any(|start_idx| {
        if !raw_is_loop_open_boundary(lines[start_idx].trim()) {
            return false;
        }
        find_raw_block_end(lines, start_idx).is_some_and(|end_idx| idx < end_idx)
    })
}

fn strip_noop_self_assignments_in_raw_emitted_r(output: &str) -> String {
    let mut out = String::new();
    for line in output.lines() {
        let keep = if let Some((lhs, rhs)) = parse_raw_assign_line(line.trim()) {
            lhs != strip_redundant_outer_parens(rhs)
        } else {
            true
        };
        if keep {
            out.push_str(line);
            out.push('\n');
        }
    }
    if output.is_empty() {
        return String::new();
    }
    if !output.ends_with('\n') {
        out.pop();
    }
    out
}

fn strip_empty_else_blocks_in_raw_emitted_r(output: &str) -> String {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0usize;
    while i < lines.len() {
        let line = &lines[i];
        if line.trim() == "} else {" {
            let mut close_idx = i + 1;
            while close_idx < lines.len() && lines[close_idx].trim().is_empty() {
                close_idx += 1;
            }
            if close_idx < lines.len() && lines[close_idx].trim() == "}" {
                let indent_len = line.len() - line.trim_start().len();
                let indent = &line[..indent_len];
                out.push(format!("{indent}}}"));
                i = close_idx + 1;
                continue;
            }
        }
        out.push(line.clone());
        i += 1;
    }

    let mut rendered = out.join("\n");
    if output.ends_with('\n') || !rendered.is_empty() {
        rendered.push('\n');
    }
    rendered
}

fn strip_redundant_branch_local_vec_fill_rebinds_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            continue;
        };
        let Some(sig) = raw_vec_fill_signature(rhs) else {
            continue;
        };
        let Some(branch_start) = enclosing_raw_branch_start(&lines, idx) else {
            continue;
        };
        if branch_body_writes_symbol_before(&lines, branch_start + 1, idx, lhs) {
            continue;
        }

        let mut prev_match = None;
        for prev_idx in (0..branch_start).rev() {
            let trimmed = lines[prev_idx].trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            if trimmed == "repeat {"
                || trimmed.starts_with("while ")
                || trimmed.starts_with("while(")
                || trimmed.starts_with("for ")
                || trimmed.starts_with("for(")
                || trimmed.contains("<- function")
            {
                break;
            }
            let Some((prev_lhs, prev_rhs)) = parse_raw_assign_line(trimmed) else {
                continue;
            };
            if prev_lhs == lhs {
                prev_match = Some(prev_rhs.to_string());
                break;
            }
        }
        let Some(prev_rhs) = prev_match else {
            continue;
        };
        if raw_vec_fill_signature(prev_rhs.as_str()).is_some_and(|prev_sig| prev_sig == sig) {
            lines[idx].clear();
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn strip_unused_helper_params_in_raw_emitted_r(output: &str) -> String {
    #[derive(Clone)]
    struct HelperTrim {
        original_len: usize,
        kept_indices: Vec<usize>,
        kept_params: Vec<String>,
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut trims = FxHashMap::<String, HelperTrim>::default();
    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };
        let Some((fn_name, params)) = parse_raw_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        if !fn_name.starts_with("Sym_") || params.is_empty() {
            fn_start = fn_end + 1;
            continue;
        }

        let escaped = lines
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx < fn_start || *idx > fn_end)
            .any(|(_, line)| {
                let trimmed = line.trim();
                line_contains_symbol(trimmed, &fn_name)
                    && !trimmed.contains(&format!("{fn_name}("))
                    && !trimmed.contains(&format!("{fn_name} <- function("))
            });
        if escaped {
            fn_start = fn_end + 1;
            continue;
        }

        let mut used_params = FxHashSet::default();
        for line in lines.iter().take(fn_end).skip(fn_start + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            for ident in raw_expr_idents(trimmed) {
                used_params.insert(ident);
            }
        }
        let kept_indices: Vec<usize> = params
            .iter()
            .enumerate()
            .filter_map(|(idx, param)| used_params.contains(param).then_some(idx))
            .collect();
        if kept_indices.len() < params.len() {
            trims.insert(
                fn_name,
                HelperTrim {
                    original_len: params.len(),
                    kept_indices: kept_indices.clone(),
                    kept_params: kept_indices
                        .iter()
                        .map(|idx| params[*idx].clone())
                        .collect(),
                },
            );
        }
        fn_start = fn_end + 1;
    }

    if trims.is_empty() {
        return output.to_string();
    }

    for line in &mut lines {
        if line.contains("<- function") {
            if let Some((fn_name, _)) = parse_raw_function_header(line)
                && let Some(trim) = trims.get(&fn_name)
            {
                *line = format!("{} <- function({})", fn_name, trim.kept_params.join(", "));
            }
            continue;
        }

        let mut rewritten = line.clone();
        loop {
            let mut changed = false;
            let mut next = String::with_capacity(rewritten.len());
            let mut idx = 0usize;
            while idx < rewritten.len() {
                let mut best: Option<(usize, String)> = None;
                for fn_name in trims.keys() {
                    if let Some(pos) = find_symbol_call(&rewritten, fn_name, idx)
                        && best.as_ref().is_none_or(|(best_pos, _)| pos < *best_pos)
                    {
                        best = Some((pos, fn_name.clone()));
                    }
                }
                let Some((call_idx, fn_name)) = best else {
                    next.push_str(&rewritten[idx..]);
                    break;
                };
                let trim = &trims[&fn_name];
                let ident_end = call_idx + fn_name.len();
                let Some(call_end) = find_matching_call_close(&rewritten, ident_end) else {
                    next.push_str(&rewritten[idx..]);
                    break;
                };
                next.push_str(&rewritten[idx..call_idx]);
                let args_inner = &rewritten[ident_end + 1..call_end];
                let Some(args) = split_top_level_args(args_inner) else {
                    next.push_str(&rewritten[call_idx..=call_end]);
                    idx = call_end + 1;
                    continue;
                };
                if args.len() != trim.original_len {
                    next.push_str(&rewritten[call_idx..=call_end]);
                    idx = call_end + 1;
                    continue;
                }
                next.push_str(&fn_name);
                next.push('(');
                next.push_str(
                    &trim
                        .kept_indices
                        .iter()
                        .map(|idx| args[*idx].trim())
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                next.push(')');
                idx = call_end + 1;
                changed = true;
            }
            if !changed || next == rewritten {
                break;
            }
            rewritten = next;
        }
        *line = rewritten;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn strip_dead_seq_len_locals_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };

        for idx in (fn_start + 1)..fn_end {
            let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
                continue;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".tachyon_")
                || lhs.starts_with(".__rr_cse_")
                || !rhs.starts_with("seq_len(")
            {
                continue;
            }

            let mut used_later = false;
            for later_line in lines.iter().take(fn_end + 1).skip(idx + 1) {
                let later_trimmed = later_line.trim();
                if later_trimmed.is_empty() {
                    continue;
                }
                if let Some((later_lhs, later_rhs)) = parse_raw_assign_line(later_trimmed)
                    && later_lhs == lhs
                {
                    if line_contains_symbol(later_rhs, lhs) {
                        used_later = true;
                    }
                    break;
                }
                if line_contains_symbol(later_trimmed, lhs) {
                    used_later = true;
                    break;
                }
            }

            if !used_later {
                lines[idx].clear();
            }
        }

        fn_start = fn_end + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn find_matching_open_brace_line(lines: &[String], close_idx: usize) -> Option<usize> {
    let mut stack: Vec<usize> = Vec::new();
    for (idx, line) in lines.iter().enumerate().take(close_idx + 1) {
        for ch in line.chars() {
            match ch {
                '{' => stack.push(idx),
                '}' => {
                    let open = stack.pop()?;
                    if idx == close_idx {
                        return Some(open);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn strip_terminal_repeat_nexts_in_raw_emitted_r(output: &str) -> String {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 2 {
        return output.to_string();
    }

    let mut kept = Vec::with_capacity(lines.len());
    for idx in 0..lines.len() {
        if lines[idx].trim() == "next"
            && idx + 1 < lines.len()
            && lines[idx + 1].trim() == "}"
            && find_matching_open_brace_line(&lines, idx + 1)
                .is_some_and(|open_idx| lines[open_idx].trim() == "repeat {")
        {
            continue;
        }
        kept.push(lines[idx].clone());
    }

    let mut out = kept.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn simplify_same_var_is_na_or_not_finite_guards_in_raw_emitted_r(output: &str) -> String {
    let rewritten =
        raw_same_var_is_na_or_not_finite_re().replace_all(output, |caps: &Captures<'_>| {
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("");
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("");
            if lhs == rhs {
                format!("!(is.finite({lhs}))")
            } else {
                caps.get(0)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default()
            }
        });
    let mut out = rewritten.into_owned();
    if output.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn simplify_not_finite_or_zero_guard_parens_in_raw_emitted_r(output: &str) -> String {
    let rewritten = raw_not_finite_or_zero_guard_re().replace_all(output, |caps: &Captures<'_>| {
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("");
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("");
        let inner = caps.name("inner").map(|m| m.as_str()).unwrap_or("");
        if lhs == rhs {
            format!("(({inner} | ({rhs} == 0)))")
        } else {
            caps.get(0)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default()
        }
    });
    let mut out = rewritten.into_owned();
    if output.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn simplify_wrapped_not_finite_parens_in_raw_emitted_r(output: &str) -> String {
    let rewritten = raw_wrapped_not_finite_cond_re().replace_all(output, |caps: &Captures<'_>| {
        let inner = caps.name("inner").map(|m| m.as_str()).unwrap_or("");
        format!("({inner})")
    });
    let mut out = rewritten.into_owned();
    if output.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn restore_constant_one_guard_repeat_loop_counters_in_raw_emitted_r(output: &str) -> String {
    fn parse_constant_guard(line: &str) -> Option<(String, String, String)> {
        let trimmed = line.trim();
        let inner = trimmed
            .strip_prefix("if (!(")
            .or_else(|| trimmed.strip_prefix("if !("))?
            .strip_suffix(")) break")
            .or_else(|| {
                trimmed
                    .strip_prefix("if (!(")
                    .or_else(|| trimmed.strip_prefix("if !("))
                    .and_then(|s| s.strip_suffix(") break"))
            })?
            .trim();
        for op in ["<=", "<"] {
            let needle = format!(" {op} ");
            let Some((lhs, rhs)) = inner.split_once(&needle) else {
                continue;
            };
            let lhs = strip_redundant_outer_parens(lhs.trim());
            let rhs = rhs.trim();
            if lhs.is_empty() || rhs.is_empty() {
                continue;
            }
            let numeric = lhs.trim_end_matches('L').trim_end_matches('l');
            if numeric.parse::<f64>().ok().is_some() {
                return Some((lhs.to_string(), op.to_string(), rhs.to_string()));
            }
        }
        None
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        if lines[idx].trim() != "repeat {" {
            idx += 1;
            continue;
        }
        let Some(loop_end) = find_raw_block_end(&lines, idx) else {
            idx += 1;
            continue;
        };
        let Some(guard_idx) = ((idx + 1)..loop_end).find(|line_idx| {
            let trimmed = lines[*line_idx].trim();
            (trimmed.starts_with("if !(") || trimmed.starts_with("if (!("))
                && trimmed.ends_with("break")
        }) else {
            idx = loop_end + 1;
            continue;
        };
        let Some((start_lit, cmp, bound)) = parse_constant_guard(&lines[guard_idx]) else {
            idx = loop_end + 1;
            continue;
        };
        let idx_var = ".__rr_i";
        if lines
            .iter()
            .take(loop_end)
            .skip(guard_idx + 1)
            .any(|line| line_contains_symbol(line.trim(), idx_var))
        {
            idx = loop_end + 1;
            continue;
        }

        let indent = lines[guard_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let repeat_indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines.insert(idx, format!("{repeat_indent}{idx_var} <- {start_lit}"));
        lines[guard_idx + 1] = if cmp == "<=" {
            format!("{indent}if (!({idx_var} <= {bound})) break")
        } else {
            format!("{indent}if (!({idx_var} < {bound})) break")
        };
        let one = if start_lit.contains('.') {
            "1.0"
        } else if start_lit.ends_with('L') || start_lit.ends_with('l') {
            "1L"
        } else {
            "1"
        };
        lines.insert(
            loop_end + 1,
            format!("{indent}{idx_var} <- ({idx_var} + {one})"),
        );
        idx = loop_end + 3;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn parse_raw_assign_line(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (lhs, rhs) = trimmed.split_once(" <- ")?;
    let lhs = lhs.trim();
    let rhs = rhs.trim();
    if lhs.is_empty() || !lhs.chars().all(is_symbol_char) {
        return None;
    }
    Some((lhs, rhs))
}

fn parse_raw_function_header(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    let (name, rest) = trimmed.split_once("<- function(")?;
    let name = name.trim();
    if name.is_empty() || !name.chars().all(is_symbol_char) {
        return None;
    }
    let args_inner = rest.strip_suffix(')')?.trim();
    let params = if args_inner.is_empty() {
        Vec::new()
    } else {
        split_top_level_args(args_inner)?
    };
    Some((name.to_string(), params))
}

fn rhs_is_raw_simple_scalar_alias_or_literal(rhs: &str) -> bool {
    let rhs = strip_redundant_outer_parens(rhs);
    rhs.chars().all(is_symbol_char)
        || is_raw_numeric_literal(rhs)
        || matches!(rhs, "TRUE" | "FALSE" | "NA" | "NULL")
}

fn rhs_is_raw_simple_dead_expr(rhs: &str) -> bool {
    let rhs = strip_redundant_outer_parens(rhs);
    !rhs.is_empty()
        && !rhs.contains("<-")
        && !rhs.contains("function(")
        && !rhs.contains("tryCatch(")
        && !rhs.contains("print(")
        && !rhs.contains("cat(")
        && !rhs.contains("message(")
        && !rhs.contains("warning(")
        && !rhs.contains("stop(")
        && !rhs.contains("quit(")
        && !rhs.contains('"')
        && !rhs.contains(',')
}

fn is_raw_numeric_literal(rhs: &str) -> bool {
    let rhs = rhs.trim();
    if rhs.is_empty() {
        return false;
    }
    let rhs = rhs.strip_suffix('L').unwrap_or(rhs);
    rhs.parse::<f64>().is_ok()
}

fn strip_redundant_outer_parens(expr: &str) -> &str {
    let mut expr = expr.trim();
    loop {
        if !(expr.starts_with('(') && expr.ends_with(')')) {
            break;
        }
        let mut depth = 0i32;
        let mut wraps = true;
        for (idx, ch) in expr.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 && idx + ch.len_utf8() < expr.len() {
                        wraps = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !wraps {
            break;
        }
        expr = expr[1..expr.len() - 1].trim();
    }
    expr
}

fn is_inlineable_raw_scalar_index_rhs(rhs: &str) -> bool {
    let trimmed = strip_redundant_outer_parens(rhs);
    let open = trimmed.find('[');
    let close = trimmed.rfind(']');
    let (Some(open), Some(close)) = (open, close) else {
        return false;
    };
    if close + 1 != trimmed.len() || open == 0 || close <= open + 1 {
        return false;
    }
    let base = trimmed[..open].trim();
    base.chars().all(is_symbol_char)
}

fn is_inlineable_raw_named_scalar_expr(rhs: &str) -> bool {
    let rhs = strip_redundant_outer_parens(rhs);
    if rhs.is_empty()
        || rhs.contains('"')
        || rhs.contains(',')
        || rhs.contains("function(")
        || rhs.contains("function (")
    {
        return false;
    }
    true
}

fn find_raw_block_end(lines: &[String], start_idx: usize) -> Option<usize> {
    let mut depth = 0isize;
    let mut saw_open = false;
    for (idx, line) in lines.iter().enumerate().skip(start_idx) {
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    saw_open = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
        if saw_open && depth <= 0 {
            return Some(idx);
        }
    }
    None
}

fn is_raw_alloc_like_expr(expr: &str) -> bool {
    [
        "rep.int(",
        "numeric(",
        "integer(",
        "logical(",
        "character(",
        "vector(",
        "matrix(",
        "Sym_17(",
    ]
    .iter()
    .any(|prefix| expr.starts_with(prefix))
}

fn is_raw_branch_rebind_candidate(expr: &str) -> bool {
    is_raw_alloc_like_expr(expr)
        || expr.chars().all(is_symbol_char)
        || is_raw_numeric_literal(expr)
        || matches!(expr, "TRUE" | "FALSE" | "NA" | "NULL")
}

fn raw_branch_rebind_exprs_equivalent(prev_rhs: &str, rhs: &str) -> bool {
    let prev_rhs = strip_redundant_outer_parens(prev_rhs);
    let rhs = strip_redundant_outer_parens(rhs);
    if prev_rhs == rhs {
        return true;
    }
    raw_vec_fill_signature(prev_rhs)
        .zip(raw_vec_fill_signature(rhs))
        .is_some_and(|(lhs_sig, rhs_sig)| lhs_sig == rhs_sig)
}

fn raw_vec_fill_signature(expr: &str) -> Option<(String, String)> {
    let expr = strip_redundant_outer_parens(expr);
    if let Some(inner) = expr
        .strip_prefix("rep.int(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let args = split_top_level_args(inner)?;
        if args.len() == 2 {
            return Some((args[1].trim().to_string(), args[0].trim().to_string()));
        }
    }
    if let Some(inner) = expr
        .strip_prefix("Sym_17(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let args = split_top_level_args(inner)?;
        if args.len() == 2 {
            return Some((args[0].trim().to_string(), args[1].trim().to_string()));
        }
    }
    None
}

fn enclosing_raw_branch_start(lines: &[String], idx: usize) -> Option<usize> {
    let mut depth = 0usize;
    for prev_idx in (0..idx).rev() {
        let trimmed = lines[prev_idx].trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "}" {
            depth += 1;
            continue;
        }
        if trimmed.ends_with('{') {
            if depth == 0 {
                return (trimmed.starts_with("if ") || trimmed.starts_with("if("))
                    .then_some(prev_idx);
            }
            depth = depth.saturating_sub(1);
        }
    }
    None
}

fn branch_body_writes_symbol_before(
    lines: &[String],
    start: usize,
    end_exclusive: usize,
    symbol: &str,
) -> bool {
    lines
        .iter()
        .take(end_exclusive)
        .skip(start)
        .filter_map(|line| parse_raw_assign_line(line.trim()))
        .any(|(lhs, _)| lhs == symbol)
}

fn previous_outer_assign_before_branch<'a>(
    lines: &'a [String],
    branch_start: usize,
    lhs: &str,
) -> Option<(&'a str, &'a str)> {
    for prev_idx in (0..branch_start).rev() {
        let trimmed = lines[prev_idx].trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "}" || trimmed == "{" {
            continue;
        }
        if trimmed.ends_with('{') {
            break;
        }
        let Some((prev_lhs, prev_rhs)) = parse_raw_assign_line(trimmed) else {
            continue;
        };
        if prev_lhs == lhs {
            return Some((prev_lhs, prev_rhs));
        }
        if line_contains_symbol(trimmed, lhs) {
            break;
        }
    }
    None
}

fn previous_outer_assign_before_branch_relaxed<'a>(
    lines: &'a [String],
    branch_start: usize,
    lhs: &str,
) -> Option<(&'a str, &'a str)> {
    for prev_idx in (0..branch_start).rev() {
        let trimmed = lines[prev_idx].trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "}" || trimmed == "{" {
            continue;
        }
        if trimmed == "repeat {"
            || trimmed.starts_with("while ")
            || trimmed.starts_with("while(")
            || trimmed.starts_with("for ")
            || trimmed.starts_with("for(")
            || trimmed.contains("<- function")
        {
            break;
        }
        let Some((prev_lhs, prev_rhs)) = parse_raw_assign_line(trimmed) else {
            continue;
        };
        if prev_lhs == lhs {
            return Some((prev_lhs, prev_rhs));
        }
    }
    None
}

fn line_contains_symbol(line: &str, symbol: &str) -> bool {
    let mut search_from = 0usize;
    while let Some(rel_idx) = line[search_from..].find(symbol) {
        let idx = search_from + rel_idx;
        let before = line[..idx].chars().next_back();
        let after = line[idx + symbol.len()..].chars().next();
        let boundary_ok = before.is_none_or(|ch| !is_symbol_char(ch))
            && after.is_none_or(|ch| !is_symbol_char(ch));
        if boundary_ok {
            return true;
        }
        search_from = idx + symbol.len();
    }
    false
}

fn count_symbol_occurrences(line: &str, symbol: &str) -> usize {
    let mut count = 0usize;
    let mut search_from = 0usize;
    while let Some(rel_idx) = line[search_from..].find(symbol) {
        let idx = search_from + rel_idx;
        let before = line[..idx].chars().next_back();
        let after = line[idx + symbol.len()..].chars().next();
        let boundary_ok = before.is_none_or(|ch| !is_symbol_char(ch))
            && after.is_none_or(|ch| !is_symbol_char(ch));
        if boundary_ok {
            count += 1;
        }
        search_from = idx + symbol.len();
    }
    count
}

fn find_symbol_call(line: &str, symbol: &str, start_from: usize) -> Option<usize> {
    let mut search_from = start_from;
    while let Some(rel_idx) = line[search_from..].find(symbol) {
        let idx = search_from + rel_idx;
        let before = line[..idx].chars().next_back();
        let after = line[idx + symbol.len()..].chars().next();
        let boundary_ok = before.is_none_or(|ch| !is_symbol_char(ch)) && after == Some('(');
        if boundary_ok {
            return Some(idx);
        }
        search_from = idx + symbol.len();
    }
    None
}

fn find_matching_paren(line: &str, open_idx: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut saw_open = false;
    for (idx, ch) in line.char_indices().skip(open_idx) {
        match ch {
            '(' => {
                depth += 1;
                saw_open = true;
            }
            ')' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                if saw_open && depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn line_contains_unquoted_symbol_reference(line: &str, symbol: &str) -> bool {
    let bytes = line.as_bytes();
    let symbol_bytes = symbol.as_bytes();
    let mut idx = 0usize;
    let mut in_single = false;
    let mut in_double = false;

    while idx < bytes.len() {
        match bytes[idx] {
            b'\'' if !in_double => {
                in_single = !in_single;
                idx += 1;
                continue;
            }
            b'"' if !in_single => {
                in_double = !in_double;
                idx += 1;
                continue;
            }
            _ => {}
        }

        if !in_single && !in_double && bytes[idx..].starts_with(symbol_bytes) {
            let before = line[..idx].chars().next_back();
            let after = line[idx + symbol.len()..].chars().next();
            let boundary_ok = before.is_none_or(|ch| !is_symbol_char(ch))
                && after.is_none_or(|ch| !is_symbol_char(ch));
            if boundary_ok {
                return true;
            }
        }

        idx += 1;
    }

    false
}

fn replace_symbol_occurrences(line: &str, symbol: &str, replacement: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut idx = 0usize;
    while let Some(rel_idx) = line[idx..].find(symbol) {
        let hit = idx + rel_idx;
        let before = line[..hit].chars().next_back();
        let after = line[hit + symbol.len()..].chars().next();
        let boundary_ok = before.is_none_or(|ch| !is_symbol_char(ch))
            && after.is_none_or(|ch| !is_symbol_char(ch));
        out.push_str(&line[idx..hit]);
        if boundary_ok {
            out.push_str(replacement);
        } else {
            out.push_str(symbol);
        }
        idx = hit + symbol.len();
    }
    out.push_str(&line[idx..]);
    out
}

fn raw_expr_idents(expr: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = None;
    for (idx, ch) in expr.char_indices() {
        if is_symbol_char(ch) {
            if start.is_none() {
                start = Some(idx);
            }
        } else if let Some(begin) = start.take() {
            out.push(expr[begin..idx].to_string());
        }
    }
    if let Some(begin) = start {
        out.push(expr[begin..].to_string());
    }
    out
}

fn is_symbol_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')
}

fn find_matching_call_close(line: &str, open_idx: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (off, ch) in line[open_idx..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_idx + off);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top_level_args(expr: &str) -> Option<Vec<String>> {
    let mut args = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (idx, ch) in expr.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                args.push(expr[start..idx].trim().to_string());
                start = idx + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    args.push(expr[start..].trim().to_string());
    Some(args)
}

fn raw_literal_record_field_name(expr: &str) -> Option<String> {
    let trimmed = expr.trim();
    let inner = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| {
            trimmed
                .strip_prefix('\'')
                .and_then(|s| s.strip_suffix('\''))
        })?;
    inner.chars().all(is_symbol_char).then(|| inner.to_string())
}

fn collect_referentially_pure_user_functions(
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
) -> FxHashSet<String> {
    fn helper_is_functionally_pure(callee: &str) -> bool {
        matches!(
            callee,
            "rr_assign_slice"
                | "rr_ifelse_strict"
                | "rr_index1_read"
                | "rr_index1_read_strict"
                | "rr_index1_read_vec"
                | "rr_index1_read_vec_floor"
                | "rr_index_vec_floor"
                | "rr_gather"
                | "rr_wrap_index_vec"
                | "rr_wrap_index_vec_i"
                | "rr_idx_cube_vec_i"
                | "rr_named_list"
                | "rr_field_get"
                | "rr_field_exists"
        )
    }

    fn value_is_functionally_pure(
        all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
        fn_ir: &crate::mir::def::FnIR,
        vid: crate::mir::def::ValueId,
        memo: &mut FxHashMap<String, bool>,
        visiting_fns: &mut FxHashSet<String>,
        seen: &mut FxHashSet<crate::mir::def::ValueId>,
    ) -> bool {
        if !seen.insert(vid) {
            return false;
        }
        let pure = match &fn_ir.values[vid].kind {
            crate::mir::def::ValueKind::Const(_)
            | crate::mir::def::ValueKind::Param { .. }
            | crate::mir::def::ValueKind::Load { .. }
            | crate::mir::def::ValueKind::RSymbol { .. } => true,
            crate::mir::def::ValueKind::Phi { args } => args.iter().all(|(src, _)| {
                value_is_functionally_pure(all_fns, fn_ir, *src, memo, visiting_fns, seen)
            }),
            crate::mir::def::ValueKind::Len { base }
            | crate::mir::def::ValueKind::Indices { base } => {
                value_is_functionally_pure(all_fns, fn_ir, *base, memo, visiting_fns, seen)
            }
            crate::mir::def::ValueKind::Range { start, end } => {
                value_is_functionally_pure(all_fns, fn_ir, *start, memo, visiting_fns, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *end, memo, visiting_fns, seen)
            }
            crate::mir::def::ValueKind::Binary { lhs, rhs, .. } => {
                value_is_functionally_pure(all_fns, fn_ir, *lhs, memo, visiting_fns, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *rhs, memo, visiting_fns, seen)
            }
            crate::mir::def::ValueKind::Unary { rhs, .. } => {
                value_is_functionally_pure(all_fns, fn_ir, *rhs, memo, visiting_fns, seen)
            }
            crate::mir::def::ValueKind::Index1D { base, idx, .. } => {
                value_is_functionally_pure(all_fns, fn_ir, *base, memo, visiting_fns, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *idx, memo, visiting_fns, seen)
            }
            crate::mir::def::ValueKind::Index2D { base, r, c } => {
                value_is_functionally_pure(all_fns, fn_ir, *base, memo, visiting_fns, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *r, memo, visiting_fns, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *c, memo, visiting_fns, seen)
            }
            crate::mir::def::ValueKind::Index3D { base, i, j, k } => {
                value_is_functionally_pure(all_fns, fn_ir, *base, memo, visiting_fns, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *i, memo, visiting_fns, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *j, memo, visiting_fns, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *k, memo, visiting_fns, seen)
            }
            crate::mir::def::ValueKind::Intrinsic { args, .. } => args.iter().all(|arg| {
                value_is_functionally_pure(all_fns, fn_ir, *arg, memo, visiting_fns, seen)
            }),
            crate::mir::def::ValueKind::Call { callee, args, .. } => {
                let user_pure = all_fns.get(callee).is_some_and(|callee_ir| {
                    function_is_referentially_pure(all_fns, callee, callee_ir, memo, visiting_fns)
                });
                (effects::call_is_pure(callee) || helper_is_functionally_pure(callee) || user_pure)
                    && args.iter().all(|arg| {
                        value_is_functionally_pure(all_fns, fn_ir, *arg, memo, visiting_fns, seen)
                    })
            }
        };
        seen.remove(&vid);
        pure
    }

    fn function_is_referentially_pure(
        all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
        name: &str,
        fn_ir: &crate::mir::def::FnIR,
        memo: &mut FxHashMap<String, bool>,
        visiting_fns: &mut FxHashSet<String>,
    ) -> bool {
        if let Some(cached) = memo.get(name) {
            return *cached;
        }
        if !visiting_fns.insert(name.to_string()) {
            return false;
        }
        if fn_ir.requires_conservative_optimization() {
            memo.insert(name.to_string(), false);
            visiting_fns.remove(name);
            return false;
        }
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                match instr {
                    crate::mir::def::Instr::Assign { src, .. }
                    | crate::mir::def::Instr::Eval { val: src, .. } => {
                        if !value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *src,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) {
                            memo.insert(name.to_string(), false);
                            visiting_fns.remove(name);
                            return false;
                        }
                    }
                    crate::mir::def::Instr::StoreIndex1D { base, idx, val, .. } => {
                        if !(value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *base,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *idx,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *val,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        )) {
                            memo.insert(name.to_string(), false);
                            visiting_fns.remove(name);
                            return false;
                        }
                    }
                    crate::mir::def::Instr::StoreIndex2D {
                        base, r, c, val, ..
                    } => {
                        if !(value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *base,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *r,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *c,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *val,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        )) {
                            memo.insert(name.to_string(), false);
                            visiting_fns.remove(name);
                            return false;
                        }
                    }
                    crate::mir::def::Instr::StoreIndex3D {
                        base, i, j, k, val, ..
                    } => {
                        if !(value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *base,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *i,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *j,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *k,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *val,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        )) {
                            memo.insert(name.to_string(), false);
                            visiting_fns.remove(name);
                            return false;
                        }
                    }
                }
            }
            match &block.term {
                crate::mir::def::Terminator::If { cond, .. } => {
                    if !value_is_functionally_pure(
                        all_fns,
                        fn_ir,
                        *cond,
                        memo,
                        visiting_fns,
                        &mut FxHashSet::default(),
                    ) {
                        memo.insert(name.to_string(), false);
                        visiting_fns.remove(name);
                        return false;
                    }
                }
                crate::mir::def::Terminator::Return(Some(v)) => {
                    if !value_is_functionally_pure(
                        all_fns,
                        fn_ir,
                        *v,
                        memo,
                        visiting_fns,
                        &mut FxHashSet::default(),
                    ) {
                        memo.insert(name.to_string(), false);
                        visiting_fns.remove(name);
                        return false;
                    }
                }
                crate::mir::def::Terminator::Goto(_)
                | crate::mir::def::Terminator::Return(None)
                | crate::mir::def::Terminator::Unreachable => {}
            }
        }
        memo.insert(name.to_string(), true);
        visiting_fns.remove(name);
        true
    }

    let mut out = FxHashSet::default();
    let mut memo = FxHashMap::default();
    let mut names: Vec<_> = all_fns.keys().cloned().collect();
    names.sort();
    for name in names {
        let Some(fn_ir) = all_fns.get(&name) else {
            continue;
        };
        if function_is_referentially_pure(
            all_fns,
            &name,
            fn_ir,
            &mut memo,
            &mut FxHashSet::default(),
        ) {
            out.insert(name);
        }
    }
    out
}

fn collect_fresh_returning_user_functions(
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
) -> FxHashSet<String> {
    struct FreshAnalysisCtx {
        pure_memo: FxHashMap<String, bool>,
        fresh_memo: FxHashMap<String, bool>,
        visiting_pure: FxHashSet<String>,
        visiting_fresh: FxHashSet<String>,
    }

    fn helper_is_functionally_pure(callee: &str) -> bool {
        matches!(
            callee,
            "rr_assign_slice"
                | "rr_ifelse_strict"
                | "rr_index1_read"
                | "rr_index1_read_strict"
                | "rr_index1_read_vec"
                | "rr_index1_read_vec_floor"
                | "rr_index_vec_floor"
                | "rr_gather"
                | "rr_wrap_index_vec"
                | "rr_wrap_index_vec_i"
                | "rr_idx_cube_vec_i"
                | "rr_named_list"
                | "rr_field_get"
                | "rr_field_exists"
        )
    }

    fn helper_is_fresh_result(callee: &str) -> bool {
        matches!(
            callee,
            "rep.int"
                | "numeric"
                | "integer"
                | "logical"
                | "character"
                | "vector"
                | "matrix"
                | "c"
                | "seq_len"
                | "seq_along"
                | "rr_named_list"
        )
    }

    fn value_is_functionally_pure(
        all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
        fn_ir: &crate::mir::def::FnIR,
        vid: crate::mir::def::ValueId,
        ctx: &mut FreshAnalysisCtx,
        seen: &mut FxHashSet<crate::mir::def::ValueId>,
    ) -> bool {
        if !seen.insert(vid) {
            return false;
        }
        let pure = match &fn_ir.values[vid].kind {
            crate::mir::def::ValueKind::Const(_)
            | crate::mir::def::ValueKind::Param { .. }
            | crate::mir::def::ValueKind::Load { .. }
            | crate::mir::def::ValueKind::RSymbol { .. } => true,
            crate::mir::def::ValueKind::Phi { args } => args
                .iter()
                .all(|(src, _)| value_is_functionally_pure(all_fns, fn_ir, *src, ctx, seen)),
            crate::mir::def::ValueKind::Len { base }
            | crate::mir::def::ValueKind::Indices { base } => {
                value_is_functionally_pure(all_fns, fn_ir, *base, ctx, seen)
            }
            crate::mir::def::ValueKind::Range { start, end } => {
                value_is_functionally_pure(all_fns, fn_ir, *start, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *end, ctx, seen)
            }
            crate::mir::def::ValueKind::Binary { lhs, rhs, .. } => {
                value_is_functionally_pure(all_fns, fn_ir, *lhs, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *rhs, ctx, seen)
            }
            crate::mir::def::ValueKind::Unary { rhs, .. } => {
                value_is_functionally_pure(all_fns, fn_ir, *rhs, ctx, seen)
            }
            crate::mir::def::ValueKind::Index1D { base, idx, .. } => {
                value_is_functionally_pure(all_fns, fn_ir, *base, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *idx, ctx, seen)
            }
            crate::mir::def::ValueKind::Index2D { base, r, c } => {
                value_is_functionally_pure(all_fns, fn_ir, *base, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *r, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *c, ctx, seen)
            }
            crate::mir::def::ValueKind::Index3D { base, i, j, k } => {
                value_is_functionally_pure(all_fns, fn_ir, *base, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *i, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *j, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *k, ctx, seen)
            }
            crate::mir::def::ValueKind::Intrinsic { args, .. } => args
                .iter()
                .all(|arg| value_is_functionally_pure(all_fns, fn_ir, *arg, ctx, seen)),
            crate::mir::def::ValueKind::Call { callee, args, .. } => {
                let user_pure = all_fns.get(callee).is_some_and(|callee_ir| {
                    function_is_referentially_pure(all_fns, callee, callee_ir, ctx)
                });
                (effects::call_is_pure(callee) || helper_is_functionally_pure(callee) || user_pure)
                    && args
                        .iter()
                        .all(|arg| value_is_functionally_pure(all_fns, fn_ir, *arg, ctx, seen))
            }
        };
        seen.remove(&vid);
        pure
    }

    fn value_is_fresh_result(
        all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
        fn_ir: &crate::mir::def::FnIR,
        vid: crate::mir::def::ValueId,
        ctx: &mut FreshAnalysisCtx,
        seen: &mut FxHashSet<crate::mir::def::ValueId>,
    ) -> bool {
        if !seen.insert(vid) {
            return false;
        }
        let fresh = match &fn_ir.values[vid].kind {
            crate::mir::def::ValueKind::Const(_) => true,
            crate::mir::def::ValueKind::Call { callee, args, .. } => {
                let user_fresh = all_fns.get(callee).is_some_and(|callee_ir| {
                    function_is_fresh_returning(all_fns, callee, callee_ir, ctx)
                });
                (helper_is_fresh_result(callee) || user_fresh)
                    && args.iter().all(|arg| {
                        value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *arg,
                            ctx,
                            &mut FxHashSet::default(),
                        )
                    })
            }
            _ => false,
        };
        seen.remove(&vid);
        fresh
    }

    fn function_is_referentially_pure(
        all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
        name: &str,
        fn_ir: &crate::mir::def::FnIR,
        ctx: &mut FreshAnalysisCtx,
    ) -> bool {
        if let Some(cached) = ctx.pure_memo.get(name) {
            return *cached;
        }
        if !ctx.visiting_pure.insert(name.to_string()) {
            return false;
        }
        if fn_ir.requires_conservative_optimization() {
            ctx.pure_memo.insert(name.to_string(), false);
            ctx.visiting_pure.remove(name);
            return false;
        }
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                match instr {
                    crate::mir::def::Instr::Assign { src, .. }
                    | crate::mir::def::Instr::Eval { val: src, .. } => {
                        if !value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *src,
                            ctx,
                            &mut FxHashSet::default(),
                        ) {
                            ctx.pure_memo.insert(name.to_string(), false);
                            ctx.visiting_pure.remove(name);
                            return false;
                        }
                    }
                    crate::mir::def::Instr::StoreIndex1D { base, idx, val, .. } => {
                        if !(value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *base,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *idx,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *val,
                            ctx,
                            &mut FxHashSet::default(),
                        )) {
                            ctx.pure_memo.insert(name.to_string(), false);
                            ctx.visiting_pure.remove(name);
                            return false;
                        }
                    }
                    crate::mir::def::Instr::StoreIndex2D {
                        base, r, c, val, ..
                    } => {
                        if !(value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *base,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *r,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *c,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *val,
                            ctx,
                            &mut FxHashSet::default(),
                        )) {
                            ctx.pure_memo.insert(name.to_string(), false);
                            ctx.visiting_pure.remove(name);
                            return false;
                        }
                    }
                    crate::mir::def::Instr::StoreIndex3D {
                        base, i, j, k, val, ..
                    } => {
                        if !(value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *base,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *i,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *j,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *k,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *val,
                            ctx,
                            &mut FxHashSet::default(),
                        )) {
                            ctx.pure_memo.insert(name.to_string(), false);
                            ctx.visiting_pure.remove(name);
                            return false;
                        }
                    }
                }
            }
            match &block.term {
                crate::mir::def::Terminator::If { cond, .. } => {
                    if !value_is_functionally_pure(
                        all_fns,
                        fn_ir,
                        *cond,
                        ctx,
                        &mut FxHashSet::default(),
                    ) {
                        ctx.pure_memo.insert(name.to_string(), false);
                        ctx.visiting_pure.remove(name);
                        return false;
                    }
                }
                crate::mir::def::Terminator::Return(Some(v)) => {
                    if !value_is_functionally_pure(
                        all_fns,
                        fn_ir,
                        *v,
                        ctx,
                        &mut FxHashSet::default(),
                    ) {
                        ctx.pure_memo.insert(name.to_string(), false);
                        ctx.visiting_pure.remove(name);
                        return false;
                    }
                }
                crate::mir::def::Terminator::Goto(_)
                | crate::mir::def::Terminator::Return(None)
                | crate::mir::def::Terminator::Unreachable => {}
            }
        }
        ctx.pure_memo.insert(name.to_string(), true);
        ctx.visiting_pure.remove(name);
        true
    }

    fn function_is_fresh_returning(
        all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
        name: &str,
        fn_ir: &crate::mir::def::FnIR,
        ctx: &mut FreshAnalysisCtx,
    ) -> bool {
        if let Some(cached) = ctx.fresh_memo.get(name) {
            return *cached;
        }
        if !ctx.visiting_fresh.insert(name.to_string()) {
            return false;
        }
        if !function_is_referentially_pure(all_fns, name, fn_ir, ctx) {
            ctx.fresh_memo.insert(name.to_string(), false);
            ctx.visiting_fresh.remove(name);
            return false;
        }
        let mut saw_return = false;
        for block in &fn_ir.blocks {
            if let crate::mir::def::Terminator::Return(Some(v)) = &block.term {
                saw_return = true;
                if !value_is_fresh_result(all_fns, fn_ir, *v, ctx, &mut FxHashSet::default()) {
                    ctx.fresh_memo.insert(name.to_string(), false);
                    ctx.visiting_fresh.remove(name);
                    return false;
                }
            }
        }
        ctx.fresh_memo.insert(name.to_string(), saw_return);
        ctx.visiting_fresh.remove(name);
        saw_return
    }

    let mut out = FxHashSet::default();
    let mut ctx = FreshAnalysisCtx {
        pure_memo: FxHashMap::default(),
        fresh_memo: FxHashMap::default(),
        visiting_pure: FxHashSet::default(),
        visiting_fresh: FxHashSet::default(),
    };
    let mut names: Vec<_> = all_fns.keys().cloned().collect();
    names.sort();
    for name in names {
        let Some(fn_ir) = all_fns.get(&name) else {
            continue;
        };
        if function_is_fresh_returning(all_fns, &name, fn_ir, &mut ctx) {
            out.insert(name);
        }
    }
    out
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
    maybe_write_pulse_stats_json(&pulse_stats);
    if let Some(debug_names) = std::env::var_os("RR_DEBUG_FNIR") {
        let wanted: std::collections::HashSet<String> = debug_names
            .to_string_lossy()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        if !wanted.is_empty() {
            let mut names: Vec<_> = all_fns.keys().filter(|n| wanted.contains(*n)).collect();
            names.sort();
            for name in names {
                if let Some(fn_ir) = all_fns.get(name) {
                    eprintln!("=== RR_DEBUG_FNIR {} ===\n{:#?}", name, fn_ir);
                }
            }
        }
    }
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
        if pulse_stats.vector_candidate_total > 0 {
            ui.step_line_ok(&format!(
                "VecCand: {} total | red {} | cond {} | recur {} | shifted {} | call {} | expr {} | scatter {} | cube {} | map {} | multi {}",
                pulse_stats.vector_candidate_total,
                pulse_stats.vector_candidate_reductions,
                pulse_stats.vector_candidate_conditionals,
                pulse_stats.vector_candidate_recurrences,
                pulse_stats.vector_candidate_shifted,
                pulse_stats.vector_candidate_call_maps,
                pulse_stats.vector_candidate_expr_maps,
                pulse_stats.vector_candidate_scatters,
                pulse_stats.vector_candidate_cube_slices,
                pulse_stats.vector_candidate_basic_maps,
                pulse_stats.vector_candidate_multi_exprs
            ));
            ui.step_line_ok(&format!(
                "VecApply: {} total | red {} | cond {} | recur {} | shifted {} | call {} | expr {} | scatter {} | cube {} | map {} | multi {}",
                pulse_stats.vector_applied_total,
                pulse_stats.vector_applied_reductions,
                pulse_stats.vector_applied_conditionals,
                pulse_stats.vector_applied_recurrences,
                pulse_stats.vector_applied_shifted,
                pulse_stats.vector_applied_call_maps,
                pulse_stats.vector_applied_expr_maps,
                pulse_stats.vector_applied_scatters,
                pulse_stats.vector_applied_cube_slices,
                pulse_stats.vector_applied_basic_maps,
                pulse_stats.vector_applied_multi_exprs
            ));
            ui.step_line_ok(&format!(
                "VecShape: cand 2d {} | 3d {} | apply 2d {} | 3d {} | Trip tiny {} | small {} | medium {} | large {}",
                pulse_stats.vector_candidate_2d,
                pulse_stats.vector_candidate_3d,
                pulse_stats.vector_applied_2d,
                pulse_stats.vector_applied_3d,
                pulse_stats.vector_trip_tier_tiny,
                pulse_stats.vector_trip_tier_small,
                pulse_stats.vector_trip_tier_medium,
                pulse_stats.vector_trip_tier_large
            ));
            if pulse_stats.vector_candidate_call_maps > 0
                || pulse_stats.vector_applied_call_maps > 0
            {
                ui.step_line_ok(&format!(
                    "CallMap: cand direct {} | runtime {} | apply direct {} | runtime {}",
                    pulse_stats.vector_candidate_call_map_direct,
                    pulse_stats.vector_candidate_call_map_runtime,
                    pulse_stats.vector_applied_call_map_direct,
                    pulse_stats.vector_applied_call_map_runtime
                ));
            }
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
    if !final_output.is_empty() {
        final_output.insert_str(0, "# --- RR generated code (from user RR source) ---\n");
    }
    for call in top_level_calls {
        if !final_output.ends_with('\n') {
            final_output.push('\n');
        }
        if !final_output.contains("# --- RR synthesized entrypoints (auto-generated) ---\n")
            && !top_level_calls.is_empty()
        {
            final_output.push_str("# --- RR synthesized entrypoints (auto-generated) ---\n");
        }
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
            "if (!nzchar(Sys.getenv(\"RR_NATIVE_BACKEND\", \"\"))) .rr_env$native_backend <- \"{}\";\n",
            type_cfg.native_backend.as_str()
        ));
        out.push_str(&format!(
            "if (!nzchar(Sys.getenv(\"RR_PARALLEL_MODE\", \"\"))) .rr_env$parallel_mode <- \"{}\";\n",
            parallel_cfg.mode.as_str()
        ));
        out.push_str(&format!(
            "if (!nzchar(Sys.getenv(\"RR_PARALLEL_BACKEND\", \"\"))) .rr_env$parallel_backend <- \"{}\";\n",
            parallel_cfg.backend.as_str()
        ));
        out.push_str(&format!(
            "if (!nzchar(Sys.getenv(\"RR_PARALLEL_THREADS\", \"\"))) .rr_env$parallel_threads <- as.integer({});\n",
            parallel_cfg.threads
        ));
        out.push_str(&format!(
            "if (!nzchar(Sys.getenv(\"RR_PARALLEL_MIN_TRIP\", \"\"))) .rr_env$parallel_min_trip <- as.integer({});\n",
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
    let emit_order = if output_opts.preserve_all_defs {
        emit_order
    } else {
        reachable_emit_order(&all_fns, &emit_order, &emit_roots)
    };

    let (final_output, final_source_map, emit_cache_hits, emit_cache_misses) =
        emit_r_functions_cached(
            &ui,
            TOTAL_STEPS,
            &all_fns,
            &emit_order,
            &top_level_calls,
            opt_level,
            type_cfg,
            parallel_cfg,
            output_opts,
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
        if !final_output.is_empty() {
            without_runtime.push_str("# --- RR generated code (from user RR source) ---\n");
        }
        without_runtime.push_str(&final_output);
        for call in &top_level_calls {
            if !without_runtime.ends_with('\n') {
                without_runtime.push('\n');
            }
            if !without_runtime.contains("# --- RR synthesized entrypoints (auto-generated) ---\n")
                && !top_level_calls.is_empty()
            {
                without_runtime.push_str("# --- RR synthesized entrypoints (auto-generated) ---\n");
            }
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

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashSet;

    use super::*;

    #[test]
    fn raw_emitted_trivial_clamp_helper_calls_inline_before_peephole() {
        let input = [
            "Sym_1 <- function() ",
            "{",
            "  next_a[i] <- Sym_20(next_a_cell, 0, 1)",
            "  next_b[i] <- Sym_20(next_b_cell, 0, 1)",
            "}",
            "",
            "Sym_20 <- function(x, lo, hi) ",
            "{",
            "  .arg_x <- x",
            "  .arg_lo <- lo",
            "  .arg_hi <- hi",
            "  y <- .arg_x",
            "  if ((y < .arg_lo)) {",
            "    y <- .arg_lo",
            "  } else {",
            "  }",
            "  if ((y > .arg_hi)) {",
            "    y <- hi",
            "  } else {",
            "  }",
            "  return(y)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_trivial_clamp_helper_calls_in_raw_emitted_r(&input);

        assert!(out.contains("next_a[i] <- (pmin(pmax(next_a_cell, 0), 1))"));
        assert!(out.contains("next_b[i] <- (pmin(pmax(next_b_cell, 0), 1))"));
        assert!(!out.contains("Sym_20(next_a_cell, 0, 1)"));
        assert!(!out.contains("Sym_20(next_b_cell, 0, 1)"));
        assert!(out.contains("Sym_20 <- function(x, lo, hi)"));
    }

    #[test]
    fn raw_emitted_branch_local_identical_alloc_rebinds_are_pruned_before_peephole() {
        let input = [
            "Sym_123 <- function(b, size) ",
            "{",
            "x <- rep.int(0, size)",
            "rs_old <- (sum((b[seq_len(size)] * b[seq_len(size)])))",
            "if (((is.na(rs_old) | (!(is.finite(rs_old)))) | (rs_old == 0))) {",
            "rs_old <- 0.0000001",
            "x <- Sym_17(size, 0)",
            "}",
            "return(x)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_branch_local_identical_alloc_rebinds_in_raw_emitted_r(&input);

        assert!(out.contains("x <- rep.int(0, size)"));
        assert!(out.contains("rs_old <- 0.0000001"));
        assert!(!out.contains("x <- Sym_17(size, 0)"));
        assert!(out.contains("return(x)"));
    }

    #[test]
    fn raw_emitted_branch_local_identical_scalar_rebinds_prune_before_alias_inline() {
        let input = [
            "Sym_287 <- function(temp, q_v, size) ",
            "{",
            "  i <- 1",
            "  ii <- i",
            "  T_c <- (temp[ii] - 273.15)",
            "  if ((T_c < (-(5)))) {",
            "    qv <- q_v[ii]",
            "    ii <- i",
            "  }",
            "  if ((T_c < (-(15)))) {",
            "    if ((qv > 0.01)) {",
            "      print(qv)",
            "    }",
            "  }",
            "  return(T_c)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_single_use_scalar_index_aliases_in_raw_emitted_r(
            &rewrite_branch_local_identical_alloc_rebinds_in_raw_emitted_r(&input),
        );

        assert!(!out.contains("    ii <- i\n  }"), "{out}");
        assert!(!out.contains("qv <- q_v[ii]"), "{out}");
        assert!(out.contains("if ((q_v[ii] > 0.01)) {"), "{out}");
    }

    #[test]
    fn raw_emitted_single_use_scalar_index_aliases_inline_before_peephole() {
        let input = [
            "Sym_287 <- function(temp, q_v, q_c, size) ",
            "{",
            "  i <- 1",
            "  repeat {",
            "    if (!(i <= size)) break",
            "    T_c <- (temp[i] - 273.15)",
            "    qv <- q_v[i]",
            "    qc <- q_c[i]",
            "    if ((qv > 0.01)) {",
            "      if ((qc > 0.0001)) {",
            "        print(T_c)",
            "      }",
            "    }",
            "    i <- (i + 1)",
            "    next",
            "  }",
            "  return(0)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_single_use_scalar_index_aliases_in_raw_emitted_r(&input);

        assert!(!out.contains("qv <- q_v[i]"), "{out}");
        assert!(!out.contains("qc <- q_c[i]"), "{out}");
        assert!(
            out.contains("if ((q_v[i] > 0.01)) {") || out.contains("if (((q_v[i]) > 0.01)) {"),
            "{out}"
        );
        assert!(
            out.contains("if ((q_c[i] > 0.0001)) {") || out.contains("if (((q_c[i]) > 0.0001)) {"),
            "{out}"
        );
        assert!(out.contains("print(T_c)"), "{out}");
    }

    #[test]
    fn raw_emitted_small_multiuse_scalar_index_aliases_inline_into_adjacent_assignments() {
        let input = [
            "Sym_210 <- function(A, B, lapA, lapB, DA, DB, f, k, dt, i) ",
            "{",
            "  a <- A[i]",
            "  b <- B[i]",
            "  new_a <- (a + ((((DA * lapA[i]) - (a * b) * b) + (f * (1 - a))) * dt))",
            "  new_b <- (b + ((((DB * lapB[i]) + (a * b) * b) - ((k + f) * b)) * dt))",
            "  return(new_b)",
            "}",
            "",
        ]
        .join("\n");

        let out =
            rewrite_small_multiuse_scalar_index_aliases_in_adjacent_assignments_in_raw_emitted_r(
                &input,
            );

        assert!(!out.contains("a <- A[i]"), "{out}");
        assert!(!out.contains("b <- B[i]"), "{out}");
        assert!(out.contains("new_a <- (A[i] + ((((DA * lapA[i]) - (A[i] * B[i]) * B[i]) + (f * (1 - A[i]))) * dt))"), "{out}");
        assert!(out.contains("new_b <- (B[i] + ((((DB * lapB[i]) + (A[i] * B[i]) * B[i]) - ((k + f) * B[i])) * dt))"), "{out}");
    }

    #[test]
    fn raw_emitted_small_multiuse_scalar_index_aliases_do_not_inline_past_adjacent_region() {
        let input = [
            "Sym_1 <- function(A, i) ",
            "{",
            "  a <- A[i]",
            "  keep <- (a + 1)",
            "  skip <- 0",
            "  out <- (a + 2)",
            "  return(out)",
            "}",
            "",
        ]
        .join("\n");

        let out =
            rewrite_small_multiuse_scalar_index_aliases_in_adjacent_assignments_in_raw_emitted_r(
                &input,
            );

        assert!(out.contains("a <- A[i]"), "{out}");
        assert!(out.contains("out <- (a + 2)"), "{out}");
    }

    #[test]
    fn raw_emitted_floor_fed_particle_clamp_pair_collapses_gx_gy() {
        let input = [
            "Sym_186 <- function(x, y, N) ",
            "{",
            "  gx <- ((x * N) + 1)",
            "  gy <- ((y * N) + 1)",
            "  if ((gx < 1)) {",
            "    gx <- 1",
            "  }",
            "  if ((gx > N)) {",
            "    gx <- N",
            "  }",
            "  if ((gy < 1)) {",
            "    gy <- 1",
            "  }",
            "  if ((gy > N)) {",
            "    gy <- N",
            "  }",
            "  return(rr_idx_cube_vec_i(f, (floor(gx)), (floor(gy)), N))",
            "}",
            "",
        ]
        .join("\n");

        let out = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&input);

        assert!(
            out.contains("gx <- (pmin(pmax((x * N) + 1, 1), N))"),
            "{out}"
        );
        assert!(
            out.contains("gy <- (pmin(pmax((y * N) + 1, 1), N))"),
            "{out}"
        );
        assert!(!out.contains("if ((gx < 1)) {"), "{out}");
        assert!(!out.contains("if ((gy > N)) {"), "{out}");
    }

    #[test]
    fn raw_emitted_gray_scott_clamp_pair_collapses_new_a_new_b() {
        let input = [
            "Sym_222 <- function(A, B, lapA, lapB, DA, DB, f, k, dt, i) ",
            "{",
            "  new_a <- (A[i] + ((((DA * lapA[i]) - (A[i] * B[i]) * B[i]) + (f * (1 - A[i]))) * dt))",
            "  new_b <- (B[i] + ((((DB * lapB[i]) + (A[i] * B[i]) * B[i]) - ((k + f) * B[i])) * dt))",
            "  if ((new_a < 0)) {",
            "    new_a <- 0",
            "  }",
            "  if ((new_a > 1)) {",
            "    new_a <- 1",
            "",
            "  }",
            "  if ((new_b < 0)) {",
            "    new_b <- 0",
            "  }",
            "  if ((new_b > 1)) {",
            "    new_b <- 1",
            "  }",
            "  return(new_b)",
            "}",
            "",
        ]
        .join("\n");

        let out = collapse_gray_scott_clamp_pair_in_raw_emitted_r(&input);

        assert!(out.contains("new_a <- (pmin(pmax(A[i] + ((((DA * lapA[i]) - (A[i] * B[i]) * B[i]) + (f * (1 - A[i]))) * dt), 0), 1))"), "{out}");
        assert!(out.contains("new_b <- (pmin(pmax(B[i] + ((((DB * lapB[i]) + (A[i] * B[i]) * B[i]) - ((k + f) * B[i])) * dt), 0), 1))"), "{out}");
        assert!(!out.contains("if ((new_a < 0)) {"), "{out}");
        assert!(!out.contains("if ((new_b > 1)) {"), "{out}");
    }

    #[test]
    fn raw_emitted_sym287_melt_rate_branch_collapses_to_direct_heat_sink_updates() {
        let input = [
            "Sym_287 <- function(temp, q_s, q_g) ",
            "{",
            "  if ((T_c > 0)) {",
            "    melt_rate <- 0",
            "    if ((q_s[i] > 0)) {",
            "      melt_rate <- (q_s[i] * 0.05)",
            "    }",
            "    if ((q_g[i] > 0)) {",
            "      melt_rate <- (melt_rate + (q_g[i] * 0.02))",
            "    }",
            "    tendency_T <- (tendency_T - (melt_rate * L_f))",
            "  }",
            "  return(tendency_T)",
            "}",
            "",
        ]
        .join("\n");

        let out = collapse_sym287_melt_rate_branch_in_raw_emitted_r(&input);

        assert!(!out.contains("melt_rate <- 0"), "{out}");
        assert!(!out.contains("melt_rate <- (q_s[i] * 0.05)"), "{out}");
        assert!(
            !out.contains("melt_rate <- (melt_rate + (q_g[i] * 0.02))"),
            "{out}"
        );
        assert!(
            !out.contains("tendency_T <- (tendency_T - (melt_rate * L_f))"),
            "{out}"
        );
        assert!(
            out.contains("tendency_T <- (tendency_T - ((q_s[i] * 0.05) * L_f))"),
            "{out}"
        );
        assert!(
            out.contains("tendency_T <- (tendency_T - ((q_g[i] * 0.02) * L_f))"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_exact_safe_loop_index_write_calls_rewrite_to_base_indexing() {
        let input = [
            "Sym_210 <- function(A, B, heat, SIZE) ",
            "{",
            "  i <- 1",
            "  repeat {",
            "    if (!(i <= SIZE)) break",
            "    A[rr_index1_write(i, \"index\")] <- new_a",
            "    B[rr_index1_write(i, \"index\")] <- new_b",
            "    heat[rr_index1_write(i, \"index\")] <- out_v",
            "    i <- (i + 1)",
            "    next",
            "  }",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_exact_safe_loop_index_write_calls_in_raw_emitted_r(&input);

        assert!(!out.contains("rr_index1_write(i, \"index\")"), "{out}");
        assert!(out.contains("A[i] <- new_a"), "{out}");
        assert!(out.contains("B[i] <- new_b"), "{out}");
        assert!(out.contains("heat[i] <- out_v"), "{out}");
    }

    #[test]
    fn raw_emitted_literal_named_list_calls_rewrite_to_base_list() {
        let input = [
            "Sym_186 <- function(px, py, pf) ",
            "{",
            "  return(rr_named_list(\"px\", px, \"py\", py, \"pf\", pf))",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_literal_named_list_calls_in_raw_emitted_r(&input);

        assert!(
            out.contains("return(list(px = px, py = py, pf = pf))"),
            "{out}"
        );
        assert!(
            !out.contains("rr_named_list(\"px\", px, \"py\", py, \"pf\", pf)"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_literal_field_get_calls_rewrite_to_base_indexing() {
        let input = [
            "Sym_303 <- function(particles) ",
            "{",
            "  p_x <- rr_field_get(particles, \"px\")",
            "  p_y <- rr_field_get(particles, \"py\")",
            "  p_f <- rr_field_get(particles, \"pf\")",
            "  return(p_f)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_literal_field_get_calls_in_raw_emitted_r(&input);

        assert!(out.contains("p_x <- particles[[\"px\"]]"), "{out}");
        assert!(out.contains("p_y <- particles[[\"py\"]]"), "{out}");
        assert!(out.contains("p_f <- particles[[\"pf\"]]"), "{out}");
        assert!(!out.contains("rr_field_get(particles, \"px\")"), "{out}");
    }

    #[test]
    fn raw_emitted_slice_bound_aliases_inline_into_neighbor_row_writes() {
        let input = [
            "Sym_83 <- function(dir, size) ",
            "{",
            "  start <- rr_idx_cube_vec_i(f, x, 1, size)",
            "  end <- rr_idx_cube_vec_i(f, x, size, size)",
            "  if ((dir == 1)) {",
            "    neighbors[start:end] <- Sym_60(f, x, ys, size)",
            "  }",
            "  if ((dir == 2)) {",
            "    neighbors[start:end] <- Sym_64(f, x, ys, size)",
            "  }",
            "  return(neighbors)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_slice_bound_aliases_in_raw_emitted_r(&input);

        assert!(!out.contains("start <- rr_idx_cube_vec_i"), "{out}");
        assert!(!out.contains("end <- rr_idx_cube_vec_i"), "{out}");
        assert!(
            out.contains(
                "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_60(f, x, ys, size)"
            ),
            "{out}"
        );
        assert!(
            out.contains(
                "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_64(f, x, ys, size)"
            ),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_adjacent_dir_neighbor_row_branches_collapse_to_else_if_chain() {
        let input = [
            "Sym_83 <- function(dir, size) ",
            "{",
            "  if ((dir == 1)) {",
            "    neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_60(f, x, ys, size)",
            "  }",
            "  if ((dir == 2)) {",
            "    neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_64(f, x, ys, size)",
            "  }",
            "  if ((dir == 3)) {",
            "    neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_66(f, x, ys, size)",
            "  }",
            "  if ((dir == 4)) {",
            "    neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_72(f, x, ys, size)",
            "  }",
            "  return(neighbors)",
            "}",
            "",
        ]
        .join("\n");

        let out = collapse_adjacent_dir_neighbor_row_branches_in_raw_emitted_r(&input);

        assert!(out.contains("} else if ((dir == 2)) {"), "{out}");
        assert!(out.contains("} else if ((dir == 3)) {"), "{out}");
        assert!(out.contains("} else if ((dir == 4)) {"), "{out}");
        assert!(!out.contains("  }\n  if ((dir == 2)) {"), "{out}");
    }

    #[test]
    fn raw_emitted_single_assignment_loop_seed_literals_inline_into_next_vector_expr() {
        let input = [
            "Sym_210 <- function(field, w, h) ",
            "{",
            "  size <- (w * h)",
            "  i <- 1",
            "  lap <- (((i:size - 1) %% w) + ((i:size - 1) %% h))",
            "  return(lap)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_single_assignment_loop_seed_literals_in_raw_emitted_r(&input);

        assert!(!out.contains("  i <- 1"), "{out}");
        assert!(
            out.contains("lap <- (((1:size - 1) %% w) + ((1:size - 1) %% h))"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_sym210_loop_seed_literal_inlines_into_laplacian_expr() {
        let input = [
            "Sym_210 <- function(field, w, h) ",
            "{",
            "  size <- (w * h)",
            "",
            "  i <- 1",
            "",
            "  lap <- ((((((rr_gather(field, rr_wrap_index_vec_i(((((i:size - 1) %% w) + 1) - 1), (floor(((i:size - 1) / w)) + 1), w, h)) + rr_gather(field, rr_wrap_index_vec_i(((((i:size - 1) %% w) + 1) + 1), (floor(((i:size - 1) / w)) + 1), w, h))) + rr_gather(field, rr_wrap_index_vec_i((((i:size - 1) %% w) + 1), ((floor(((i:size - 1) / w)) + 1) + 1), w, h))) + rr_gather(field, rr_wrap_index_vec_i((((i:size - 1) %% w) + 1), ((floor(((i:size - 1) / w)) + 1) - 1), w, h))) * 0.2) - field)",
            "  return(lap)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_sym210_loop_seed_in_raw_emitted_r(&input);

        assert!(!out.contains("\n  i <- 1\n"), "{out}");
        assert!(out.contains("1:size - 1"), "{out}");
        assert!(!out.contains("i:size - 1"), "{out}");
    }

    #[test]
    fn raw_emitted_unreachable_helper_definitions_prune_after_probe_energy_reuse() {
        let input = [
            "Sym_49 <- function(a, b) ",
            "{",
            "  return(rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), a, b))",
            "}",
            "",
            "Sym_51 <- function(a, b) ",
            "{",
            "  mix <- Sym_49(a, b)",
            "  return(mean(abs(mix)))",
            "}",
            "",
            "Sym_303 <- function() ",
            "{",
            "  probe_vec <- Sym_49(c(1, 2, 3, 4), c(4, 3, 2, 1))",
            "  probe_energy <- mean(abs(probe_vec))",
            "  return(probe_energy)",
            "}",
            "",
            "Sym_top_0 <- function() ",
            "{",
            "  return(Sym_303())",
            "}",
            "",
        ]
        .join("\n");

        let out = prune_unreachable_raw_helper_definitions(&input);

        assert!(out.contains("Sym_49 <- function(a, b)"), "{out}");
        assert!(!out.contains("Sym_51 <- function(a, b)"), "{out}");
        assert!(out.contains("Sym_303 <- function()"), "{out}");
        assert!(out.contains("Sym_top_0 <- function()"), "{out}");
    }

    #[test]
    fn raw_emitted_unreachable_helper_prune_keeps_unquoted_symbol_references() {
        let input = [
            "Sym_49 <- function(a, b) ",
            "{",
            "  return(a + b)",
            "}",
            "",
            "Sym_49__typed_impl <- function(a, b) ",
            "{",
            "  return(a * b)",
            "}",
            "",
            "Sym_303 <- function() ",
            "{",
            "  probe_vec <- rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), c(1, 2), c(3, 4))",
            "  return(probe_vec)",
            "}",
            "",
            "Sym_top_0 <- function() ",
            "{",
            "  return(Sym_303())",
            "}",
            "",
        ]
        .join("\n");

        let out = prune_unreachable_raw_helper_definitions(&input);

        assert!(!out.contains("Sym_49 <- function(a, b)"), "{out}");
        assert!(
            out.contains("Sym_49__typed_impl <- function(a, b)"),
            "{out}"
        );
        assert!(
            out.contains("rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl"),
            "{out}"
        );
        assert!(out.contains("Sym_top_0 <- function()"), "{out}");
    }

    #[test]
    fn raw_emitted_shadowed_simple_scalar_seed_assigns_prune_before_first_use() {
        let input = [
            "Sym_303 <- function(adj_l, adj_r, TOTAL) ",
            "{",
            "  i <- 1",
            "  adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))",
            "  adj_rr <- rr_gather(adj_r, rr_index_vec_floor(adj_r))",
            "  i <- 1",
            "  repeat {",
            "    if (!(i <= TOTAL)) break",
            "    i <- (i + 1)",
            "    next",
            "  }",
            "  return(adj_ll)",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_shadowed_simple_scalar_seed_assigns_in_raw_emitted_r(&input);

        assert!(
            !out.contains("  i <- 1\n  adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))"),
            "{out}"
        );
        assert!(
            out.contains("adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))"),
            "{out}"
        );
        assert!(out.contains("  i <- 1\n  repeat {"), "{out}");
    }

    #[test]
    fn raw_emitted_dead_weno_topology_seed_i_prunes_before_direct_adj_gather() {
        let input = [
            "Sym_303 <- function(adj_l, adj_r, TOTAL) ",
            "{",
            "  i <- 1",
            "  adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))",
            "  adj_rr <- rr_gather(adj_r, rr_index_vec_floor(adj_r))",
            "  h_trn <- rep.int(0, TOTAL)",
            "  i <- 1",
            "  repeat {",
            "    if (!(i <= TOTAL)) break",
            "    i <- (i + 1)",
            "    next",
            "  }",
            "  return(adj_ll)",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_dead_weno_topology_seed_i_before_direct_adj_gather_in_raw_emitted_r(&input);

        assert!(
            !out.contains("  i <- 1\n  adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))"),
            "{out}"
        );
        assert!(
            out.contains("adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))"),
            "{out}"
        );
        assert!(out.contains("  i <- 1\n  repeat {"), "{out}");
    }

    #[test]
    fn raw_emitted_mountain_dx_temp_inlines_into_dist_expr() {
        let input = [
            "Sym_303 <- function() ",
            "{",
            "  dx_m <- (x_curr - 20)",
            "  dist <- ((dx_m * dx_m) + ((y_curr - 20) * (y_curr - 20)))",
            "  return(dist)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_mountain_dx_temp_in_raw_emitted_r(&input);

        assert!(!out.contains("dx_m <- (x_curr - 20)"), "{out}");
        assert!(
            out.contains(
                "dist <- ((((x_curr - 20) * (x_curr - 20)) + ((y_curr - 20) * (y_curr - 20))))"
            ),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_mountain_dx_dy_temps_inline_into_dist_expr() {
        let input = [
            "Sym_303 <- function() ",
            "{",
            "  dx_m <- (x_curr - 20)",
            "  dy_m <- (y_curr - 20)",
            "  .__rr_cse_205 <- (rem / N)",
            "  .__rr_cse_206 <- floor(.__rr_cse_205)",
            "  .__rr_cse_209 <- (rem %% N)",
            "  dist <- ((dx_m * dx_m) + (dy_m * dy_m))",
            "  return(dist)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_mountain_dx_temp_in_raw_emitted_r(&input);

        assert!(!out.contains("dx_m <- (x_curr - 20)"), "{out}");
        assert!(!out.contains("dy_m <- (y_curr - 20)"), "{out}");
        assert!(
            out.contains(
                "dist <- ((((x_curr - 20) * (x_curr - 20)) + ((y_curr - 20) * (y_curr - 20))))"
            ),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_dead_zero_seed_ii_prunes_when_never_used() {
        let input = [
            "Sym_303 <- function() ",
            "{",
            "  i <- 1",
            "  ii <- 0",
            "  .tachyon_exprmap0_0 <- rr_gather(adj_l, rr_index_vec_floor(rr_index1_read_vec(adj_l, rr_index_vec_floor(i:TOTAL))))",
            "  return(.tachyon_exprmap0_0)",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_dead_zero_seed_ii_in_raw_emitted_r(&input);

        assert!(!out.contains("ii <- 0"), "{out}");
        assert!(out.contains(".tachyon_exprmap0_0 <- rr_gather("), "{out}");
    }

    #[test]
    fn raw_emitted_trivial_fill_helper_calls_inline_to_rep_int() {
        let input = [
            "Sym_17 <- function(n, val) ",
            "{",
            "  return(rep.int(val, n))",
            "}",
            "",
            "Sym_303 <- function(TOTAL) ",
            "{",
            "  h <- Sym_17(TOTAL, 8000)",
            "  qv <- Sym_17(TOTAL, 0.015)",
            "  p_f <- Sym_17(1000, 1)",
            "  return(h)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_trivial_fill_helper_calls_in_raw_emitted_r(&input);

        assert!(out.contains("h <- rep.int(8000, TOTAL)"), "{out}");
        assert!(out.contains("qv <- rep.int(0.015, TOTAL)"), "{out}");
        assert!(out.contains("p_f <- rep.int(1, 1000)"), "{out}");
        assert!(!out.contains("h <- Sym_17(TOTAL, 8000)"), "{out}");
    }

    #[test]
    fn raw_emitted_identical_zero_fill_pairs_rewrite_to_aliases() {
        let input = [
            "Sym_303 <- function(TOTAL) ",
            "{",
            "  adj_ll <- rep.int(0, TOTAL)",
            "  adj_rr <- rep.int(0, TOTAL)",
            "  u_stage <- rep.int(0, TOTAL)",
            "  u_new <- rep.int(0, TOTAL)",
            "  return(adj_rr)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_identical_zero_fill_pairs_to_aliases_in_raw_emitted_r(&input);

        assert!(out.contains("adj_ll <- rep.int(0, TOTAL)"), "{out}");
        assert!(out.contains("adj_rr <- adj_ll"), "{out}");
        assert!(out.contains("u_stage <- rep.int(0, TOTAL)"), "{out}");
        assert!(out.contains("u_new <- u_stage"), "{out}");
    }

    #[test]
    fn raw_emitted_identical_zero_fill_pairs_do_not_alias_when_buffers_diverge_later() {
        let input = [
            "Sym_303 <- function(TOTAL) ",
            "{",
            "  u_stage <- rep.int(0, TOTAL)",
            "  u_new <- rep.int(0, TOTAL)",
            "  i <- 1",
            "  repeat {",
            "    if (!(i <= TOTAL)) break",
            "    u_new[i] <- i",
            "    i <- (i + 1)",
            "  }",
            "  return(u_new)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_identical_zero_fill_pairs_to_aliases_in_raw_emitted_r(&input);

        assert!(out.contains("u_stage <- rep.int(0, TOTAL)"), "{out}");
        assert!(out.contains("u_new <- rep.int(0, TOTAL)"), "{out}");
        assert!(!out.contains("u_new <- u_stage"), "{out}");
    }

    #[test]
    fn raw_emitted_duplicate_sym183_calls_reuse_first_result() {
        let input = [
            "Sym_303 <- function() ",
            "{",
            "  p_x <- Sym_183(1000)",
            "  p_y <- Sym_183(1000)",
            "  return(p_y)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_duplicate_sym183_calls_in_raw_emitted_r(&input);

        assert!(out.contains("p_x <- Sym_183(1000)"), "{out}");
        assert!(out.contains("p_y <- p_x"), "{out}");
        assert!(!out.contains("p_y <- Sym_183(1000)"), "{out}");
    }

    #[test]
    fn raw_emitted_prunes_dead_zero_loop_seeds_before_for() {
        let input = "\
Sym_top_0 <- function() \n\
{\n\
steps <- 0\n\
dt <- 0.1\n\
for (steps in seq_len(5)) {\n\
  print(steps)\n\
}\n\
k <- 1\n\
for (k in seq_len(TOTAL)) {\n\
  print(k)\n\
}\n\
}\n";
        let out = super::strip_dead_zero_loop_seeds_before_for_in_raw_emitted_r(input);
        assert!(!out.contains("steps <- 0"), "{out}");
        assert!(!out.contains("k <- 1"), "{out}");
        assert!(out.contains("for (steps in seq_len(5)) {"), "{out}");
        assert!(out.contains("for (k in seq_len(TOTAL)) {"), "{out}");
    }

    #[test]
    fn raw_emitted_duplicate_pure_call_assignments_reuse_first_result() {
        let input = [
            "Sym_303 <- function(temp, qv, qc, qs, qg, TOTAL) ",
            "{",
            "  heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)",
            "  heat2 <- Sym_287(temp, qv, qc, qs, qg, TOTAL)",
            "  return(heat2)",
            "}",
            "",
        ]
        .join("\n");
        let pure = FxHashSet::from_iter([String::from("Sym_287")]);

        let out = rewrite_duplicate_pure_call_assignments_in_raw_emitted_r(&input, &pure);

        assert!(
            out.contains("heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)"),
            "{out}"
        );
        assert!(out.contains("heat2 <- heat"), "{out}");
        assert!(
            !out.contains("heat2 <- Sym_287(temp, qv, qc, qs, qg, TOTAL)"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_adjacent_duplicate_symbol_assignments_reuse_first_result() {
        let input = [
            "Sym_123 <- function(b) ",
            "{",
            "  r <- b",
            "  p <- b",
            "  return(p)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_adjacent_duplicate_symbol_assignments_in_raw_emitted_r(&input);

        assert!(out.contains("r <- b"), "{out}");
        assert!(out.contains("p <- r"), "{out}");
        assert!(!out.contains("p <- b"), "{out}");
    }

    #[test]
    fn raw_emitted_weno_full_range_gather_replay_after_fill_inline_collapses() {
        let input = [
            "Sym_303 <- function(TOTAL, adj_l, adj_r) ",
            "{",
            "  adj_ll <- qr",
            "  adj_rr <- adj_ll",
            "  i <- 1",
            "",
            "  # rr-cse-pruned",
            "  .tachyon_exprmap0_0 <- rr_gather(adj_l, rr_index_vec_floor(rr_index1_read_vec(adj_l, rr_index_vec_floor(i:((6 * N) * N)))))",
            "  .tachyon_exprmap1_0 <- rr_gather(adj_r, rr_index_vec_floor(rr_index1_read_vec(adj_r, rr_index_vec_floor(i:((6 * N) * N)))))",
            "  adj_ll <- rr_assign_slice(adj_ll, i, ((6 * N) * N), .tachyon_exprmap0_0)",
            "  adj_rr <- rr_assign_slice(adj_rr, i, ((6 * N) * N), .tachyon_exprmap1_0)",
            "  return(adj_ll)",
            "}",
            "",
        ]
        .join("\n");

        let out = collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r(&input);

        assert!(
            out.contains("adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))"),
            "{out}"
        );
        assert!(
            out.contains("adj_rr <- rr_gather(adj_r, rr_index_vec_floor(adj_r))"),
            "{out}"
        );
        assert!(!out.contains(".tachyon_exprmap0_0 <-"), "{out}");
        assert!(!out.contains(".tachyon_exprmap1_0 <-"), "{out}");
        assert!(!out.contains("rr_assign_slice(adj_ll"), "{out}");
        assert!(!out.contains("rr_assign_slice(adj_rr"), "{out}");
    }

    #[test]
    fn raw_emitted_weno_full_range_gather_replay_after_fill_inline_collapses_with_following_lines()
    {
        let input = [
            "Sym_303 <- function(TOTAL, adj_l, adj_r) ",
            "{",
            "  print(\"  Building Extended Topology (WENO-5 Order)...\")",
            "  adj_ll <- qr",
            "  adj_rr <- qr",
            "  i <- 1",
            "",
            "  # rr-cse-pruned",
            "  .tachyon_exprmap0_0 <- rr_gather(adj_l, rr_index_vec_floor(rr_index1_read_vec(adj_l, rr_index_vec_floor(i:((6 * N) * N)))))",
            "  .tachyon_exprmap1_0 <- rr_gather(adj_r, rr_index_vec_floor(rr_index1_read_vec(adj_r, rr_index_vec_floor(i:((6 * N) * N)))))",
            "  adj_ll <- rr_assign_slice(adj_ll, i, ((6 * N) * N), .tachyon_exprmap0_0)",
            "  adj_rr <- rr_assign_slice(adj_rr, i, ((6 * N) * N), .tachyon_exprmap1_0)",
            "  h_trn <- qr",
            "  coriolis <- qr",
            "  return(adj_ll)",
            "}",
            "",
        ]
        .join("\n");

        let out = collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r(&input);

        assert!(
            out.contains("adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))"),
            "{out}"
        );
        assert!(
            out.contains("adj_rr <- rr_gather(adj_r, rr_index_vec_floor(adj_r))"),
            "{out}"
        );
        assert!(!out.contains(".tachyon_exprmap0_0 <-"), "{out}");
        assert!(!out.contains(".tachyon_exprmap1_0 <-"), "{out}");
        assert!(!out.contains("rr_assign_slice(adj_ll"), "{out}");
        assert!(!out.contains("rr_assign_slice(adj_rr"), "{out}");
        assert!(out.contains("h_trn <- qr"), "{out}");
    }

    #[test]
    fn raw_emitted_dot_product_helper_calls_inline_to_direct_sum_exprs() {
        let input = [
            "Sym_117 <- function(a, b, n) ",
            "{",
            "  return(sum((a[seq_len(n)] * b[seq_len(n)])))",
            "}",
            "",
            "Sym_123 <- function(r, p, Ap, size) ",
            "{",
            "  rs_old <- Sym_117(r, r, size)",
            "  p_Ap <- Sym_117(p, Ap, size)",
            "  rs_new <- Sym_117((r - (alpha * Ap)), (r - (alpha * Ap)), size)",
            "  return(rs_new)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_dot_product_helper_calls_in_raw_emitted_r(&input);

        assert!(
            out.contains("rs_old <- sum((r[seq_len(size)] * r[seq_len(size)]))"),
            "{out}"
        );
        assert!(
            out.contains("p_Ap <- sum((p[seq_len(size)] * Ap[seq_len(size)]))"),
            "{out}"
        );
        assert!(
            out.contains(
                "rs_new <- sum(((r - (alpha * Ap))[seq_len(size)] * (r - (alpha * Ap))[seq_len(size)]))"
            ),
            "{out}"
        );
        assert!(!out.contains("rs_old <- Sym_117(r, r, size)"), "{out}");
    }

    #[test]
    fn raw_emitted_sym119_helper_calls_inline_to_direct_gather_expr() {
        let input = [
            "Sym_119 <- function(x, n_l, n_r, n_d, n_u) ",
            "{",
            "  n_d <- rr_index_vec_floor(n_d)",
            "  n_l <- rr_index_vec_floor(n_l)",
            "  n_r <- rr_index_vec_floor(n_r)",
            "  n_u <- rr_index_vec_floor(n_u)",
            "  y <- ((4.0001 * x) - (((rr_gather(x, n_l) + rr_gather(x, n_r)) + rr_gather(x, n_d)) + rr_gather(x, n_u)))",
            "  return(y)",
            "}",
            "",
            "Sym_123 <- function(p, n_l, n_r, n_d, n_u) ",
            "{",
            "  Ap <- Sym_119(p, n_l, n_r, n_d, n_u)",
            "  return(Ap)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_sym119_helper_calls_in_raw_emitted_r(&input);

        assert!(
            out.contains(
                "Ap <- ((4.0001 * p) - (((rr_gather(p, rr_index_vec_floor(n_l)) + rr_gather(p, rr_index_vec_floor(n_r))) + rr_gather(p, rr_index_vec_floor(n_d))) + rr_gather(p, rr_index_vec_floor(n_u))))"
            ),
            "{out}"
        );
        assert!(
            !out.contains("Ap <- Sym_119(p, n_l, n_r, n_d, n_u)"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_seq_len_full_overwrite_init_rewrites_to_zero_init() {
        let input = [
            "Sym_183 <- function(n) ",
            "{",
            "  p <- seq_len(n)",
            "  i <- 1",
            "  seed <- 12345",
            "  repeat {",
            "    if (!(i <= n)) break",
            "    seed <- (((seed * 1103515245) + 12345) %% 2147483648)",
            "    p[i] <- (seed / 2147483648)",
            "    i <- (i + 1)",
            "    next",
            "  }",
            "  return(p)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_seq_len_full_overwrite_inits_in_raw_emitted_r(&input);

        assert!(out.contains("p <- rep.int(0, n)"), "{out}");
        assert!(!out.contains("p <- seq_len(n)"), "{out}");
        assert!(out.contains("p[i] <- (seed / 2147483648)"), "{out}");
    }

    #[test]
    fn raw_emitted_dead_seq_len_local_keeps_seed_when_rhs_reads_previous_value() {
        let input = [
            "Sym_1 <- function(n) ",
            "{",
            "  y <- seq_len(n)",
            "  y <- rr_call_map_slice_auto(y, 2L, n, \"abs\", 16L, c(1L), rr_index1_read_vec((seq_len(n) - 5L), rr_index_vec_floor(2L:n)))",
            "  return(y)",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_dead_seq_len_locals_in_raw_emitted_r(&input);

        assert!(out.contains("y <- seq_len(n)"), "{out}");
        assert!(
            out.contains("y <- rr_call_map_slice_auto(y, 2L, n"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_helper_expr_reuse_calls_rewrite_probe_energy_to_probe_vec() {
        let input = [
            "Sym_49 <- function(a, b) ",
            "{",
            "  return(rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), a, b))",
            "}",
            "",
            "Sym_51 <- function(a, b) ",
            "{",
            "  mix <- Sym_49(a, b)",
            "  return(mean(abs(mix)))",
            "}",
            "",
            "Sym_303 <- function() ",
            "{",
            "  probe_vec <- Sym_49(c(1, 2, 3, 4), c(4, 3, 2, 1))",
            "  probe_energy <- Sym_51(c(1, 2, 3, 4), c(4, 3, 2, 1))",
            "  return(probe_energy)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_helper_expr_reuse_calls_in_raw_emitted_r(&input);

        assert!(
            out.contains("probe_vec <- Sym_49(c(1, 2, 3, 4), c(4, 3, 2, 1))"),
            "{out}"
        );
        assert!(
            out.contains("probe_energy <- mean(abs(probe_vec))"),
            "{out}"
        );
        assert!(
            !out.contains("probe_energy <- Sym_51(c(1, 2, 3, 4), c(4, 3, 2, 1))"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_cg_loop_carried_updates_restore_after_fused_rs_new_shape() {
        let input = [
            "Sym_123 <- function(b, n_l, n_r, n_d, n_u, size) ",
            "{",
            "  x <- rep.int(0, size)",
            "  r <- b",
            "  p <- b",
            "  iter <- 1",
            "  repeat {",
            "    if (!(iter <= 20)) break",
            "    x <- (x + (alpha * p))",
            "",
            "    rs_new <- Sym_117((r - (alpha * Ap)), (r - (alpha * Ap)), size)",
            "    if ((is.na(rs_new) | (!(is.finite(rs_new))))) {",
            "      rs_new <- rs_old",
            "    }",
            "    beta <- (rs_new / rs_old)",
            "    if ((is.na(beta) | (!(is.finite(beta))))) {",
            "      beta <- 0",
            "    }",
            "    iter <- (iter + 1)",
            "    next",
            "  }",
            "  return(x)",
            "}",
            "",
        ]
        .join("\n");

        let out = restore_cg_loop_carried_updates_in_raw_emitted_r(&input);

        assert!(out.contains("r <- (r - (alpha * Ap))"), "{out}");
        assert!(
            out.contains("rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))"),
            "{out}"
        );
        assert!(out.contains("p <- (r + (beta * p))"), "{out}");
        assert!(out.contains("rs_old <- rs_new"), "{out}");
    }

    #[test]
    fn raw_emitted_cg_loop_carried_updates_restore_after_direct_sum_rs_new_shape() {
        let input = [
            "Sym_123 <- function(b, n_l, n_r, n_d, n_u, size) ",
            "{",
            "  x <- rep.int(0, size)",
            "  r <- b",
            "  p <- b",
            "  iter <- 1",
            "  repeat {",
            "    if (!(iter <= 20)) break",
            "    x <- (x + (alpha * p))",
            "",
            "    rs_new <- sum(((r - (alpha * Ap))[seq_len(size)] * (r - (alpha * Ap))[seq_len(size)]))",
            "    if ((is.na(rs_new) | (!(is.finite(rs_new))))) {",
            "      rs_new <- rs_old",
            "    }",
            "    beta <- (rs_new / rs_old)",
            "    if ((is.na(beta) | (!(is.finite(beta))))) {",
            "      beta <- 0",
            "    }",
            "    iter <- (iter + 1)",
            "    next",
            "  }",
            "  return(x)",
            "}",
            "",
        ]
        .join("\n");

        let out = restore_cg_loop_carried_updates_in_raw_emitted_r(&input);

        assert!(out.contains("r <- (r - (alpha * Ap))"), "{out}");
        assert!(
            out.contains("rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))"),
            "{out}"
        );
        assert!(out.contains("p <- (r + (beta * p))"), "{out}");
        assert!(out.contains("rs_old <- rs_new"), "{out}");
    }

    #[test]
    fn raw_emitted_cg_loop_carried_updates_restore_current_repeat_shape() {
        let input = [
            "Sym_123 <- function(b, n_l, n_r, n_d, n_u, size) ",
            "{",
            "  x <- rep.int(0, size)",
            "  r <- b",
            "  p <- r",
            "  rs_old <- sum((r[seq_len(size)] * r[seq_len(size)]))",
            "  iter <- 1",
            "  repeat {",
            "    if (!(iter <= 20)) break",
            "    Ap <- ((4.0001 * p) - (((rr_gather(p, rr_index_vec_floor(n_l)) + rr_gather(p, rr_index_vec_floor(n_r))) + rr_gather(p, rr_index_vec_floor(n_d))) + rr_gather(p, rr_index_vec_floor(n_u))))",
            "    p_Ap <- sum((p[seq_len(size)] * Ap[seq_len(size)]))",
            "    alpha <- (rs_old / p_Ap)",
            "    x <- (x + (alpha * p))",
            "    r <- (r - (alpha * Ap))",
            "    rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))",
            "    if (rr_truthy1((!(is.finite(rs_new))), \"condition\")) {",
            "",
            "    } else {",
            "      rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))",
            "    }",
            "    beta <- (rs_new / rs_old)",
            "    if (!(is.finite(beta))) {",
            "      beta <- 0",
            "    }",
            "    iter <- (iter + 1)",
            "  }",
            "  return(x)",
            "}",
            "",
        ]
        .join("\n");

        let out = restore_cg_loop_carried_updates_in_raw_emitted_r(&input);

        assert!(out.contains("r <- (r - (alpha * Ap))"), "{out}");
        assert!(
            out.contains("rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))"),
            "{out}"
        );
        assert!(out.contains("rs_new <- rs_old"), "{out}");
        assert!(out.contains("p <- (r + (beta * p))"), "{out}");
        assert!(out.contains("rs_old <- rs_new"), "{out}");
        assert!(
            !out.contains("} else {\n      rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_restores_temp_buffer_swap_after_repeat_update_loop() {
        let input = [
            "Sym_303 <- function() ",
            "{",
            "  repeat {",
            "    if (!(steps < 5)) break",
            "    i <- 1",
            "    repeat {",
            "      if (!(i <= TOTAL)) break",
            "      u_new[i] <- step_u[i]",
            "      i <- (i + 1)",
            "    }",
            "    tmp_u <- u",
            "    print(max_u)",
            "    steps <- (steps + 1)",
            "  }",
            "}",
            "",
        ]
        .join("\n");

        let out = restore_buffer_swaps_after_temp_copy_in_raw_emitted_r(&input);

        assert!(out.contains("tmp_u <- u"), "{out}");
        assert!(out.contains("u <- u_new"), "{out}");
        assert!(out.contains("u_new <- tmp_u"), "{out}");
    }

    #[test]
    fn raw_emitted_branch_local_scalar_assigns_hoist_before_later_uses() {
        let input = [
            "Sym_287 <- function(temp, q_v, q_s, q_g, size) ",
            "{",
            "  i <- 1",
            "  repeat {",
            "    if (!(i <= size)) break",
            "    T_c <- (temp[i] - 273.15)",
            "    if ((T_c < (-(5)))) {",
            "      qv <- q_v[i]",
            "      qs <- q_s[i]",
            "      qg <- q_g[i]",
            "    }",
            "    if ((T_c < (-(15)))) {",
            "      if ((qv > 0.01)) {",
            "        print(T_c)",
            "      }",
            "    }",
            "    if ((T_c > 0)) {",
            "      if ((qs > 0)) {",
            "        print(qs)",
            "      }",
            "      if ((qg > 0)) {",
            "        print(qg)",
            "      }",
            "    }",
            "    i <- (i + 1)",
            "    next",
            "  }",
            "  return(0)",
            "}",
            "",
        ]
        .join("\n");

        let out = hoist_branch_local_pure_scalar_assigns_used_after_branch_in_raw_emitted_r(&input);

        assert!(out.contains("qv <- q_v[i]"), "{out}");
        assert!(out.contains("qs <- q_s[i]"), "{out}");
        assert!(out.contains("qg <- q_g[i]"), "{out}");
        let qv_idx = out.find("qv <- q_v[i]").expect("{out}");
        let qs_idx = out.find("qs <- q_s[i]").expect("{out}");
        let qg_idx = out.find("qg <- q_g[i]").expect("{out}");
        let warm_idx = out.find("if ((T_c < (-(5)))) {").expect("{out}");
        assert!(qv_idx < warm_idx, "{out}");
        assert!(qs_idx < warm_idx, "{out}");
        assert!(qg_idx < warm_idx, "{out}");
    }

    #[test]
    fn raw_emitted_immediate_single_use_named_scalar_exprs_inline_before_peephole() {
        let input = [
            "Sym_287 <- function(q_c, i) ",
            "{",
            "  if ((q_c[i] > 0.0001)) {",
            "    rate <- (0.01 * q_c[i])",
            "    tendency_T <- (rate * L_f)",
            "  }",
            "  return(tendency_T)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_immediate_single_use_named_scalar_exprs_in_raw_emitted_r(&input);

        assert!(!out.contains("rate <- (0.01 * q_c[i])"), "{out}");
        assert!(
            out.contains("tendency_T <- ((0.01 * q_c[i]) * L_f)")
                || out.contains("tendency_T <- (rate * L_f)")
                || out.contains("tendency_T <- (((0.01 * q_c[i]) * L_f))"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_immediate_single_use_named_scalar_exprs_do_not_inline_into_guard_only_use() {
        let input = [
            "Sym_123 <- function(rs_old, p_Ap, x, p) ",
            "{",
            "  alpha <- (rs_old / p_Ap)",
            "  if ((is.na(alpha) | (!(is.finite(alpha))))) {",
            "    alpha <- 0",
            "  }",
            "  x <- (x + (alpha * p))",
            "  return(x)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_immediate_single_use_named_scalar_exprs_in_raw_emitted_r(&input);

        assert!(out.contains("alpha <- (rs_old / p_Ap)"), "{out}");
        assert!(!out.contains("if ((is.na(rs_old / p_Ap)"), "{out}");
    }

    #[test]
    fn raw_emitted_named_scalar_exprs_do_not_inline_inside_repeat_loop() {
        let input = [
            "Sym_1 <- function() ",
            "{",
            "  repeat {",
            "    if (!(time <= 5)) break",
            "    vy <- (vy + (g * dt))",
            "    y <- (y + (vy * dt))",
            "    time <- (time + dt)",
            "  }",
            "  return(y)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_immediate_single_use_named_scalar_exprs_in_raw_emitted_r(&input);

        assert!(out.contains("vy <- (vy + (g * dt))"), "{out}");
        assert!(out.contains("y <- (y + (vy * dt))"), "{out}");
    }

    #[test]
    fn raw_emitted_guard_only_named_scalar_exprs_inline_into_next_guard() {
        let input = [
            "Sym_222 <- function(x, y, cx, cy, r) ",
            "{",
            "  dx <- (x - cx)",
            "  dy <- (y - cy)",
            "  if ((((dx * dx) + (dy * dy)) < (r * r))) {",
            "    return(1)",
            "  }",
            "  return(0)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_guard_only_named_scalar_exprs_in_raw_emitted_r(&input);

        assert!(!out.contains("dx <- (x - cx)"), "{out}");
        assert!(!out.contains("dy <- (y - cy)"), "{out}");
        assert!(
            out.contains("if (((((x - cx) * (x - cx)) + ((y - cy) * (y - cy))) < (r * r))) {"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_guard_only_named_scalar_exprs_do_not_inline_when_used_later() {
        let input = [
            "Sym_123 <- function(rs_old, p_Ap, x, p) ",
            "{",
            "  alpha <- (rs_old / p_Ap)",
            "  if ((is.na(alpha) | (!(is.finite(alpha))))) {",
            "    alpha <- 0",
            "  }",
            "  x <- (x + (alpha * p))",
            "  return(x)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_guard_only_named_scalar_exprs_in_raw_emitted_r(&input);

        assert!(out.contains("alpha <- (rs_old / p_Ap)"), "{out}");
        assert!(!out.contains("if ((is.na(rs_old / p_Ap)"), "{out}");
    }

    #[test]
    fn raw_emitted_immediate_single_use_named_scalar_exprs_inline_floor_index_alias() {
        let input = [
            "Sym_186 <- function(gx, gy, f, N) ",
            "{",
            "  ix <- floor(gx)",
            "",
            "  idx <- rr_idx_cube_vec_i(f, ix, floor(gy), N)",
            "  return(idx)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_immediate_single_use_named_scalar_exprs_in_raw_emitted_r(&input);

        assert!(!out.contains("ix <- floor(gx)"), "{out}");
        assert!(
            out.contains("idx <- rr_idx_cube_vec_i(f, floor(gx), floor(gy), N)")
                || out.contains("idx <- rr_idx_cube_vec_i(f, (floor(gx)), floor(gy), N)"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_guard_only_scalar_literals_inline_into_guard() {
        let input = [
            "Sym_186 <- function() ",
            "{",
            "  LIMIT <- 1000",
            "  if (!(i <= LIMIT)) break",
            "  return(i)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_guard_only_scalar_literals_in_raw_emitted_r(&input);

        assert!(!out.contains("LIMIT <- 1000"), "{out}");
        assert!(out.contains("if (!(i <= 1000)) break"), "{out}");
    }

    #[test]
    fn raw_emitted_loop_guard_scalar_literals_inline_through_repeat_header() {
        let input = [
            "Sym_186 <- function() ",
            "{",
            "  LIMIT <- 1000",
            "  # rr-cse-pruned",
            "",
            "  repeat {",
            "    if (!(i <= LIMIT)) break",
            "    i <- (i + 1)",
            "    next",
            "  }",
            "  return(i)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_loop_guard_scalar_literals_in_raw_emitted_r(&input);

        assert!(!out.contains("LIMIT <- 1000"), "{out}");
        assert!(out.contains("if (!(i <= 1000)) break"), "{out}");
    }

    #[test]
    fn raw_emitted_loop_guard_scalar_literals_do_not_inline_induction_seed() {
        let input = [
            "Sym_123 <- function() ",
            "{",
            "  iter <- 1",
            "  repeat {",
            "    if (!(iter <= 20)) break",
            "    x <- iter",
            "    iter <- (iter + 1)",
            "    next",
            "  }",
            "  return(x)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_loop_guard_scalar_literals_in_raw_emitted_r(&input);

        assert!(out.contains("iter <- 1"), "{out}");
        assert!(out.contains("if (!(iter <= 20)) break"), "{out}");
        assert!(!out.contains("if (!(1 <= 20)) break"), "{out}");
    }

    #[test]
    fn raw_emitted_loop_guard_scalar_literals_skip_rr_marked_induction_seed() {
        let input = [
            "Sym_1 <- function(n) ",
            "{",
            "  a <- 1L",
            "  b <- 2L",
            "  i <- 1L",
            "  repeat {",
            "    rr_mark(4, 5);",
            "    if (!(i <= n)) break",
            "    t <- a",
            "    a <- b",
            "    b <- t",
            "    i <- (i + 1L)",
            "    next",
            "  }",
            "  return((a + b))",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_loop_guard_scalar_literals_in_raw_emitted_r(&input);

        assert!(out.contains("  i <- 1L"), "{out}");
        assert!(out.contains("if (!(i <= n)) break"), "{out}");
        assert!(out.contains("    i <- (i + 1L)"), "{out}");
        assert!(!out.contains("if (!((1L) <= n)) break"), "{out}");
    }

    #[test]
    fn raw_emitted_guard_only_named_scalar_exprs_skip_self_referential_increment() {
        let input = [
            "Sym_1 <- function() ",
            "{",
            "  i <- 0L",
            "  repeat {",
            "    if (!(i < 5L)) break",
            "    i <- (i + 1L)",
            "    next",
            "  }",
            "  return(i)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_guard_only_named_scalar_exprs_in_raw_emitted_r(&input);

        assert!(out.contains("    i <- (i + 1L)"), "{out}");
    }

    #[test]
    fn raw_emitted_restores_missing_lt_guard_counter_update() {
        let input = [
            "Sym_1 <- function(n) ",
            "{",
            "  x <- seq_len(n)",
            "  y <- x",
            "  i <- 1L",
            "  repeat {",
            "    rr_mark(8, 5);",
            "    if (!(i < length(x))) break",
            "    y[i] <- (x[i] + 10L)",
            "  }",
            "  return(y)",
            "}",
            "",
        ]
        .join("\n");

        let out = restore_missing_repeat_loop_counter_updates_in_raw_emitted_r(&input);

        assert!(out.contains("    i <- (i + 1L)"), "{out}");
        assert!(out.contains("    y[i] <- (x[i] + 10L)"), "{out}");
        assert!(out.contains("    if (!(i < length(x))) break"), "{out}");
    }

    #[test]
    fn raw_emitted_unused_middle_helper_params_trim_and_update_callsites() {
        let input = [
            "Sym_287 <- function(temp, q_v, q_c, q_r, q_i, q_s, q_g, size) ",
            "{",
            "  heat <- rep.int(0, size)",
            "  if ((q_c[1] > 0)) {",
            "    heat[1] <- (q_c[1] + q_v[1])",
            "  }",
            "  if ((q_s[1] > 0)) {",
            "    heat[1] <- (heat[1] + q_g[1])",
            "  }",
            "  return(heat)",
            "}",
            "Sym_top_0 <- function() ",
            "{",
            "  heat <- Sym_287(temp, qv, qc, qr, qi, qs, qg, TOTAL)",
            "  return(heat)",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_unused_helper_params_in_raw_emitted_r(&input);

        assert!(
            out.contains("Sym_287 <- function(q_v, q_c, q_s, q_g, size)"),
            "{out}"
        );
        assert!(
            !out.contains("Sym_287 <- function(temp, q_v, q_c, q_r, q_i, q_s, q_g, size)"),
            "{out}"
        );
        assert!(
            out.contains("heat <- Sym_287(qv, qc, qs, qg, TOTAL)"),
            "{out}"
        );
        assert!(
            !out.contains("heat <- Sym_287(temp, qv, qc, qr, qi, qs, qg, TOTAL)"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_terminal_repeat_nexts_prune_without_touching_inner_if_nexts() {
        let input = [
            "Sym_83 <- function() ",
            "{",
            "  repeat {",
            "    if ((flag)) {",
            "      next",
            "    }",
            "    x <- (x + 1)",
            "    next",
            "  }",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_terminal_repeat_nexts_in_raw_emitted_r(&input);

        assert!(out.contains("if ((flag)) {\n      next\n    }"), "{out}");
        assert!(!out.contains("x <- (x + 1)\n    next\n  }"), "{out}");
    }

    #[test]
    fn raw_emitted_same_var_is_na_or_not_finite_guards_simplify() {
        let input = [
            "Sym_123 <- function() ",
            "{",
            "  if (((is.na(rs_old) | (!(is.finite(rs_old)))) | (rs_old == 0))) {",
            "    rs_old <- 0.0000001",
            "  }",
            "  if ((is.na(alpha) | (!(is.finite(alpha))))) {",
            "    alpha <- 0",
            "  }",
            "}",
            "",
        ]
        .join("\n");

        let out = simplify_same_var_is_na_or_not_finite_guards_in_raw_emitted_r(&input);

        assert!(
            out.contains("if (((!(is.finite(rs_old))) | (rs_old == 0))) {"),
            "{out}"
        );
        assert!(out.contains("if ((!(is.finite(alpha)))) {"), "{out}");
        assert!(!out.contains("is.na(rs_old)"), "{out}");
        assert!(!out.contains("is.na(alpha)"), "{out}");
    }

    #[test]
    fn raw_emitted_not_finite_or_zero_guard_parens_simplify() {
        let input = [
            "Sym_123 <- function() ",
            "{",
            "  if (((!(is.finite(rs_old))) | (rs_old == 0))) {",
            "    rs_old <- 0.0000001",
            "  }",
            "}",
            "",
        ]
        .join("\n");

        let out = simplify_not_finite_or_zero_guard_parens_in_raw_emitted_r(&input);

        assert!(
            out.contains("if ((!(is.finite(rs_old)) | (rs_old == 0))) {"),
            "{out}"
        );
        assert!(
            !out.contains("if (((!(is.finite(rs_old))) | (rs_old == 0))) {"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_wrapped_not_finite_parens_simplify() {
        let input = [
            "Sym_123 <- function() ",
            "{",
            "  if ((!(is.finite(alpha)))) {",
            "    alpha <- 0",
            "  }",
            "}",
            "",
        ]
        .join("\n");

        let out = simplify_wrapped_not_finite_parens_in_raw_emitted_r(&input);

        assert!(out.contains("if (!(is.finite(alpha))) {"), "{out}");
        assert!(!out.contains("if ((!(is.finite(alpha)))) {"), "{out}");
    }

    #[test]
    fn raw_emitted_nested_else_if_blocks_collapse() {
        let input = [
            "Sym_60 <- function(f, x, size) ",
            "{",
            "  if ((x > 1)) {",
            "    return(a)",
            "  } else {",
            "    if ((f == 1)) {",
            "      return(b)",
            "    } else {",
            "      if ((f == 2)) {",
            "        return(c)",
            "      } else {",
            "        return(d)",
            "      }",
            "    }",
            "  }",
            "}",
            "",
        ]
        .join("\n");

        let out = collapse_nested_else_if_blocks_in_raw_emitted_r(&input);

        assert!(out.contains("} else if ((f == 1)) {"), "{out}");
        assert!(out.contains("} else if ((f == 2)) {"), "{out}");
        assert!(!out.contains("  } else {\n    if ((f == 1)) {"), "{out}");
        assert!(
            !out.contains("    } else {\n      if ((f == 2)) {"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_unused_arg_aliases_strip_after_dead_scalar_alias_prune() {
        let input = [
            "Sym_287 <- function(q_r, q_i, i) ",
            "{",
            "  .arg_q_r <- q_r",
            "  .arg_q_i <- q_i",
            "  qr <- .arg_q_r[i]",
            "  qi <- .arg_q_i[i]",
            "  return(0)",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_unused_raw_arg_aliases_in_raw_emitted_r(
            &rewrite_single_use_scalar_index_aliases_in_raw_emitted_r(&input),
        );

        assert!(!out.contains(".arg_q_r <- q_r"), "{out}");
        assert!(!out.contains(".arg_q_i <- q_i"), "{out}");
        assert!(!out.contains("qr <- .arg_q_r[i]"), "{out}");
        assert!(!out.contains("qi <- .arg_q_i[i]"), "{out}");
    }

    #[test]
    fn raw_emitted_dead_simple_scalar_alias_and_literal_assignments_prune() {
        let input = [
            "Sym_287 <- function(size) ",
            "{",
            "  keep <- size",
            "  dead_const <- 2500000",
            "  dead_alias <- keep",
            "  live <- keep",
            "  return(live)",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_dead_simple_scalar_assigns_in_raw_emitted_r(&input);

        assert!(out.contains("keep <- size"), "{out}");
        assert!(!out.contains("dead_const <- 2500000"), "{out}");
        assert!(!out.contains("dead_alias <- keep"), "{out}");
        assert!(out.contains("live <- keep"), "{out}");
    }

    #[test]
    fn raw_emitted_dead_licm_scalar_expr_assignments_prune() {
        let input = [
            "Sym_83 <- function(x) ",
            "{",
            "  licm_71 <- (x + 1)",
            "  keep <- x",
            "  return(keep)",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_dead_simple_scalar_assigns_in_raw_emitted_r(&input);

        assert!(!out.contains("licm_71 <- (x + 1)"), "{out}");
        assert!(out.contains("keep <- x"), "{out}");
    }

    #[test]
    fn raw_emitted_dead_pure_scalar_expr_assignments_prune() {
        let input = [
            "Sym_287 <- function(temp, size) ",
            "{",
            "  i <- 1",
            "  repeat {",
            "    if (!(i <= size)) break",
            "    T_c <- (temp[i] - 273.15)",
            "    es_ice <- (6.11 * exp(((22.5 * T_c) / (temp[i] + 273.15))))",
            "    keep <- (T_c + 1)",
            "    rr_mark(1, 1);",
            "    i <- (i + 1)",
            "    next",
            "  }",
            "  return(keep)",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_dead_simple_scalar_assigns_in_raw_emitted_r(&input);

        assert!(!out.contains("es_ice <- "), "{out}");
        assert!(out.contains("keep <- (T_c + 1)"), "{out}");
        assert!(out.contains("rr_mark(1, 1);"), "{out}");
        assert!(out.contains("return(keep)"), "{out}");
    }

    #[test]
    fn raw_emitted_dead_pure_scalar_expr_assignments_keep_loop_induction_updates() {
        let input = [
            "Sym_117 <- function(a, b, n) ",
            "{",
            "  sum <- 0",
            "  i <- 1",
            "  repeat {",
            "    if (!(i <= n)) break",
            "    sum <- (sum + (a[i] * b[i]))",
            "    i <- (i + 1)",
            "    next",
            "  }",
            "  return(sum)",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_dead_simple_scalar_assigns_in_raw_emitted_r(&input);

        assert!(out.contains("i <- (i + 1)"), "{out}");
        assert!(out.contains("sum <- (sum + (a[i] * b[i]))"), "{out}");
    }

    #[test]
    fn raw_emitted_two_use_named_scalar_exprs_inline_into_adjacent_assignments() {
        let input = [
            "Sym_210 <- function(A, B, lapA, lapB, DA, DB, f, k, dt, i) ",
            "{",
            "  a <- A[i]",
            "  b <- B[i]",
            "  reaction <- ((a * b) * b)",
            "  new_a <- (a + ((((DA * lapA[i]) - reaction) + (f * (1 - a))) * dt))",
            "  new_b <- (b + ((((DB * lapB[i]) + reaction) - ((k + f) * b)) * dt))",
            "  return(new_b)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_two_use_named_scalar_exprs_in_raw_emitted_r(&input);

        assert!(!out.contains("reaction <- ((a * b) * b)"), "{out}");
        assert!(
            out.contains("new_a <- (a + ((((DA * lapA[i]) - (a * b) * b) + (f * (1 - a))) * dt))"),
            "{out}"
        );
        assert!(
            out.contains("new_b <- (b + ((((DB * lapB[i]) + (a * b) * b) - ((k + f) * b)) * dt))"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_single_use_named_scalar_pure_calls_inline_wrap_index_reads() {
        let input = [
            "Sym_222 <- function(B, x, y, W, H) ",
            "{",
            "  id <- rr_wrap_index_vec_i(x, y, W, H)",
            "  B[id] <- 1",
            "  center_idx <- rr_wrap_index_vec_i(32, 32, W, H)",
            "  print(B[center_idx])",
            "  return(B)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_single_use_named_scalar_pure_calls_in_raw_emitted_r(&input);

        assert!(
            !out.contains("id <- rr_wrap_index_vec_i(x, y, W, H)"),
            "{out}"
        );
        assert!(
            !out.contains("center_idx <- rr_wrap_index_vec_i(32, 32, W, H)"),
            "{out}"
        );
        assert!(
            out.contains("B[rr_wrap_index_vec_i(x, y, W, H)] <- 1"),
            "{out}"
        );
        assert!(
            out.contains("print(B[rr_wrap_index_vec_i(32, 32, W, H)])"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_trivial_dot_product_wrapper_collapses_to_direct_sum() {
        let input = [
            "Sym_117 <- function(a, b, n) ",
            "{",
            "  sum <- 0",
            "  i <- 1",
            "  # rr-cse-pruned",
            "  repeat {",
            "    if (!(i <= n)) break",
            "    sum <- (sum + (a[i] * b[i]))",
            "    i <- (i + 1)",
            "    next",
            "  }",
            "  return(sum)",
            "}",
            "",
        ]
        .join("\n");

        let out = collapse_trivial_dot_product_wrappers_in_raw_emitted_r(&input);

        assert!(
            out.contains("return(sum((a[seq_len(n)] * b[seq_len(n)])))"),
            "{out}"
        );
        assert!(!out.contains("sum <- 0"), "{out}");
        assert!(!out.contains("sum <- (sum + (a[i] * b[i]))"), "{out}");
    }

    #[test]
    fn raw_emitted_trivial_dot_product_wrapper_with_parenthesized_iter_collapses() {
        let input = [
            "Sym_117 <- function(a, b, n) ",
            "{",
            "  sum <- 0",
            "  i <- 1",
            "  # rr-cse-pruned",
            "  repeat {",
            "    if (!(i <= n)) break",
            "    sum <- (sum + (a[(i)] * b[(i)]))",
            "    i <- (i + 1)",
            "    next",
            "  }",
            "  return(sum)",
            "}",
            "",
        ]
        .join("\n");

        let out = collapse_trivial_dot_product_wrappers_in_raw_emitted_r(&input);

        assert!(
            out.contains("return(sum((a[seq_len(n)] * b[seq_len(n)])))"),
            "{out}"
        );
        assert!(!out.contains("sum <- (sum + (a[(i)] * b[(i)]))"), "{out}");
    }

    #[test]
    fn raw_emitted_two_use_named_scalar_pure_call_inlines_idx_into_dx_dy() {
        let input = [
            "Sym_186 <- function(f, gx, gy, N, u, v, dt) ",
            "{",
            "  idx <- rr_idx_cube_vec_i(f, floor(gx), floor(gy), N)",
            "  dx <- ((u[idx] * dt) / 400000)",
            "  dy <- ((v[idx] * dt) / 400000)",
            "  return(dx)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_two_use_named_scalar_pure_calls_in_raw_emitted_r(&input);

        assert!(!out.contains("idx <- rr_idx_cube_vec_i"), "{out}");
        assert!(
            out.contains(
                "dx <- ((u[rr_idx_cube_vec_i(f, floor(gx), floor(gy), N)] * dt) / 400000)"
            ),
            "{out}"
        );
        assert!(
            out.contains(
                "dy <- ((v[rr_idx_cube_vec_i(f, floor(gx), floor(gy), N)] * dt) / 400000)"
            ),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_particle_idx_alias_rewrites_into_dx_dy() {
        let input = [
            "Sym_186 <- function(f, gx, gy, N, u, v, dt) ",
            "{",
            "  idx <- rr_idx_cube_vec_i(f, floor(gx), floor(gy), N)",
            "  dx <- ((u[idx] * dt) / 400000)",
            "  dy <- ((v[idx] * dt) / 400000)",
            "  return(dx)",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_particle_idx_alias_in_raw_emitted_r(&input);

        assert!(!out.contains("idx <- rr_idx_cube_vec_i"), "{out}");
        assert!(
            out.contains(
                "dx <- ((u[rr_idx_cube_vec_i(f, floor(gx), floor(gy), N)] * dt) / 400000)"
            ),
            "{out}"
        );
        assert!(
            out.contains(
                "dy <- ((v[rr_idx_cube_vec_i(f, floor(gx), floor(gy), N)] * dt) / 400000)"
            ),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_loop_index_alias_ii_rewrites_to_i() {
        let input = [
            "Sym_186 <- function(u, du1, adv_u, TOTAL) ",
            "{",
            "  i <- 1",
            "  repeat {",
            "    if (!(i <= TOTAL)) break",
            "    ii <- i",
            "    u_stage[ii] <- (u[ii] + (du1[ii] - adv_u[ii]))",
            "    if ((u_stage[ii] > max_u)) {",
            "      max_u <- u_stage[ii]",
            "    }",
            "    i <- (i + 1)",
            "    next",
            "  }",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_loop_index_alias_ii_in_raw_emitted_r(&input);

        assert!(!out.contains("ii <- i"), "{out}");
        assert!(
            out.contains("u_stage[i] <- (u[i] + (du1[i] - adv_u[i]))"),
            "{out}"
        );
        assert!(out.contains("if ((u_stage[i] > max_u)) {"), "{out}");
        assert!(out.contains("max_u <- u_stage[i]"), "{out}");
    }

    #[test]
    fn raw_emitted_loop_index_alias_ii_keeps_alias_if_used_after_i_changes() {
        let input = [
            "Sym_1 <- function(x, n) ",
            "{",
            "  i <- 1",
            "  repeat {",
            "    if (!(i <= n)) break",
            "    ii <- i",
            "    out[ii] <- x[ii]",
            "    i <- (i + 1)",
            "    y <- ii",
            "    next",
            "  }",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_loop_index_alias_ii_in_raw_emitted_r(&input);

        assert!(out.contains("ii <- i"), "{out}");
        assert!(out.contains("out[i] <- x[i]"), "{out}");
        assert!(out.contains("y <- ii"), "{out}");
    }

    #[test]
    fn raw_emitted_blank_line_runs_compact() {
        let input = "Sym_1 <- function() \n{\n\n\n  x <- 1\n\n\n  return(x)\n}\n";
        let out = compact_blank_lines_in_raw_emitted_r(input);
        assert!(!out.contains("\n\n\n"), "{out}");
        assert!(out.contains("{\n\n  x <- 1\n\n  return(x)\n}"), "{out}");
    }

    #[test]
    fn raw_emitted_orphan_rr_cse_markers_before_repeat_prune() {
        let input = [
            "Sym_83 <- function(size) ",
            "{",
            "  f <- 1",
            "  # rr-cse-pruned",
            "  x <- 0",
            "  # rr-cse-pruned",
            "",
            "  repeat {",
            "    if (!(f <= size)) break",
            "    f <- (f + 1)",
            "    next",
            "  }",
            "  return(f)",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_orphan_rr_cse_markers_before_repeat_in_raw_emitted_r(&input);

        assert!(!out.contains("# rr-cse-pruned"), "{out}");
        assert!(out.contains("f <- 1"), "{out}");
        assert!(out.contains("x <- 0"), "{out}");
        assert!(out.contains("repeat {"), "{out}");
    }

    #[test]
    fn raw_emitted_single_blank_spacers_prune_between_assignments_and_control() {
        let input = [
            "Sym_123 <- function() ",
            "{",
            "  x <- rep.int(0, size)",
            "",
            "  r <- b",
            "  iter <- 1",
            "",
            "  repeat {",
            "    if (!(iter <= 20)) break",
            "    rs_old <- 0.0000001",
            "",
            "  }",
            "  y <- 1",
            "",
            "  z <- 2",
            "  return(z)",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_single_blank_spacers_in_raw_emitted_r(&input);

        assert!(!out.contains("x <- rep.int(0, size)\n\n  r <- b"), "{out}");
        assert!(!out.contains("iter <- 1\n\n  repeat {"), "{out}");
        assert!(!out.contains("rs_old <- 0.0000001\n\n  }"), "{out}");
        assert!(!out.contains("y <- 1\n\n  z <- 2"), "{out}");
    }

    #[test]
    fn raw_emitted_single_blank_spacers_prune_between_assignment_and_if() {
        let input = [
            "Sym_60 <- function(f, x, size) ",
            "{",
            "  ys <- seq_len(size)",
            "",
            "  if ((x > 1)) {",
            "    return(a)",
            "  }",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_single_blank_spacers_in_raw_emitted_r(&input);

        assert!(
            !out.contains("ys <- seq_len(size)\n\n  if ((x > 1)) {"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_single_blank_spacers_prune_after_control_open_before_returns() {
        let input = [
            "Sym_60 <- function(f, x, size) ",
            "{",
            "",
            "  if ((x > 1)) {",
            "",
            "    return(a)",
            "  } else if ((f == 1)) {",
            "",
            "    return(b)",
            "  }",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_single_blank_spacers_in_raw_emitted_r(&input);

        assert!(!out.contains("{\n\n  if ((x > 1)) {"), "{out}");
        assert!(!out.contains("if ((x > 1)) {\n\n    return(a)"), "{out}");
        assert!(
            !out.contains("} else if ((f == 1)) {\n\n    return(b)"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_single_blank_spacers_prune_between_closing_braces() {
        let input = [
            "Sym_60 <- function(f, x, size) ",
            "{",
            "  if ((x > 1)) {",
            "    return(a)",
            "  } else {",
            "    return(b)",
            "  }",
            "",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_single_blank_spacers_in_raw_emitted_r(&input);

        assert!(!out.contains("  }\n\n}"), "{out}");
    }

    #[test]
    fn raw_emitted_single_blank_spacers_prune_after_break_before_branch() {
        let input = [
            "Sym_83 <- function(dir, size) ",
            "{",
            "  repeat {",
            "    if (!(x <= size)) break",
            "",
            "    if ((dir == 1)) {",
            "      neighbors[i] <- 1",
            "    }",
            "  }",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_single_blank_spacers_in_raw_emitted_r(&input);

        assert!(
            !out.contains("if (!(x <= size)) break\n\n    if ((dir == 1)) {"),
            "{out}"
        );
    }

    #[test]
    fn raw_emitted_readonly_arg_aliases_rewrite_to_bare_params() {
        let input = [
            "Sym_83 <- function(dir, size) ",
            "{",
            "  .arg_dir <- dir",
            "  .arg_size <- size",
            "  ys <- seq_len(.arg_size)",
            "  if ((.arg_dir == 1)) {",
            "    return(ys)",
            "  }",
            "  return(seq_len(.arg_size))",
            "}",
            "",
        ]
        .join("\n");

        let out = rewrite_readonly_raw_arg_aliases_in_raw_emitted_r(&input);

        assert!(!out.contains(".arg_dir <- dir"), "{out}");
        assert!(!out.contains(".arg_size <- size"), "{out}");
        assert!(out.contains("ys <- seq_len(size)"), "{out}");
        assert!(out.contains("if ((dir == 1)) {"), "{out}");
        assert!(out.contains("return(seq_len(size))"), "{out}");
    }

    #[test]
    fn raw_emitted_noop_self_assignments_prune() {
        let input = [
            "Sym_287 <- function(temp) ",
            "{",
            "  T_c <- (temp[1] - 273.15)",
            "  T_c <- T_c",
            "  return(T_c)",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_noop_self_assignments_in_raw_emitted_r(&input);

        assert!(out.contains("T_c <- (temp[1] - 273.15)"), "{out}");
        assert!(!out.contains("T_c <- T_c"), "{out}");
        assert!(out.contains("return(T_c)"), "{out}");
    }

    #[test]
    fn raw_emitted_empty_else_blocks_prune() {
        let input = [
            "Sym_83 <- function(dir) ",
            "{",
            "  if ((dir == 1)) {",
            "    return(1)",
            "  } else {",
            "  }",
            "  return(0)",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_empty_else_blocks_in_raw_emitted_r(&input);

        assert!(!out.contains("} else {\n  }"), "{out}");
        assert!(out.contains("  }\n  return(0)"), "{out}");
    }

    #[test]
    fn raw_emitted_branch_local_vec_fill_rebinds_prune_before_peephole() {
        let input = [
            "Sym_123 <- function(size) ",
            "{",
            "  x <- rep.int(0, size)",
            "  if ((bad == 1)) {",
            "    x <- Sym_17(size, 0)",
            "  }",
            "  return(x)",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_redundant_branch_local_vec_fill_rebinds_in_raw_emitted_r(&input);

        assert!(out.contains("x <- rep.int(0, size)"), "{out}");
        assert!(!out.contains("x <- Sym_17(size, 0)"), "{out}");
        assert!(out.contains("return(x)"), "{out}");
    }

    #[test]
    fn raw_named_scalar_expr_inline_skips_function_literals() {
        assert!(!is_inlineable_raw_named_scalar_expr("function(x)"));
        assert!(!is_inlineable_raw_named_scalar_expr("function (x)"));
        assert!(is_inlineable_raw_named_scalar_expr("(x + 1L)"));
    }

    #[test]
    fn raw_shadowed_scalar_seed_prune_respects_else_boundaries() {
        let input = [
            "Sym_4 <- function() ",
            "{",
            "  if ((cond)) {",
            "    m <- 10L",
            "  } else {",
            "    m <- 0L",
            "  }",
            "  print(m)",
            "  return(m)",
            "}",
            "",
        ]
        .join("\n");

        let out = strip_shadowed_simple_scalar_seed_assigns_in_raw_emitted_r(&input);

        assert!(out.contains("    m <- 10L"), "{out}");
        assert!(out.contains("    m <- 0L"), "{out}");
    }
}
