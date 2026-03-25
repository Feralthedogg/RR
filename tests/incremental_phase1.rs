mod common;

use RR::compiler::{
    IncrementalOptions, OptLevel, compile_with_configs_incremental, module_tree_fingerprint,
    module_tree_snapshot, parallel_config_from_env, type_config_from_env,
};
use common::unique_dir;
use std::fs;
use std::path::PathBuf;

#[test]
fn compiler_build_hash_env_is_set_for_incremental_cache_keys() {
    let build_hash = option_env!("RR_COMPILER_BUILD_HASH");
    assert!(
        matches!(build_hash, Some(value) if !value.is_empty()),
        "compiler build hash must be present so compiler changes invalidate incremental caches"
    );
}

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

#[test]
fn module_tree_fingerprint_tracks_imported_module_changes() {
    let _env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("incremental_phase1");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "watch_fp");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = proj_dir.join("main.rr");
    let module_path = proj_dir.join("module.rr");
    let main_source = r#"
import "./module.rr"

fn main() {
  print(answer())
}
main()
"#;
    fs::write(&main_path, main_source).expect("failed to write main.rr");
    fs::write(
        &module_path,
        r#"
fn answer() {
  return 1L
}
"#,
    )
    .expect("failed to write module.rr");

    let first = module_tree_fingerprint(&main_path.to_string_lossy(), main_source)
        .expect("failed to compute first module tree fingerprint");
    let unchanged = module_tree_fingerprint(&main_path.to_string_lossy(), main_source)
        .expect("failed to recompute unchanged module tree fingerprint");
    assert_eq!(
        first, unchanged,
        "module tree fingerprint should remain stable when inputs are unchanged"
    );

    fs::write(
        &module_path,
        r#"
fn answer() {
  return 2L
}
"#,
    )
    .expect("failed to update module.rr");

    let changed = module_tree_fingerprint(&main_path.to_string_lossy(), main_source)
        .expect("failed to compute changed module tree fingerprint");
    assert_ne!(
        first, changed,
        "module tree fingerprint should change when an imported module changes"
    );
}

#[test]
fn module_tree_snapshot_includes_imported_modules() {
    let _env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("incremental_phase1");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "snapshot");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = proj_dir.join("main.rr");
    let module_path = proj_dir.join("module.rr");
    let main_source = r#"
import "./module.rr"

fn main() {
  print(answer())
}
main()
"#;
    fs::write(&main_path, main_source).expect("failed to write main.rr");
    fs::write(
        &module_path,
        r#"
fn answer() {
  return 1L
}
"#,
    )
    .expect("failed to write module.rr");

    let snapshot = module_tree_snapshot(&main_path.to_string_lossy(), main_source)
        .expect("failed to collect module tree snapshot");
    let mut names: Vec<String> = snapshot
        .iter()
        .filter_map(|(path, _)| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_string())
        })
        .collect();
    names.sort();
    assert_eq!(names, vec!["main.rr".to_string(), "module.rr".to_string()]);
}
