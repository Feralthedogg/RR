mod common;

use RR::compiler::{
    IncrementalOptions, IncrementalSession, OptLevel, compile_with_configs_incremental,
    parallel_config_from_env, type_config_from_env,
};
use common::unique_dir;
use std::fs;
use std::path::PathBuf;

#[test]
fn strict_incremental_verify_checks_cached_outputs() {
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

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn id(x) {
  return x;
}
fn main() {
  print(id(7L));
}
main();
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");

    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = type_config_from_env();
    let parallel_cfg = parallel_config_from_env();
    let opts = IncrementalOptions {
        enabled: true,
        phase1: true,
        phase2: true,
        phase3: true,
        strict_verify: true,
    };

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
    assert!(first.stats.strict_verification_passed);

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
    assert!(second.stats.strict_verification_passed);
    assert_eq!(first.r_code, second.r_code);
    // SAFETY: Paired with scoped set_var above to restore environment state.
    unsafe {
        std::env::remove_var("RR_INCREMENTAL_CACHE_DIR");
    }
}
