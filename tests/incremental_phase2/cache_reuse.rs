use rr::compiler::{
    CompileOutputOptions, CompileProfile, CompilerParallelConfig, CompilerParallelMode,
    IncrementalCompileRequest, IncrementalOptions, OptLevel, compile_incremental_request,
    compile_with_configs_incremental, default_parallel_config, default_type_config,
};
use std::fs;
use std::path::{Path, PathBuf};

use super::common::{self, unique_dir};

pub(crate) fn clear_whole_output_emit_caches(cache_dir: &Path) {
    let fn_cache_dir = cache_dir.join("function-emits");
    let entries = fs::read_dir(&fn_cache_dir).expect("function cache dir should exist");
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext == "Rraw" || ext == "Rpee" || ext == "linemap")
        {
            fs::remove_file(&path).expect("failed to remove whole-output emit cache");
        }
    }
}

pub(crate) fn clear_peephole_emit_caches(cache_dir: &Path) {
    let fn_cache_dir = cache_dir.join("function-emits");
    let entries = fs::read_dir(&fn_cache_dir).expect("function cache dir should exist");
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext == "Rpee" || ext == "linemap" || ext == "pee.meta")
        {
            let _ = fs::remove_file(&path);
        }
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".pee.meta"))
        {
            let _ = fs::remove_file(&path);
        }
    }
}

pub(crate) fn clear_optimized_fragment_caches(cache_dir: &Path) {
    let fn_cache_dir = cache_dir.join("function-emits");
    let entries = fs::read_dir(&fn_cache_dir).expect("function cache dir should exist");
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext == "Roptfn" || ext == "optmap")
        {
            fs::remove_file(&path).expect("failed to remove optimized fragment cache");
        }
    }
}

pub(crate) fn corrupt_optimized_fragment_caches(cache_dir: &Path) {
    let fn_cache_dir = cache_dir.join("function-emits");
    let entries = fs::read_dir(&fn_cache_dir).expect("function cache dir should exist");
    let mut corrupted_any = false;
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext == "Roptfn" || ext == "optmap")
        {
            fs::write(&path, "corrupted").expect("failed to corrupt optimized fragment cache");
            corrupted_any = true;
        }
    }
    assert!(
        corrupted_any,
        "expected at least one optimized fragment cache artifact in {}",
        fn_cache_dir.display()
    );
}

pub(crate) fn corrupt_function_emit_caches(cache_dir: &Path) {
    let fn_cache_dir = cache_dir.join("function-emits");
    let entries = fs::read_dir(&fn_cache_dir).expect("function cache dir should exist");
    let mut corrupted_any = false;
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext == "Rfn" || ext == "map")
        {
            fs::write(&path, "corrupted").expect("failed to corrupt function emit cache");
            corrupted_any = true;
        }
    }
    assert!(
        corrupted_any,
        "expected at least one function emit cache artifact in {}",
        fn_cache_dir.display()
    );
}

pub(crate) fn clear_optimized_reuse_artifacts(cache_dir: &Path) {
    let fn_cache_dir = cache_dir.join("function-emits");
    let entries = fs::read_dir(&fn_cache_dir).expect("function cache dir should exist");
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.ends_with(".Roptfn")
                    || name.ends_with(".optmap")
                    || name.ends_with(".optfrag.meta")
                    || name.ends_with(".Roptasm")
                    || name.ends_with(".optasm.map")
                    || name.ends_with(".optasm.meta")
                    || name.ends_with(".optfinal.map")
                    || name.ends_with(".optok")
                    || name.ends_with(".optrawok")
                    || name.ends_with(".optpeeok")
            })
        {
            let _ = fs::remove_file(&path);
        }
    }
}

pub(crate) fn corrupt_raw_rewrite_caches(cache_dir: &Path) {
    let fn_cache_dir = cache_dir.join("function-emits");
    let entries = fs::read_dir(&fn_cache_dir).expect("function cache dir should exist");
    let mut corrupted_any = false;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "Rraw") {
            fs::write(&path, "corrupted-raw\n").expect("failed to corrupt raw rewrite cache");
            corrupted_any = true;
        }
    }
    assert!(
        corrupted_any,
        "expected at least one raw rewrite cache artifact in {}",
        fn_cache_dir.display()
    );
}

pub(crate) fn corrupt_peephole_caches(cache_dir: &Path) {
    let fn_cache_dir = cache_dir.join("function-emits");
    let entries = fs::read_dir(&fn_cache_dir).expect("function cache dir should exist");
    let mut corrupted_any = false;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "Rpee") {
            fs::write(&path, "corrupted-peephole\n").expect("failed to corrupt peephole cache");
            corrupted_any = true;
        }
    }
    assert!(
        corrupted_any,
        "expected at least one peephole cache artifact in {}",
        fn_cache_dir.display()
    );
}

#[test]
pub(crate) fn incremental_phase2_reuses_function_emit_cache() {
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
pub(crate) fn incremental_phase2_reuses_emit_cache_under_compiler_parallel_scheduler() {
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

    let first = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: rr::compiler::CompileOutputOptions::default(),
        session: None,
        profile: None,
    })
    .expect("phase2 parallel first compile failed");
    let second = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: rr::compiler::CompileOutputOptions::default(),
        session: None,
        profile: None,
    })
    .expect("phase2 parallel second compile failed");

    assert!(first.stats.phase2_emit_misses > 0);
    assert!(second.stats.phase2_emit_hits > 0);
    assert_eq!(first.r_code, second.r_code);
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
pub(crate) fn incremental_phase2_parallel_scheduler_reuses_emit_cache_for_many_helpers() {
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

    let first = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: rr::compiler::CompileOutputOptions::default(),
        session: None,
        profile: None,
    })
    .expect("phase2 parallel-many first compile failed");
    let second = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O1,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: rr::compiler::CompileOutputOptions::default(),
        session: None,
        profile: None,
    })
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

#[test]
pub(crate) fn incremental_phase2_seeds_optimized_fragment_cache() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_phase2_optimized_fragments");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn square_phase2_optfrag(x) {
  return x * x
}

fn bump_phase2_optfrag(x) {
  return x + 1L
}

fn main() {
  let a = square_phase2_optfrag(3L)
  print(bump_phase2_optfrag(a))
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
    .expect("phase2 optimized-fragment first compile failed");
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
    .expect("phase2 optimized-fragment second compile failed");

    assert!(second.stats.phase2_emit_hits > 0);
    assert_eq!(first.r_code, second.r_code);
    let fn_cache_dir = cache_dir.join("function-emits");
    let optimized_fragments: Vec<_> = fs::read_dir(&fn_cache_dir)
        .expect("function cache dir should exist")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "Roptfn"))
        .collect();
    assert!(
        !optimized_fragments.is_empty(),
        "expected optimized fragment artifacts in {}",
        fn_cache_dir.display()
    );
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
pub(crate) fn incremental_phase2_trivial_program_uses_optimized_fragment_fast_path() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_phase2_optimized_fast_path");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn main() {
  print(1L)
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
    .expect("phase2 optimized fast-path first compile failed");
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
    .expect("phase2 optimized fast-path second compile failed");

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
    .expect("phase2 optimized fast-path third compile failed");

    assert!(third.stats.phase2_emit_hits > 0);
    assert_eq!(first.r_code, third.r_code);
    assert!(third_profile.tachyon.optimized_mir_cache_hit);
    assert_eq!(third_profile.emit.breakdown.raw_rewrite_elapsed_ns, 0);
    assert_eq!(third_profile.emit.breakdown.peephole_elapsed_ns, 0);
    let direct_markers: Vec<_> = fs::read_dir(cache_dir.join("function-emits"))
        .expect("function cache dir should exist")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "optok"))
        .collect();
    assert!(
        !direct_markers.is_empty(),
        "expected direct optimized assembly safety marker in {}",
        cache_dir.join("function-emits").display()
    );
    clear_whole_output_emit_caches(&cache_dir);

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
    .expect("phase2 optimized fast-path fourth compile failed");

    assert!(third.stats.phase2_emit_hits > 0);
    assert_eq!(first.r_code, third.r_code);
    assert_eq!(third_profile.emit.breakdown.raw_rewrite_elapsed_ns, 0);
    assert_eq!(third_profile.emit.breakdown.peephole_elapsed_ns, 0);
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
pub(crate) fn incremental_phase2_direct_marker_survives_optimized_fragment_cache_reseed() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_phase2_optimized_reseed_fast_path");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn main() {
  print(1L)
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
    .expect("phase2 optimized reseed fast-path first compile failed");
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
    .expect("phase2 optimized reseed fast-path second compile failed");
    assert!(second.stats.phase2_emit_hits > 0);

    let direct_markers: Vec<_> = fs::read_dir(cache_dir.join("function-emits"))
        .expect("function cache dir should exist")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "optok"))
        .collect();
    assert!(
        !direct_markers.is_empty(),
        "expected direct optimized assembly safety marker in {}",
        cache_dir.join("function-emits").display()
    );

    clear_whole_output_emit_caches(&cache_dir);
    clear_optimized_fragment_caches(&cache_dir);

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
    .expect("phase2 optimized reseed fast-path third compile failed");

    assert!(third.stats.phase2_emit_hits > 0);
    assert_eq!(first.r_code, third.r_code);
    assert!(
        third_profile.emit.breakdown.optimized_fragment_cache_misses > 0,
        "expected optimized fragment cache reseed on third compile"
    );
    assert_eq!(third_profile.emit.breakdown.raw_rewrite_elapsed_ns, 0);
    assert_eq!(third_profile.emit.breakdown.peephole_elapsed_ns, 0);
    assert!(
        third_profile
            .emit
            .breakdown
            .optimized_fragment_fast_path_direct_hits
            + third_profile
                .emit
                .breakdown
                .optimized_fragment_final_artifact_hits
            > 0,
        "expected direct fragment or optimized final artifact hit on reseed compile"
    );
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
pub(crate) fn incremental_phase2_optimized_mir_cache_tracks_phase_ordering() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("incremental_phase2");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "phase_ordering_optimized_mir");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn helper(x) {
  return x + 1L
}

fn main() {
  let x = seq_len(8L)
  let y = seq_len(8L)
  for (i in 1..length(x)) {
    y[i] = helper(x[i])
  }
  print(sum(y))
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

    common::set_env_var_for_test(&env_guard, "RR_PHASE_ORDERING", "balanced");
    let mut first_profile = CompileProfile::default();
    compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O2,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: output_opts,
        session: None,
        profile: Some(&mut first_profile),
    })
    .expect("balanced phase ordering compile failed");
    assert!(
        !first_profile.tachyon.optimized_mir_cache_hit,
        "first compile should seed, not hit, the optimized MIR cache"
    );

    common::set_env_var_for_test(&env_guard, "RR_PHASE_ORDERING", "off");
    let mut second_profile = CompileProfile::default();
    let second = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O2,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: output_opts,
        session: None,
        profile: Some(&mut second_profile),
    })
    .expect("changed phase ordering compile failed");
    assert!(
        !second_profile.tachyon.optimized_mir_cache_hit,
        "phase-ordering changes must not reuse an optimized MIR artifact from a different mode"
    );
    assert!(
        second
            .stats
            .miss_reasons
            .contains(&"phase_ordering_changed".to_string()),
        "expected phase_ordering_changed miss reason, got {:?}",
        second.stats.miss_reasons
    );

    let mut third_profile = CompileProfile::default();
    compile_incremental_request(IncrementalCompileRequest {
        entry_path: &path_str,
        entry_input: source,
        opt_level: OptLevel::O2,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        options: opts,
        output_options: output_opts,
        session: None,
        profile: Some(&mut third_profile),
    })
    .expect("same phase ordering recompile failed");
    assert!(
        third_profile.tachyon.optimized_mir_cache_hit,
        "same phase ordering should reuse its own optimized MIR artifact"
    );

    common::remove_env_var_for_test(&env_guard, "RR_PHASE_ORDERING");
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
pub(crate) fn incremental_phase2_corrupted_optimized_fragment_cache_falls_back_and_reseeds() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("incremental_phase2_corrupt_optfrag");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn main() {
  print(1L)
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
    .expect("phase2 corrupted optimized fragment first compile failed");
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
    .expect("phase2 corrupted optimized fragment second compile failed");
    assert!(second.stats.phase2_emit_hits > 0);

    corrupt_optimized_fragment_caches(&cache_dir);
    clear_whole_output_emit_caches(&cache_dir);

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
    .expect("phase2 corrupted optimized fragment third compile failed");

    assert!(third.stats.phase2_emit_hits > 0);
    assert_eq!(first.r_code, third.r_code);
    assert!(
        third_profile.emit.breakdown.optimized_fragment_cache_misses > 0,
        "expected corrupted optimized fragment cache to fall back to a miss"
    );
    assert_eq!(third_profile.emit.breakdown.peephole_elapsed_ns, 0);
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}
