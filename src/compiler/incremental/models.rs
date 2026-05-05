use super::*;
#[derive(Clone, Debug, Default)]
pub(crate) struct ModuleFingerprint {
    pub(crate) canonical_path: PathBuf,
    pub(crate) content_hash: u64,
    pub(crate) direct_imports: Vec<PathBuf>,
    pub(crate) exported_symbol_fingerprint: u64,
    pub(crate) function_body_fingerprint: u64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct IncrementalModuleNode {
    pub(crate) canonical_path: PathBuf,
    pub(crate) direct_imports: Vec<PathBuf>,
    pub(crate) reverse_deps: Vec<PathBuf>,
    pub(crate) exported_symbol_fingerprint: u64,
    pub(crate) function_body_fingerprint: u64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct IncrementalDependencyGraph {
    pub(crate) nodes: Vec<IncrementalModuleNode>,
    pub(crate) fingerprint: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct ArtifactKeyInputs {
    pub(crate) modules: Vec<ModuleFingerprint>,
    pub(crate) dependency_graph: IncrementalDependencyGraph,
    pub(crate) entry_content_hash: u64,
    pub(crate) import_fingerprint: u64,
    pub(crate) opt_level: OptLevel,
    pub(crate) phase_ordering_mode: String,
    pub(crate) type_cfg: TypeConfig,
    pub(crate) parallel_cfg: ParallelConfig,
    pub(crate) options: IncrementalOptions,
    pub(crate) output_options: CompileOutputOptions,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct StoredBuildMeta {
    pub(crate) entry_content_hash: u64,
    pub(crate) import_fingerprint: u64,
    pub(crate) opt_level: String,
    pub(crate) phase_ordering_mode: String,
    pub(crate) type_mode: String,
    pub(crate) native_backend: String,
    pub(crate) parallel_mode: String,
    pub(crate) parallel_backend: String,
    pub(crate) parallel_threads: usize,
    pub(crate) parallel_min_trip: usize,
    pub(crate) inject_runtime: bool,
    pub(crate) preserve_all_defs: bool,
    pub(crate) strict_let: bool,
    pub(crate) warn_implicit_decl: bool,
    pub(crate) compile_mode: String,
    pub(crate) phase2: bool,
    pub(crate) strict_verify: bool,
}
