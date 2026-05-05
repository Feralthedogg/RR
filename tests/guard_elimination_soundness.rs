use rr::compiler::internal::typeck::{NativeBackend, TypeConfig, TypeMode};
use rr::compiler::{OptLevel, compile_with_config};

#[test]
fn typed_condition_elides_truthy_wrapper_at_branch_site() {
    let src = r#"
fn main(flag: bool) -> int {
  if (flag) {
    return 1L
  }
  return 0L
}
print(main(true))
"#;
    let (code, _map) = compile_with_config(
        "guard_elide.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    assert!(
        !code.contains("if (rr_truthy1("),
        "expected branch condition wrapper to be eliminated"
    );
}

#[test]
fn unresolved_condition_keeps_wrapper() {
    let src = r#"
fn main(x) {
  if (x) {
    return 1L
  }
  return 0L
}
print(main(TRUE))
"#;
    let (code, _map) = compile_with_config(
        "guard_keep.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Gradual,
            native_backend: NativeBackend::Off,
        },
    )
    .expect("compile");

    assert!(
        code.contains("if (rr_truthy1("),
        "expected wrapper for unresolved condition"
    );
}
