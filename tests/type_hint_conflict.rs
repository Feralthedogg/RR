use RR::compiler::{OptLevel, compile_with_config};
use RR::error::RRCode;
use RR::typeck::{NativeBackend, TypeConfig, TypeMode};

#[test]
fn strict_mode_reports_return_type_hint_conflict() {
    let src = r#"
fn bad(a: float) -> float {
  return "oops";
}
bad(1.0);
"#;

    let res = compile_with_config(
        "type_hint_conflict.rr",
        src,
        OptLevel::O1,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    );

    let err = res.expect_err("compile should fail");
    assert!(matches!(err.code, RRCode::E1010));
}
