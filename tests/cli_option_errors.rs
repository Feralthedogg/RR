#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::{fs, path::Path};

fn rr_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_RR"))
}

fn stderr_text(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn assert_parse_failure(args: &[&str], expected: &str) {
    let output = Command::new(rr_bin())
        .args(args)
        .output()
        .expect("failed to run rr");
    assert!(!output.status.success(), "expected parse failure");
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains(expected),
        "expected parse diagnostic '{}', got:\n{}",
        expected,
        stderr
    );
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

#[test]
fn legacy_missing_compiler_parallel_flag_values_are_reported() {
    let cases = [
        (
            "--compiler-parallel-mode",
            "Missing value after --compiler-parallel-mode (off|auto|on)",
        ),
        (
            "--compiler-parallel-threads",
            "Missing value after --compiler-parallel-threads",
        ),
        (
            "--compiler-parallel-min-functions",
            "Missing value after --compiler-parallel-min-functions",
        ),
        (
            "--compiler-parallel-min-fn-ir",
            "Missing value after --compiler-parallel-min-fn-ir",
        ),
        (
            "--compiler-parallel-max-jobs",
            "Missing value after --compiler-parallel-max-jobs",
        ),
    ];

    for (flag, expected) in cases {
        assert_parse_failure(&[flag], expected);
    }
}

#[test]
fn legacy_invalid_compiler_parallel_flag_values_are_reported() {
    let cases = [
        (
            "--compiler-parallel-mode",
            "bad",
            "Invalid --compiler-parallel-mode. Use off|auto|on",
        ),
        (
            "--compiler-parallel-threads",
            "bad",
            "Invalid --compiler-parallel-threads. Use a non-negative integer.",
        ),
        (
            "--compiler-parallel-min-functions",
            "bad",
            "Invalid --compiler-parallel-min-functions. Use a non-negative integer.",
        ),
        (
            "--compiler-parallel-min-fn-ir",
            "bad",
            "Invalid --compiler-parallel-min-fn-ir. Use a non-negative integer.",
        ),
        (
            "--compiler-parallel-max-jobs",
            "bad",
            "Invalid --compiler-parallel-max-jobs. Use a non-negative integer.",
        ),
    ];

    for (flag, value, expected) in cases {
        assert_parse_failure(&[flag, value], expected);
    }
}

#[test]
fn run_missing_main_reports_recovery_hint() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root.join("target").join("tests").join("cli_option_errors");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = sandbox.join(format!("run_missing_main_{}", std::process::id()));
    let _ = fs::remove_dir_all(&proj);
    fs::create_dir_all(&proj).expect("failed to create project dir");

    let output = Command::new(rr_bin())
        .arg("run")
        .arg(&proj)
        .output()
        .expect("failed to run rr run <dir-without-main>");
    assert!(
        !output.status.success(),
        "expected run target resolution failure"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("src/main.rr or main.rr not found"),
        "expected missing-entry diagnostic, got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("add src/main.rr")
            || stderr.contains("legacy main.rr")
            || stderr.contains("explicit .rr file path"),
        "expected recovery hint for missing entry file, got:\n{}",
        stderr
    );
}

#[test]
fn watch_missing_main_reports_recovery_hint() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root.join("target").join("tests").join("cli_option_errors");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = sandbox.join(format!("watch_missing_main_{}", std::process::id()));
    let _ = fs::remove_dir_all(&proj);
    fs::create_dir_all(&proj).expect("failed to create project dir");

    let output = Command::new(rr_bin())
        .arg("watch")
        .arg(&proj)
        .arg("--once")
        .arg("--poll-ms")
        .arg("1")
        .output()
        .expect("failed to run rr watch <dir-without-main>");
    assert!(
        !output.status.success(),
        "expected watch target resolution failure"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("src/main.rr or main.rr not found"),
        "expected missing-entry diagnostic, got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("RR watch") && stderr.contains("explicit .rr file path"),
        "expected watch-specific recovery hint, got:\n{}",
        stderr
    );
}

#[test]
fn watch_non_rr_file_reports_watch_specific_hint() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root.join("target").join("tests").join("cli_option_errors");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = sandbox.join(format!("watch_non_rr_file_{}", std::process::id()));
    let _ = fs::remove_dir_all(&proj);
    fs::create_dir_all(&proj).expect("failed to create project dir");
    let txt = proj.join("notes.txt");
    fs::write(&txt, "not rr").expect("failed to write notes.txt");

    let output = Command::new(rr_bin())
        .arg("watch")
        .arg(&txt)
        .arg("--once")
        .arg("--poll-ms")
        .arg("1")
        .output()
        .expect("failed to run rr watch on non-rr file");
    assert!(
        !output.status.success(),
        "expected watch target resolution failure"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("watch target must be a .rr file or directory"),
        "expected watch-specific invalid-target diagnostic, got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("point RR watch at a directory containing src/main.rr or main.rr"),
        "expected watch-specific invalid-target recovery hint, got:\n{}",
        stderr
    );
}

#[test]
fn build_missing_target_reports_recovery_hint() {
    let output = Command::new(rr_bin())
        .arg("build")
        .arg("definitely_missing_rr_target")
        .output()
        .expect("failed to run rr build missing-target");
    assert!(
        !output.status.success(),
        "expected build target resolution failure"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("build target not found"),
        "expected build-target diagnostic, got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("pass an existing directory or .rr file"),
        "expected build recovery hint, got:\n{}",
        stderr
    );
}

#[test]
fn build_empty_project_reports_recovery_hint() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root.join("target").join("tests").join("cli_option_errors");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = sandbox.join(format!("build_empty_project_{}", std::process::id()));
    let _ = fs::remove_dir_all(&proj);
    fs::create_dir_all(&proj).expect("failed to create empty project dir");

    let output = Command::new(rr_bin())
        .arg("build")
        .arg(&proj)
        .output()
        .expect("failed to run rr build empty-project");
    assert!(
        !output.status.success(),
        "expected build failure for empty project"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("no .rr files found under"),
        "expected empty-project diagnostic, got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("add at least one .rr source file")
            || stderr.contains("point RR build at a specific .rr file"),
        "expected empty-project recovery hint, got:\n{}",
        stderr
    );
}

#[cfg(unix)]
#[test]
fn build_project_mode_ignores_unreadable_unrelated_subtree() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root.join("target").join("tests").join("cli_option_errors");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = sandbox.join(format!("build_scan_failure_{}", std::process::id()));
    let _ = fs::remove_dir_all(&proj);
    fs::create_dir_all(&proj).expect("failed to create project dir");
    fs::write(
        proj.join("main.rr"),
        r#"
fn main() {
  print(1L)
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let blocked = proj.join("blocked");
    fs::create_dir_all(&blocked).expect("failed to create blocked dir");
    let mut perms = fs::metadata(&blocked)
        .expect("failed to stat blocked dir")
        .permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&blocked, perms).expect("failed to chmod blocked dir unreadable");

    let output = Command::new(rr_bin())
        .arg("build")
        .arg(&proj)
        .output()
        .expect("failed to run rr build with unreadable subtree");

    let mut restore = fs::metadata(&blocked)
        .expect("failed to restat blocked dir")
        .permissions();
    restore.set_mode(0o755);
    fs::set_permissions(&blocked, restore).expect("failed to restore blocked dir permissions");

    assert!(
        output.status.success(),
        "project-mode build should ignore unrelated unreadable subtree:\n{}",
        stderr_text(&output)
    );
}

#[cfg(unix)]
#[test]
fn build_unreadable_input_reports_recovery_hint() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root.join("target").join("tests").join("cli_option_errors");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = sandbox.join(format!("build_unreadable_input_{}", std::process::id()));
    let _ = fs::remove_dir_all(&proj);
    fs::create_dir_all(&proj).expect("failed to create project dir");

    let main_path = proj.join("main.rr");
    fs::write(
        &main_path,
        r#"
fn main() {
  print(1L)
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let mut perms = fs::metadata(&main_path)
        .expect("failed to stat main.rr")
        .permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&main_path, perms).expect("failed to chmod main.rr unreadable");

    let output = Command::new(rr_bin())
        .arg("build")
        .arg(&main_path)
        .output()
        .expect("failed to run rr build with unreadable input");

    let mut restore = fs::metadata(&main_path)
        .expect("failed to restat main.rr")
        .permissions();
    restore.set_mode(0o644);
    fs::set_permissions(&main_path, restore).expect("failed to restore main.rr permissions");

    assert!(
        !output.status.success(),
        "expected build failure when input is unreadable"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("Failed to read") && stderr.contains("make the build input readable"),
        "expected build input recovery hint, got:\n{}",
        stderr
    );
}

#[test]
fn run_missing_rscript_reports_recovery_hint() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root.join("target").join("tests").join("cli_option_errors");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = sandbox.join(format!("run_missing_rscript_{}", std::process::id()));
    let _ = fs::remove_dir_all(&proj);
    fs::create_dir_all(&proj).expect("failed to create project dir");
    fs::write(
        proj.join("main.rr"),
        r#"
fn main() {
  print(1L)
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let output = Command::new(rr_bin())
        .current_dir(&proj)
        .arg("run")
        .arg(".")
        .env("PATH", "")
        .env("RRSCRIPT", "/definitely/missing/Rscript")
        .output()
        .expect("failed to run rr run with missing RRSCRIPT");
    assert!(
        !output.status.success(),
        "expected runner failure when Rscript is unavailable"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("failed to execute '/definitely/missing/Rscript'"),
        "expected runner execution failure, got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("install Rscript or set RRSCRIPT=/absolute/path/to/Rscript"),
        "expected Rscript recovery hint, got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("rerun with --keep-r"),
        "expected generated-artifact hint, got:\n{}",
        stderr
    );
    assert!(
        !proj.join("main.gen.R").exists(),
        "generated artifact should be removed when --keep-r is not set"
    );
}

#[test]
fn run_missing_rscript_with_keep_r_preserves_generated_artifact() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root.join("target").join("tests").join("cli_option_errors");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = sandbox.join(format!("run_missing_rscript_keep_r_{}", std::process::id()));
    let _ = fs::remove_dir_all(&proj);
    fs::create_dir_all(&proj).expect("failed to create project dir");
    fs::write(
        proj.join("main.rr"),
        r#"
fn main() {
  print(1L)
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let output = Command::new(rr_bin())
        .current_dir(&proj)
        .arg("run")
        .arg(".")
        .arg("--keep-r")
        .env("PATH", "")
        .env("RRSCRIPT", "/definitely/missing/Rscript")
        .output()
        .expect("failed to run rr run with missing RRSCRIPT and --keep-r");
    assert!(
        !output.status.success(),
        "expected runner failure when Rscript is unavailable"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("generated artifact kept at"),
        "expected kept-artifact hint, got:\n{}",
        stderr
    );
    assert!(
        proj.join("main.gen.R").exists(),
        "generated artifact should be preserved when --keep-r is set"
    );
}

#[cfg(unix)]
#[test]
fn run_unwritable_generated_artifact_path_reports_recovery_hint() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root.join("target").join("tests").join("cli_option_errors");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = sandbox.join(format!("run_unwritable_gen_{}", std::process::id()));
    let _ = fs::remove_dir_all(&proj);
    fs::create_dir_all(&proj).expect("failed to create project dir");
    fs::write(
        proj.join("main.rr"),
        r#"
fn main() {
  print(1L)
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let mut perms = fs::metadata(&proj)
        .expect("failed to stat project dir")
        .permissions();
    perms.set_mode(0o555);
    fs::set_permissions(&proj, perms).expect("failed to chmod project dir read-only");

    let output = Command::new(rr_bin())
        .current_dir(&proj)
        .arg("run")
        .arg(".")
        .arg("--no-incremental")
        .arg("--cold")
        .output()
        .expect("failed to run rr run with unwritable generated artifact path");

    let mut restore = fs::metadata(&proj)
        .expect("failed to restat project dir")
        .permissions();
    restore.set_mode(0o755);
    fs::set_permissions(&proj, restore).expect("failed to restore project dir permissions");

    assert!(
        !output.status.success(),
        "expected runner failure when generated artifact path is unwritable"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("failed to write generated R file"),
        "expected generated-artifact write failure, got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("writes a temporary generated artifact at")
            && stderr.contains("make that directory writable")
            && stderr.contains("RR build --out-dir <dir>"),
        "expected generated-artifact recovery hint, got:\n{}",
        stderr
    );
    assert!(
        !proj.join("main.gen.R").exists(),
        "generated artifact should not exist when write fails"
    );
}

#[cfg(unix)]
#[test]
fn run_unreadable_input_reports_recovery_hint() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root.join("target").join("tests").join("cli_option_errors");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = sandbox.join(format!("run_unreadable_input_{}", std::process::id()));
    let _ = fs::remove_dir_all(&proj);
    fs::create_dir_all(&proj).expect("failed to create project dir");

    let main_path = proj.join("main.rr");
    fs::write(
        &main_path,
        r#"
fn main() {
  print(1L)
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let mut perms = fs::metadata(&main_path)
        .expect("failed to stat main.rr")
        .permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&main_path, perms).expect("failed to chmod main.rr unreadable");

    let output = Command::new(rr_bin())
        .current_dir(&proj)
        .arg("run")
        .arg(".")
        .arg("--no-incremental")
        .output()
        .expect("failed to run rr run with unreadable input");

    let mut restore = fs::metadata(&main_path)
        .expect("failed to restat main.rr")
        .permissions();
    restore.set_mode(0o644);
    fs::set_permissions(&main_path, restore).expect("failed to restore main.rr permissions");

    assert!(
        !output.status.success(),
        "expected run failure when input is unreadable"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("Failed to read") && stderr.contains("make the run input readable"),
        "expected run input recovery hint, got:\n{}",
        stderr
    );
}

#[cfg(unix)]
#[test]
fn watch_unreadable_input_reports_recovery_hint() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root.join("target").join("tests").join("cli_option_errors");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = sandbox.join(format!("watch_unreadable_input_{}", std::process::id()));
    let _ = fs::remove_dir_all(&proj);
    fs::create_dir_all(&proj).expect("failed to create project dir");

    let main_path = proj.join("main.rr");
    fs::write(
        &main_path,
        r#"
fn main() {
  print(1L)
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let mut perms = fs::metadata(&main_path)
        .expect("failed to stat main.rr")
        .permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&main_path, perms).expect("failed to chmod main.rr unreadable");

    let output = Command::new(rr_bin())
        .current_dir(&proj)
        .arg("watch")
        .arg(".")
        .arg("--once")
        .arg("--poll-ms")
        .arg("1")
        .output()
        .expect("failed to run rr watch with unreadable input");

    let mut restore = fs::metadata(&main_path)
        .expect("failed to restat main.rr")
        .permissions();
    restore.set_mode(0o644);
    fs::set_permissions(&main_path, restore).expect("failed to restore main.rr permissions");

    assert!(
        !output.status.success(),
        "expected watch failure when input is unreadable"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("Failed to read") && stderr.contains("make the watch input readable"),
        "expected watch input recovery hint, got:\n{}",
        stderr
    );
}

#[test]
fn watch_unusable_output_dir_reports_recovery_hint() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root.join("target").join("tests").join("cli_option_errors");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = sandbox.join(format!("watch_bad_out_dir_{}", std::process::id()));
    let _ = fs::remove_dir_all(&proj);
    fs::create_dir_all(&proj).expect("failed to create project dir");
    fs::write(
        proj.join("main.rr"),
        r#"
fn main() {
  print(1L)
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let bad_root = proj.join("not_a_directory");
    fs::write(&bad_root, "file").expect("failed to create blocking file path");
    let out_file = bad_root.join("watched.R");

    let output = Command::new(rr_bin())
        .current_dir(&proj)
        .arg("watch")
        .arg(".")
        .arg("--once")
        .arg("--poll-ms")
        .arg("1")
        .arg("-o")
        .arg(&out_file)
        .output()
        .expect("failed to run rr watch with unusable output dir");

    assert!(
        !output.status.success(),
        "expected watch failure when output dir cannot be created"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("Failed to create")
            && stderr.contains("choose a different watch output directory"),
        "expected watch output-dir recovery hint, got:\n{}",
        stderr
    );
}

#[cfg(unix)]
#[test]
fn watch_unwritable_output_file_reports_recovery_hint() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root.join("target").join("tests").join("cli_option_errors");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = sandbox.join(format!(
        "watch_unwritable_output_file_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&proj);
    fs::create_dir_all(&proj).expect("failed to create project dir");
    fs::write(
        proj.join("main.rr"),
        r#"
fn main() {
  print(1L)
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let out_file = proj.join("watched.R");
    fs::write(&out_file, "seed").expect("failed to seed watched.R");
    let mut perms = fs::metadata(&out_file)
        .expect("failed to stat watched.R")
        .permissions();
    perms.set_mode(0o444);
    fs::set_permissions(&out_file, perms).expect("failed to chmod watched.R read-only");

    let output = Command::new(rr_bin())
        .current_dir(&proj)
        .arg("watch")
        .arg(".")
        .arg("--once")
        .arg("--poll-ms")
        .arg("1")
        .arg("-o")
        .arg(&out_file)
        .output()
        .expect("failed to run rr watch with unwritable output file");

    let mut restore = fs::metadata(&out_file)
        .expect("failed to restat watched.R")
        .permissions();
    restore.set_mode(0o644);
    fs::set_permissions(&out_file, restore).expect("failed to restore watched.R permissions");

    assert!(
        !output.status.success(),
        "expected watch failure when output file is unwritable"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("Failed to write")
            && stderr.contains("choose a writable watch output path"),
        "expected watch output-file recovery hint, got:\n{}",
        stderr
    );
}

#[test]
fn build_unusable_out_dir_reports_recovery_hint() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root.join("target").join("tests").join("cli_option_errors");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = sandbox.join(format!("build_bad_out_dir_{}", std::process::id()));
    let _ = fs::remove_dir_all(&proj);
    fs::create_dir_all(&proj).expect("failed to create project dir");
    fs::write(
        proj.join("main.rr"),
        r#"
fn main() {
  print(1L)
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let bad_root = proj.join("not_a_directory");
    fs::write(&bad_root, "file").expect("failed to create blocking file path");
    let out_dir = bad_root.join("build");

    let output = Command::new(rr_bin())
        .arg("build")
        .arg(&proj)
        .arg("--out-dir")
        .arg(&out_dir)
        .output()
        .expect("failed to run rr build with unusable out dir");

    assert!(
        !output.status.success(),
        "expected build failure when out-dir cannot be created"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("Failed to create")
            && stderr.contains("choose a different build --out-dir destination"),
        "expected build out-dir recovery hint, got:\n{}",
        stderr
    );
}

#[cfg(unix)]
#[test]
fn build_unwritable_output_file_reports_recovery_hint() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sandbox = root.join("target").join("tests").join("cli_option_errors");
    fs::create_dir_all(&sandbox).expect("failed to create sandbox root");
    let proj = sandbox.join(format!(
        "build_unwritable_output_file_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&proj);
    fs::create_dir_all(&proj).expect("failed to create project dir");
    fs::write(
        proj.join("main.rr"),
        r#"
fn main() {
  print(1L)
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let out_dir = proj.join("build");
    fs::create_dir_all(&out_dir).expect("failed to create build dir");
    let out_file = out_dir.join("main.R");
    fs::write(&out_file, "seed").expect("failed to seed main.R");
    let mut perms = fs::metadata(&out_file)
        .expect("failed to stat main.R")
        .permissions();
    perms.set_mode(0o444);
    fs::set_permissions(&out_file, perms).expect("failed to chmod main.R read-only");

    let output = Command::new(rr_bin())
        .arg("build")
        .arg(&proj)
        .arg("--out-dir")
        .arg(&out_dir)
        .output()
        .expect("failed to run rr build with unwritable output file");

    let mut restore = fs::metadata(&out_file)
        .expect("failed to restat main.R")
        .permissions();
    restore.set_mode(0o644);
    fs::set_permissions(&out_file, restore).expect("failed to restore main.R permissions");

    assert!(
        !output.status.success(),
        "expected build failure when output file is unwritable"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("Failed to write")
            && stderr.contains("choose a writable build output path"),
        "expected build output-file recovery hint, got:\n{}",
        stderr
    );
}
