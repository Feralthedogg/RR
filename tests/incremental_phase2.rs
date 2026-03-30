mod common;

use RR::compiler::{
    CompilerParallelConfig, CompilerParallelMode, IncrementalOptions, OptLevel,
    compile_with_configs_incremental,
    compile_with_configs_incremental_with_output_options_and_compiler_parallel,
    default_parallel_config, default_type_config,
};
use common::unique_dir;
use std::fs;
use std::path::PathBuf;

#[test]
fn incremental_phase2_reuses_function_emit_cache() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("incremental_phase2");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn square_phase2_cache(x) {
  return x * x
}

fn bump_phase2_cache(x) {
  return x + 1L
}

fn main() {
  let a = square_phase2_cache(3L)
  print(bump_phase2_cache(a))
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
        phase2: true,
        phase3: false,
        strict_verify: false,
    };

    let first = compile_with_configs_incremental(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        opts,
        None,
    )
    .expect("phase2 first compile failed");
    assert!(
        first.stats.phase2_emit_misses > 0,
        "first compile should populate function emit cache"
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
    .expect("phase2 second compile failed");
    assert!(
        second.stats.phase2_emit_hits > 0,
        "second compile should reuse function emit cache"
    );
    assert!(
        second.stats.phase2_emit_hits >= first.stats.phase2_emit_hits,
        "second compile should not reduce emit cache hits"
    );
    assert_eq!(first.r_code, second.r_code);
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn incremental_phase2_reuses_emit_cache_under_compiler_parallel_scheduler() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_phase2_parallel");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn square_phase2_parallel_cache(x) {
  return x * x
}

fn main() {
  print(square_phase2_parallel_cache(4L))
}
main()
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");

    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = default_type_config();
    let parallel_cfg = default_parallel_config();
    let compiler_parallel_cfg = CompilerParallelConfig {
        mode: CompilerParallelMode::On,
        threads: 2,
        min_functions: 1,
        min_fn_ir: 1,
        max_jobs: 0,
    };

    let opts = IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: false,
        phase2: true,
        phase3: false,
        strict_verify: false,
    };

    let first = compile_with_configs_incremental_with_output_options_and_compiler_parallel(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        opts,
        RR::compiler::CompileOutputOptions::default(),
        None,
    )
    .expect("phase2 parallel first compile failed");
    let second = compile_with_configs_incremental_with_output_options_and_compiler_parallel(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        opts,
        RR::compiler::CompileOutputOptions::default(),
        None,
    )
    .expect("phase2 parallel second compile failed");

    assert!(first.stats.phase2_emit_misses > 0);
    assert!(second.stats.phase2_emit_hits > 0);
    assert_eq!(first.r_code, second.r_code);
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn incremental_phase2_parallel_scheduler_reuses_emit_cache_for_many_helpers() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_phase2_parallel_many");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn square_phase2_parallel_many(x) {
  return x * x
}

fn bump_phase2_parallel_many(x) {
  return x + 1L
}

fn scale_phase2_parallel_many(x) {
  return x * 2L
}

fn shift_phase2_parallel_many(x) {
  return x + 3L
}

fn mix_phase2_parallel_many(x) {
  return shift_phase2_parallel_many(scale_phase2_parallel_many(bump_phase2_parallel_many(square_phase2_parallel_many(x))))
}

fn series_phase2_parallel_many(n) {
  let out = numeric(n)
  let i = 1L
  while (i <= n) {
    out[i] = mix_phase2_parallel_many(i)
    i = i + 1L
  }
  return out
}

fn main() {
  print(series_phase2_parallel_many(6L))
}
main()
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");

    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = default_type_config();
    let parallel_cfg = default_parallel_config();
    let compiler_parallel_cfg = CompilerParallelConfig {
        mode: CompilerParallelMode::On,
        threads: 3,
        min_functions: 1,
        min_fn_ir: 1,
        max_jobs: 0,
    };

    let opts = IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: false,
        phase2: true,
        phase3: false,
        strict_verify: false,
    };

    let first = compile_with_configs_incremental_with_output_options_and_compiler_parallel(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        opts,
        RR::compiler::CompileOutputOptions::default(),
        None,
    )
    .expect("phase2 parallel-many first compile failed");
    let second = compile_with_configs_incremental_with_output_options_and_compiler_parallel(
        &path_str,
        source,
        OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        opts,
        RR::compiler::CompileOutputOptions::default(),
        None,
    )
    .expect("phase2 parallel-many second compile failed");

    assert!(
        first.stats.phase2_emit_misses > 3,
        "expected multiple cache misses on first compile, got {:?}",
        first.stats
    );
    assert!(
        second.stats.phase2_emit_hits > 3,
        "expected multiple cache hits on second compile, got {:?}",
        second.stats
    );
    assert_eq!(first.r_code, second.r_code);
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}
