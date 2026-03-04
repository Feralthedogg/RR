use std::path::PathBuf;
use std::process::Command;

fn rr_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_RR"))
}

fn stderr_text(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn legacy_unknown_flag_is_reported() {
    let output = Command::new(rr_bin())
        .arg("--unknown-flag")
        .output()
        .expect("failed to run rr");
    assert!(!output.status.success(), "expected parse failure");
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("Unknown option: --unknown-flag"),
        "expected unknown-option diagnostic, got:\n{}",
        stderr
    );
}

#[test]
fn build_missing_out_dir_value_is_reported() {
    let output = Command::new(rr_bin())
        .arg("build")
        .arg("--out-dir")
        .output()
        .expect("failed to run rr");
    assert!(!output.status.success(), "expected parse failure");
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("Missing directory path after --out-dir"),
        "expected missing-value diagnostic, got:\n{}",
        stderr
    );
}

#[test]
fn run_missing_parallel_mode_value_is_reported() {
    let output = Command::new(rr_bin())
        .arg("run")
        .arg(".")
        .arg("--parallel-mode")
        .output()
        .expect("failed to run rr");
    assert!(!output.status.success(), "expected parse failure");
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("Missing value after --parallel-mode (off|optional|required)"),
        "expected missing-value diagnostic, got:\n{}",
        stderr
    );
}

#[test]
fn run_invalid_parallel_mode_value_is_reported() {
    let output = Command::new(rr_bin())
        .arg("run")
        .arg(".")
        .arg("--parallel-mode")
        .arg("bad")
        .output()
        .expect("failed to run rr");
    assert!(!output.status.success(), "expected parse failure");
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("Invalid --parallel-mode. Use off|optional|required"),
        "expected invalid-value diagnostic, got:\n{}",
        stderr
    );
}
