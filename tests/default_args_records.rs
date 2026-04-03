use RR::compiler::{OptLevel, compile};

#[test]
fn user_functions_preserve_default_and_named_args_in_emitted_r() {
    let src = r#"
fn add(x = 3, y = 4) {
  return x + y
}

fn main() {
  print(add())
  print(add(y = 10))
}

main()
"#;

    let (code, _map) = compile("default_args_records.rr", src, OptLevel::O0).expect("compile");
    assert!(code.contains("x = 3L"), "{code}");
    assert!(code.contains("y = 4L"), "{code}");
    assert!(code.contains("y = 10L"), "{code}");
}

#[test]
fn record_literals_and_field_gets_emit_direct_r_shapes() {
    let src = r#"
fn main() {
  let cfg = { alpha: 1, beta: 2 }
  cfg.alpha = 7
  print(cfg.alpha)
}

main()
"#;

    let (code, _map) =
        compile("default_args_records_record.rr", src, OptLevel::O0).expect("compile");
    assert!(code.contains("list(alpha = 1L, beta = 2L)"), "{code}");
    assert!(code.contains("cfg[[\"alpha\"]] <- 7L"), "{code}");
    assert!(code.contains("[[\"alpha\"]]"), "{code}");
    assert!(!code.contains("rr_named_list("), "{code}");
    assert!(!code.contains("rr_field_get("), "{code}");
    assert!(!code.contains("rr_field_set("), "{code}");
}

#[test]
fn missing_required_arg_still_fails_with_defaults_present() {
    let src = r#"
fn add(x = 3, y) {
  return x + y
}

fn main() {
  print(add())
}

main()
"#;

    let err = compile(
        "default_args_records_missing_required.rr",
        src,
        OptLevel::O0,
    )
    .expect_err("missing required arg should fail");
    let err_text = format!("{err:?}");
    assert!(
        err_text.contains("missing required argument"),
        "{}",
        err_text
    );
}
