mod common;

use common::unique_dir;
use rr::compiler::{
    CompileOutputOptions, CompilerParallelConfig, CompilerParallelMode, IncrementalCompileRequest,
    IncrementalOptions, IncrementalSession, OptLevel, compile_incremental_request,
    compile_with_configs_incremental, default_parallel_config, default_type_config,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::MutexGuard;

fn lock_env_guard() -> MutexGuard<'static, ()> {
    common::env_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

fn strict_opts() -> IncrementalOptions {
    IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: true,
        phase2: true,
        phase3: true,
        strict_verify: true,
    }
}

fn phase1_only_opts() -> IncrementalOptions {
    IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: true,
        phase2: false,
        phase3: false,
        strict_verify: false,
    }
}

fn compiler_parallel_cfg() -> CompilerParallelConfig {
    CompilerParallelConfig {
        mode: CompilerParallelMode::On,
        threads: 2,
        min_functions: 1,
        min_fn_ir: 1,
        max_jobs: 2,
    }
}

fn write_basic_project(root: &Path) -> (PathBuf, &'static str) {
    let main_path = root.join("main.rr");
    let source = r#"
fn main() {
  let x = c(1L, 2L, 3L)
  let i = 1L
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

fn code_artifacts(cache_dir: &Path) -> Vec<PathBuf> {
    let artifact_dir = cache_dir.join("artifacts");
    let mut code_files: Vec<PathBuf> = fs::read_dir(&artifact_dir)
        .expect("missing incremental artifact dir")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("R"))
        .collect();
    code_files.sort();
    assert!(
        !code_files.is_empty(),
        "expected at least one .R artifact in {}",
        artifact_dir.display()
    );
    code_files
}

#[test]
fn strict_incremental_verify_checks_cached_outputs() {
    let env_guard = lock_env_guard();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_strict_verify");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let (main_path, source) = write_basic_project(&proj_dir);
    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = default_type_config();
    let parallel_cfg = default_parallel_config();
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
    assert!(
        second.stats.phase2_emit_hits > 0,
        "strict verify should still reuse phase2 emit cache during verification"
    );
    assert!(second.stats.strict_verification_checked);
    assert!(second.stats.strict_verification_passed);
    assert_eq!(first.r_code, second.r_code);
    assert!(
        !first.source_map.is_empty(),
        "strict verify fixture should produce a non-empty source map"
    );
    assert_eq!(first.source_map, second.source_map);

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn strict_incremental_verify_rejects_source_map_drift() {
    let env_guard = lock_env_guard();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_strict_verify");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let (main_path, source) = write_basic_project(&proj_dir);
    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = default_type_config();
    let parallel_cfg = default_parallel_config();
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
    assert!(
        err.notes
            .iter()
            .any(|note| note.contains("incremental cache root:")),
        "strict verify error should mention cache root:\n{err:?}"
    );
    assert!(
        err.helps
            .iter()
            .any(|help| help.contains("--no-incremental")),
        "strict verify error should suggest --no-incremental:\n{err:?}"
    );
    assert!(
        err.fixes
            .iter()
            .any(|fix| fix.message.contains("clear the incremental cache")),
        "strict verify error should suggest clearing the cache:\n{err:?}"
    );

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn strict_incremental_verify_checks_cached_outputs_under_compiler_parallel() {
    let env_guard = lock_env_guard();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_strict_verify");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "compiler_parallel_strict");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let (main_path, source) = write_basic_project(&proj_dir);
    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = default_type_config();
    let parallel_cfg = default_parallel_config();
    let opts = strict_opts();
    let mut session = IncrementalSession::default();

    let first = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg: compiler_parallel_cfg(),
        options: opts,
        output_options: CompileOutputOptions::default(),
        session: Some(&mut session),
        profile: None,
    })
    .expect("strict verify first compiler-parallel compile failed");
    assert!(
        !first.stats.strict_verification_checked,
        "first strict compiler-parallel compile should not claim a cache comparison"
    );

    let second = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg: compiler_parallel_cfg(),
        options: opts,
        output_options: CompileOutputOptions::default(),
        session: Some(&mut session),
        profile: None,
    })
    .expect("strict verify second compiler-parallel compile failed");
    assert!(second.stats.phase3_memory_hit);
    assert!(second.stats.phase1_artifact_hit);
    assert!(
        second.stats.phase2_emit_hits > 0,
        "compiler-parallel strict verify should still reuse phase2 emits"
    );
    assert!(second.stats.strict_verification_checked);
    assert!(second.stats.strict_verification_passed);
    assert_eq!(first.r_code, second.r_code);
    assert_eq!(first.source_map, second.source_map);

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn incremental_cache_separates_runtime_injection_mode() {
    let env_guard = lock_env_guard();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_strict_verify");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "output_mode");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let (main_path, source) = write_basic_project(&proj_dir);
    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = default_type_config();
    let parallel_cfg = default_parallel_config();
    let opts = phase1_only_opts();

    let helper_only = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg: rr::compiler::CompilerParallelConfig::default(),
        options: opts,
        output_options: CompileOutputOptions {
            inject_runtime: false,
            preserve_all_defs: false,
            ..Default::default()
        },
        session: None,
        profile: None,
    })
    .expect("helper-only compile failed");
    assert!(
        !helper_only.stats.phase1_artifact_hit,
        "first helper-only compile should seed phase1 cache"
    );
    assert!(
        !helper_only.r_code.contains(".rr_env$file <- \"main.rr\";"),
        "helper-only output should omit runtime bootstrap"
    );

    let runtime_injected = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg: rr::compiler::CompilerParallelConfig::default(),
        options: opts,
        output_options: CompileOutputOptions {
            inject_runtime: true,
            preserve_all_defs: false,
            ..Default::default()
        },
        session: None,
        profile: None,
    })
    .expect("runtime-injected compile failed");
    assert!(
        !runtime_injected.stats.phase1_artifact_hit,
        "phase1 artifact key must distinguish runtime injection mode"
    );
    assert!(
        runtime_injected
            .r_code
            .contains(".rr_env$file <- \"main.rr\";"),
        "runtime-injected output should include runtime bootstrap"
    );

    let runtime_cached = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg: rr::compiler::CompilerParallelConfig::default(),
        options: opts,
        output_options: CompileOutputOptions {
            inject_runtime: true,
            preserve_all_defs: false,
            ..Default::default()
        },
        session: None,
        profile: None,
    })
    .expect("cached runtime-injected compile failed");
    assert!(
        runtime_cached.stats.phase1_artifact_hit,
        "same output mode should reuse phase1 artifact cache"
    );

    let preserve_defs = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg: rr::compiler::CompilerParallelConfig::default(),
        options: opts,
        output_options: CompileOutputOptions {
            inject_runtime: true,
            preserve_all_defs: true,
            ..Default::default()
        },
        session: None,
        profile: None,
    })
    .expect("preserve-defs compile failed");
    assert!(
        !preserve_defs.stats.phase1_artifact_hit,
        "phase1 artifact key must distinguish preserve-all-defs mode"
    );

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn malformed_incremental_source_map_surfaces_recovery_guidance() {
    let env_guard = lock_env_guard();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_strict_verify");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "malformed_map");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let (main_path, source) = write_basic_project(&proj_dir);
    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = default_type_config();
    let parallel_cfg = default_parallel_config();
    let opts = phase1_only_opts();

    compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        None,
    )
    .expect("phase1 seed compile failed");

    for map_path in map_artifacts(&cache_dir) {
        fs::write(&map_path, "this-is-not-a-valid-source-map-entry\n")
            .expect("failed to corrupt incremental source map");
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
    .expect_err("malformed incremental source map should be rejected");
    assert!(
        err.message
            .contains("failed to parse incremental source map"),
        "unexpected malformed map error: {}",
        err.message
    );
    assert!(
        err.helps
            .iter()
            .any(|help| help.contains("--no-incremental")),
        "malformed map error should suggest --no-incremental:\n{err:?}"
    );
    assert!(
        err.fixes
            .iter()
            .any(|fix| fix.message.contains("clear the incremental cache")),
        "malformed map error should suggest clearing the cache:\n{err:?}"
    );

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn unreadable_incremental_artifact_surfaces_recovery_guidance() {
    let env_guard = lock_env_guard();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_strict_verify");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "invalid_utf8_artifact");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let (main_path, source) = write_basic_project(&proj_dir);
    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = default_type_config();
    let parallel_cfg = default_parallel_config();
    let opts = phase1_only_opts();

    compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        None,
    )
    .expect("phase1 seed compile failed");

    for code_path in code_artifacts(&cache_dir) {
        fs::write(&code_path, [0xff, 0xfe, 0xfd]).expect("failed to corrupt incremental artifact");
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
    .expect_err("invalid utf8 incremental artifact should be rejected");
    assert!(
        err.message.contains("failed to read incremental artifact"),
        "unexpected unreadable artifact error: {}",
        err.message
    );
    assert!(
        err.notes
            .iter()
            .any(|note| note.contains("incremental cache root:")),
        "unreadable artifact error should mention cache root:\n{err:?}"
    );
    assert!(
        err.helps
            .iter()
            .any(|help| help.contains("--no-incremental")),
        "unreadable artifact error should suggest --no-incremental:\n{err:?}"
    );
    assert!(
        err.fixes
            .iter()
            .any(|fix| fix.message.contains("clear the incremental cache")),
        "unreadable artifact error should suggest clearing the cache:\n{err:?}"
    );

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn incremental_cache_write_failure_surfaces_recovery_guidance() {
    let env_guard = lock_env_guard();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_strict_verify");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "cache_write_failure");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_root_file = proj_dir.join("cache-root-file");
    fs::write(&cache_root_file, "not-a-directory").expect("failed to write cache root file");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_root_file);

    let (main_path, source) = write_basic_project(&proj_dir);
    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = default_type_config();
    let parallel_cfg = default_parallel_config();
    let opts = phase1_only_opts();

    let err = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        None,
    )
    .expect_err("cache-root file should reject incremental writes");
    assert!(
        err.message.contains("failed to create cache directory"),
        "unexpected incremental write failure error: {}",
        err.message
    );
    assert!(
        err.notes
            .iter()
            .any(|note| note.contains("incremental cache root:")),
        "incremental write failure should mention cache root:\n{err:?}"
    );
    assert!(
        err.helps
            .iter()
            .any(|help| help.contains("--no-incremental")),
        "incremental write failure should suggest --no-incremental:\n{err:?}"
    );
    assert!(
        err.fixes
            .iter()
            .any(|fix| fix.message.contains("clear the incremental cache")),
        "incremental write failure should suggest clearing the cache:\n{err:?}"
    );

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}
