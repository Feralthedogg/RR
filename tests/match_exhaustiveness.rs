use RR::compiler::{
    OptLevel, compile_with_configs, parallel_config_from_env, type_config_from_env,
};
use RR::error::RRCode;

#[test]
fn non_exhaustive_match_is_rejected_during_lowering() {
    let source = r#"
fn classify(v) {
  return match(v) {
    1 => 10
  };
}

print(classify(2));
"#;

    let err = compile_with_configs(
        "non_exhaustive_match.rr",
        source,
        OptLevel::O0,
        type_config_from_env(),
        parallel_config_from_env(),
    )
    .expect_err("non-exhaustive match must fail during compilation");

    assert!(matches!(err.code, RRCode::E3001));
    assert!(
        err.message.contains("non-exhaustive match"),
        "unexpected error: {}",
        err.message
    );
}
