mod common;

use RR::compiler::{
    IncrementalOptions, OptLevel, compile_with_configs_incremental, parallel_config_from_env,
    type_config_from_env,
};
use common::unique_dir;
use std::fs;
use std::path::PathBuf;

#[test]
fn incremental_phase1_reuses_artifact_when_inputs_unchanged() {
    let _env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("incremental_phase1");
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
fn helper(x) {
  return x + 1L
}

fn main() {
  let y = helper(2L)
  print(y)
}
main()
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");

    let opts = IncrementalOptions::phase1_only();
    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = type_config_from_env();
    let parallel_cfg = parallel_config_from_env();

    let first = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        None,
    )
    .expect("phase1 first compile failed");
    assert!(
        !first.stats.phase1_artifact_hit,
        "first compile should build artifact, not hit cache"
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
    .expect("phase1 second compile failed");
    assert!(
        second.stats.phase1_artifact_hit,
        "second compile should reuse phase1 artifact cache"
    );
    assert_eq!(first.r_code, second.r_code);
    assert_eq!(first.source_map.len(), second.source_map.len());
    // SAFETY: Paired with scoped set_var above to restore environment state.
    unsafe {
        std::env::remove_var("RR_INCREMENTAL_CACHE_DIR");
    }
}

#[test]
fn incremental_phase1_invalidates_artifact_when_imported_module_changes() {
    let _env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("incremental_phase1");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    // SAFETY: Scoped test setup; value is removed at the end of this test.
    unsafe {
        std::env::set_var("RR_INCREMENTAL_CACHE_DIR", &cache_dir);
    }

    let main_path = proj_dir.join("main.rr");
    let module_path = proj_dir.join("module.rr");
    let main_source = r#"
import "./module.rr"

fn main() {
  print(answer())
}
main()
"#;
    let first_module = r#"
fn answer() {
  return 1L
}
"#;
    let second_module = r#"
fn answer() {
  return 2L
}
"#;
    fs::write(&main_path, main_source).expect("failed to write main.rr");
    fs::write(&module_path, first_module).expect("failed to write module.rr");

    let opts = IncrementalOptions::phase1_only();
    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = type_config_from_env();
    let parallel_cfg = parallel_config_from_env();

    let first = compile_with_configs_incremental(
        &path_str,
        main_source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        None,
    )
    .expect("phase1 first compile failed");
    assert!(
        !first.stats.phase1_artifact_hit,
        "first compile should build artifact, not hit cache"
    );
    assert!(
        first.r_code.contains("return(1L)"),
        "first compile should embed the original imported module"
    );

    fs::write(&module_path, second_module).expect("failed to update module.rr");

    let second = compile_with_configs_incremental(
        &path_str,
        main_source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        None,
    )
    .expect("phase1 second compile failed");
    assert!(
        !second.stats.phase1_artifact_hit,
        "phase1 cache must invalidate when an imported module changes"
    );
    assert!(
        second.r_code.contains("return(2L)"),
        "second compile should rebuild against the updated imported module"
    );

    // SAFETY: Paired with scoped set_var above to restore environment state.
    unsafe {
        std::env::remove_var("RR_INCREMENTAL_CACHE_DIR");
    }
}
