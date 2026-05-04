mod common;

use common::unique_dir;
use rr::compiler::{
    IncrementalOptions, IncrementalSession, OptLevel, compile_with_configs_incremental,
    default_parallel_config, default_type_config,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn incremental_auto_reuses_disk_artifact_without_session() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("incremental_auto");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "disk");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn helper(x) {
  return x + 1L
}

fn main() {
  print(helper(2L))
}
main()
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");

    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = default_type_config();
    let parallel_cfg = default_parallel_config();
    let opts = IncrementalOptions::auto();

    let first = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        None,
    )
    .expect("auto incremental first compile failed");
    assert!(
        !first.stats.phase1_artifact_hit,
        "first auto compile should build the phase1 artifact"
    );
    assert!(
        first.stats.phase2_emit_misses > 0,
        "auto compile should enable phase2 emit caching on rebuilds"
    );

    let second = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        None,
    )
    .expect("auto incremental second compile failed");
    assert!(
        second.stats.phase1_artifact_hit,
        "second auto compile should reuse the phase1 artifact"
    );
    assert_eq!(first.r_code, second.r_code);

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn incremental_auto_uses_phase3_when_session_is_available() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("incremental_auto");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "session");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn main() {
  print(123L)
}
main()
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");

    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = default_type_config();
    let parallel_cfg = default_parallel_config();
    let opts = IncrementalOptions::auto();
    let mut session = IncrementalSession::default();

    let first = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O0,
        type_cfg,
        parallel_cfg,
        opts,
        Some(&mut session),
    )
    .expect("auto incremental session first compile failed");
    assert!(
        !first.stats.phase3_memory_hit,
        "first auto compile should populate the phase3 memory cache"
    );

    let second = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O0,
        type_cfg,
        parallel_cfg,
        opts,
        Some(&mut session),
    )
    .expect("auto incremental session second compile failed");
    assert!(
        second.stats.phase3_memory_hit,
        "auto compile with a live session should reuse phase3 memory artifacts"
    );

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}
