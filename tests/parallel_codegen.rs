use rr::compiler::internal::typeck::{NativeBackend, TypeConfig, TypeMode};
use rr::compiler::{OptLevel, ParallelBackend, ParallelConfig, ParallelMode, compile_with_configs};

#[test]
fn emits_parallel_runtime_prelude_and_vector_wrapper_call() {
    let src = r#"
fn addv(x: vector<float>, y: vector<float>) -> vector<float> {
  return x + y

}

print(addv(c(1.0, 2.0), c(3.0, 4.0)))

"#;

    let (code, _map) = compile_with_configs(
        "parallel_codegen.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
        ParallelConfig {
            mode: ParallelMode::Optional,
            backend: ParallelBackend::OpenMp,
            threads: 4,
            min_trip: 64,
        },
    )
    .expect("compile should succeed");

    assert!(!code.contains("if (!nzchar(Sys.getenv(\"RR_PARALLEL_MODE\", \"\")))"));
    assert!(code.contains(".rr_env$parallel_mode <- \"optional\";"));
    assert!(code.contains(".rr_env$parallel_backend <- \"openmp\";"));
    assert!(code.contains(".rr_env$parallel_threads <- as.integer(4);"));
    assert!(code.contains(".rr_env$parallel_min_trip <- as.integer(64);"));
    assert!(
        code.contains("rr_parallel_vec_add_f64("),
        "typed vector add should lower to parallel-safe wrapper"
    );
}
