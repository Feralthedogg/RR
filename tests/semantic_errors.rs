mod common;

use common::run_compile_case;

fn run_compile(source: &str, file_name: &str) -> (bool, String, String) {
    run_compile_case("semantic_errors", source, file_name, "-O1", &[])
}

fn run_compile_with_env(
    source: &str,
    file_name: &str,
    env_kv: &[(&str, &str)],
) -> (bool, String, String) {
    run_compile_case("semantic_errors", source, file_name, "-O1", env_kv)
}

#[test]
fn undefined_variable_must_fail() {
    let src = r#"
fn main() {
  let x = 1
  return y + x
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "undefined_var.rr");
    assert!(!ok, "compile must fail for undefined variable");
    assert!(
        stdout.contains("** (RR.SemanticError)"),
        "missing semantic error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("undefined variable 'y'"),
        "missing undefined variable detail:\n{}",
        stdout
    );
}

#[test]
fn undefined_function_must_fail() {
    let src = r#"
fn main() {
  return foo(1)
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "undefined_fn.rr");
    assert!(!ok, "compile must fail for undefined function");
    assert!(
        stdout.contains("** (RR.SemanticError)"),
        "missing semantic error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("undefined function 'foo'"),
        "missing undefined function detail:\n{}",
        stdout
    );
}

#[test]
fn arity_mismatch_must_fail() {
    let src = r#"
fn add(a, b) {
  return a + b
}
fn main() {
  return add(1)
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "arity_mismatch.rr");
    assert!(!ok, "compile must fail for arity mismatch");
    assert!(
        stdout.contains("** (RR.SemanticError)"),
        "missing semantic error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("expects 2 argument(s), got 1"),
        "missing arity mismatch detail:\n{}",
        stdout
    );
}

#[test]
fn implicit_declaration_warns_by_default() {
    let src = r#"
fn main() {
  x <- 1
  x <- x + 1
  return x
}
main()
"#;
    let (ok, _stdout, stderr) = run_compile_with_env(
        src,
        "implicit_decl_warn.rr",
        &[("RR_WARN_IMPLICIT_DECL", "1")],
    );
    assert!(
        ok,
        "compile should succeed by default for implicit declaration"
    );
    assert!(
        stderr.contains("implicit declaration via assignment"),
        "expected implicit declaration warning in stderr, got:\n{}",
        stderr
    );
}

#[test]
fn strict_let_mode_rejects_implicit_declaration() {
    let src = r#"
fn main() {
  x <- 1
  return x
}
main()
"#;
    let (ok, stdout, _stderr) =
        run_compile_with_env(src, "implicit_decl_strict.rr", &[("RR_STRICT_LET", "1")]);
    assert!(!ok, "compile must fail in strict let mode");
    assert!(
        stdout.contains("** (RR.SemanticError)"),
        "missing semantic error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("assignment to undeclared variable 'x'"),
        "missing strict-let detail:\n{}",
        stdout
    );
}
