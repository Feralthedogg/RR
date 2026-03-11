mod common;

use common::run_compile_case;

fn run_compile(source: &str, file_name: &str) -> (bool, String, String) {
    run_compile_case("statement_boundaries", source, file_name, "-O1", &[])
}

#[test]
fn semicolons_are_rejected() {
    let src = r#"
fn main() {
  x <- 1L;
  return x
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "semicolon_rejected.rr");
    assert!(!ok, "compile must fail when semicolons are present");
    assert!(
        stdout.contains("semicolons are not supported"),
        "semicolon rejection diagnostic:\n{}",
        stdout
    );
}

#[test]
fn same_line_statement_boundary_must_fail() {
    let src = r#"
fn main() {
  x <- 1L y <- 2L
  return x + y
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "same_line_statement_boundary.rr");
    assert!(
        !ok,
        "compile must fail when same-line statements are not separated by a newline"
    );
    assert!(
        stdout.contains("statements must be separated by a newline or '}'"),
        "same-line statement boundary diagnostic:\n{}",
        stdout
    );
}

#[test]
fn multiline_unary_expression_still_enforces_statement_boundary() {
    let src = r#"
fn main() {
  x <- -
    1L y <- 2L
  return x
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "multiline_unary_boundary.rr");
    assert!(
        !ok,
        "compile must fail when a multiline unary expression is followed by a same-line statement"
    );
    assert!(
        stdout.contains("statements must be separated by a newline or '}'"),
        "multiline unary statement boundary diagnostic:\n{}",
        stdout
    );
}

#[test]
fn recovery_after_semicolon_keeps_following_function_boundary_intact() {
    let src = r#"
fn bad() {
  return 1L;
}

fn good() {
  return 2L
}

good()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "semicolon_recovery.rr");
    assert!(!ok, "compile must fail when semicolons are present");
    assert!(
        stdout.contains("semicolons are not supported"),
        "semicolon recovery diagnostic:\n{}",
        stdout
    );
    assert!(
        !stdout.contains("before LBrace"),
        "semicolon recovery must not skip the next function boundary:\n{}",
        stdout
    );
}

#[test]
fn postfix_try_can_end_before_next_line_statement() {
    let src = r#"
fn main() {
  let x = (1L)?
  let y = 2L
  return y
}
main()
"#;
    let (ok, stdout, stderr) = run_compile(src, "postfix_try_newline.rr");
    assert!(
        ok,
        "postfix try should allow a newline-delimited next statement\nstdout:\n{}\nstderr:\n{}",
        stdout, stderr
    );
}

#[test]
fn newline_separator_without_semicolon_is_allowed() {
    let src = r#"
fn main() {
  let x = 1L
  let y = 2L
  return x + y
}
main()
"#;
    let (ok, _stdout, _stderr) = run_compile(src, "newline_separated.rr");
    assert!(ok, "newline separated statements should compile");
}
