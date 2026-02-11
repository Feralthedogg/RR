use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_dir(root: &PathBuf, name: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    root.join(format!("{}_{}_{}", name, std::process::id(), ts))
}

fn run_compile(source: &str, file_name: &str) -> (bool, String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("semantic_errors");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_path = proj_dir.join(file_name);
    let out_path = proj_dir.join("out.R");
    fs::write(&rr_path, source).expect("failed to write source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let output = Command::new(rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .output()
        .expect("failed to run RR");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn run_compile_with_env(
    source: &str,
    file_name: &str,
    env_kv: &[(&str, &str)],
) -> (bool, String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("semantic_errors");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj_env");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_path = proj_dir.join(file_name);
    let out_path = proj_dir.join("out.R");
    fs::write(&rr_path, source).expect("failed to write source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let mut cmd = Command::new(rr_bin);
    cmd.arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1");
    for (k, v) in env_kv {
        cmd.env(k, v);
    }
    let output = cmd.output().expect("failed to run RR");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn undefined_variable_must_fail() {
    let src = r#"
fn main() {
  let x = 1;
  return y + x;
}
main();
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
  return foo(1);
}
main();
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
  return a + b;
}
fn main() {
  return add(1);
}
main();
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
  x <- 1;
  x <- x + 1;
  return x;
}
main();
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
  x <- 1;
  return x;
}
main();
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
