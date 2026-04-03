mod common;

use common::{normalize, run_compile_case};

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

fn normalize_primary_diagnostic(stdout: &str, file_name: &str) -> String {
    let stdout = normalize(stdout);
    let start = stdout
        .find("** (")
        .expect("expected diagnostic header in compiler stdout");
    let mut out = String::new();
    for line in stdout[start..].lines() {
        let normalized_line = normalize_diag_line(line, file_name);
        out.push_str(&normalized_line);
        out.push('\n');
    }
    out
}

fn normalize_diag_line(line: &str, file_name: &str) -> String {
    if let Some(at_pos) = line.rfind(" at ")
        && let Some(file_pos_rel) = line[at_pos + 4..].find(file_name)
    {
        let file_pos = at_pos + 4 + file_pos_rel;
        return format!(
            "{}<FILE>{}",
            &line[..at_pos + 4],
            &line[file_pos + file_name.len()..]
        );
    }
    if let Some(file_pos) = line.find(file_name)
        && let Some(slash_pos) = line[..file_pos].rfind('/')
    {
        return format!(
            "{}<FILE>{}",
            &line[..slash_pos + 1],
            &line[file_pos + file_name.len()..]
        );
    }
    line.to_string()
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
fn dead_bare_identifier_after_return_must_fail() {
    let src = r#"
fn main() {
  return 0
  foo
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "dead_bare_identifier.rr");
    assert!(!ok, "compile must fail for dead bare identifier statements");
    assert!(
        stdout.contains("** (RR.SemanticError)"),
        "missing semantic error header:\n{}",
        stdout
    );
    assert!(
        stdout.contains("undefined variable 'foo'"),
        "missing dead-code undefined variable detail:\n{}",
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
fn undefined_variable_suggests_nearby_binding() {
    let src = r#"
fn main() {
  let total = 1
  return toatl + total
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "undefined_var_suggest.rr");
    assert!(!ok, "compile must fail for undefined variable");
    assert!(
        stdout.contains("help: did you mean `total`?"),
        "missing undefined variable suggestion:\n{}",
        stdout
    );
}

#[test]
fn undefined_function_suggests_nearby_builtin() {
    let src = r#"
fn main() {
  return pritn(1)
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile(src, "undefined_fn_suggest.rr");
    assert!(!ok, "compile must fail for undefined function");
    assert!(
        stdout.contains("help: did you mean one of:") && stdout.contains("`print`"),
        "missing undefined function suggestion set:\n{}",
        stdout
    );
}

#[test]
fn undefined_function_diagnostic_output_is_exact() {
    let file_name = "undefined_fn_exact.rr";
    let src = "fn main() {\n  return pritn(1)\n}\nmain()\n";
    let (ok, stdout, _stderr) = run_compile(src, file_name);
    assert!(!ok, "compile must fail for undefined function");
    let actual = normalize_primary_diagnostic(&stdout, file_name);
    let expected = concat!(
        "** (RR.SemanticError) undefined function 'pritn'\n",
        "    error[E1001]: undefined function 'pritn'\n",
        "    at <FILE>:2:10 (MIR)\n",
        "   2 |   return pritn(1)\n",
        "                ^ [primary]\n",
        "note (R): Define or import the function before calling it.\n",
        "help: did you mean one of: `pmin`, `print`?\n",
    );
    assert_eq!(actual, expected, "undefined-function diagnostic drifted");
}

#[test]
fn undefined_variable_diagnostic_output_is_exact() {
    let file_name = "undefined_var_exact.rr";
    let src = "fn main() {\n  let total = 1\n  return toatl + total\n}\nmain()\n";
    let (ok, stdout, _stderr) = run_compile(src, file_name);
    assert!(!ok, "compile must fail for undefined variable");
    let actual = normalize_primary_diagnostic(&stdout, file_name);
    let expected = concat!(
        "** (RR.SemanticError) undefined variable 'toatl'\n",
        "    error[E1001]: undefined variable 'toatl'\n",
        "    at <FILE>:3:10 (MIR)\n",
        "   3 |   return toatl + total\n",
        "                ^ [primary]\n",
        "    stacktrace:\n",
        "      (rr) mir::semantics::validate_function/2 at <FILE>:3:10\n",
        "hint: Declare the variable with let before use.\n",
        "help: did you mean `total`?\n",
    );
    assert_eq!(actual, expected, "undefined-variable diagnostic drifted");
}

#[test]
fn strict_let_diagnostic_output_is_exact() {
    let file_name = "implicit_decl_strict_exact.rr";
    let src = "fn main() {\n  let total = 1\n  toatl <- 2\n  return total\n}\nmain()\n";
    let (ok, stdout, _stderr) = run_compile_with_env(src, file_name, &[("RR_STRICT_LET", "1")]);
    assert!(!ok, "compile must fail in strict let mode");
    let actual = normalize_primary_diagnostic(&stdout, file_name);
    let expected = concat!(
        "** (RR.SemanticError) assignment to undeclared variable 'toatl'\n",
        "    error[E1001]: assignment to undeclared variable 'toatl'\n",
        "    at <FILE>:3:3 (Lower)\n",
        "   3 |   toatl <- 2\n",
        "         ^ [primary]\n",
        "hint: Declare it first with `let` before assignment.\n",
        "help: did you mean `total`?\n",
    );
    assert_eq!(actual, expected, "strict-let diagnostic drifted");
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
fn implicit_declaration_warns_when_strict_let_is_disabled() {
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
        &[("RR_STRICT_LET", "0"), ("RR_WARN_IMPLICIT_DECL", "1")],
    );
    assert!(
        ok,
        "compile should succeed when strict let is explicitly disabled"
    );
    assert!(
        stderr.contains("implicit declaration via assignment"),
        "expected implicit declaration warning in stderr, got:\n{}",
        stderr
    );
}

#[test]
fn default_mode_rejects_implicit_declaration() {
    let src = r#"
fn main() {
  x <- 1
  return x
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile_with_env(src, "implicit_decl_strict.rr", &[]);
    assert!(!ok, "compile must fail by default in strict let mode");
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

#[test]
fn default_mode_suggests_nearby_binding() {
    let src = r#"
fn main() {
  let total = 1
  toatl <- 2
  return total
}
main()
"#;
    let (ok, stdout, _stderr) = run_compile_with_env(src, "implicit_decl_strict_suggest.rr", &[]);
    assert!(!ok, "compile must fail by default in strict let mode");
    assert!(
        stdout.contains("help: did you mean `total`?"),
        "missing strict-let suggestion:\n{}",
        stdout
    );
}

#[test]
fn strict_let_can_be_disabled_for_legacy_code() {
    let src = r#"
fn main() {
  x <- 1
  x <- x + 1
  return x
}
main()
"#;
    let (ok, stdout, stderr) = run_compile_with_env(
        src,
        "implicit_decl_legacy_opt_out.rr",
        &[("RR_STRICT_LET", "0")],
    );
    assert!(ok, "compile should succeed when strict let is disabled");
    assert!(
        !stdout.contains("** (RR.SemanticError)"),
        "unexpected semantic error when strict let is disabled:\n{}",
        stdout
    );
    assert!(
        stderr.is_empty(),
        "unexpected stderr when strict let is disabled without warnings:\n{}",
        stderr
    );
}
