//! Compiler pipeline coordinator.
//!
//! This module owns driver-facing compile configuration, cache salts, and the
//! stable containers that carry lowered program state between major phases.
//! Heavy raw-emitted-R rewrites and phase-specific implementation details live
//! in sibling `pipeline/*` modules so this file can stay focused on orchestration.

use crate::codegen::mir_emit::MapEntry;
use crate::compiler::scheduler::{CompilerParallelConfig, CompilerScheduler};
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
use std::sync::{Arc, OnceLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

#[path = "pipeline/raw_rewrites.rs"]
mod raw_rewrites;
pub(crate) use raw_rewrites::*;

#[path = "pipeline/helper_raw_rewrites.rs"]
mod helper_raw_rewrites;
pub(crate) use helper_raw_rewrites::*;

#[path = "pipeline/scalar_raw_rewrites.rs"]
mod scalar_raw_rewrites;
#[cfg(test)]
pub(crate) use scalar_raw_rewrites::*;

#[path = "pipeline/cleanup_raw_rewrites.rs"]
mod cleanup_raw_rewrites;
pub(crate) use cleanup_raw_rewrites::*;

#[path = "pipeline/structural_raw_rewrites.rs"]
mod structural_raw_rewrites;
pub(crate) use structural_raw_rewrites::*;

#[path = "pipeline/raw_utils.rs"]
mod raw_utils;
pub(crate) use raw_utils::*;

#[path = "pipeline/function_props.rs"]
mod function_props;
pub(crate) use function_props::*;

#[path = "pipeline/compile_api.rs"]
mod compile_api;
pub use compile_api::*;

#[path = "pipeline/profile.rs"]
mod profile;
pub use profile::*;

#[path = "pipeline/loop_repairs.rs"]
mod loop_repairs;
pub(crate) use loop_repairs::*;

#[path = "pipeline/late_raw_rewrites.rs"]
mod late_raw_rewrites;
pub(crate) use late_raw_rewrites::*;

#[path = "pipeline/phases.rs"]
mod phases;
pub(crate) use phases::*;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CompileMode {
    #[default]
    Standard,
    FastDev,
}

impl CompileMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::FastDev => "fast-dev",
        }
    }

    pub fn disabled_pass_groups(self) -> &'static [&'static str] {
        match self {
            Self::Standard => &[],
            Self::FastDev => &["poly"],
        }
    }
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
    pub strict_let: bool,
    pub warn_implicit_decl: bool,
    pub compile_mode: CompileMode,
}

impl Default for CompileOutputOptions {
    fn default() -> Self {
        Self {
            inject_runtime: true,
            preserve_all_defs: false,
            strict_let: true,
            warn_implicit_decl: false,
            compile_mode: CompileMode::Standard,
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

pub fn default_type_config() -> TypeConfig {
    TypeConfig {
        mode: TypeMode::Strict,
        native_backend: NativeBackend::Off,
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

pub fn default_parallel_config() -> ParallelConfig {
    ParallelConfig {
        mode: ParallelMode::Off,
        backend: ParallelBackend::Auto,
        threads: 0,
        min_trip: 4096,
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

fn fn_ir_work_size(fn_ir: &crate::mir::def::FnIR) -> usize {
    fn_ir.values.len()
        + fn_ir.blocks.len()
        + fn_ir.blocks.iter().map(|bb| bb.instrs.len()).sum::<usize>()
}

fn emitted_segment_line_count(code: &str) -> u32 {
    if code.is_empty() {
        1
    } else {
        let base = code.lines().count() as u32;
        if code.ends_with('\n') { base + 1 } else { base }
    }
}

fn shifted_source_map_for_final_output_prefix(
    mut map: Vec<MapEntry>,
    final_code: &str,
) -> Vec<MapEntry> {
    const GENERATED_CODE_HEADER: &str = "# --- RR generated code (from user RR source) ---\n";
    if map.is_empty() {
        return map;
    }
    let Some(header_idx) = final_code.find(GENERATED_CODE_HEADER) else {
        return map;
    };
    let prefix = &final_code[..header_idx + GENERATED_CODE_HEADER.len()];
    let line_offset = prefix.bytes().filter(|b| *b == b'\n').count() as u32;
    for entry in &mut map {
        entry.r_line = entry.r_line.saturating_add(line_offset);
    }
    map
}

fn contains_generated_poly_loop_controls(code: &str) -> bool {
    code.contains(".__poly_gen_iv_")
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

pub(crate) trait EmitFunctionCache: Send + Sync {
    fn load(&self, key: &str) -> crate::error::RR<Option<(String, Vec<MapEntry>)>>;
    fn store(&self, key: &str, code: &str, map: &[MapEntry]) -> crate::error::RR<()>;

    fn load_raw_rewrite(&self, _key: &str) -> crate::error::RR<Option<String>> {
        Ok(None)
    }

    fn store_raw_rewrite(&self, _key: &str, _code: &str) -> crate::error::RR<()> {
        Ok(())
    }

    fn load_peephole(&self, _key: &str) -> crate::error::RR<Option<(String, Vec<u32>)>> {
        Ok(None)
    }

    fn store_peephole(&self, _key: &str, _code: &str, _line_map: &[u32]) -> crate::error::RR<()> {
        Ok(())
    }

    fn load_optimized_fragment(
        &self,
        _key: &str,
    ) -> crate::error::RR<Option<(String, Vec<MapEntry>)>> {
        Ok(None)
    }

    fn store_optimized_fragment(
        &self,
        _key: &str,
        _code: &str,
        _map: &[MapEntry],
    ) -> crate::error::RR<()> {
        Ok(())
    }

    fn load_optimized_assembly_artifact(
        &self,
        _key: &str,
    ) -> crate::error::RR<Option<(String, Vec<MapEntry>)>> {
        Ok(None)
    }

    fn store_optimized_assembly_artifact(
        &self,
        _key: &str,
        _code: &str,
        _map: &[MapEntry],
    ) -> crate::error::RR<()> {
        Ok(())
    }

    fn load_optimized_assembly_source_map(
        &self,
        _key: &str,
    ) -> crate::error::RR<Option<Vec<MapEntry>>> {
        Ok(None)
    }

    fn store_optimized_assembly_source_map(
        &self,
        _key: &str,
        _map: &[MapEntry],
    ) -> crate::error::RR<()> {
        Ok(())
    }

    fn has_optimized_assembly_safe(&self, _key: &str) -> crate::error::RR<bool> {
        Ok(false)
    }

    fn store_optimized_assembly_safe(&self, _key: &str) -> crate::error::RR<()> {
        Ok(())
    }

    fn has_optimized_raw_assembly_safe(&self, _key: &str) -> crate::error::RR<bool> {
        Ok(false)
    }

    fn store_optimized_raw_assembly_safe(&self, _key: &str) -> crate::error::RR<()> {
        Ok(())
    }

    fn has_optimized_peephole_assembly_safe(&self, _key: &str) -> crate::error::RR<bool> {
        Ok(false)
    }

    fn store_optimized_peephole_assembly_safe(&self, _key: &str) -> crate::error::RR<()> {
        Ok(())
    }
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
        ^ stable_hash_bytes(include_str!("../codegen/mod.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/mir_emit.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/backend/mod.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/backend/state.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/backend/setup.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/mod.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/assign.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/bindings.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/branches.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/cse.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/cse_prune.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/index.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/instr.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/render.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/resolve.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/rewrite.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/rewrite/poly_index.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/rewrite/literal_calls.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/rewrite/raw_text.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/rewrite/raw_text_helpers.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/rewrite/scalar_alias.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/rewrite/loop_alias.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/rewrite/duplicate_alias.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/rewrite/temp_seed.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/rewrite/final_cleanup.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/structured.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/structured_analysis.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../codegen/emit/control_flow.rs").as_bytes())
        ^ stable_hash_bytes(build_hash.as_bytes())
}

pub(crate) fn compile_output_cache_salt() -> u64 {
    fn_emit_cache_salt()
        ^ stable_hash_bytes(crate::runtime::R_RUNTIME.as_bytes())
        ^ stable_hash_bytes(include_str!("../runtime/subset.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("../runtime/source.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("r_peephole.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/raw_rewrites.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/helper_raw_rewrites.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/scalar_raw_rewrites.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/cleanup_raw_rewrites.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/structural_raw_rewrites.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/raw_utils.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/function_props.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/compile_api.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/phases.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/phases/source_emit.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/phases/tachyon_runtime.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/loop_repairs.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/late_raw_rewrites.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/late_raw_rewrites/buffer_swap.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/late_raw_rewrites/cg.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/late_raw_rewrites/clamp.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/late_raw_rewrites/melt_rate.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("pipeline/late_raw_rewrites/prune.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/mod.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/patterns.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/alias.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/helpers.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/helpers/cleanup.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/helpers/helper_calls.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/helpers/metric.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/dead_code.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/emitted_ir.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/emitted_ir/model.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/emitted_ir/cleanup.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/emitted_ir/passthrough.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/emitted_ir/helper_alias.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/emitted_ir/wrapper_cleanup.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/emitted_ir/wrapper_tail_cleanup.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/emitted_ir/exact_reuse.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/core_utils.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/expr_reuse.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/expr_reuse/temp_tail.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/expr_reuse/forward.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/facts.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/full_range.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/guard_simplify.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/inline_scalar.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/index_reads.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/late_pass.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/loop_restore.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/pipeline_impl.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/pipeline_stage.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/scalar_reuse.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/shadow_alias.rs").as_bytes())
        ^ stable_hash_bytes(include_str!("peephole/vector.rs").as_bytes())
}

pub(crate) fn peephole_output_cache_key(
    raw_output: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    preserve_all_defs: bool,
    compile_mode: CompileMode,
) -> String {
    let mut pure_names = pure_user_calls.iter().cloned().collect::<Vec<_>>();
    pure_names.sort();
    let mut fresh_names = fresh_user_calls.iter().cloned().collect::<Vec<_>>();
    fresh_names.sort();
    let payload = format!(
        "rr-peephole-v1|{}|{}|{}|{}|{:?}|{:?}|{}",
        compile_output_cache_salt(),
        direct_builtin_call_map,
        preserve_all_defs,
        compile_mode.as_str(),
        pure_names,
        fresh_names,
        raw_output,
    );
    format!("{:016x}", stable_hash_bytes(payload.as_bytes()))
}

pub(crate) fn optimized_fragment_output_cache_key(
    emitted_fragment: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    preserve_all_defs: bool,
    compile_mode: CompileMode,
) -> String {
    let mut pure_names = pure_user_calls.iter().cloned().collect::<Vec<_>>();
    pure_names.sort();
    let mut fresh_names = fresh_user_calls.iter().cloned().collect::<Vec<_>>();
    fresh_names.sort();
    let payload = format!(
        "rr-opt-frag-v1|{}|{}|{}|{}|{:?}|{:?}|{}",
        compile_output_cache_salt(),
        direct_builtin_call_map,
        preserve_all_defs,
        compile_mode.as_str(),
        pure_names,
        fresh_names,
        emitted_fragment,
    );
    format!("{:016x}", stable_hash_bytes(payload.as_bytes()))
}

pub(crate) fn optimized_assembly_cache_key(
    raw_output: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    preserve_all_defs: bool,
    compile_mode: CompileMode,
) -> String {
    let mut pure_names = pure_user_calls.iter().cloned().collect::<Vec<_>>();
    pure_names.sort();
    let mut fresh_names = fresh_user_calls.iter().cloned().collect::<Vec<_>>();
    fresh_names.sort();
    let payload = format!(
        "rr-opt-asm-v1|{}|{}|{}|{}|{:?}|{:?}|{}",
        compile_output_cache_salt(),
        direct_builtin_call_map,
        preserve_all_defs,
        compile_mode.as_str(),
        pure_names,
        fresh_names,
        raw_output,
    );
    format!("{:016x}", stable_hash_bytes(payload.as_bytes()))
}

pub(crate) fn raw_rewrite_output_cache_key(
    raw_output: &str,
    pure_user_calls: &FxHashSet<String>,
    preserve_all_defs: bool,
    compile_mode: CompileMode,
) -> String {
    let mut pure_names = pure_user_calls.iter().cloned().collect::<Vec<_>>();
    pure_names.sort();
    let payload = format!(
        "rr-raw-rewrite-v1|{}|{}|{}|{:?}|{}",
        compile_output_cache_salt(),
        preserve_all_defs,
        compile_mode.as_str(),
        pure_names,
        raw_output,
    );
    format!("{:016x}", stable_hash_bytes(payload.as_bytes()))
}

fn fn_emit_cache_key(
    fn_ir: &crate::mir::def::FnIR,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    compile_mode: CompileMode,
    seq_len_param_end_slots: Option<&FxHashMap<usize, usize>>,
) -> String {
    let mut seq_len_summary: Vec<(usize, usize)> = seq_len_param_end_slots
        .map(|slots| slots.iter().map(|(param, end)| (*param, *end)).collect())
        .unwrap_or_default();
    seq_len_summary.sort_unstable();
    let payload = format!(
        "rr-fn-emit-v4|{}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{:?}",
        fn_ir.name,
        opt_level.label(),
        type_cfg.mode.as_str(),
        type_cfg.native_backend.as_str(),
        parallel_cfg.mode.as_str(),
        parallel_cfg.backend.as_str(),
        parallel_cfg.threads,
        compile_mode.as_str(),
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

pub(crate) type FnSlot = usize;

pub(crate) struct FnUnit {
    name: String,
    ir: Option<crate::mir::def::FnIR>,
    is_public: bool,
    is_top_level: bool,
}

pub(crate) struct ProgramIR {
    fns: Vec<FnUnit>,
    by_name: FxHashMap<String, FnSlot>,
    emit_order: Vec<FnSlot>,
    emit_roots: Vec<FnSlot>,
    top_level_calls: Vec<FnSlot>,
}

impl ProgramIR {
    fn from_parts(
        mut all_fns: FxHashMap<String, crate::mir::def::FnIR>,
        emit_order_names: Vec<String>,
        emit_root_names: Vec<String>,
        top_level_call_names: Vec<String>,
        meta_by_name: FxHashMap<String, (bool, bool)>,
    ) -> crate::error::RR<Self> {
        let mut fns = Vec::new();
        let mut by_name = FxHashMap::default();
        for name in &emit_order_names {
            let Some(fn_ir) = all_fns.remove(name.as_str()) else {
                return Err(InternalCompilerError::new(
                    Stage::Mir,
                    format!("ProgramIR IR missing function '{}'", name),
                )
                .into_exception());
            };
            let Some((is_public, is_top_level)) = meta_by_name.get(name).copied() else {
                return Err(InternalCompilerError::new(
                    Stage::Mir,
                    format!("ProgramIR metadata missing function '{}'", name),
                )
                .into_exception());
            };
            let slot = fns.len();
            fns.push(FnUnit {
                name: name.clone(),
                ir: Some(fn_ir),
                is_public,
                is_top_level,
            });
            by_name.insert(name.clone(), slot);
        }
        let mut remaining_names: Vec<String> = all_fns
            .keys()
            .filter(|name| !by_name.contains_key(*name))
            .cloned()
            .collect();
        remaining_names.sort();
        for name in remaining_names {
            let Some(fn_ir) = all_fns.remove(name.as_str()) else {
                return Err(InternalCompilerError::new(
                    Stage::Mir,
                    format!("ProgramIR IR missing late-bound function '{}'", name),
                )
                .into_exception());
            };
            let (is_public, is_top_level) = meta_by_name
                .get(&name)
                .copied()
                .unwrap_or((false, name.starts_with("Sym_top_")));
            let slot = fns.len();
            fns.push(FnUnit {
                name: name.clone(),
                ir: Some(fn_ir),
                is_public,
                is_top_level,
            });
            by_name.insert(name, slot);
        }

        let map_slots = |names: Vec<String>,
                         label: &str,
                         by_name: &FxHashMap<String, FnSlot>|
         -> crate::error::RR<Vec<FnSlot>> {
            names
                .into_iter()
                .map(|name| {
                    by_name.get(&name).copied().ok_or_else(|| {
                        InternalCompilerError::new(
                            Stage::Mir,
                            format!("ProgramIR {} references missing function '{}'", label, name),
                        )
                        .into_exception()
                    })
                })
                .collect()
        };
        let emit_order = map_slots(emit_order_names, "emit order", &by_name)?;
        let emit_roots = map_slots(emit_root_names, "emit roots", &by_name)?;
        let top_level_calls = map_slots(top_level_call_names, "top-level call list", &by_name)?;

        Ok(Self {
            fns,
            by_name,
            emit_order,
            emit_roots,
            top_level_calls,
        })
    }

    fn names_for_slots(&self, slots: &[FnSlot]) -> Vec<String> {
        slots
            .iter()
            .filter_map(|slot| self.fns.get(*slot))
            .map(|unit| unit.name.clone())
            .collect()
    }

    fn emit_order_names(&self) -> Vec<String> {
        self.names_for_slots(&self.emit_order)
    }

    fn emit_root_names(&self) -> Vec<String> {
        self.names_for_slots(&self.emit_roots)
    }

    fn top_level_call_names(&self) -> Vec<String> {
        self.names_for_slots(&self.top_level_calls)
    }

    fn get(&self, name: &str) -> Option<&crate::mir::def::FnIR> {
        let slot = self.by_name.get(name).copied()?;
        self.get_slot(slot)
    }

    fn get_slot(&self, slot: FnSlot) -> Option<&crate::mir::def::FnIR> {
        self.fns.get(slot)?.ir.as_ref()
    }

    fn contains_name(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    fn all_slots(&self) -> impl Iterator<Item = FnSlot> + '_ {
        0..self.fns.len()
    }

    fn take_all_fns_map(&mut self) -> crate::error::RR<FxHashMap<String, crate::mir::def::FnIR>> {
        let mut out = FxHashMap::default();
        for unit in &mut self.fns {
            let Some(fn_ir) = unit.ir.take() else {
                return Err(InternalCompilerError::new(
                    Stage::Mir,
                    format!("ProgramIR function '{}' already taken", unit.name),
                )
                .into_exception());
            };
            out.insert(unit.name.clone(), fn_ir);
        }
        Ok(out)
    }

    fn restore_all_fns_map(
        &mut self,
        mut all_fns: FxHashMap<String, crate::mir::def::FnIR>,
    ) -> crate::error::RR<()> {
        for unit in &mut self.fns {
            let Some(fn_ir) = all_fns.remove(unit.name.as_str()) else {
                return Err(InternalCompilerError::new(
                    Stage::Mir,
                    format!("ProgramIR restore missing function '{}'", unit.name),
                )
                .into_exception());
            };
            unit.ir = Some(fn_ir);
        }
        if !all_fns.is_empty() {
            let mut leftovers: Vec<String> = all_fns.keys().cloned().collect();
            leftovers.sort();
            return Err(InternalCompilerError::new(
                Stage::Mir,
                format!(
                    "ProgramIR restore had unexpected functions: {}",
                    leftovers.join(", ")
                ),
            )
            .into_exception());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "pipeline/tests.rs"]
mod tests;
