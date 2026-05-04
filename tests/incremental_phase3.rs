mod common;

use common::unique_dir;
use rr::compiler::{
    CompileOutputOptions, IncrementalCompileRequest, IncrementalOptions, IncrementalSession,
    OptLevel, compile_incremental_request, compile_with_configs_incremental,
    default_compiler_parallel_config, default_parallel_config, default_type_config,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn incremental_phase3_reuses_in_memory_session_cache() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("incremental_phase3");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
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
    let opts = IncrementalOptions {
        enabled: true,
        auto: false,
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
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn incremental_phase3_hit_refreshes_latest_build_metadata() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("incremental_phase3");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "meta_hit");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn main() {
  print(456L)
}
main()
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");

    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = default_type_config();
    let parallel_cfg = default_parallel_config();
    let opts = IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: false,
        phase2: false,
        phase3: true,
        strict_verify: false,
    };
    let mut session = IncrementalSession::default();

    compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        Some(&mut session),
    )
    .expect("phase3 O1 seed compile failed");

    let o2 = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O2,
        type_cfg,
        parallel_cfg,
        opts,
        Some(&mut session),
    )
    .expect("phase3 O2 compile failed");
    assert!(
        o2.stats
            .miss_reasons
            .contains(&"opt_level_changed".to_string())
    );

    let o1_hit = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        Some(&mut session),
    )
    .expect("phase3 O1 hit compile failed");
    assert!(o1_hit.stats.phase3_memory_hit);

    let output_changed = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg: default_compiler_parallel_config(),
        options: opts,
        output_options: CompileOutputOptions {
            inject_runtime: false,
            ..Default::default()
        },
        session: Some(&mut session),
        profile: None,
    })
    .expect("phase3 output-option change compile failed");
    assert_eq!(
        output_changed.stats.miss_reasons,
        vec!["output_options_changed".to_string()],
        "phase3 hits should refresh latest-build metadata before later miss diagnostics"
    );

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn incremental_phase3_keeps_o3_and_oz_memory_artifacts_distinct() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("incremental_phase3");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "o3_oz_memory");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn helper(x) {
  let y = x + 1L
  return y * y
}

fn main() {
  print(helper(4L))
}
main()
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");

    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = default_type_config();
    let parallel_cfg = default_parallel_config();
    let opts = IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: false,
        phase2: false,
        phase3: true,
        strict_verify: false,
    };
    let mut session = IncrementalSession::default();

    let o3_seed = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O3,
        type_cfg,
        parallel_cfg,
        opts,
        Some(&mut session),
    )
    .expect("phase3 O3 seed compile failed");
    assert!(
        !o3_seed.stats.phase3_memory_hit,
        "first O3 compile should populate the phase3 memory cache"
    );

    let oz = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::Oz,
        type_cfg,
        parallel_cfg,
        opts,
        Some(&mut session),
    )
    .expect("phase3 Oz compile failed");
    assert!(
        !oz.stats.phase3_memory_hit,
        "Oz must not reuse the O3 phase3 artifact"
    );
    assert!(
        oz.stats
            .miss_reasons
            .contains(&"opt_level_changed".to_string()),
        "expected O3 -> Oz to miss by opt level, got {:?}",
        oz.stats.miss_reasons
    );

    let o3_hit = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O3,
        type_cfg,
        parallel_cfg,
        opts,
        Some(&mut session),
    )
    .expect("phase3 O3 hit compile failed");
    assert!(
        o3_hit.stats.phase3_memory_hit,
        "returning to O3 should hit only the O3 phase3 artifact"
    );

    let oz_hit = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::Oz,
        type_cfg,
        parallel_cfg,
        opts,
        Some(&mut session),
    )
    .expect("phase3 Oz hit compile failed");
    assert!(
        oz_hit.stats.phase3_memory_hit,
        "returning to Oz should hit only the Oz phase3 artifact"
    );

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}
