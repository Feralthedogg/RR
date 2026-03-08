mod common;

use common::{rscript_available, rscript_path, unique_dir};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn build_command_writes_r_files_into_build_dir() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_build");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(proj_dir.join("src")).expect("failed to create project dirs");

    let main_src = r#"
fn main() {
  let x = 1
  print(x)
}
main()
"#;
    let util_src = r#"
fn helper(x) {
  return x + 1
}
"#;
    fs::write(proj_dir.join("main.rr"), main_src).expect("failed to write main.rr");
    fs::write(proj_dir.join("src").join("util.rr"), util_src).expect("failed to write util.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out_dir = proj_dir.join("build");
    let status = Command::new(&rr_bin)
        .arg("build")
        .arg(&proj_dir)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("-O0")
        .status()
        .expect("failed to run rr build");
    assert!(status.success(), "rr build failed");

    assert!(
        out_dir.join("main.R").exists(),
        "expected build/main.R to be generated"
    );
    assert!(
        out_dir.join("src").join("util.R").exists(),
        "expected build/src/util.R to be generated"
    );
}

#[test]
fn run_command_finds_main_rr_from_dot() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping run command test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_run");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_src = r#"
fn main() {
  print(123)
}
main()
"#;
    fs::write(proj_dir.join("main.rr"), main_src).expect("failed to write main.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("run")
        .arg(".")
        .arg("-O0")
        .env("RRSCRIPT", &rscript)
        .output()
        .expect("failed to run rr run .");

    assert!(
        output.status.success(),
        "rr run . failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[1] 123"),
        "expected runtime output from main.rr, got:\n{}",
        stdout
    );
}

#[test]
fn version_flag_prints_crate_version() {
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let output = Command::new(&rr_bin)
        .arg("--version")
        .output()
        .expect("failed to run rr --version");

    assert!(
        output.status.success(),
        "rr --version failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        format!("RR Tachyon v{}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn version_command_prints_crate_version() {
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let output = Command::new(&rr_bin)
        .arg("version")
        .output()
        .expect("failed to run rr version");

    assert!(
        output.status.success(),
        "rr version failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        format!("RR Tachyon v{}", env!("CARGO_PKG_VERSION"))
    );
}
