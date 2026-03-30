mod common;

use common::{rscript_available, rscript_path, unique_dir};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
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
fn build_command_accepts_compiler_parallel_flags() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_build");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "compiler_parallel");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_src = r#"
fn square(x) {
  return x * x
}

fn main() {
  print(square(7L))
}
main()
"#;
    fs::write(proj_dir.join("main.rr"), main_src).expect("failed to write main.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out_dir = proj_dir.join("build");
    let status = Command::new(&rr_bin)
        .arg("build")
        .arg(&proj_dir)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("-O1")
        .arg("--compiler-parallel-mode")
        .arg("on")
        .arg("--compiler-parallel-threads")
        .arg("2")
        .arg("--compiler-parallel-min-functions")
        .arg("1")
        .arg("--compiler-parallel-min-fn-ir")
        .arg("1")
        .arg("--compiler-parallel-max-jobs")
        .arg("2")
        .status()
        .expect("failed to run rr build with compiler parallel flags");
    assert!(
        status.success(),
        "rr build with compiler parallel flags failed"
    );
    assert!(
        out_dir.join("main.R").is_file(),
        "expected build/main.R to be generated"
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

#[cfg(unix)]
#[test]
fn run_command_uses_rscript_env_override() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_run");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "rscript_override");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_src = r#"
fn main() {
  print(123)
}
main()
"#;
    fs::write(proj_dir.join("main.rr"), main_src).expect("failed to write main.rr");

    let fake_rscript = proj_dir.join("fake_rscript.sh");
    fs::write(&fake_rscript, "#!/bin/sh\nprintf '[1] 777\\n'\nexit 0\n")
        .expect("failed to write fake Rscript");
    let mut perms = fs::metadata(&fake_rscript)
        .expect("failed to stat fake Rscript")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&fake_rscript, perms).expect("failed to chmod fake Rscript");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("run")
        .arg(".")
        .arg("-O0")
        .env("PATH", "")
        .env("RRSCRIPT", &fake_rscript)
        .output()
        .expect("failed to run rr run . with fake RRSCRIPT");

    assert!(
        output.status.success(),
        "rr run . with fake RRSCRIPT failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[1] 777"),
        "expected fake RRSCRIPT output, got:\n{}",
        stdout
    );
}

#[cfg(unix)]
#[test]
fn run_keep_r_preserves_generated_artifact_on_success() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_run");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "keep_r_success");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_src = r#"
fn main() {
  print(123)
}
main()
"#;
    fs::write(proj_dir.join("main.rr"), main_src).expect("failed to write main.rr");

    let fake_rscript = proj_dir.join("fake_rscript.sh");
    fs::write(&fake_rscript, "#!/bin/sh\nprintf '[1] 888\\n'\nexit 0\n")
        .expect("failed to write fake Rscript");
    let mut perms = fs::metadata(&fake_rscript)
        .expect("failed to stat fake Rscript")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&fake_rscript, perms).expect("failed to chmod fake Rscript");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("run")
        .arg(".")
        .arg("-O0")
        .arg("--keep-r")
        .env("PATH", "")
        .env("RRSCRIPT", &fake_rscript)
        .output()
        .expect("failed to run rr run . --keep-r with fake RRSCRIPT");

    assert!(
        output.status.success(),
        "rr run . --keep-r failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("[1] 888"),
        "expected fake RRSCRIPT output, got:\n{}",
        stdout
    );
    assert!(
        stderr.contains("help: kept generated artifact at"),
        "expected kept-artifact hint, got:\n{}",
        stderr
    );
    assert!(
        proj_dir.join("main.gen.R").exists(),
        "expected generated artifact to be preserved by --keep-r"
    );
}

#[cfg(unix)]
#[test]
fn run_keep_r_accepts_compiler_parallel_flags() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_run");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "compiler_parallel");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_src = r#"
fn square(x) {
  return x * x
}

fn main() {
  print(square(11L))
}
main()
"#;
    fs::write(proj_dir.join("main.rr"), main_src).expect("failed to write main.rr");

    let fake_rscript = proj_dir.join("fake_rscript.sh");
    fs::write(&fake_rscript, "#!/bin/sh\nprintf '[1] 999\\n'\nexit 0\n")
        .expect("failed to write fake Rscript");
    let mut perms = fs::metadata(&fake_rscript)
        .expect("failed to stat fake Rscript")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&fake_rscript, perms).expect("failed to chmod fake Rscript");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("run")
        .arg(".")
        .arg("-O1")
        .arg("--keep-r")
        .arg("--compiler-parallel-mode")
        .arg("on")
        .arg("--compiler-parallel-threads")
        .arg("2")
        .arg("--compiler-parallel-min-functions")
        .arg("1")
        .arg("--compiler-parallel-min-fn-ir")
        .arg("1")
        .arg("--compiler-parallel-max-jobs")
        .arg("2")
        .env("PATH", "")
        .env("RRSCRIPT", &fake_rscript)
        .output()
        .expect("failed to run rr run with compiler parallel flags");

    assert!(
        output.status.success(),
        "rr run with compiler parallel flags failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("[1] 999"),
        "expected fake RRSCRIPT output, got:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        proj_dir.join("main.gen.R").exists(),
        "expected generated artifact to be preserved by --keep-r"
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
