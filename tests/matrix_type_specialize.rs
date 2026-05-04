use rr::compiler::internal::typeck::{NativeBackend, TypeConfig, TypeMode};
use rr::compiler::{OptLevel, compile_with_config};

#[test]
fn o2_matrix_numeric_ops_can_emit_intrinsic_helpers() {
    let src = r#"
fn main() {
  let a = matrix(seq_len(6L), 2L, 3L)
  let b = matrix(seq_len(6L) + 1L, 2L, 3L)
  let c = a + b
  let d = abs(c)
  print(sum(d))
}
main()
"#;

    let (code, _map) = compile_with_config(
        "matrix_type_specialize.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    assert!(
        code.contains("rr_intrinsic_vec_add_f64(")
            || code.contains("c <- (a + b)")
            || code.contains("abs((a + b))")
            || code.contains("print(sum((abs((a + b)))))"),
        "expected matrix add to stay in direct numeric form or intrinsic helper"
    );
    assert!(
        code.contains("rr_intrinsic_vec_abs_f64(")
            || code.contains("d <- abs(c)")
            || code.contains("abs((a + b))")
            || code.contains("print(sum((abs((a + b)))))"),
        "expected matrix abs to stay in direct numeric form or intrinsic helper"
    );
}
