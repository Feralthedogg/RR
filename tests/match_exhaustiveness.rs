use RR::compiler::{OptLevel, compile_with_configs, default_parallel_config, default_type_config};
use RR::error::RRCode;

#[test]
fn non_exhaustive_match_is_rejected_during_lowering() {
    let source = r#"
fn classify(v) {
  return match(v) {
    1 => 10
  }

}

print(classify(2))

"#;

    let err = compile_with_configs(
        "non_exhaustive_match.rr",
        source,
        OptLevel::O0,
        default_type_config(),
        default_parallel_config(),
    )
    .expect_err("non-exhaustive match must fail during compilation");

    assert!(matches!(err.code, RRCode::E3001));
    assert!(
        err.message.contains("non-exhaustive match"),
        "unexpected error: {}",
        err.message
    );
}
