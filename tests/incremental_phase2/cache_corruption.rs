use rr::compiler::{
    CompileOutputOptions, CompileProfile, CompilerParallelConfig, IncrementalCompileRequest,
    IncrementalOptions, OptLevel, compile_incremental_request, default_parallel_config,
    default_type_config,
};
use std::fs;
use std::path::PathBuf;

use super::cache_reuse::{
    clear_optimized_reuse_artifacts, clear_peephole_emit_caches, clear_whole_output_emit_caches,
    corrupt_function_emit_caches, corrupt_peephole_caches, corrupt_raw_rewrite_caches,
};
use super::common::{self, unique_dir};

#[test]
pub(crate) fn incremental_phase2_corrupted_function_emit_cache_falls_back_and_reseeds() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_phase2_corrupt_fn_emit");
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
    let compiler_parallel_cfg = CompilerParallelConfig::default();
    let output_opts = CompileOutputOptions::default();
    let opts = IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: false,
        phase2: true,
        phase3: false,
        strict_verify: false,
    };

    let first = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: output_opts,
        session: None,
        profile: Some(&mut CompileProfile::default()),
    })
    .expect("phase2 corrupted function emit first compile failed");
    assert!(first.stats.phase2_emit_misses > 0);

    let second = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: output_opts,
        session: None,
        profile: Some(&mut CompileProfile::default()),
    })
    .expect("phase2 corrupted function emit second compile failed");
    assert!(second.stats.phase2_emit_hits > 0);

    corrupt_function_emit_caches(&cache_dir);

    let third = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: output_opts,
        session: None,
        profile: Some(&mut CompileProfile::default()),
    })
    .expect("phase2 corrupted function emit third compile failed");

    assert!(third.stats.phase2_emit_misses > 0);
    assert_eq!(first.r_code, third.r_code);
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
pub(crate) fn incremental_phase2_corrupted_raw_rewrite_cache_falls_back_and_reseeds() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_phase2_corrupt_raw");
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
    let compiler_parallel_cfg = CompilerParallelConfig::default();
    let output_opts = CompileOutputOptions::default();
    let opts = IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: false,
        phase2: true,
        phase3: false,
        strict_verify: false,
    };

    let first = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: output_opts,
        session: None,
        profile: Some(&mut CompileProfile::default()),
    })
    .expect("phase2 corrupted raw first compile failed");
    let second = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: output_opts,
        session: None,
        profile: Some(&mut CompileProfile::default()),
    })
    .expect("phase2 corrupted raw second compile failed");
    assert!(second.stats.phase2_emit_hits > 0);

    clear_optimized_reuse_artifacts(&cache_dir);
    clear_peephole_emit_caches(&cache_dir);
    corrupt_raw_rewrite_caches(&cache_dir);

    let mut third_profile = CompileProfile::default();
    let third = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: output_opts,
        session: None,
        profile: Some(&mut third_profile),
    })
    .expect("phase2 corrupted raw third compile failed");

    assert_eq!(first.r_code, third.r_code);
    assert!(
        third_profile.emit.breakdown.raw_rewrite_elapsed_ns > 0,
        "expected raw rewrite stage to rerun after corrupted raw cache miss"
    );
    for entry in fs::read_dir(cache_dir.join("function-emits"))
        .expect("function cache dir should exist")
        .flatten()
    {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "Rraw") {
            let text = fs::read_to_string(&path).expect("failed to read reseeded raw cache");
            assert_ne!(text, "corrupted-raw\n");
        }
    }
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
pub(crate) fn incremental_phase2_corrupted_peephole_cache_falls_back_and_reseeds() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_phase2_corrupt_pee");
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
    let compiler_parallel_cfg = CompilerParallelConfig::default();
    let output_opts = CompileOutputOptions::default();
    let opts = IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: false,
        phase2: true,
        phase3: false,
        strict_verify: false,
    };

    let first = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: output_opts,
        session: None,
        profile: Some(&mut CompileProfile::default()),
    })
    .expect("phase2 corrupted peephole first compile failed");
    let second = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: output_opts,
        session: None,
        profile: Some(&mut CompileProfile::default()),
    })
    .expect("phase2 corrupted peephole second compile failed");
    assert!(second.stats.phase2_emit_hits > 0);

    clear_optimized_reuse_artifacts(&cache_dir);
    corrupt_peephole_caches(&cache_dir);

    let mut third_profile = CompileProfile::default();
    let third = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: output_opts,
        session: None,
        profile: Some(&mut third_profile),
    })
    .expect("phase2 corrupted peephole third compile failed");

    assert_eq!(first.r_code, third.r_code);
    assert!(
        third_profile.emit.breakdown.peephole_elapsed_ns > 0,
        "expected peephole stage to rerun after corrupted peephole cache miss"
    );
    for entry in fs::read_dir(cache_dir.join("function-emits"))
        .expect("function cache dir should exist")
        .flatten()
    {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "Rpee") {
            let text = fs::read_to_string(&path).expect("failed to read reseeded peephole cache");
            assert_ne!(text, "corrupted-peephole\n");
        }
    }
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
pub(crate) fn incremental_phase2_nontrivial_program_generates_optimized_peephole_marker() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_phase2_optimized_raw_fast_path");
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
    let compiler_parallel_cfg = CompilerParallelConfig::default();
    let output_opts = CompileOutputOptions::default();
    let opts = IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: false,
        phase2: true,
        phase3: false,
        strict_verify: false,
    };

    let first = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: output_opts,
        session: None,
        profile: Some(&mut CompileProfile::default()),
    })
    .expect("phase2 optimized peephole fast-path first compile failed");
    assert!(first.stats.phase2_emit_misses > 0);

    let second = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: output_opts,
        session: None,
        profile: Some(&mut CompileProfile::default()),
    })
    .expect("phase2 optimized peephole fast-path second compile failed");

    assert!(second.stats.phase2_emit_hits > 0);
    let mut third_profile = CompileProfile::default();
    let third = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: output_opts,
        session: None,
        profile: Some(&mut third_profile),
    })
    .expect("phase2 optimized peephole fast-path third compile failed");

    assert!(third.stats.phase2_emit_hits > 0);
    assert_eq!(first.r_code, third.r_code);
    assert_eq!(third_profile.emit.breakdown.raw_rewrite_elapsed_ns, 0);
    let assembly_markers: Vec<_> = fs::read_dir(cache_dir.join("function-emits"))
        .expect("function cache dir should exist")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "optpeeok"))
        .collect();
    assert!(
        !assembly_markers.is_empty(),
        "expected optimized peephole assembly safety marker in {}",
        cache_dir.join("function-emits").display()
    );
    clear_whole_output_emit_caches(&cache_dir);

    let mut fourth_profile = CompileProfile::default();
    let fourth = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: output_opts,
        session: None,
        profile: Some(&mut fourth_profile),
    })
    .expect("phase2 optimized peephole fast-path fourth compile failed");

    assert!(fourth.stats.phase2_emit_hits > 0);
    assert_eq!(first.r_code, fourth.r_code);
    assert!(fourth_profile.tachyon.optimized_mir_cache_hit);
    assert_eq!(fourth_profile.emit.breakdown.raw_rewrite_elapsed_ns, 0);
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
pub(crate) fn incremental_phase1_output_mode_miss_still_hits_optimized_mir_cache() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_phase1_optimized_mir");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn square_phase1_optmir(x) {
  return x * x
}

fn main() {
  print(square_phase1_optmir(4L))
}
main()
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");

    let path_str = main_path.to_string_lossy().to_string();
    let type_cfg = default_type_config();
    let parallel_cfg = default_parallel_config();
    let compiler_parallel_cfg = CompilerParallelConfig::default();
    let opts = IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: true,
        phase2: false,
        phase3: false,
        strict_verify: false,
    };

    let runtime_out = CompileOutputOptions {
        inject_runtime: true,
        ..CompileOutputOptions::default()
    };
    let helper_only_out = CompileOutputOptions {
        inject_runtime: false,
        ..CompileOutputOptions::default()
    };

    let first = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: runtime_out,
        session: None,
        profile: Some(&mut CompileProfile::default()),
    })
    .expect("phase1 optimized-mir first compile failed");
    assert!(!first.stats.phase1_artifact_hit);

    let mut second_profile = CompileProfile::default();
    let second = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: helper_only_out,
        session: None,
        profile: Some(&mut second_profile),
    })
    .expect("phase1 optimized-mir second compile failed");

    assert!(
        !second.stats.phase1_artifact_hit,
        "output mode change should miss final artifact cache"
    );
    assert!(
        second_profile.tachyon.optimized_mir_cache_hit,
        "expected optimized MIR cache hit on output-only cache miss"
    );
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}
