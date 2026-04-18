use crate::codegen::mir_emit::MapEntry;
use crate::compiler::pipeline::{
    CompileOutputOptions, CompileProfile, EmitFunctionCache, compile_output_cache_salt,
    compile_with_configs_using_emit_cache_and_compiler_parallel,
};
use crate::compiler::{
    CompilerParallelConfig, OptLevel, ParallelConfig,
    compile_with_configs_with_options_and_compiler_parallel_and_profile,
};
use crate::error::{InternalCompilerError, RR, RRCode, RRException, Stage};
use crate::typeck::TypeConfig;
use crate::utils::Span;
use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const CACHE_VERSION: &str = concat!("rr-incremental-v2|", env!("CARGO_PKG_VERSION"));
const IMPORT_PATTERN: &str = r#"(?m)^\s*import\s+"([^"]+)"\s*(?:#.*)?$"#;
static IMPORT_RE: OnceLock<Regex> = OnceLock::new();

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IncrementalOptions {
    pub enabled: bool,
    pub auto: bool,
    pub phase1: bool,
    pub phase2: bool,
    pub phase3: bool,
    pub strict_verify: bool,
}

impl IncrementalOptions {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn auto() -> Self {
        Self {
            enabled: true,
            auto: true,
            phase1: false,
            phase2: false,
            phase3: false,
            strict_verify: false,
        }
    }

    pub fn phase1_only() -> Self {
        Self {
            enabled: true,
            auto: false,
            phase1: true,
            phase2: false,
            phase3: false,
            strict_verify: false,
        }
    }

    pub fn all_phases() -> Self {
        Self {
            enabled: true,
            auto: false,
            phase1: true,
            phase2: true,
            phase3: true,
            strict_verify: false,
        }
    }

    pub fn resolve(self, has_session: bool) -> Self {
        if !self.enabled || !self.auto {
            return self;
        }
        Self {
            enabled: true,
            auto: false,
            phase1: true,
            phase2: true,
            phase3: has_session,
            strict_verify: self.strict_verify,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IncrementalStats {
    pub phase1_artifact_hit: bool,
    pub phase2_emit_hits: usize,
    pub phase2_emit_misses: usize,
    pub phase3_memory_hit: bool,
    pub strict_verification_checked: bool,
    pub strict_verification_passed: bool,
    pub miss_reasons: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct IncrementalSession {
    phase3_artifacts: FxHashMap<String, CachedArtifact>,
}

#[derive(Clone, Debug, Default)]
pub struct IncrementalCompileOutput {
    pub r_code: String,
    pub source_map: Vec<MapEntry>,
    pub stats: IncrementalStats,
}

#[derive(Clone, Debug, Default)]
struct CachedArtifact {
    r_code: String,
    source_map: Vec<MapEntry>,
}

#[derive(Clone, Copy, Debug)]
enum StrictArtifactTier {
    Phase1Disk,
    Phase3Memory,
}

fn required_artifact_cache_key<'a>(cache_key: Option<&'a String>, context: &str) -> RR<&'a String> {
    cache_key.ok_or_else(|| {
        InternalCompilerError::new(
            Stage::Codegen,
            format!(
                "incremental artifact cache key missing while {}: pipeline invariant violated",
                context
            ),
        )
        .into_exception()
    })
}

impl StrictArtifactTier {
    fn label(self) -> &'static str {
        match self {
            Self::Phase1Disk => "phase1 disk artifact",
            Self::Phase3Memory => "phase3 memory artifact",
        }
    }
}

pub(crate) struct DiskFnEmitCache {
    root: PathBuf,
}

struct CachedCodeMapArtifactMeta {
    content_hash: u64,
}

impl DiskFnEmitCache {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn paths(&self, key: &str) -> (PathBuf, PathBuf) {
        (
            self.root.join(format!("{}.Rfn", key)),
            self.root.join(format!("{}.map", key)),
        )
    }

    fn function_emit_meta_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.fn.meta", key))
    }

    fn peephole_paths(&self, key: &str) -> (PathBuf, PathBuf) {
        (
            self.root.join(format!("{}.Rpee", key)),
            self.root.join(format!("{}.linemap", key)),
        )
    }

    fn peephole_meta_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.pee.meta", key))
    }

    fn raw_rewrite_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.Rraw", key))
    }

    fn raw_rewrite_meta_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.raw.meta", key))
    }

    fn optimized_fragment_paths(&self, key: &str) -> (PathBuf, PathBuf) {
        (
            self.root.join(format!("{}.Roptfn", key)),
            self.root.join(format!("{}.optmap", key)),
        )
    }

    fn optimized_fragment_meta_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.optfrag.meta", key))
    }

    fn optimized_assembly_safe_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.optok", key))
    }

    fn optimized_assembly_artifact_paths(&self, key: &str) -> (PathBuf, PathBuf) {
        (
            self.root.join(format!("{}.Roptasm", key)),
            self.root.join(format!("{}.optasm.map", key)),
        )
    }

    fn optimized_assembly_meta_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.optasm.meta", key))
    }

    fn optimized_assembly_source_map_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.optfinal.map", key))
    }

    fn optimized_raw_assembly_safe_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.optrawok", key))
    }

    fn optimized_peephole_assembly_safe_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.optpeeok", key))
    }
}

impl EmitFunctionCache for DiskFnEmitCache {
    fn load(&self, key: &str) -> RR<Option<(String, Vec<MapEntry>)>> {
        let (code_path, map_path) = self.paths(key);
        let meta_path = self.function_emit_meta_path(key);
        if !code_path.is_file() || !map_path.is_file() || !meta_path.is_file() {
            return Ok(None);
        }
        let code = match fs::read_to_string(&code_path) {
            Ok(code) => code,
            Err(_) => return Ok(None),
        };
        let map = match read_source_map(&map_path) {
            Ok(map) => map,
            Err(_) => return Ok(None),
        };
        let Some(meta) = read_cached_code_map_artifact_meta(&meta_path, "rr-function-emit-meta")
        else {
            return Ok(None);
        };
        if code_map_artifact_hash("rr-fn-emit-hash-v1", &code, &map) != meta.content_hash {
            return Ok(None);
        }
        Ok(Some((code, map)))
    }

    fn store(&self, key: &str, code: &str, map: &[MapEntry]) -> RR<()> {
        let (code_path, map_path) = self.paths(key);
        let meta_path = self.function_emit_meta_path(key);
        if let Some(parent) = code_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create function cache dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        fs::write(&code_path, code).map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to write function emit cache '{}': {}",
                        code_path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })?;
        write_source_map(&map_path, map)?;
        write_cached_code_map_artifact_meta(
            &meta_path,
            "rr-function-emit-meta",
            &CachedCodeMapArtifactMeta {
                content_hash: code_map_artifact_hash("rr-fn-emit-hash-v1", code, map),
            },
        )
    }

    fn load_raw_rewrite(&self, key: &str) -> RR<Option<String>> {
        let path = self.raw_rewrite_path(key);
        let meta_path = self.raw_rewrite_meta_path(key);
        if !path.is_file() || !meta_path.is_file() {
            return Ok(None);
        }
        let code = match fs::read_to_string(&path) {
            Ok(code) => code,
            Err(_) => return Ok(None),
        };
        let Some(meta) = read_cached_code_map_artifact_meta(&meta_path, "rr-raw-rewrite-meta")
        else {
            return Ok(None);
        };
        if text_artifact_hash("rr-raw-rewrite-hash-v1", &code) != meta.content_hash {
            return Ok(None);
        }
        Ok(Some(code))
    }

    fn store_raw_rewrite(&self, key: &str, code: &str) -> RR<()> {
        let path = self.raw_rewrite_path(key);
        let meta_path = self.raw_rewrite_meta_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create raw rewrite cache dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        fs::write(&path, code).map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to write raw rewrite cache '{}': {}",
                        path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })?;
        write_cached_code_map_artifact_meta(
            &meta_path,
            "rr-raw-rewrite-meta",
            &CachedCodeMapArtifactMeta {
                content_hash: text_artifact_hash("rr-raw-rewrite-hash-v1", code),
            },
        )
    }

    fn load_peephole(&self, key: &str) -> RR<Option<(String, Vec<u32>)>> {
        let (code_path, map_path) = self.peephole_paths(key);
        let meta_path = self.peephole_meta_path(key);
        if !code_path.is_file() || !map_path.is_file() || !meta_path.is_file() {
            return Ok(None);
        }
        let code = match fs::read_to_string(&code_path) {
            Ok(code) => code,
            Err(_) => return Ok(None),
        };
        let line_map = match read_line_map_cache(&map_path) {
            Ok(line_map) => line_map,
            Err(_) => return Ok(None),
        };
        let Some(meta) = read_cached_code_map_artifact_meta(&meta_path, "rr-peephole-meta") else {
            return Ok(None);
        };
        if code_line_map_artifact_hash("rr-peephole-hash-v1", &code, &line_map) != meta.content_hash
        {
            return Ok(None);
        }
        Ok(Some((code, line_map)))
    }

    fn store_peephole(&self, key: &str, code: &str, line_map: &[u32]) -> RR<()> {
        let (code_path, map_path) = self.peephole_paths(key);
        let meta_path = self.peephole_meta_path(key);
        if let Some(parent) = code_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create peephole cache dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        fs::write(&code_path, code).map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to write peephole cache '{}': {}",
                        code_path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })?;
        write_line_map_cache(&map_path, line_map)?;
        write_cached_code_map_artifact_meta(
            &meta_path,
            "rr-peephole-meta",
            &CachedCodeMapArtifactMeta {
                content_hash: code_line_map_artifact_hash("rr-peephole-hash-v1", code, line_map),
            },
        )
    }

    fn load_optimized_fragment(&self, key: &str) -> RR<Option<(String, Vec<MapEntry>)>> {
        let (code_path, map_path) = self.optimized_fragment_paths(key);
        let meta_path = self.optimized_fragment_meta_path(key);
        if !code_path.is_file() || !map_path.is_file() || !meta_path.is_file() {
            return Ok(None);
        }
        let code = match fs::read_to_string(&code_path) {
            Ok(code) => code,
            Err(_) => return Ok(None),
        };
        let map = match read_source_map(&map_path) {
            Ok(map) => map,
            Err(_) => return Ok(None),
        };
        let Some(meta) =
            read_cached_code_map_artifact_meta(&meta_path, "rr-optimized-fragment-meta")
        else {
            return Ok(None);
        };
        if code_map_artifact_hash("rr-optfrag-hash-v1", &code, &map) != meta.content_hash {
            return Ok(None);
        }
        Ok(Some((code, map)))
    }

    fn store_optimized_fragment(&self, key: &str, code: &str, map: &[MapEntry]) -> RR<()> {
        let (code_path, map_path) = self.optimized_fragment_paths(key);
        let meta_path = self.optimized_fragment_meta_path(key);
        if let Some(parent) = code_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create optimized fragment cache dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        fs::write(&code_path, code).map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to write optimized fragment cache '{}': {}",
                        code_path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })?;
        write_source_map(&map_path, map)?;
        write_cached_code_map_artifact_meta(
            &meta_path,
            "rr-optimized-fragment-meta",
            &CachedCodeMapArtifactMeta {
                content_hash: code_map_artifact_hash("rr-optfrag-hash-v1", code, map),
            },
        )
    }

    fn has_optimized_assembly_safe(&self, key: &str) -> RR<bool> {
        Ok(self.optimized_assembly_safe_path(key).is_file())
    }

    fn load_optimized_assembly_artifact(&self, key: &str) -> RR<Option<(String, Vec<MapEntry>)>> {
        let (code_path, map_path) = self.optimized_assembly_artifact_paths(key);
        let meta_path = self.optimized_assembly_meta_path(key);
        if !code_path.is_file() || !map_path.is_file() || !meta_path.is_file() {
            return Ok(None);
        }
        let code = match fs::read_to_string(&code_path) {
            Ok(code) => code,
            Err(_) => return Ok(None),
        };
        let map = match read_source_map(&map_path) {
            Ok(map) => map,
            Err(_) => return Ok(None),
        };
        let Some(meta) =
            read_cached_code_map_artifact_meta(&meta_path, "rr-optimized-assembly-meta")
        else {
            return Ok(None);
        };
        if code_map_artifact_hash("rr-optasm-hash-v1", &code, &map) != meta.content_hash {
            return Ok(None);
        }
        Ok(Some((code, map)))
    }

    fn store_optimized_assembly_artifact(&self, key: &str, code: &str, map: &[MapEntry]) -> RR<()> {
        let (code_path, map_path) = self.optimized_assembly_artifact_paths(key);
        let meta_path = self.optimized_assembly_meta_path(key);
        if let Some(parent) = code_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create optimized assembly artifact dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        fs::write(&code_path, code).map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to write optimized assembly artifact '{}': {}",
                        code_path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })?;
        write_source_map(&map_path, map)?;
        write_cached_code_map_artifact_meta(
            &meta_path,
            "rr-optimized-assembly-meta",
            &CachedCodeMapArtifactMeta {
                content_hash: code_map_artifact_hash("rr-optasm-hash-v1", code, map),
            },
        )
    }

    fn load_optimized_assembly_source_map(&self, key: &str) -> RR<Option<Vec<MapEntry>>> {
        let path = self.optimized_assembly_source_map_path(key);
        if !path.is_file() {
            return Ok(None);
        }
        match read_source_map(&path) {
            Ok(map) => Ok(Some(map)),
            Err(_) => Ok(None),
        }
    }

    fn store_optimized_assembly_source_map(&self, key: &str, map: &[MapEntry]) -> RR<()> {
        let path = self.optimized_assembly_source_map_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create optimized assembly source map dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        write_source_map(&path, map)
    }

    fn store_optimized_assembly_safe(&self, key: &str) -> RR<()> {
        let path = self.optimized_assembly_safe_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create optimized assembly cache dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        fs::write(&path, b"ok").map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to write optimized assembly cache '{}': {}",
                        path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })
    }

    fn has_optimized_raw_assembly_safe(&self, key: &str) -> RR<bool> {
        Ok(self.optimized_raw_assembly_safe_path(key).is_file())
    }

    fn store_optimized_raw_assembly_safe(&self, key: &str) -> RR<()> {
        let path = self.optimized_raw_assembly_safe_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create optimized raw assembly cache dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        fs::write(&path, b"ok").map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to write optimized raw assembly cache '{}': {}",
                        path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })
    }

    fn has_optimized_peephole_assembly_safe(&self, key: &str) -> RR<bool> {
        Ok(self.optimized_peephole_assembly_safe_path(key).is_file())
    }

    fn store_optimized_peephole_assembly_safe(&self, key: &str) -> RR<()> {
        let path = self.optimized_peephole_assembly_safe_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                attach_incremental_cache_recovery_guidance(
                    RRException::new(
                        "RR.CompilerError",
                        RRCode::ICE9001,
                        Stage::Codegen,
                        format!(
                            "failed to create optimized peephole assembly cache dir '{}': {}",
                            parent.display(),
                            e
                        ),
                    ),
                    incremental_cache_root_for_path(&self.root),
                )
            })?;
        }
        fs::write(&path, b"ok").map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to write optimized peephole assembly cache '{}': {}",
                        path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })
    }
}

#[derive(Clone, Debug, Default)]
struct ModuleFingerprint {
    canonical_path: PathBuf,
    content_hash: u64,
}

#[derive(Clone, Debug)]
struct ArtifactKeyInputs {
    modules: Vec<ModuleFingerprint>,
    entry_content_hash: u64,
    import_fingerprint: u64,
    opt_level: OptLevel,
    phase_ordering_mode: String,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    options: IncrementalOptions,
    output_options: CompileOutputOptions,
}

#[derive(Clone, Debug, Default)]
struct StoredBuildMeta {
    entry_content_hash: u64,
    import_fingerprint: u64,
    opt_level: String,
    phase_ordering_mode: String,
    type_mode: String,
    native_backend: String,
    parallel_mode: String,
    parallel_backend: String,
    parallel_threads: usize,
    parallel_min_trip: usize,
    inject_runtime: bool,
    preserve_all_defs: bool,
    strict_let: bool,
    warn_implicit_decl: bool,
    compile_mode: String,
    phase2: bool,
    strict_verify: bool,
}

pub fn compile_with_configs_incremental(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    options: IncrementalOptions,
    session: Option<&mut IncrementalSession>,
) -> RR<IncrementalCompileOutput> {
    compile_with_configs_incremental_with_output_options_and_compiler_parallel(
        entry_path,
        entry_input,
        opt_level,
        type_cfg,
        parallel_cfg,
        CompilerParallelConfig::default(),
        options,
        CompileOutputOptions::default(),
        session,
    )
}

pub fn module_tree_fingerprint(entry_path: &str, entry_input: &str) -> RR<u64> {
    crate::pkg::with_project_root_hint(entry_path, || {
        let modules = collect_module_fingerprints(entry_path, entry_input)?;
        let mut payload = String::new();
        payload.push_str(CACHE_VERSION);
        for module in modules {
            payload.push('|');
            payload.push_str(&module.canonical_path.to_string_lossy());
            payload.push(':');
            payload.push_str(&module.content_hash.to_string());
        }
        Ok(stable_hash_bytes(payload.as_bytes()))
    })
}

pub fn module_tree_snapshot(entry_path: &str, entry_input: &str) -> RR<Vec<(PathBuf, u64)>> {
    crate::pkg::with_project_root_hint(entry_path, || {
        Ok(collect_module_fingerprints(entry_path, entry_input)?
            .into_iter()
            .map(|module| (module.canonical_path, module.content_hash))
            .collect())
    })
}

#[allow(clippy::too_many_arguments)]
pub fn compile_with_configs_incremental_with_output_options(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    options: IncrementalOptions,
    output_options: CompileOutputOptions,
    session: Option<&mut IncrementalSession>,
) -> RR<IncrementalCompileOutput> {
    compile_with_configs_incremental_with_output_options_and_compiler_parallel(
        entry_path,
        entry_input,
        opt_level,
        type_cfg,
        parallel_cfg,
        CompilerParallelConfig::default(),
        options,
        output_options,
        session,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn compile_with_configs_incremental_with_output_options_and_compiler_parallel(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    compiler_parallel_cfg: CompilerParallelConfig,
    options: IncrementalOptions,
    output_options: CompileOutputOptions,
    session: Option<&mut IncrementalSession>,
) -> RR<IncrementalCompileOutput> {
    compile_with_configs_incremental_with_output_options_and_compiler_parallel_and_profile(
        entry_path,
        entry_input,
        opt_level,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options,
        output_options,
        session,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn compile_with_configs_incremental_with_output_options_and_compiler_parallel_and_profile(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    compiler_parallel_cfg: CompilerParallelConfig,
    options: IncrementalOptions,
    output_options: CompileOutputOptions,
    session: Option<&mut IncrementalSession>,
    mut profile: Option<&mut CompileProfile>,
) -> RR<IncrementalCompileOutput> {
    let mut resolved = options.resolve(session.is_some());
    if raw_r_debug_dump_requested() {
        // Raw debug dumps are emitted during the normal pipeline before the
        // peephole pass. Cached final artifacts cannot reproduce that side
        // output, so bypass the artifact-hit tiers while keeping phase2 emit
        // cache reuse available.
        resolved.phase1 = false;
        resolved.phase3 = false;
    }
    let inputs = build_artifact_key_inputs(
        entry_path,
        entry_input,
        opt_level,
        type_cfg,
        parallel_cfg,
        resolved,
        output_options,
    )?;
    if !resolved.enabled || (!resolved.phase1 && !resolved.phase2 && !resolved.phase3) {
        let (r_code, source_map) =
            compile_with_configs_with_options_and_compiler_parallel_and_profile(
                entry_path,
                entry_input,
                opt_level,
                type_cfg,
                parallel_cfg,
                compiler_parallel_cfg,
                output_options,
                profile.as_deref_mut(),
            )?;
        if let Some(profile) = profile.as_deref_mut() {
            profile.incremental.enabled = false;
        }
        return Ok(IncrementalCompileOutput {
            r_code,
            source_map,
            stats: IncrementalStats::default(),
        });
    }

    let mut stats = IncrementalStats::default();
    let mut strict_expected = Vec::new();
    let cache_key = if resolved.phase1 || resolved.phase3 {
        Some(build_artifact_key(&inputs))
    } else {
        None
    };
    let cache_root = cache_root_for_entry(entry_path);

    if resolved.phase3
        && let Some(cache_key) = cache_key.as_ref()
        && let Some(s) = session.as_ref()
        && let Some(hit) = s.phase3_artifacts.get(cache_key)
    {
        stats.phase3_memory_hit = true;
        if resolved.strict_verify {
            strict_expected.push((StrictArtifactTier::Phase3Memory, hit.clone()));
        } else {
            maybe_fill_incremental_profile(profile.as_deref_mut(), &stats);
            return Ok(IncrementalCompileOutput {
                r_code: hit.r_code.clone(),
                source_map: hit.source_map.clone(),
                stats,
            });
        }
    }

    if resolved.phase1
        && let Some(cache_key) = cache_key.as_ref()
        && let Some(hit) = load_artifact(&cache_root, cache_key)?
    {
        stats.phase1_artifact_hit = true;
        if resolved.strict_verify {
            strict_expected.push((StrictArtifactTier::Phase1Disk, hit.clone()));
        } else {
            if resolved.phase3
                && let Some(s) = session
            {
                s.phase3_artifacts.insert(cache_key.clone(), hit.clone());
            }
            maybe_fill_incremental_profile(profile.as_deref_mut(), &stats);
            return Ok(IncrementalCompileOutput {
                r_code: hit.r_code,
                source_map: hit.source_map,
                stats,
            });
        }
    }

    stats.miss_reasons = derive_incremental_miss_reasons(&cache_root, &inputs);

    let optimized_mir_cache_root = cache_root.join("optimized-mir");
    let (r_code, source_map) = if resolved.phase2 {
        let fn_cache = DiskFnEmitCache::new(cache_root.join("function-emits"));
        let (code, map, hits, misses) =
            compile_with_configs_using_emit_cache_and_compiler_parallel(
                crate::compiler::pipeline::CompilePipelineRequest {
                    entry_path,
                    entry_input,
                    opt_level,
                    type_cfg,
                    parallel_cfg,
                    compiler_parallel_cfg,
                    cache: Some(&fn_cache),
                    optimized_mir_cache_root: Some(optimized_mir_cache_root.clone()),
                    output_opts: output_options,
                    profile: profile.as_deref_mut(),
                },
            )?;
        stats.phase2_emit_hits = hits;
        stats.phase2_emit_misses = misses;
        (code, map)
    } else {
        let (code, map, _, _) = compile_with_configs_using_emit_cache_and_compiler_parallel(
            crate::compiler::pipeline::CompilePipelineRequest {
                entry_path,
                entry_input,
                opt_level,
                type_cfg,
                parallel_cfg,
                compiler_parallel_cfg,
                cache: None,
                optimized_mir_cache_root: Some(optimized_mir_cache_root),
                output_opts: output_options,
                profile: profile.as_deref_mut(),
            },
        )?;
        (code, map)
    };
    let built = CachedArtifact { r_code, source_map };

    if resolved.phase1 {
        let cache_key = required_artifact_cache_key(cache_key.as_ref(), "storing phase1 artifact")?;
        store_artifact(&cache_root, cache_key, &built)?;
    }
    if resolved.phase3
        && let Some(s) = session
    {
        let cache_key = required_artifact_cache_key(cache_key.as_ref(), "storing phase3 artifact")?;
        s.phase3_artifacts.insert(cache_key.clone(), built.clone());
    }
    if resolved.strict_verify {
        stats.strict_verification_checked = !strict_expected.is_empty();
        for (tier, expected) in &strict_expected {
            verify_strict_artifact_match(*tier, &cache_root, expected, &built)?;
        }
        stats.strict_verification_passed = stats.strict_verification_checked;
    }
    store_latest_build_meta(&cache_root, &inputs)?;
    maybe_fill_incremental_profile(profile.as_deref_mut(), &stats);
    Ok(IncrementalCompileOutput {
        r_code: built.r_code,
        source_map: built.source_map,
        stats,
    })
}

fn raw_r_debug_dump_requested() -> bool {
    env::var_os("RR_DEBUG_RAW_R_PATH").is_some()
}

fn maybe_fill_incremental_profile(profile: Option<&mut CompileProfile>, stats: &IncrementalStats) {
    let Some(profile) = profile else {
        return;
    };
    profile.incremental.enabled = true;
    profile.incremental.phase1_artifact_hit = stats.phase1_artifact_hit;
    profile.incremental.phase2_emit_hits = stats.phase2_emit_hits;
    profile.incremental.phase2_emit_misses = stats.phase2_emit_misses;
    profile.incremental.phase3_memory_hit = stats.phase3_memory_hit;
    profile.incremental.strict_verification_checked = stats.strict_verification_checked;
    profile.incremental.strict_verification_passed = stats.strict_verification_passed;
    profile.incremental.miss_reasons = stats.miss_reasons.clone();
}

fn collect_module_fingerprints(entry_path: &str, entry_input: &str) -> RR<Vec<ModuleFingerprint>> {
    let entry = normalize_module_path(Path::new(entry_path));
    let import_re = import_regex()?;

    let mut queue: Vec<PathBuf> = vec![entry.clone()];
    let mut visited = FxHashSet::default();
    let mut content_by_path: FxHashMap<PathBuf, String> = FxHashMap::default();
    content_by_path.insert(entry.clone(), entry_input.to_string());
    let mut modules = Vec::new();

    while let Some(path) = queue.pop() {
        let canonical = normalize_module_path(&path);
        if !canonical.is_absolute() {
            return Err(RRException::new(
                "RR.ParseError",
                RRCode::E0001,
                Stage::Parse,
                format!(
                    "relative import resolution requires an absolute entry path; normalize '{}' before incremental compilation",
                    entry_path
                ),
            ));
        }
        if !visited.insert(canonical.clone()) {
            continue;
        }

        let content = if let Some(s) = content_by_path.remove(&canonical) {
            s
        } else {
            fs::read_to_string(&canonical).map_err(|e| {
                RRException::new(
                    "RR.ParseError",
                    RRCode::E0001,
                    Stage::Parse,
                    format!(
                        "failed to load imported module '{}': {}",
                        canonical.display(),
                        e
                    ),
                )
            })?
        };

        modules.push(ModuleFingerprint {
            canonical_path: canonical.clone(),
            content_hash: stable_hash_bytes(content.as_bytes()),
        });

        for cap in import_re.captures_iter(&content) {
            let Some(m) = cap.get(1) else {
                continue;
            };
            let raw_import = m.as_str();
            let resolved = crate::pkg::resolve_import_path(&canonical, raw_import)?;
            if !visited.contains(&resolved) {
                queue.push(resolved);
            }
        }
    }

    modules.sort_by(|a, b| a.canonical_path.cmp(&b.canonical_path));
    Ok(modules)
}

fn build_artifact_key_inputs(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    options: IncrementalOptions,
    output_options: CompileOutputOptions,
) -> RR<ArtifactKeyInputs> {
    let modules = collect_module_fingerprints(entry_path, entry_input)?;
    let normalized_entry = normalize_module_path(Path::new(entry_path));
    let entry_content_hash = modules
        .iter()
        .find(|module| module.canonical_path == normalized_entry)
        .map(|module| module.content_hash)
        .unwrap_or_else(|| stable_hash_bytes(entry_input.as_bytes()));
    let import_fingerprint = compute_import_fingerprint(&modules, &normalized_entry);
    Ok(ArtifactKeyInputs {
        modules,
        entry_content_hash,
        import_fingerprint,
        opt_level,
        phase_ordering_mode: crate::mir::opt::TachyonEngine::phase_ordering_mode_for_opt_level(
            opt_level,
        )
        .label()
        .to_string(),
        type_cfg,
        parallel_cfg,
        options,
        output_options,
    })
}

fn compute_import_fingerprint(modules: &[ModuleFingerprint], entry_path: &Path) -> u64 {
    let mut payload = String::new();
    for module in modules {
        if module.canonical_path == entry_path {
            continue;
        }
        payload.push('|');
        payload.push_str(&module.canonical_path.to_string_lossy());
        payload.push(':');
        payload.push_str(&module.content_hash.to_string());
    }
    stable_hash_bytes(payload.as_bytes())
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

fn import_regex() -> RR<&'static Regex> {
    if let Some(re) = IMPORT_RE.get() {
        return Ok(re);
    }

    let compiled = Regex::new(IMPORT_PATTERN).map_err(|e| {
        InternalCompilerError::new(
            Stage::Parse,
            format!("failed to compile import regex: {}", e),
        )
        .into_exception()
    })?;
    let _ = IMPORT_RE.set(compiled);
    IMPORT_RE.get().ok_or_else(|| {
        InternalCompilerError::new(Stage::Parse, "failed to initialize import regex")
            .into_exception()
    })
}

fn build_artifact_key(inputs: &ArtifactKeyInputs) -> String {
    let mut payload = String::new();
    payload.push_str(CACHE_VERSION);
    payload.push('|');
    payload.push_str(inputs.opt_level.label());
    payload.push('|');
    payload.push_str(&inputs.phase_ordering_mode);
    payload.push('|');
    payload.push_str(inputs.type_cfg.mode.as_str());
    payload.push('|');
    payload.push_str(inputs.type_cfg.native_backend.as_str());
    payload.push('|');
    payload.push_str(inputs.parallel_cfg.mode.as_str());
    payload.push('|');
    payload.push_str(inputs.parallel_cfg.backend.as_str());
    payload.push('|');
    payload.push_str(&inputs.parallel_cfg.threads.to_string());
    payload.push('|');
    payload.push_str(&inputs.parallel_cfg.min_trip.to_string());
    payload.push('|');
    payload.push_str(if inputs.options.phase2 { "p2" } else { "nop2" });
    payload.push('|');
    payload.push_str(if inputs.options.strict_verify {
        "strict"
    } else {
        "nostrict"
    });
    payload.push('|');
    payload.push_str(if inputs.output_options.inject_runtime {
        "runtime"
    } else {
        "helper-only"
    });
    payload.push('|');
    payload.push_str(if inputs.output_options.preserve_all_defs {
        "preserve-defs"
    } else {
        "strip-defs"
    });
    payload.push('|');
    payload.push_str(if inputs.output_options.strict_let {
        "strict-let"
    } else {
        "legacy-implicit-decl"
    });
    payload.push('|');
    payload.push_str(if inputs.output_options.warn_implicit_decl {
        "warn-implicit-decl"
    } else {
        "silent-implicit-decl"
    });
    payload.push('|');
    payload.push_str(inputs.output_options.compile_mode.as_str());
    payload.push('|');
    payload.push_str(&compile_output_cache_salt().to_string());
    for module in &inputs.modules {
        payload.push('|');
        payload.push_str(&module.canonical_path.to_string_lossy());
        payload.push(':');
        payload.push_str(&module.content_hash.to_string());
    }
    let hash = stable_hash_bytes(payload.as_bytes());
    format!("{:016x}", hash)
}

fn latest_build_meta_path(cache_root: &Path) -> PathBuf {
    cache_root.join("latest-build.meta")
}

fn store_latest_build_meta(cache_root: &Path, inputs: &ArtifactKeyInputs) -> RR<()> {
    if let Some(parent) = latest_build_meta_path(cache_root).parent() {
        fs::create_dir_all(parent).map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to create incremental metadata dir '{}': {}",
                        parent.display(),
                        e
                    ),
                ),
                Some(cache_root),
            )
        })?;
    }
    let payload = format!(
        concat!(
            "entry_content_hash={}\n",
            "import_fingerprint={}\n",
            "opt_level={}\n",
            "phase_ordering_mode={}\n",
            "type_mode={}\n",
            "native_backend={}\n",
            "parallel_mode={}\n",
            "parallel_backend={}\n",
            "parallel_threads={}\n",
            "parallel_min_trip={}\n",
            "inject_runtime={}\n",
            "preserve_all_defs={}\n",
            "strict_let={}\n",
            "warn_implicit_decl={}\n",
            "compile_mode={}\n",
            "phase2={}\n",
            "strict_verify={}\n"
        ),
        inputs.entry_content_hash,
        inputs.import_fingerprint,
        inputs.opt_level.label(),
        inputs.phase_ordering_mode,
        inputs.type_cfg.mode.as_str(),
        inputs.type_cfg.native_backend.as_str(),
        inputs.parallel_cfg.mode.as_str(),
        inputs.parallel_cfg.backend.as_str(),
        inputs.parallel_cfg.threads,
        inputs.parallel_cfg.min_trip,
        inputs.output_options.inject_runtime,
        inputs.output_options.preserve_all_defs,
        inputs.output_options.strict_let,
        inputs.output_options.warn_implicit_decl,
        inputs.output_options.compile_mode.as_str(),
        inputs.options.phase2,
        inputs.options.strict_verify,
    );
    fs::write(latest_build_meta_path(cache_root), payload).map_err(|e| {
        attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to write incremental metadata '{}': {}",
                    latest_build_meta_path(cache_root).display(),
                    e
                ),
            ),
            Some(cache_root),
        )
    })
}

fn load_latest_build_meta(cache_root: &Path) -> Option<StoredBuildMeta> {
    let path = latest_build_meta_path(cache_root);
    let text = fs::read_to_string(path).ok()?;
    let mut meta = StoredBuildMeta::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "entry_content_hash" => meta.entry_content_hash = value.parse().ok()?,
            "import_fingerprint" => meta.import_fingerprint = value.parse().ok()?,
            "opt_level" => meta.opt_level = value.to_string(),
            "phase_ordering_mode" => meta.phase_ordering_mode = value.to_string(),
            "type_mode" => meta.type_mode = value.to_string(),
            "native_backend" => meta.native_backend = value.to_string(),
            "parallel_mode" => meta.parallel_mode = value.to_string(),
            "parallel_backend" => meta.parallel_backend = value.to_string(),
            "parallel_threads" => meta.parallel_threads = value.parse().ok()?,
            "parallel_min_trip" => meta.parallel_min_trip = value.parse().ok()?,
            "inject_runtime" => meta.inject_runtime = value.parse().ok()?,
            "preserve_all_defs" => meta.preserve_all_defs = value.parse().ok()?,
            "strict_let" => meta.strict_let = value.parse().ok()?,
            "warn_implicit_decl" => meta.warn_implicit_decl = value.parse().ok()?,
            "compile_mode" => meta.compile_mode = value.to_string(),
            "phase2" => meta.phase2 = value.parse().ok()?,
            "strict_verify" => meta.strict_verify = value.parse().ok()?,
            _ => {}
        }
    }
    Some(meta)
}

fn derive_incremental_miss_reasons(cache_root: &Path, inputs: &ArtifactKeyInputs) -> Vec<String> {
    let Some(previous) = load_latest_build_meta(cache_root) else {
        return vec!["cold_start".to_string()];
    };

    let mut reasons = Vec::new();
    if previous.entry_content_hash != inputs.entry_content_hash {
        reasons.push("entry_changed".to_string());
    }
    if previous.import_fingerprint != inputs.import_fingerprint {
        reasons.push("import_fingerprint_changed".to_string());
    }
    if previous.opt_level != inputs.opt_level.label() {
        reasons.push("opt_level_changed".to_string());
    }
    if previous.type_mode != inputs.type_cfg.mode.as_str()
        || previous.native_backend != inputs.type_cfg.native_backend.as_str()
        || previous.parallel_mode != inputs.parallel_cfg.mode.as_str()
        || previous.parallel_backend != inputs.parallel_cfg.backend.as_str()
        || previous.parallel_threads != inputs.parallel_cfg.threads
        || previous.parallel_min_trip != inputs.parallel_cfg.min_trip
    {
        reasons.push("compile_config_changed".to_string());
    }
    if previous.inject_runtime != inputs.output_options.inject_runtime
        || previous.preserve_all_defs != inputs.output_options.preserve_all_defs
        || previous.strict_let != inputs.output_options.strict_let
        || previous.warn_implicit_decl != inputs.output_options.warn_implicit_decl
        || previous.compile_mode != inputs.output_options.compile_mode.as_str()
    {
        reasons.push("output_options_changed".to_string());
    }
    if previous.phase2 != inputs.options.phase2
        || previous.strict_verify != inputs.options.strict_verify
    {
        reasons.push("incremental_options_changed".to_string());
    }
    if reasons.is_empty() {
        reasons.push("artifact_unavailable".to_string());
    }
    reasons
}

pub(crate) fn cache_root_for_entry(entry_path: &str) -> PathBuf {
    if let Some(v) = env::var_os("RR_INCREMENTAL_CACHE_DIR")
        && !v.is_empty()
    {
        return PathBuf::from(v);
    }

    let normalized_entry = normalize_module_path(Path::new(entry_path));
    let mut cur = normalized_entry
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    loop {
        let managed_root = cur.join("rr.mod").is_file()
            || cur.join("src").join("main.rr").is_file()
            || cur.join("src").join("lib.rr").is_file();
        let legacy_root = cur.file_name().and_then(|name| name.to_str()) != Some("src")
            && cur.join("main.rr").is_file();
        if managed_root || legacy_root {
            return cur.join("Build").join("incremental");
        }
        let Some(parent) = cur.parent() else {
            break;
        };
        cur = parent.to_path_buf();
    }
    normalized_entry
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Build")
        .join("incremental")
}

fn artifact_paths(cache_root: &Path, key: &str) -> (PathBuf, PathBuf) {
    let artifact_dir = cache_root.join("artifacts");
    (
        artifact_dir.join(format!("{}.R", key)),
        artifact_dir.join(format!("{}.map", key)),
    )
}

fn verify_strict_artifact_match(
    tier: StrictArtifactTier,
    cache_root: &Path,
    expected: &CachedArtifact,
    built: &CachedArtifact,
) -> RR<()> {
    if expected.r_code != built.r_code {
        return Err(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "strict incremental verification failed: {} R output mismatch",
                    tier.label()
                ),
            )
            .note(format!("incremental cache root: {}", cache_root.display()))
            .help("rerun with --no-incremental to bypass cached artifacts for this compile")
            .fix(format!(
                "clear the incremental cache at `{}` and rebuild if you expect the cache to be stale or corrupted",
                cache_root.display()
            )),
        );
    }
    if expected.source_map != built.source_map {
        return Err(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "strict incremental verification failed: {} source map mismatch",
                    tier.label()
                ),
            )
            .note(format!("incremental cache root: {}", cache_root.display()))
            .help("rerun with --no-incremental to bypass cached artifacts for this compile")
            .fix(format!(
                "clear the incremental cache at `{}` and rebuild if you expect the cache to be stale or corrupted",
                cache_root.display()
            )),
        );
    }
    Ok(())
}

fn attach_incremental_cache_recovery_guidance(
    mut err: RRException,
    cache_root: Option<&Path>,
) -> RRException {
    err = err.help("rerun with --no-incremental to bypass cached artifacts for this compile");
    if let Some(cache_root) = cache_root {
        err = err
            .note(format!("incremental cache root: {}", cache_root.display()))
            .fix(format!(
                "clear the incremental cache at `{}` and rebuild if you expect the cache to be stale, malformed, or inaccessible",
                cache_root.display()
            ));
    }
    err
}

fn incremental_cache_root_for_path(path: &Path) -> Option<&Path> {
    cache_root_for_artifact_path(path)
}

fn load_artifact(cache_root: &Path, key: &str) -> RR<Option<CachedArtifact>> {
    let (code_path, map_path) = artifact_paths(cache_root, key);
    if !code_path.is_file() || !map_path.is_file() {
        return Ok(None);
    }
    let r_code = fs::read_to_string(&code_path).map_err(|e| {
        attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to read incremental artifact '{}': {}",
                    code_path.display(),
                    e
                ),
            ),
            Some(cache_root),
        )
    })?;
    let source_map = read_source_map(&map_path)?;
    Ok(Some(CachedArtifact { r_code, source_map }))
}

fn store_artifact(cache_root: &Path, key: &str, artifact: &CachedArtifact) -> RR<()> {
    let (code_path, map_path) = artifact_paths(cache_root, key);
    if let Some(parent) = code_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to create cache directory '{}': {}",
                        parent.display(),
                        e
                    ),
                ),
                Some(cache_root),
            )
        })?;
    }
    fs::write(&code_path, &artifact.r_code).map_err(|e| {
        attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to write incremental artifact '{}': {}",
                    code_path.display(),
                    e
                ),
            ),
            Some(cache_root),
        )
    })?;
    write_source_map(&map_path, &artifact.source_map)
}

fn render_source_map_cache_contents(map: &[MapEntry]) -> String {
    let mut out = String::new();
    for entry in map {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            entry.r_line,
            entry.rr_span.start_byte,
            entry.rr_span.end_byte,
            entry.rr_span.start_line,
            entry.rr_span.start_col,
            entry.rr_span.end_line,
            entry.rr_span.end_col
        ));
    }
    out
}

fn code_map_artifact_hash(kind: &str, code: &str, map: &[MapEntry]) -> u64 {
    let map_contents = render_source_map_cache_contents(map);
    let mut payload = String::new();
    payload.push_str(kind);
    payload.push('|');
    payload.push_str(code);
    payload.push('|');
    payload.push_str(&map_contents);
    stable_hash_bytes(payload.as_bytes())
}

fn render_line_map_cache_contents(map: &[u32]) -> String {
    let mut out = String::new();
    for line in map {
        out.push_str(&line.to_string());
        out.push('\n');
    }
    out
}

fn code_line_map_artifact_hash(kind: &str, code: &str, line_map: &[u32]) -> u64 {
    let line_map_contents = render_line_map_cache_contents(line_map);
    let mut payload = String::new();
    payload.push_str(kind);
    payload.push('|');
    payload.push_str(code);
    payload.push('|');
    payload.push_str(&line_map_contents);
    stable_hash_bytes(payload.as_bytes())
}

fn text_artifact_hash(kind: &str, text: &str) -> u64 {
    let mut payload = String::new();
    payload.push_str(kind);
    payload.push('|');
    payload.push_str(text);
    stable_hash_bytes(payload.as_bytes())
}

fn write_cached_code_map_artifact_meta(
    path: &Path,
    schema: &str,
    meta: &CachedCodeMapArtifactMeta,
) -> RR<()> {
    let payload = format!(
        "schema={}\nversion={}\nhash={:016x}\n",
        schema,
        env!("CARGO_PKG_VERSION"),
        meta.content_hash
    );
    fs::write(path, payload).map_err(|e| {
        attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to write code-map artifact metadata '{}': {}",
                    path.display(),
                    e
                ),
            ),
            incremental_cache_root_for_path(path),
        )
    })
}

fn read_cached_code_map_artifact_meta(
    path: &Path,
    expected_schema: &str,
) -> Option<CachedCodeMapArtifactMeta> {
    let content = fs::read_to_string(path).ok()?;
    let mut schema = None;
    let mut version = None;
    let mut hash = None;
    for line in content.lines() {
        let (key, value) = line.split_once('=')?;
        match key {
            "schema" => schema = Some(value),
            "version" => version = Some(value),
            "hash" => hash = u64::from_str_radix(value, 16).ok(),
            _ => {}
        }
    }
    if schema != Some(expected_schema) || version != Some(env!("CARGO_PKG_VERSION")) {
        return None;
    }
    Some(CachedCodeMapArtifactMeta {
        content_hash: hash?,
    })
}

fn write_source_map(path: &Path, map: &[MapEntry]) -> RR<()> {
    let out = render_source_map_cache_contents(map);
    fs::write(path, out).map_err(|e| {
        attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to write incremental source map '{}': {}",
                    path.display(),
                    e
                ),
            ),
            incremental_cache_root_for_path(path),
        )
    })
}

fn write_line_map_cache(path: &Path, map: &[u32]) -> RR<()> {
    let out = render_line_map_cache_contents(map);
    fs::write(path, out).map_err(|e| {
        attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to write peephole line map '{}': {}",
                    path.display(),
                    e
                ),
            ),
            incremental_cache_root_for_path(path),
        )
    })
}

fn read_source_map(path: &Path) -> RR<Vec<MapEntry>> {
    let content = fs::read_to_string(path).map_err(|e| {
        attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to read incremental source map '{}': {}",
                    path.display(),
                    e
                ),
            ),
            incremental_cache_root_for_path(path),
        )
    })?;
    let mut out = Vec::new();
    for (line_no, line) in content.lines().enumerate() {
        if let Some(entry) = parse_source_map_entry(line) {
            out.push(entry);
        } else if !line.trim().is_empty() {
            let err = RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to parse incremental source map '{}': malformed entry at line {}",
                    path.display(),
                    line_no + 1
                ),
            )
            .note("the cached source map entry is malformed or was written by an incompatible compiler shape");
            return Err(attach_incremental_cache_recovery_guidance(
                err,
                incremental_cache_root_for_path(path),
            ));
        }
    }
    Ok(out)
}

fn read_line_map_cache(path: &Path) -> RR<Vec<u32>> {
    let content = fs::read_to_string(path).map_err(|e| {
        attach_incremental_cache_recovery_guidance(
            RRException::new(
                "RR.CompilerError",
                RRCode::ICE9001,
                Stage::Codegen,
                format!(
                    "failed to read peephole line map '{}': {}",
                    path.display(),
                    e
                ),
            ),
            incremental_cache_root_for_path(path),
        )
    })?;
    let mut out = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parsed = trimmed.parse::<u32>().map_err(|_| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to parse peephole line map '{}': malformed entry at line {}",
                        path.display(),
                        idx + 1
                    ),
                ),
                incremental_cache_root_for_path(path),
            )
        })?;
        out.push(parsed);
    }
    Ok(out)
}

fn cache_root_for_artifact_path(path: &Path) -> Option<&Path> {
    path.ancestors()
        .find(|ancestor| ancestor.file_name().and_then(|name| name.to_str()) == Some(".rr-cache"))
}

fn parse_source_map_entry(line: &str) -> Option<MapEntry> {
    if line.trim().is_empty() {
        return None;
    }
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() != 7 {
        return None;
    }
    let parsed = (
        parts[0].parse::<u32>(),
        parts[1].parse::<usize>(),
        parts[2].parse::<usize>(),
        parts[3].parse::<u32>(),
        parts[4].parse::<u32>(),
        parts[5].parse::<u32>(),
        parts[6].parse::<u32>(),
    );
    let (
        Ok(r_line),
        Ok(start_byte),
        Ok(end_byte),
        Ok(start_line),
        Ok(start_col),
        Ok(end_line),
        Ok(end_col),
    ) = parsed
    else {
        return None;
    };
    Some(MapEntry {
        r_line,
        rr_span: Span {
            start_byte,
            end_byte,
            start_line,
            start_col,
            end_line,
            end_col,
        },
    })
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
