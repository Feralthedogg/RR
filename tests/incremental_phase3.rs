mod common;

use RR::compiler::{
    IncrementalOptions, IncrementalSession, OptLevel, compile_with_configs_incremental,
    parallel_config_from_env, type_config_from_env,
};
use common::unique_dir;
use std::fs;
use std::path::PathBuf;

#[test]
fn incremental_phase3_reuses_in_memory_session_cache() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("incremental_phase3");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    // SAFETY: Scoped test setup; value is removed at the end of this test.
    unsafe {
        std::env::set_var("RR_INCREMENTAL_CACHE_DIR", &cache_dir);
    }

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn main() {
  print(123L);
}
main();
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");

    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = type_config_from_env();
    let parallel_cfg = parallel_config_from_env();
    let opts = IncrementalOptions {
        enabled: true,
        phase1: false,
        phase2: false,
        phase3: true,
        strict_verify: false,
    };
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
    .expect("phase3 first compile failed");
    assert!(
        !first.stats.phase3_memory_hit,
        "first compile should populate phase3 memory cache"
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
    .expect("phase3 second compile failed");
    assert!(
        second.stats.phase3_memory_hit,
        "second compile should hit phase3 memory cache"
    );
    assert_eq!(first.r_code, second.r_code);
    // SAFETY: Paired with scoped set_var above to restore environment state.
    unsafe {
        std::env::remove_var("RR_INCREMENTAL_CACHE_DIR");
    }
}
