use crate::codegen::mir_emit::MapEntry;
use crate::compiler::pipeline::{
    CompileOutputOptions, EmitFunctionCache, compile_output_cache_salt,
    compile_with_configs_using_emit_cache,
};
use crate::compiler::{OptLevel, ParallelConfig, compile_with_configs_with_options};
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

impl StrictArtifactTier {
    fn label(self) -> &'static str {
        match self {
            Self::Phase1Disk => "phase1 disk artifact",
            Self::Phase3Memory => "phase3 memory artifact",
        }
    }
}

struct DiskFnEmitCache {
    root: PathBuf,
}

impl DiskFnEmitCache {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn paths(&self, key: &str) -> (PathBuf, PathBuf) {
        (
            self.root.join(format!("{}.Rfn", key)),
            self.root.join(format!("{}.map", key)),
        )
    }
}

impl EmitFunctionCache for DiskFnEmitCache {
    fn load(&mut self, key: &str) -> RR<Option<(String, Vec<MapEntry>)>> {
        let (code_path, map_path) = self.paths(key);
        if !code_path.is_file() || !map_path.is_file() {
            return Ok(None);
        }
        let code = fs::read_to_string(&code_path).map_err(|e| {
            attach_incremental_cache_recovery_guidance(
                RRException::new(
                    "RR.CompilerError",
                    RRCode::ICE9001,
                    Stage::Codegen,
                    format!(
                        "failed to read function emit cache '{}': {}",
                        code_path.display(),
                        e
                    ),
                ),
                incremental_cache_root_for_path(&self.root),
            )
        })?;
        let map = read_source_map(&map_path)?;
        Ok(Some((code, map)))
    }

    fn store(&mut self, key: &str, code: &str, map: &[MapEntry]) -> RR<()> {
        let (code_path, map_path) = self.paths(key);
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
        write_source_map(&map_path, map)
    }
}

#[derive(Clone, Debug, Default)]
struct ModuleFingerprint {
    canonical_path: PathBuf,
    content_hash: u64,
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
    compile_with_configs_incremental_with_output_options(
        entry_path,
        entry_input,
        opt_level,
        type_cfg,
        parallel_cfg,
        options,
        CompileOutputOptions::default(),
        session,
    )
}

pub fn module_tree_fingerprint(entry_path: &str, entry_input: &str) -> RR<u64> {
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
}

pub fn module_tree_snapshot(entry_path: &str, entry_input: &str) -> RR<Vec<(PathBuf, u64)>> {
    Ok(collect_module_fingerprints(entry_path, entry_input)?
        .into_iter()
        .map(|module| (module.canonical_path, module.content_hash))
        .collect())
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
    let resolved = options.resolve(session.is_some());
    if !resolved.enabled {
        let (r_code, source_map) = compile_with_configs_with_options(
            entry_path,
            entry_input,
            opt_level,
            type_cfg,
            parallel_cfg,
            output_options,
        )?;
        return Ok(IncrementalCompileOutput {
            r_code,
            source_map,
            stats: IncrementalStats::default(),
        });
    }

    let mut stats = IncrementalStats::default();
    let fingerprint = collect_module_fingerprints(entry_path, entry_input)?;
    let cache_key = build_artifact_key(
        &fingerprint,
        opt_level,
        type_cfg,
        parallel_cfg,
        resolved,
        output_options,
    );
    let mut strict_expected = Vec::new();

    if resolved.phase3
        && let Some(s) = session.as_ref()
        && let Some(hit) = s.phase3_artifacts.get(&cache_key)
    {
        stats.phase3_memory_hit = true;
        if resolved.strict_verify {
            strict_expected.push((StrictArtifactTier::Phase3Memory, hit.clone()));
        } else {
            return Ok(IncrementalCompileOutput {
                r_code: hit.r_code.clone(),
                source_map: hit.source_map.clone(),
                stats,
            });
        }
    }

    let cache_root = cache_root_for_entry(entry_path);
    if resolved.phase1
        && let Some(hit) = load_artifact(&cache_root, &cache_key)?
    {
        stats.phase1_artifact_hit = true;
        if resolved.strict_verify {
            strict_expected.push((StrictArtifactTier::Phase1Disk, hit.clone()));
        } else {
            if resolved.phase3
                && let Some(s) = session
            {
                s.phase3_artifacts.insert(cache_key, hit.clone());
            }
            return Ok(IncrementalCompileOutput {
                r_code: hit.r_code,
                source_map: hit.source_map,
                stats,
            });
        }
    }

    let (r_code, source_map) = if resolved.phase2 {
        let mut fn_cache = DiskFnEmitCache::new(cache_root.join("function-emits"));
        let (code, map, hits, misses) = compile_with_configs_using_emit_cache(
            entry_path,
            entry_input,
            opt_level,
            type_cfg,
            parallel_cfg,
            Some(&mut fn_cache),
            output_options,
        )?;
        stats.phase2_emit_hits = hits;
        stats.phase2_emit_misses = misses;
        (code, map)
    } else {
        compile_with_configs_with_options(
            entry_path,
            entry_input,
            opt_level,
            type_cfg,
            parallel_cfg,
            output_options,
        )?
    };
    let built = CachedArtifact { r_code, source_map };

    if resolved.phase1 {
        store_artifact(&cache_root, &cache_key, &built)?;
    }
    if resolved.phase3
        && let Some(s) = session
    {
        s.phase3_artifacts.insert(cache_key, built.clone());
    }
    if resolved.strict_verify {
        stats.strict_verification_checked = !strict_expected.is_empty();
        for (tier, expected) in &strict_expected {
            verify_strict_artifact_match(*tier, &cache_root, expected, &built)?;
        }
        stats.strict_verification_passed = stats.strict_verification_checked;
    }
    Ok(IncrementalCompileOutput {
        r_code: built.r_code,
        source_map: built.source_map,
        stats,
    })
}

fn collect_module_fingerprints(entry_path: &str, entry_input: &str) -> RR<Vec<ModuleFingerprint>> {
    let entry = normalize_module_path(Path::new(entry_path));
    let entry_parent = entry.parent().unwrap_or(Path::new("."));
    let import_re = import_regex()?;

    let mut queue: Vec<PathBuf> = vec![entry.clone()];
    let mut visited = FxHashSet::default();
    let mut content_by_path: FxHashMap<PathBuf, String> = FxHashMap::default();
    content_by_path.insert(entry.clone(), entry_input.to_string());
    let mut modules = Vec::new();

    while let Some(path) = queue.pop() {
        let canonical = normalize_module_path(&path);
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
            let raw_rel = m.as_str();
            let dir = canonical.parent().unwrap_or(entry_parent);
            let resolved = normalize_module_path(&dir.join(raw_rel));
            if !visited.contains(&resolved) {
                queue.push(resolved);
            }
        }
    }

    modules.sort_by(|a, b| a.canonical_path.cmp(&b.canonical_path));
    Ok(modules)
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

fn build_artifact_key(
    modules: &[ModuleFingerprint],
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    options: IncrementalOptions,
    output_options: CompileOutputOptions,
) -> String {
    let mut payload = String::new();
    payload.push_str(CACHE_VERSION);
    payload.push('|');
    payload.push_str(opt_level.label());
    payload.push('|');
    payload.push_str(type_cfg.mode.as_str());
    payload.push('|');
    payload.push_str(type_cfg.native_backend.as_str());
    payload.push('|');
    payload.push_str(parallel_cfg.mode.as_str());
    payload.push('|');
    payload.push_str(parallel_cfg.backend.as_str());
    payload.push('|');
    payload.push_str(&parallel_cfg.threads.to_string());
    payload.push('|');
    payload.push_str(&parallel_cfg.min_trip.to_string());
    payload.push('|');
    payload.push_str(if options.phase2 { "p2" } else { "nop2" });
    payload.push('|');
    payload.push_str(if options.strict_verify {
        "strict"
    } else {
        "nostrict"
    });
    payload.push('|');
    payload.push_str(if output_options.inject_runtime {
        "runtime"
    } else {
        "helper-only"
    });
    payload.push('|');
    payload.push_str(if output_options.preserve_all_defs {
        "preserve-defs"
    } else {
        "strip-defs"
    });
    payload.push('|');
    payload.push_str(&compile_output_cache_salt().to_string());
    for module in modules {
        payload.push('|');
        payload.push_str(&module.canonical_path.to_string_lossy());
        payload.push(':');
        payload.push_str(&module.content_hash.to_string());
    }
    let hash = stable_hash_bytes(payload.as_bytes());
    format!("{:016x}", hash)
}

fn cache_root_for_entry(entry_path: &str) -> PathBuf {
    if let Some(v) = env::var_os("RR_INCREMENTAL_CACHE_DIR")
        && !v.is_empty()
    {
        return PathBuf::from(v);
    }

    let mut cur = normalize_module_path(Path::new(entry_path))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    loop {
        let probe = cur.join("Cargo.toml");
        if probe.is_file() {
            return cur.join("target").join(".rr-cache");
        }
        let Some(parent) = cur.parent() else {
            break;
        };
        cur = parent.to_path_buf();
    }
    PathBuf::from("target").join(".rr-cache")
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

fn write_source_map(path: &Path, map: &[MapEntry]) -> RR<()> {
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
