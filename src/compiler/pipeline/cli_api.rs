use super::*;
// Compiler pipeline coordinator.
//
// This module owns driver-facing compile configuration, cache salts, and the
// stable containers that carry lowered program state between major phases.
// Heavy raw-emitted-R rewrites and phase-specific implementation details live
// in sibling `pipeline/*` modules so this file can stay focused on orchestration.

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
use std::sync::{OnceLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

#[path = "raw_rewrites.rs"]
pub(crate) mod raw_rewrites;
pub(crate) use raw_rewrites::*;

#[path = "helper_raw_rewrites.rs"]
pub(crate) mod helper_raw_rewrites;
pub(crate) use helper_raw_rewrites::*;

#[path = "scalar_raw_rewrites.rs"]
pub(crate) mod scalar_raw_rewrites;
#[cfg(not(test))]
pub(crate) use scalar_raw_rewrites::rewrite_static_record_scalarization_lines;
#[cfg(test)]
pub(crate) use scalar_raw_rewrites::*;

#[path = "cleanup_raw_rewrites.rs"]
pub(crate) mod cleanup_raw_rewrites;
pub(crate) use cleanup_raw_rewrites::*;

#[path = "structural_raw_rewrites.rs"]
pub(crate) mod structural_raw_rewrites;
pub(crate) use structural_raw_rewrites::*;

#[path = "raw_utils.rs"]
pub(crate) mod raw_utils;
pub(crate) use raw_utils::*;

#[path = "function_props.rs"]
pub(crate) mod function_props;
pub(crate) use function_props::*;

#[path = "compile_api.rs"]
pub(crate) mod compile_api;
pub use compile_api::*;

#[path = "profile.rs"]
pub(crate) mod profile;
pub use profile::*;

#[path = "loop_repairs.rs"]
pub(crate) mod loop_repairs;
pub(crate) use loop_repairs::*;

#[path = "late_raw_rewrites.rs"]
pub(crate) mod late_raw_rewrites;
pub(crate) use late_raw_rewrites::*;

#[path = "phases.rs"]
pub(crate) mod phases;
pub(crate) use phases::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptLevel {
    O0,
    O1,
    O2,
    O3,
    Oz,
}

impl OptLevel {
    pub(crate) fn is_optimized(self) -> bool {
        !matches!(self, Self::O0)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::O0 => "O0",
            Self::O1 => "O1",
            Self::O2 => "O2",
            Self::O3 => "O3",
            Self::Oz => "Oz",
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

pub(crate) fn raw_same_var_is_na_or_not_finite_re() -> Option<&'static Regex> {
    static RE: OnceLock<Result<Regex, String>> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"is\.na\((?P<lhs>[A-Za-z_][A-Za-z0-9_]*)\)\s*\|\s*\(!\(is\.finite\((?P<rhs>[A-Za-z_][A-Za-z0-9_]*)\)\)\)",
        )
        .map_err(|err| err.to_string())
    })
    .as_ref()
    .ok()
}

pub(crate) fn raw_wrapped_not_finite_cond_re() -> Option<&'static Regex> {
    static RE: OnceLock<Result<Regex, String>> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\(\((?P<inner>!\(is\.finite\([A-Za-z_][A-Za-z0-9_]*\)\))\)\)")
            .map_err(|err| err.to_string())
    })
    .as_ref()
    .ok()
}

pub(crate) fn raw_not_finite_or_zero_guard_re() -> Option<&'static Regex> {
    static RE: OnceLock<Result<Regex, String>> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"\(\(\((?P<inner>!\(is\.finite\((?P<lhs>[A-Za-z_][A-Za-z0-9_]*)\)\))\)\s*\|\s*\((?P<rhs>[A-Za-z_][A-Za-z0-9_]*) == 0\)\)\)",
        )
        .map_err(|err| err.to_string())
    })
    .as_ref()
    .ok()
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
    pub(crate) color: bool,
    pub(crate) quiet: bool,
    pub(crate) detailed: bool,
    pub(crate) slow_step_ms: usize,
    pub(crate) slow_step_repeat_ms: usize,
}

pub(crate) const CLI_SECTION_WIDTH: usize = 72;

pub(crate) struct CliStep {
    pub(crate) started: Instant,
    pub(crate) stop_tx: Option<mpsc::Sender<()>>,
    pub(crate) watcher: Option<thread::JoinHandle<()>>,
}

impl CliStep {
    pub(crate) fn new(started: Instant) -> Self {
        Self {
            started,
            stop_tx: None,
            watcher: None,
        }
    }

    pub(crate) fn with_watcher(
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

    pub(crate) fn elapsed(&self) -> Duration {
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

pub(crate) fn parse_nonnegative_usize_env(key: &str) -> Option<usize> {
    env::var(key)
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
}

pub(crate) fn env_truthy(key: &str) -> bool {
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

pub(crate) fn maybe_write_pulse_stats_json(stats: &crate::mir::opt::TachyonPulseStats) {
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

    pub(crate) fn style(&self, code: &str, text: &str) -> String {
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

    pub(crate) fn green_bold(&self, text: &str) -> String {
        self.style("1;92", text)
    }

    pub(crate) fn cyan_bold(&self, text: &str) -> String {
        self.style("1;96", text)
    }

    pub(crate) fn magenta_bold(&self, text: &str) -> String {
        self.style("1;95", text)
    }

    pub fn white_bold(&self, text: &str) -> String {
        self.style("1;97", text)
    }

    pub(crate) fn section_header_text(title: &str) -> String {
        let prefix = format!("==[ {title} ]");
        let fill = CLI_SECTION_WIDTH.saturating_sub(prefix.len()).max(2);
        format!("{prefix}{}", "=".repeat(fill))
    }

    pub fn section_header(&self, title: &str) {
        if self.quiet {
            return;
        }
        println!("{}", self.red_bold(&Self::section_header_text(title)));
    }

    pub fn field_line(&self, key: &str, value: &str) {
        if self.quiet {
            return;
        }
        println!(
            "   {} :: {}",
            self.dim(&format!("{key:<6}")),
            self.white_bold(value)
        );
    }

    pub(crate) fn banner(&self, input: &str, level: OptLevel) {
        if self.quiet {
            return;
        }
        self.section_header(&format!("RR Tachyon v{}", env!("CARGO_PKG_VERSION")));
        self.field_line("input", input);
        self.field_line("opt", level.label());
    }

    pub(crate) fn slow_step_hint(title: &str) -> &'static str {
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

    pub(crate) fn step_start(
        &self,
        idx: usize,
        total: usize,
        title: &str,
        detail: &str,
    ) -> CliStep {
        if self.quiet {
            return CliStep::new(Instant::now());
        }
        println!(
            "{} {} {}",
            self.cyan_bold(&format!("{:>12}", "Running")),
            self.red_bold(title),
            self.magenta_bold(&format!("[{}/{}]", idx, total))
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
        let verbose_hint = "set RR_VERBOSE_LOG=1 for additional progress details.";
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

    pub(crate) fn status_line(&self, status: &str, msg: &str) {
        println!(
            "{} {}",
            self.green_bold(&format!("{:>12}", status)),
            self.white_bold(msg)
        );
    }

    pub(crate) fn step_line_ok(&self, detail: &str) {
        if self.quiet {
            return;
        }
        if std::env::var_os("RR_VERBOSE_LOG").is_none() {
            if let Some(rest) = detail.strip_prefix("Finished ") {
                self.status_line("Finished", rest);
            }
            return;
        }
        if let Some(rest) = detail.strip_prefix("Pulse: ") {
            self.status_line("Optimized", rest);
            return;
        }
        if let Some(rest) = detail.strip_prefix("Finished ") {
            self.status_line("Finished", rest);
            return;
        }
        if detail.contains(" | ")
            && let Some((label, rest)) = detail.split_once(':')
        {
            println!(
                "{} {} {}",
                self.green_bold(&format!("{:>12}", "Done")),
                self.white_bold(&format!("{label}:")),
                self.white_bold(rest.trim())
            );
            return;
        }
        self.status_line("Done", detail);
    }

    pub(crate) fn trace(&self, label: &str, detail: &str) {
        if self.detailed {
            println!(
                "   {} {} {}",
                self.dim("*"),
                self.dim(label),
                self.dim(detail)
            );
        }
    }

    pub(crate) fn pulse_success(&self, total: Duration) {
        if self.quiet {
            return;
        }
        println!(
            "{} {} ({})",
            self.green_bold(&format!("{:>12}", "Finished")),
            self.green_bold("Tachyon Pulse Successful"),
            self.green_bold(&format_duration(total))
        );
    }

    pub fn success(&self, msg: &str) {
        if self.quiet {
            return;
        }
        if let Some(rest) = msg.strip_prefix("Built ") {
            self.status_line("Built", rest);
            return;
        }
        self.status_line("Finished", msg);
    }
    pub fn warn(&self, msg: &str) {
        if self.quiet {
            return;
        }
        eprintln!(
            "{} {}",
            self.yellow_bold(&format!("{:>12}", "Warning")),
            self.yellow_bold(msg)
        );
    }

    pub fn error(&self, msg: &str) {
        if self.quiet {
            return;
        }
        eprintln!("{} {}", self.red_bold("x"), self.red_bold(msg));
    }
}

pub(crate) fn format_duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.2}s", d.as_secs_f64())
    }
}

pub(crate) fn human_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else {
        format!("{:.0}KB", (bytes as f64) / 1024.0)
    }
}

pub(crate) fn fn_ir_work_size(fn_ir: &crate::mir::def::FnIR) -> usize {
    fn_ir.values.len()
        + fn_ir.blocks.len()
        + fn_ir.blocks.iter().map(|bb| bb.instrs.len()).sum::<usize>()
}

pub(crate) fn emitted_segment_line_count(code: &str) -> u32 {
    if code.is_empty() {
        1
    } else {
        let base = code.lines().count() as u32;
        if code.ends_with('\n') { base + 1 } else { base }
    }
}

pub(crate) fn shifted_source_map_for_final_output_prefix(
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

pub(crate) fn contains_generated_poly_loop_controls(code: &str) -> bool {
    code.contains(".__poly_gen_iv_")
}

pub(crate) fn escape_r_string(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

pub(crate) fn normalize_module_path(path: &Path) -> PathBuf {
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
