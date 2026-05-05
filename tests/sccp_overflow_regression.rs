use std::panic::{AssertUnwindSafe, catch_unwind};

use rr::compiler::internal::typeck::{NativeBackend, TypeConfig, TypeMode};
use rr::compiler::{OptLevel, compile_with_config};

fn compile_o2_gradual_no_panic(name: &str, src: &str) -> String {
    let run = catch_unwind(AssertUnwindSafe(|| {
        compile_with_config(
            name,
            src,
            OptLevel::O2,
            TypeConfig {
                mode: TypeMode::Gradual,
                native_backend: NativeBackend::Off,
            },
        )
    }));
    assert!(
        run.is_ok(),
        "compiler must not panic for SCCP overflow cases"
    );
    let compiled = run.expect("panic already checked");
    assert!(
        compiled.is_ok(),
        "compile should succeed even when SCCP constant fold overflows: {:?}",
        compiled.err()
    );
    let (code, _map) = compiled.expect("compile success");
    code
}

#[test]
fn sccp_integer_multiply_overflow_does_not_panic() {
    let src = r#"
fn main() {
  let x = 3037000500L * 3037000500L
  print(x)
}

main()
"#;
    let code = compile_o2_gradual_no_panic("sccp_mul_overflow_regression.rr", src);
    assert!(
        code.contains("3037000500L * 3037000500L"),
        "overflowing mul should stay as runtime expression, not panic-folded"
    );
}

#[test]
fn sccp_integer_add_overflow_does_not_panic() {
    let src = r#"
fn main() {
  let x = 9223372036854775807L + 1L
  print(x)
}

main()
"#;
    let code = compile_o2_gradual_no_panic("sccp_add_overflow_regression.rr", src);
    assert!(
        code.contains("9223372036854775807L + 1L"),
        "overflowing add should stay as runtime expression"
    );
}

#[test]
fn semantic_const_eval_integer_add_overflow_does_not_panic() {
    let src = r#"
fn main() {
  if ((9223372036854775807L + 1L) == 0L) {
    print(1L)
  } else {
    print(2L)
  }
}

main()
"#;
    let code = compile_o2_gradual_no_panic("const_eval_add_overflow_regression.rr", src);
    assert!(
        code.contains("9223372036854775807L + 1L"),
        "semantic const eval should leave overflowing integer add for runtime evaluation"
    );
}

#[test]
fn sccp_integer_div_overflow_does_not_panic() {
    let src = r#"
fn main() {
  let min_i64 = (0L - 9223372036854775807L) - 1L
  let x = min_i64 / -1L
  print(x)
}

main()
"#;
    let _ = compile_o2_gradual_no_panic("sccp_div_overflow_regression.rr", src);
}
