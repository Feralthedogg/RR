use crate::codegen::mir_emit::MapEntry;
use crate::compiler::pipeline::{
    CompileOutputOptions, CompileProfile, CompileWithProfileRequest, EmitFunctionCache,
    compile_output_cache_salt, compile_with_configs_using_emit_cache_and_compiler_parallel,
    compile_with_profile_request,
};
use crate::compiler::{CompilerParallelConfig, OptLevel, ParallelConfig};
use crate::error::{InternalCompilerError, RR, RRCode, RRException, Stage};
use crate::typeck::TypeConfig;
use crate::utils::Span;
use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const CACHE_VERSION: &str = concat!("rr-incremental-v3|", env!("CARGO_PKG_VERSION"));
const IMPORT_PATTERN: &str = r#"(?m)^\s*import\s+"([^"]+)"\s*(?:#.*)?$"#;
static IMPORT_RE: OnceLock<Regex> = OnceLock::new();

#[path = "incremental/options.rs"]
mod options;
pub use self::options::*;
#[path = "incremental/emit_cache.rs"]
mod emit_cache;
pub(crate) use self::emit_cache::*;
#[path = "incremental/models.rs"]
mod models;
pub(crate) use self::models::*;
#[path = "incremental/driver.rs"]
mod driver;
pub use self::driver::*;
#[path = "incremental/profile.rs"]
mod profile;
pub(crate) use self::profile::*;
#[path = "incremental/fingerprint.rs"]
mod fingerprint;
pub(crate) use self::fingerprint::*;
#[path = "incremental/cache_key.rs"]
mod cache_key;
pub(crate) use self::cache_key::*;
#[path = "incremental/build_meta.rs"]
mod build_meta;
pub(crate) use self::build_meta::*;
#[path = "incremental/paths.rs"]
mod paths;
pub(crate) use self::paths::*;
#[path = "incremental/artifact_io.rs"]
mod artifact_io;
pub(crate) use self::artifact_io::*;
