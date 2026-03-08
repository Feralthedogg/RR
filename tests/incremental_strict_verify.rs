mod common;

use RR::compiler::{
    CompileOutputOptions, IncrementalOptions, IncrementalSession, OptLevel,
    compile_with_configs_incremental, compile_with_configs_incremental_with_output_options,
    parallel_config_from_env, type_config_from_env,
};
use common::unique_dir;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn strict_opts() -> IncrementalOptions {
    IncrementalOptions {
        enabled: true,
        phase1: true,
        phase2: true,
        phase3: true,
        strict_verify: true,
    }
}

fn phase1_only_opts() -> IncrementalOptions {
    IncrementalOptions {
        enabled: true,
        phase1: true,
        phase2: false,
        phase3: false,
        strict_verify: false,
    }
}

fn write_basic_project(root: &Path) -> (PathBuf, &'static str) {
    let main_path = root.join("main.rr");
    let source = r#"
fn main() {
  x = c(1L, 2L, 3L)
  i = 1L
  while (i <= 3L) {
    print(x[i])
    i = i + 1L
  }
}
main()
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");
    (main_path, source)
}

fn map_artifacts(cache_dir: &Path) -> Vec<PathBuf> {
    let artifact_dir = cache_dir.join("artifacts");
    let mut map_files: Vec<PathBuf> = fs::read_dir(&artifact_dir)
        .expect("missing incremental artifact dir")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("map"))
        .collect();
    map_files.sort();
    assert!(
        !map_files.is_empty(),
        "expected at least one .map artifact in {}",
        artifact_dir.display()
    );
    map_files
}

#[test]
fn strict_incremental_verify_checks_cached_outputs() {
    let _guard = env_lock()
        .lock()
        .expect("failed to lock incremental strict verify env guard");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_strict_verify");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    // SAFETY: Scoped test setup; value is removed at the end of this test.
    unsafe {
        std::env::set_var("RR_INCREMENTAL_CACHE_DIR", &cache_dir);
    }

    let (main_path, source) = write_basic_project(&proj_dir);
    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = type_config_from_env();
    let parallel_cfg = parallel_config_from_env();
    let opts = strict_opts();

    let mut session = IncrementalSession::default();
    let first = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        Some(&mut session),
    )
    .expect("strict verify first compile failed");
    assert!(
        !first.stats.strict_verification_checked,
        "first strict compile should not claim a cache comparison"
    );
    assert!(
        !first.stats.strict_verification_passed,
        "first strict compile should not pass without a cached baseline"
    );

    let second = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        Some(&mut session),
    )
    .expect("strict verify second compile failed");
    assert!(
        second.stats.phase3_memory_hit,
        "strict verify should compare against the phase3 memory artifact when available"
    );
    assert!(
        second.stats.phase1_artifact_hit,
        "strict verify should compare against the phase1 disk artifact when available"
    );
    assert!(second.stats.strict_verification_checked);
    assert!(second.stats.strict_verification_passed);
    assert_eq!(first.r_code, second.r_code);
    assert!(
        !first.source_map.is_empty(),
        "strict verify fixture should produce a non-empty source map"
    );
    assert_eq!(first.source_map, second.source_map);

    // SAFETY: Paired with scoped set_var above to restore environment state.
    unsafe {
        std::env::remove_var("RR_INCREMENTAL_CACHE_DIR");
    }
}

#[test]
fn strict_incremental_verify_rejects_source_map_drift() {
    let _guard = env_lock()
        .lock()
        .expect("failed to lock incremental strict verify env guard");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_strict_verify");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    // SAFETY: Scoped test setup; value is removed at the end of this test.
    unsafe {
        std::env::set_var("RR_INCREMENTAL_CACHE_DIR", &cache_dir);
    }

    let (main_path, source) = write_basic_project(&proj_dir);
    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = type_config_from_env();
    let parallel_cfg = parallel_config_from_env();
    let opts = strict_opts();

    compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        None,
    )
    .expect("strict verify seed compile failed");

    for map_path in map_artifacts(&cache_dir) {
        fs::write(&map_path, "").expect("failed to corrupt incremental source map");
    }

    let err = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        None,
    )
    .expect_err("strict verify should reject source map drift");
    assert!(
        err.message.contains("source map mismatch"),
        "unexpected strict verify error: {}",
        err.message
    );

    // SAFETY: Paired with scoped set_var above to restore environment state.
    unsafe {
        std::env::remove_var("RR_INCREMENTAL_CACHE_DIR");
    }
}

#[test]
fn incremental_cache_separates_runtime_injection_mode() {
    let _guard = env_lock()
        .lock()
        .expect("failed to lock incremental strict verify env guard");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_strict_verify");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "output_mode");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    // SAFETY: Scoped test setup; value is removed at the end of this test.
    unsafe {
        std::env::set_var("RR_INCREMENTAL_CACHE_DIR", &cache_dir);
    }

    let (main_path, source) = write_basic_project(&proj_dir);
    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = type_config_from_env();
    let parallel_cfg = parallel_config_from_env();
    let opts = phase1_only_opts();

    let helper_only = compile_with_configs_incremental_with_output_options(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        CompileOutputOptions {
            inject_runtime: false,
        },
        None,
    )
    .expect("helper-only compile failed");
    assert!(
        !helper_only.stats.phase1_artifact_hit,
        "first helper-only compile should seed phase1 cache"
    );
    assert!(
        !helper_only.r_code.contains("rr_set_source(\""),
        "helper-only output should omit runtime bootstrap"
    );

    let runtime_injected = compile_with_configs_incremental_with_output_options(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        CompileOutputOptions {
            inject_runtime: true,
        },
        None,
    )
    .expect("runtime-injected compile failed");
    assert!(
        !runtime_injected.stats.phase1_artifact_hit,
        "phase1 artifact key must distinguish runtime injection mode"
    );
    assert!(
        runtime_injected.r_code.contains("rr_set_source(\""),
        "runtime-injected output should include runtime bootstrap"
    );

    let runtime_cached = compile_with_configs_incremental_with_output_options(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        CompileOutputOptions {
            inject_runtime: true,
        },
        None,
    )
    .expect("cached runtime-injected compile failed");
    assert!(
        runtime_cached.stats.phase1_artifact_hit,
        "same output mode should reuse phase1 artifact cache"
    );

    // SAFETY: Paired with scoped set_var above to restore environment state.
    unsafe {
        std::env::remove_var("RR_INCREMENTAL_CACHE_DIR");
    }
}
