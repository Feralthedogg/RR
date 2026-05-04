use super::common::{self, normalize, unique_dir};
use super::fixtures_core::*;
use rr::compiler::{
    CompileMode, IncrementalCompileRequest, IncrementalSession, OptLevel,
    compile_incremental_request, compile_with_configs_with_options_and_compiler_parallel,
    default_parallel_config, default_type_config,
};
use std::fs;
use std::path::PathBuf;

#[test]
pub(crate) fn extended_vectorized_fast_dev_matches_standard_across_parallel_and_incremental_paths()
{
    let rscript = match common::rscript_path() {
        Some(path) if common::rscript_available(&path) => path,
        _ => {
            eprintln!(
                "Skipping extended vectorized compile-mode stability runtime test: Rscript unavailable."
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
    let proj_dir = unique_dir(&sandbox_root, "extended_vectorized_runtime_equiv");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_fixture_with_source(&proj_dir, EXTENDED_VECTOR_MATRIX_FIXTURE);
    let entry = main_path.to_string_lossy().to_string();

    let (standard_serial_code, _) = compile_with_configs_with_options_and_compiler_parallel(
        &entry,
        EXTENDED_VECTOR_MATRIX_FIXTURE,
        OptLevel::O2,
        default_type_config(),
        default_parallel_config(),
        serial_compiler_parallel_cfg(),
        output_opts(CompileMode::Standard),
    )
    .expect("standard serial extended vectorized compile failed");
    let (fast_serial_code, _) = compile_with_configs_with_options_and_compiler_parallel(
        &entry,
        EXTENDED_VECTOR_MATRIX_FIXTURE,
        OptLevel::O2,
        default_type_config(),
        default_parallel_config(),
        serial_compiler_parallel_cfg(),
        output_opts(CompileMode::FastDev),
    )
    .expect("fast-dev serial extended vectorized compile failed");

    let cache_dir = proj_dir.join(".rr-cache-extended-vectorized-runtime");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let mut standard_session = IncrementalSession::default();
    let _ = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &entry,
        entry_input: EXTENDED_VECTOR_MATRIX_FIXTURE,
        opt_level: OptLevel::O2,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: enabled_compiler_parallel_cfg(),
        options: strict_incremental_opts(),
        output_options: output_opts(CompileMode::Standard),
        session: Some(&mut standard_session),
        profile: None,
    })
    .expect("standard extended vectorized parallel seed compile failed");
    let standard_parallel_cached = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &entry,
        entry_input: EXTENDED_VECTOR_MATRIX_FIXTURE,
        opt_level: OptLevel::O2,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: enabled_compiler_parallel_cfg(),
        options: strict_incremental_opts(),
        output_options: output_opts(CompileMode::Standard),
        session: Some(&mut standard_session),
        profile: None,
    })
    .expect("standard extended vectorized parallel cached compile failed");

    let mut fast_session = IncrementalSession::default();
    let _ = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &entry,
        entry_input: EXTENDED_VECTOR_MATRIX_FIXTURE,
        opt_level: OptLevel::O2,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: enabled_compiler_parallel_cfg(),
        options: strict_incremental_opts(),
        output_options: output_opts(CompileMode::FastDev),
        session: Some(&mut fast_session),
        profile: None,
    })
    .expect("fast-dev extended vectorized parallel seed compile failed");
    let fast_parallel_cached = compile_incremental_request(IncrementalCompileRequest {
        entry_path: &entry,
        entry_input: EXTENDED_VECTOR_MATRIX_FIXTURE,
        opt_level: OptLevel::O2,
        type_cfg: default_type_config(),
        parallel_cfg: default_parallel_config(),
        compiler_parallel_cfg: enabled_compiler_parallel_cfg(),
        options: strict_incremental_opts(),
        output_options: output_opts(CompileMode::FastDev),
        session: Some(&mut fast_session),
        profile: None,
    })
    .expect("fast-dev extended vectorized parallel cached compile failed");

    assert!(
        standard_parallel_cached.stats.phase1_artifact_hit
            || standard_parallel_cached.stats.phase3_memory_hit,
        "standard extended vectorized parallel cached compile should reuse prior work"
    );
    assert!(
        fast_parallel_cached.stats.phase1_artifact_hit
            || fast_parallel_cached.stats.phase3_memory_hit,
        "fast-dev extended vectorized parallel cached compile should reuse prior work"
    );

    let standard_serial_path = proj_dir.join("extended_vector_standard_serial.R");
    let fast_serial_path = proj_dir.join("extended_vector_fast_serial.R");
    let standard_parallel_path = proj_dir.join("extended_vector_standard_parallel_cached.R");
    let fast_parallel_path = proj_dir.join("extended_vector_fast_parallel_cached.R");
    fs::write(&standard_serial_path, standard_serial_code)
        .expect("failed to write standard serial extended vectorized R");
    fs::write(&fast_serial_path, fast_serial_code)
        .expect("failed to write fast-dev serial extended vectorized R");
    fs::write(&standard_parallel_path, &standard_parallel_cached.r_code)
        .expect("failed to write standard parallel cached extended vectorized R");
    fs::write(&fast_parallel_path, &fast_parallel_cached.r_code)
        .expect("failed to write fast-dev parallel cached extended vectorized R");

    let standard_serial = run_compiled_rscript(
        "extended_vector_standard_serial",
        &rscript,
        &standard_serial_path,
    );
    let fast_serial =
        run_compiled_rscript("extended_vector_fast_serial", &rscript, &fast_serial_path);
    let standard_parallel = run_compiled_rscript(
        "extended_vector_standard_parallel_cached",
        &rscript,
        &standard_parallel_path,
    );
    let fast_parallel = run_compiled_rscript(
        "extended_vector_fast_parallel_cached",
        &rscript,
        &fast_parallel_path,
    );

    let baseline_stdout = normalize(&standard_serial.stdout);
    let baseline_stderr = normalize(&standard_serial.stderr);

    assert_eq!(
        baseline_stdout,
        normalize(&fast_serial.stdout),
        "extended vectorized standard serial and fast-dev serial should match at runtime"
    );
    assert_eq!(
        baseline_stderr,
        normalize(&fast_serial.stderr),
        "extended vectorized standard serial and fast-dev serial should match stderr"
    );
    assert_eq!(
        baseline_stdout,
        normalize(&standard_parallel.stdout),
        "extended vectorized standard serial and standard parallel cached should match at runtime"
    );
    assert_eq!(
        baseline_stdout,
        normalize(&fast_parallel.stdout),
        "extended vectorized standard serial and fast-dev parallel cached should match at runtime"
    );
    assert_eq!(
        baseline_stderr,
        normalize(&standard_parallel.stderr),
        "extended vectorized standard serial and standard parallel cached should match stderr"
    );
    assert_eq!(
        baseline_stderr,
        normalize(&fast_parallel.stderr),
        "extended vectorized standard serial and fast-dev parallel cached should match stderr"
    );

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}
