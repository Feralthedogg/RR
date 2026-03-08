use RR::compiler::{OptLevel, compile_with_config};
use RR::typeck::{NativeBackend, TypeConfig, TypeMode};

#[test]
fn o2_vector_callmap_emits_intrinsic_helpers() {
    let src = r#"
fn call_abs(n: int) {
  let x = seq_len(n) - 4

  let y = seq_len(n)

  for (i in 1..length(x)) {
    y[i] = abs(x[i])

  }
  return y

}
print(call_abs(5L))

"#;

    let (code, _map) = compile_with_config(
        "intrinsic_vectorization.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    assert!(
        code.contains("rr_intrinsic_vec_abs_f64(") || code.contains("rr_intrinsic_vec_sum_f64("),
        "expected intrinsic helper call in optimized output"
    );
}
