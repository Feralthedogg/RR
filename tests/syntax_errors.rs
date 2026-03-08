mod common;

use common::unique_dir;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn invalid_character_must_fail_compile() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("syntax_errors");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "invalid_char");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let bad_src = r#"
fn main() {
  let x = 1$
  print(x)
}
main()
"#;
    let rr_path = proj_dir.join("bad.rr");
    let out_path = proj_dir.join("bad.R");
    fs::write(&rr_path, bad_src).expect("failed to write bad.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let output = Command::new(rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O0")
        .output()
        .expect("failed to run RR");

    assert!(
        !output.status.success(),
        "compile must fail for invalid syntax"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("** (RR.ParseError)"),
        "expected formatted parse error header, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("unexpected character '$'"),
        "expected invalid character detail, got:\n{}",
        stdout
    );
}

#[test]
fn unterminated_string_must_fail_compile() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("syntax_errors");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "unterminated_string");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let bad_src = r#"
fn main() {
  let s = "hello;
  print(s)
}
main()
"#;
    let rr_path = proj_dir.join("bad_string.rr");
    let out_path = proj_dir.join("bad_string.R");
    fs::write(&rr_path, bad_src).expect("failed to write bad_string.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let output = Command::new(rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O0")
        .output()
        .expect("failed to run RR");

    assert!(
        !output.status.success(),
        "compile must fail for unterminated string"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("** (RR.ParseError)"),
        "expected formatted parse error header, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("unterminated string literal"),
        "expected unterminated string detail, got:\n{}",
        stdout
    );
}
