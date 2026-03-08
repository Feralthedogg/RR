mod common;

use common::{normalize, rscript_available, rscript_path, unique_dir};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn licm_does_not_speculate_potentially_failing_call_from_zero_iter_loop() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping LICM speculation safety test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("licm_speculation_safety");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_src = r#"
fn main() {
  let i = 0
  while (i < 0) {
    let t = rr_bool(NA)
    i = i + 1
  }
  print(42)
  return 0
}
main()
"#;

    let rr_path = proj_dir.join("licm_speculation.rr");
    fs::write(&rr_path, rr_src).expect("failed to write source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let base_out = proj_dir.join("base.R");
    let licm_out = proj_dir.join("licm.R");

    let base_compile = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&base_out)
        .arg("--no-runtime")
        .arg("-O1")
        .output()
        .expect("failed to run base compile");
    assert!(
        base_compile.status.success(),
        "base compile failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&base_compile.stdout),
        String::from_utf8_lossy(&base_compile.stderr)
    );

    let licm_compile = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&licm_out)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_ENABLE_LICM", "1")
        .output()
        .expect("failed to run LICM compile");
    assert!(
        licm_compile.status.success(),
        "LICM compile failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&licm_compile.stdout),
        String::from_utf8_lossy(&licm_compile.stderr)
    );

    let base_run = Command::new(&rscript)
        .arg("--vanilla")
        .arg(&base_out)
        .output()
        .expect("failed to run baseline R output");
    let licm_run = Command::new(&rscript)
        .arg("--vanilla")
        .arg(&licm_out)
        .output()
        .expect("failed to run LICM R output");

    assert_eq!(
        base_run.status.code().unwrap_or(-1),
        licm_run.status.code().unwrap_or(-1),
        "LICM changed runtime exit status"
    );
    assert_eq!(
        normalize(&String::from_utf8_lossy(&base_run.stdout)),
        normalize(&String::from_utf8_lossy(&licm_run.stdout)),
        "LICM changed runtime stdout"
    );
    assert_eq!(
        normalize(&String::from_utf8_lossy(&base_run.stderr)),
        normalize(&String::from_utf8_lossy(&licm_run.stderr)),
        "LICM changed runtime stderr"
    );
}
