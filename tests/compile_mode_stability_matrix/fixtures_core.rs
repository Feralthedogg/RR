use super::common::{self, normalize, unique_dir};
use rr::compiler::{
    CompileMode, CompileOutputOptions, CompilerParallelConfig, CompilerParallelMode,
    IncrementalCompileRequest, IncrementalOptions, IncrementalSession, OptLevel,
    compile_incremental_request, compile_with_configs_with_options_and_compiler_parallel,
    default_parallel_config, default_type_config,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::MutexGuard;

pub(crate) const MATRIX_FIXTURE: &str = r#"
fn adjust(v, idx) {
  if v > 4.0 {
    return (v - 1.0) * idx
  }
  return (v + 2.0) * idx
}

fn build(xs) {
  let out = numeric(length(xs))
  let i = 1.0
  while i <= length(xs) {
    out[i] = adjust(xs[i], i)
    i += 1.0
  }
  return out
}

fn score(xs) {
  let ys = build(xs)
  return sum(ys) + max(ys)
}

fn main() {
  let xs = c(2.0, 5.0, 4.0, 7.0, 3.0, 6.0)
  print(score(xs))
  print(build(xs)[3.0])
}

main()
"#;

pub(crate) const VECTOR_MATRIX_FIXTURE: &str = r#"
fn main() {
  let xs = c(1.0, -2.0, 3.0, -4.0, 5.0, -6.0, 7.0, -8.0)
  let idx = c(8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0)
  let ys = rep.int(0.0, length(xs))
  let zs = rep.int(0.0, length(xs))
  let i = 1.0
  while i <= length(xs) {
    if xs[i] > 0.0 {
      ys[i] = abs(xs[i])
    } else {
      ys[i] = xs[i] + 3.0
    }
    zs[idx[i]] = ys[i] * 2.0
    i += 1.0
  }
  print(sum(ys) + sum(zs))
}

main()
"#;

pub(crate) const EXTENDED_VECTOR_MATRIX_FIXTURE: &str = r#"
fn main() {
  let xs = c(1.0, -2.0, 3.0, -4.0, 5.0, -6.0, 7.0, -8.0)
  let idx = c(8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0)
  let ys = rep.int(0.0, length(xs))
  let zs = rep.int(0.0, length(xs))
  let ws = rep.int(0.0, length(xs))
  let i = 1.0
  while i <= length(xs) {
    let a = abs(xs[i])
    let b = (a + 1.0) * 2.0
    ys[i] = b - 3.0
    zs[i] = xs[idx[i]]
    ws[idx[i]] = ys[i] + zs[i]
    i += 1.0
  }
  print(sum(ys) + sum(zs) + sum(ws))
}

main()
"#;

pub(crate) fn lock_env_guard() -> MutexGuard<'static, ()> {
    common::env_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

pub(crate) fn serial_compiler_parallel_cfg() -> CompilerParallelConfig {
    CompilerParallelConfig {
        mode: CompilerParallelMode::Off,
        ..CompilerParallelConfig::default()
    }
}

pub(crate) fn enabled_compiler_parallel_cfg() -> CompilerParallelConfig {
    CompilerParallelConfig {
        mode: CompilerParallelMode::On,
        threads: 2,
        min_functions: 1,
        min_fn_ir: 1,
        max_jobs: 2,
    }
}

pub(crate) fn strict_incremental_opts() -> IncrementalOptions {
    IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: true,
        phase2: true,
        phase3: true,
        strict_verify: true,
    }
}

pub(crate) fn output_opts(compile_mode: CompileMode) -> CompileOutputOptions {
    CompileOutputOptions {
        inject_runtime: true,
        compile_mode,
        ..CompileOutputOptions::default()
    }
}

pub(crate) fn write_fixture(root: &Path) -> PathBuf {
    let main_path = root.join("main.rr");
    fs::write(&main_path, MATRIX_FIXTURE).expect("failed to write matrix fixture");
    main_path
}

pub(crate) fn write_fixture_with_source(root: &Path, source: &str) -> PathBuf {
    let main_path = root.join("main.rr");
    fs::write(&main_path, source).expect("failed to write matrix fixture");
    main_path
}

pub(crate) fn run_compiled_rscript(label: &str, rscript: &str, script: &Path) -> common::RunResult {
    let result = common::run_rscript(rscript, script);
    assert_eq!(
        result.status, 0,
        "{label}: generated R failed:\nstdout={}\nstderr={}",
        result.stdout, result.stderr
    );
    result
}

#[test]
pub(crate) fn incremental_matrix_preserves_output_and_source_maps_per_compile_mode() {
    let env_guard = lock_env_guard();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compile_mode_stability_matrix");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "compile_matrix");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_fixture(&proj_dir);
    let entry = main_path.to_string_lossy().to_string();

    for (parallel_label, compiler_parallel_cfg) in [
        ("serial", serial_compiler_parallel_cfg()),
        ("parallel", enabled_compiler_parallel_cfg()),
    ] {
        for compile_mode in [CompileMode::Standard, CompileMode::FastDev] {
            let cache_dir = proj_dir.join(format!(
                "cache_{}_{}",
                parallel_label,
                compile_mode.as_str()
            ));
            common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

            let baseline = compile_with_configs_with_options_and_compiler_parallel(
                &entry,
                MATRIX_FIXTURE,
                OptLevel::O2,
                default_type_config(),
                default_parallel_config(),
                compiler_parallel_cfg,
                output_opts(compile_mode),
            )
            .unwrap_or_else(|err| {
                panic!(
                    "{parallel_label}/{} baseline compile failed: {err:?}",
                    compile_mode.as_str()
                )
            });

            let mut session = IncrementalSession::default();
            let first = compile_incremental_request(IncrementalCompileRequest {
                entry_path: &entry,
                entry_input: MATRIX_FIXTURE,
                opt_level: OptLevel::O2,
                type_cfg: default_type_config(),
                parallel_cfg: default_parallel_config(),
                compiler_parallel_cfg,
                options: strict_incremental_opts(),
                output_options: output_opts(compile_mode),
                session: Some(&mut session),
                profile: None,
            })
            .unwrap_or_else(|err| {
                panic!(
                    "{parallel_label}/{} first incremental compile failed: {err:?}",
                    compile_mode.as_str()
                )
            });
            let second = compile_incremental_request(IncrementalCompileRequest {
                entry_path: &entry,
                entry_input: MATRIX_FIXTURE,
                opt_level: OptLevel::O2,
                type_cfg: default_type_config(),
                parallel_cfg: default_parallel_config(),
                compiler_parallel_cfg,
                options: strict_incremental_opts(),
                output_options: output_opts(compile_mode),
                session: Some(&mut session),
                profile: None,
            })
            .unwrap_or_else(|err| {
                panic!(
                    "{parallel_label}/{} second incremental compile failed: {err:?}",
                    compile_mode.as_str()
                )
            });

            assert_eq!(
                baseline.0,
                first.r_code,
                "{parallel_label}/{}: incremental seed changed emitted R",
                compile_mode.as_str()
            );
            assert_eq!(
                baseline.0,
                second.r_code,
                "{parallel_label}/{}: cached incremental compile changed emitted R",
                compile_mode.as_str()
            );
            assert_eq!(
                baseline.1,
                first.source_map,
                "{parallel_label}/{}: incremental seed changed source map",
                compile_mode.as_str()
            );
            assert_eq!(
                baseline.1,
                second.source_map,
                "{parallel_label}/{}: cached incremental compile changed source map",
                compile_mode.as_str()
            );
            assert!(
                second.stats.phase1_artifact_hit || second.stats.phase3_memory_hit,
                "{parallel_label}/{}: expected incremental reuse on second compile",
                compile_mode.as_str()
            );
            assert!(
                second.stats.strict_verification_checked && second.stats.strict_verification_passed,
                "{parallel_label}/{}: strict verification should run and pass on cached compile",
                compile_mode.as_str()
            );
        }
    }

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
pub(crate) fn extended_vectorized_incremental_matrix_preserves_output_and_source_maps_per_compile_mode()
 {
    let env_guard = lock_env_guard();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compile_mode_stability_matrix");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "extended_vector_compile_matrix");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_fixture_with_source(&proj_dir, EXTENDED_VECTOR_MATRIX_FIXTURE);
    let entry = main_path.to_string_lossy().to_string();

    for (parallel_label, compiler_parallel_cfg) in [
        ("serial", serial_compiler_parallel_cfg()),
        ("parallel", enabled_compiler_parallel_cfg()),
    ] {
        for compile_mode in [CompileMode::Standard, CompileMode::FastDev] {
            let cache_dir = proj_dir.join(format!(
                "extended_vector_cache_{}_{}",
                parallel_label,
                compile_mode.as_str()
            ));
            common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

            let baseline = compile_with_configs_with_options_and_compiler_parallel(
                &entry,
                EXTENDED_VECTOR_MATRIX_FIXTURE,
                OptLevel::O2,
                default_type_config(),
                default_parallel_config(),
                compiler_parallel_cfg,
                output_opts(compile_mode),
            )
            .unwrap_or_else(|err| {
                panic!(
                    "{parallel_label}/{} extended vector baseline compile failed: {err:?}",
                    compile_mode.as_str()
                )
            });

            let mut session = IncrementalSession::default();
            let first = compile_incremental_request(IncrementalCompileRequest {
                entry_path: &entry,
                entry_input: EXTENDED_VECTOR_MATRIX_FIXTURE,
                opt_level: OptLevel::O2,
                type_cfg: default_type_config(),
                parallel_cfg: default_parallel_config(),
                compiler_parallel_cfg,
                options: strict_incremental_opts(),
                output_options: output_opts(compile_mode),
                session: Some(&mut session),
                profile: None,
            })
            .unwrap_or_else(|err| {
                panic!(
                    "{parallel_label}/{} extended vector first incremental compile failed: {err:?}",
                    compile_mode.as_str()
                )
            });
            let second =
                compile_incremental_request(IncrementalCompileRequest {
                    entry_path: &entry,
                    entry_input: EXTENDED_VECTOR_MATRIX_FIXTURE,
                    opt_level: OptLevel::O2,
                    type_cfg: default_type_config(),
                    parallel_cfg: default_parallel_config(),
                    compiler_parallel_cfg,
                    options: strict_incremental_opts(),
                    output_options: output_opts(compile_mode),
                    session: Some(&mut session),
                    profile: None,
                })
                .unwrap_or_else(|err| {
                    panic!(
                        "{parallel_label}/{} extended vector second incremental compile failed: {err:?}",
                        compile_mode.as_str()
                    )
                });

            assert_eq!(
                baseline.0,
                first.r_code,
                "{parallel_label}/{}: extended vector incremental seed changed emitted R",
                compile_mode.as_str()
            );
            assert_eq!(
                baseline.0,
                second.r_code,
                "{parallel_label}/{}: extended vector cached incremental compile changed emitted R",
                compile_mode.as_str()
            );
            assert_eq!(
                baseline.1,
                first.source_map,
                "{parallel_label}/{}: extended vector incremental seed changed source map",
                compile_mode.as_str()
            );
            assert_eq!(
                baseline.1,
                second.source_map,
                "{parallel_label}/{}: extended vector cached incremental compile changed source map",
                compile_mode.as_str()
            );
            assert!(
                second.stats.phase1_artifact_hit || second.stats.phase3_memory_hit,
                "{parallel_label}/{}: expected extended vector incremental reuse on second compile",
                compile_mode.as_str()
            );
            assert!(
                second.stats.strict_verification_checked && second.stats.strict_verification_passed,
                "{parallel_label}/{}: extended vector strict verification should run and pass on cached compile",
                compile_mode.as_str()
            );
        }
    }

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
pub(crate) fn incremental_cache_separates_standard_and_fast_dev_artifacts() {
    let env_guard = lock_env_guard();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compile_mode_stability_matrix");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "compile_mode_cache_split");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_fixture(&proj_dir);
    let entry = main_path.to_string_lossy().to_string();
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let mut session = IncrementalSession::default();
    let standard_seed = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &entry,
        entry_input: MATRIX_FIXTURE,
        opt_level: OptLevel::O2,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: enabled_compiler_parallel_cfg(),
        options: strict_incremental_opts(),
        output_options: output_opts(CompileMode::Standard),
        session: Some(&mut session),
        profile: None,
    })
    .expect("standard seed compile failed");
    assert!(
        !standard_seed.stats.phase1_artifact_hit && !standard_seed.stats.phase3_memory_hit,
        "first standard compile should not claim a cache hit"
    );

    let standard_hit = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &entry,
        entry_input: MATRIX_FIXTURE,
        opt_level: OptLevel::O2,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: enabled_compiler_parallel_cfg(),
        options: strict_incremental_opts(),
        output_options: output_opts(CompileMode::Standard),
        session: Some(&mut session),
        profile: None,
    })
    .expect("standard cache-hit compile failed");
    assert!(
        standard_hit.stats.phase1_artifact_hit || standard_hit.stats.phase3_memory_hit,
        "second standard compile should reuse cached artifacts"
    );

    let fast_seed = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &entry,
        entry_input: MATRIX_FIXTURE,
        opt_level: OptLevel::O2,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: enabled_compiler_parallel_cfg(),
        options: strict_incremental_opts(),
        output_options: output_opts(CompileMode::FastDev),
        session: Some(&mut session),
        profile: None,
    })
    .expect("fast-dev seed compile failed");
    assert!(
        !fast_seed.stats.phase1_artifact_hit && !fast_seed.stats.phase3_memory_hit,
        "switching compile mode should miss final artifacts instead of reusing standard cache"
    );
    assert!(
        fast_seed
            .stats
            .miss_reasons
            .iter()
            .any(|reason| reason == "output_options_changed"),
        "compile-mode change should be reported as output_options_changed, got {:?}",
        fast_seed.stats.miss_reasons
    );

    let fast_hit = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &entry,
        entry_input: MATRIX_FIXTURE,
        opt_level: OptLevel::O2,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: enabled_compiler_parallel_cfg(),
        options: strict_incremental_opts(),
        output_options: output_opts(CompileMode::FastDev),
        session: Some(&mut session),
        profile: None,
    })
    .expect("fast-dev cache-hit compile failed");
    assert!(
        fast_hit.stats.phase1_artifact_hit || fast_hit.stats.phase3_memory_hit,
        "second fast-dev compile should reuse its own cached artifacts"
    );
    assert!(
        fast_hit.stats.strict_verification_checked && fast_hit.stats.strict_verification_passed,
        "fast-dev cached compile should pass strict verification"
    );

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
pub(crate) fn fast_dev_matches_standard_across_parallel_and_incremental_paths() {
    let rscript = match common::rscript_path() {
        Some(path) if common::rscript_available(&path) => path,
        _ => {
            eprintln!("Skipping compile-mode stability runtime test: Rscript unavailable.");
            return;
        }
    };

    let env_guard = lock_env_guard();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compile_mode_stability_matrix");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "runtime_equiv");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_fixture(&proj_dir);
    let entry = main_path.to_string_lossy().to_string();

    let (standard_serial_code, _) = compile_with_configs_with_options_and_compiler_parallel(
        &entry,
        MATRIX_FIXTURE,
        OptLevel::O2,
        default_type_config(),
        default_parallel_config(),
        serial_compiler_parallel_cfg(),
        output_opts(CompileMode::Standard),
    )
    .expect("standard serial compile failed");
    let (fast_serial_code, _) = compile_with_configs_with_options_and_compiler_parallel(
        &entry,
        MATRIX_FIXTURE,
        OptLevel::O2,
        default_type_config(),
        default_parallel_config(),
        serial_compiler_parallel_cfg(),
        output_opts(CompileMode::FastDev),
    )
    .expect("fast-dev serial compile failed");

    let cache_dir = proj_dir.join(".rr-cache-runtime");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let mut standard_session = IncrementalSession::default();
    let _ = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &entry,
        entry_input: MATRIX_FIXTURE,
        opt_level: OptLevel::O2,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: enabled_compiler_parallel_cfg(),
        options: strict_incremental_opts(),
        output_options: output_opts(CompileMode::Standard),
        session: Some(&mut standard_session),
        profile: None,
    })
    .expect("standard parallel seed compile failed");
    let standard_parallel_cached = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &entry,
        entry_input: MATRIX_FIXTURE,
        opt_level: OptLevel::O2,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: enabled_compiler_parallel_cfg(),
        options: strict_incremental_opts(),
        output_options: output_opts(CompileMode::Standard),
        session: Some(&mut standard_session),
        profile: None,
    })
    .expect("standard parallel cached compile failed");

    let mut fast_session = IncrementalSession::default();
    let _ = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &entry,
        entry_input: MATRIX_FIXTURE,
        opt_level: OptLevel::O2,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: enabled_compiler_parallel_cfg(),
        options: strict_incremental_opts(),
        output_options: output_opts(CompileMode::FastDev),
        session: Some(&mut fast_session),
        profile: None,
    })
    .expect("fast-dev parallel seed compile failed");
    let fast_parallel_cached = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &entry,
        entry_input: MATRIX_FIXTURE,
        opt_level: OptLevel::O2,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: enabled_compiler_parallel_cfg(),
        options: strict_incremental_opts(),
        output_options: output_opts(CompileMode::FastDev),
        session: Some(&mut fast_session),
        profile: None,
    })
    .expect("fast-dev parallel cached compile failed");

    assert!(
        standard_parallel_cached.stats.phase1_artifact_hit
            || standard_parallel_cached.stats.phase3_memory_hit,
        "standard parallel cached compile should reuse prior work"
    );
    assert!(
        fast_parallel_cached.stats.phase1_artifact_hit
            || fast_parallel_cached.stats.phase3_memory_hit,
        "fast-dev parallel cached compile should reuse prior work"
    );

    let standard_serial_path = proj_dir.join("standard_serial.R");
    let fast_serial_path = proj_dir.join("fast_serial.R");
    let standard_parallel_path = proj_dir.join("standard_parallel_cached.R");
    let fast_parallel_path = proj_dir.join("fast_parallel_cached.R");
    fs::write(&standard_serial_path, standard_serial_code)
        .expect("failed to write standard serial R");
    fs::write(&fast_serial_path, fast_serial_code).expect("failed to write fast-dev serial R");
    fs::write(&standard_parallel_path, &standard_parallel_cached.r_code)
        .expect("failed to write standard parallel cached R");
    fs::write(&fast_parallel_path, &fast_parallel_cached.r_code)
        .expect("failed to write fast-dev parallel cached R");

    let standard_serial = run_compiled_rscript("standard_serial", &rscript, &standard_serial_path);
    let fast_serial = run_compiled_rscript("fast_serial", &rscript, &fast_serial_path);
    let standard_parallel = run_compiled_rscript(
        "standard_parallel_cached",
        &rscript,
        &standard_parallel_path,
    );
    let fast_parallel = run_compiled_rscript("fast_parallel_cached", &rscript, &fast_parallel_path);

    let baseline_stdout = normalize(&standard_serial.stdout);
    let baseline_stderr = normalize(&standard_serial.stderr);

    assert_eq!(
        baseline_stdout,
        normalize(&fast_serial.stdout),
        "standard serial and fast-dev serial should match at runtime"
    );
    assert_eq!(
        baseline_stderr,
        normalize(&fast_serial.stderr),
        "standard serial and fast-dev serial should match stderr"
    );
    assert_eq!(
        baseline_stdout,
        normalize(&standard_parallel.stdout),
        "standard serial and standard parallel cached should match at runtime"
    );
    assert_eq!(
        baseline_stdout,
        normalize(&fast_parallel.stdout),
        "standard serial and fast-dev parallel cached should match at runtime"
    );
    assert_eq!(
        baseline_stderr,
        normalize(&standard_parallel.stderr),
        "standard serial and standard parallel cached should match stderr"
    );
    assert_eq!(
        baseline_stderr,
        normalize(&fast_parallel.stderr),
        "standard serial and fast-dev parallel cached should match stderr"
    );

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
pub(crate) fn vectorized_fast_dev_matches_standard_across_parallel_and_incremental_paths() {
    let rscript = match common::rscript_path() {
        Some(path) if common::rscript_available(&path) => path,
        _ => {
            eprintln!(
                "Skipping vectorized compile-mode stability runtime test: Rscript unavailable."
            );
            return;
        }
    };

    let env_guard = lock_env_guard();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compile_mode_stability_matrix");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "vectorized_runtime_equiv");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_fixture_with_source(&proj_dir, VECTOR_MATRIX_FIXTURE);
    let entry = main_path.to_string_lossy().to_string();

    let (standard_serial_code, _) = compile_with_configs_with_options_and_compiler_parallel(
        &entry,
        VECTOR_MATRIX_FIXTURE,
        OptLevel::O2,
        default_type_config(),
        default_parallel_config(),
        serial_compiler_parallel_cfg(),
        output_opts(CompileMode::Standard),
    )
    .expect("standard serial vectorized compile failed");
    let (fast_serial_code, _) = compile_with_configs_with_options_and_compiler_parallel(
        &entry,
        VECTOR_MATRIX_FIXTURE,
        OptLevel::O2,
        default_type_config(),
        default_parallel_config(),
        serial_compiler_parallel_cfg(),
        output_opts(CompileMode::FastDev),
    )
    .expect("fast-dev serial vectorized compile failed");

    let cache_dir = proj_dir.join(".rr-cache-vectorized-runtime");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let mut standard_session = IncrementalSession::default();
    let _ = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &entry,
        entry_input: VECTOR_MATRIX_FIXTURE,
        opt_level: OptLevel::O2,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: enabled_compiler_parallel_cfg(),
        options: strict_incremental_opts(),
        output_options: output_opts(CompileMode::Standard),
        session: Some(&mut standard_session),
        profile: None,
    })
    .expect("standard vectorized parallel seed compile failed");
    let standard_parallel_cached = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &entry,
        entry_input: VECTOR_MATRIX_FIXTURE,
        opt_level: OptLevel::O2,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: enabled_compiler_parallel_cfg(),
        options: strict_incremental_opts(),
        output_options: output_opts(CompileMode::Standard),
        session: Some(&mut standard_session),
        profile: None,
    })
    .expect("standard vectorized parallel cached compile failed");

    let mut fast_session = IncrementalSession::default();
    let _ = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &entry,
        entry_input: VECTOR_MATRIX_FIXTURE,
        opt_level: OptLevel::O2,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: enabled_compiler_parallel_cfg(),
        options: strict_incremental_opts(),
        output_options: output_opts(CompileMode::FastDev),
        session: Some(&mut fast_session),
        profile: None,
    })
    .expect("fast-dev vectorized parallel seed compile failed");
    let fast_parallel_cached = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &entry,
        entry_input: VECTOR_MATRIX_FIXTURE,
        opt_level: OptLevel::O2,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: enabled_compiler_parallel_cfg(),
        options: strict_incremental_opts(),
        output_options: output_opts(CompileMode::FastDev),
        session: Some(&mut fast_session),
        profile: None,
    })
    .expect("fast-dev vectorized parallel cached compile failed");

    assert!(
        standard_parallel_cached.stats.phase1_artifact_hit
            || standard_parallel_cached.stats.phase3_memory_hit,
        "standard vectorized parallel cached compile should reuse prior work"
    );
    assert!(
        fast_parallel_cached.stats.phase1_artifact_hit
            || fast_parallel_cached.stats.phase3_memory_hit,
        "fast-dev vectorized parallel cached compile should reuse prior work"
    );

    let standard_serial_path = proj_dir.join("vector_standard_serial.R");
    let fast_serial_path = proj_dir.join("vector_fast_serial.R");
    let standard_parallel_path = proj_dir.join("vector_standard_parallel_cached.R");
    let fast_parallel_path = proj_dir.join("vector_fast_parallel_cached.R");
    fs::write(&standard_serial_path, standard_serial_code)
        .expect("failed to write standard serial vectorized R");
    fs::write(&fast_serial_path, fast_serial_code)
        .expect("failed to write fast-dev serial vectorized R");
    fs::write(&standard_parallel_path, &standard_parallel_cached.r_code)
        .expect("failed to write standard parallel cached vectorized R");
    fs::write(&fast_parallel_path, &fast_parallel_cached.r_code)
        .expect("failed to write fast-dev parallel cached vectorized R");

    let standard_serial =
        run_compiled_rscript("vector_standard_serial", &rscript, &standard_serial_path);
    let fast_serial = run_compiled_rscript("vector_fast_serial", &rscript, &fast_serial_path);
    let standard_parallel = run_compiled_rscript(
        "vector_standard_parallel_cached",
        &rscript,
        &standard_parallel_path,
    );
    let fast_parallel =
        run_compiled_rscript("vector_fast_parallel_cached", &rscript, &fast_parallel_path);

    let baseline_stdout = normalize(&standard_serial.stdout);
    let baseline_stderr = normalize(&standard_serial.stderr);

    assert_eq!(
        baseline_stdout,
        normalize(&fast_serial.stdout),
        "vectorized standard serial and fast-dev serial should match at runtime"
    );
    assert_eq!(
        baseline_stderr,
        normalize(&fast_serial.stderr),
        "vectorized standard serial and fast-dev serial should match stderr"
    );
    assert_eq!(
        baseline_stdout,
        normalize(&standard_parallel.stdout),
        "vectorized standard serial and standard parallel cached should match at runtime"
    );
    assert_eq!(
        baseline_stdout,
        normalize(&fast_parallel.stdout),
        "vectorized standard serial and fast-dev parallel cached should match at runtime"
    );
    assert_eq!(
        baseline_stderr,
        normalize(&standard_parallel.stderr),
        "vectorized standard serial and standard parallel cached should match stderr"
    );
    assert_eq!(
        baseline_stderr,
        normalize(&fast_parallel.stderr),
        "vectorized standard serial and fast-dev parallel cached should match stderr"
    );

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}
