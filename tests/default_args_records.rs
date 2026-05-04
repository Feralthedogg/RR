use rr::compiler::{OptLevel, compile};

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
fn helper_inlining_preserves_record_field_labels() {
    let src = r#"
fn make_state(capacity: int) {
  let bucket = {first: capacity, buffer_len: capacity}
  let state = {bucket: bucket, capacity: capacity, used: 1L, marks: integer(0L)}
  return state
}

print(make_state(4L).capacity)
"#;

    let (code, _map) =
        compile("default_args_records_helper_inline.rr", src, OptLevel::O0).expect("compile");
    assert!(
        code.contains("list(bucket = bucket, capacity = capacity, used = 1L, marks = integer(0L))")
            || code.contains(
                "list(bucket = list(first = 4L, buffer_len = 4L), capacity = 4L, used = 1L, marks = integer(0L))"
            ),
        "{code}"
    );
    assert!(
        !code.contains("list(list(first = 4L, buffer_len = 4L) ="),
        "{code}"
    );
    assert!(!code.contains(", 4L = 4L"), "{code}");
}

#[test]
fn field_writes_invalidate_alias_cleanup() {
    let src = r#"
fn f(state) {
  let marked = state
  marked.marks = c(marked.marks, marked.used)
  let out = NULL
  out = rr_field_set(out, "state", marked)
  out = rr_field_set(out, "mark", marked.used)
  return out
}

let s = rr_field_set(rr_field_set(NULL, "marks", integer(0L)), "used", 3L)
print(f(s).state.marks)
"#;

    let (code, _map) =
        compile("default_args_records_field_alias.rr", src, OptLevel::O0).expect("compile");
    assert!(code.contains("marked[[\"marks\"]] <-"), "{code}");
    assert!(
        code.contains("out <- rr_field_set(NULL, \"state\", marked)"),
        "{code}"
    );
    assert!(
        !code.contains("out <- rr_field_set(NULL, \"state\", state)"),
        "{code}"
    );
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
