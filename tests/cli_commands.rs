mod common;

use common::{rscript_available, rscript_path, unique_dir};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn build_command_on_project_dir_builds_entry_only() {
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
    fs::write(proj_dir.join("main.rr"), main_src).expect("failed to write main.rr");
    fs::write(
        proj_dir.join("src").join("util.rr"),
        r#"
fn helper(x) {
  return x + 1
}
"#,
    )
    .expect("failed to write util.rr");

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
        !out_dir.join("src").join("util.R").exists(),
        "project build should not emit sibling source files independently"
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
fn new_command_creates_cargo_like_binary_project() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_new");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "bin_project");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg("new")
        .arg("github.com/acme/demo-app")
        .arg(&proj_dir)
        .status()
        .expect("failed to run rr new");
    assert!(status.success(), "rr new failed");

    assert!(proj_dir.join("rr.mod").is_file(), "expected rr.mod");
    assert!(proj_dir.join("rr.lock").is_file(), "expected rr.lock");
    assert!(
        proj_dir.join("src").join("main.rr").is_file(),
        "expected src/main.rr"
    );
    assert!(proj_dir.join("Build").is_dir(), "expected Build directory");

    let manifest = fs::read_to_string(proj_dir.join("rr.mod")).expect("failed to read rr.mod");
    assert!(
        manifest.contains("module github.com/acme/demo-app"),
        "unexpected rr.mod:\n{}",
        manifest
    );

    let gitignore =
        fs::read_to_string(proj_dir.join(".gitignore")).expect("failed to read .gitignore");
    assert!(
        gitignore.lines().any(|line| line.trim() == "Build/"),
        "expected Build/ entry in .gitignore, got:\n{}",
        gitignore
    );
}

#[test]
fn new_command_supports_plain_module_paths_in_current_directory() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_new");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "current_dir_app");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("new")
        .arg("demo-app")
        .arg(".")
        .status()
        .expect("failed to run rr new demo-app .");
    assert!(status.success(), "rr new demo-app . failed");

    let manifest = fs::read_to_string(proj_dir.join("rr.mod")).expect("failed to read rr.mod");
    assert!(
        manifest.contains("module demo-app"),
        "unexpected rr.mod:\n{}",
        manifest
    );
    assert!(proj_dir.join("rr.lock").is_file(), "expected rr.lock");
    assert!(
        proj_dir.join("src").join("main.rr").is_file(),
        "expected src/main.rr"
    );
    let gitignore =
        fs::read_to_string(proj_dir.join(".gitignore")).expect("failed to read .gitignore");
    assert!(
        gitignore.lines().any(|line| line.trim() == "Build/"),
        "expected Build/ entry in .gitignore, got:\n{}",
        gitignore
    );
}

#[test]
fn init_command_creates_cargo_like_library_project_in_place() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_new");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "lib_project");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("init")
        .arg("--lib")
        .arg("github.com/acme/mathlib")
        .status()
        .expect("failed to run rr init --lib");
    assert!(status.success(), "rr init --lib failed");

    assert!(proj_dir.join("rr.mod").is_file(), "expected rr.mod");
    assert!(proj_dir.join("rr.lock").is_file(), "expected rr.lock");
    assert!(
        proj_dir.join("src").join("lib.rr").is_file(),
        "expected src/lib.rr"
    );
    assert!(proj_dir.join("Build").is_dir(), "expected Build directory");
}

#[test]
fn new_command_dot_uses_current_directory_name_as_module_path() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_new");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "dot_project");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("new")
        .arg(".")
        .status()
        .expect("failed to run rr new .");
    assert!(status.success(), "rr new . failed");

    let manifest = fs::read_to_string(proj_dir.join("rr.mod")).expect("failed to read rr.mod");
    let expected_module = proj_dir
        .file_name()
        .and_then(|name| name.to_str())
        .expect("project dir name should be valid utf-8");
    assert!(
        manifest.contains(&format!("module {}", expected_module)),
        "unexpected rr.mod:\n{}",
        manifest
    );

    let main_rr = fs::read_to_string(proj_dir.join("src").join("main.rr"))
        .expect("failed to read src/main.rr");
    let expected_main = format!(
        "fn main() {{\n  print(\"Hello from {expected_module}\")\n}}\n\n/*\nmain <- function() {{\n  print(\"Hello from {expected_module}\")\n}}\n*/\n"
    );
    assert_eq!(
        main_rr, expected_main,
        "unexpected scaffolded src/main.rr:\n{}",
        main_rr
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
fn run_command_prefers_src_main_rr_for_managed_projects() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_run");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "managed_src_main");
    fs::create_dir_all(proj_dir.join("src")).expect("failed to create src dir");
    fs::write(
        proj_dir.join("rr.mod"),
        "module example/managed\n\nrr 8.0\n",
    )
    .expect("failed to write rr.mod");
    fs::write(
        proj_dir.join("src").join("main.rr"),
        r#"
fn main() {
  print(321L)
}
"#,
    )
    .expect("failed to write src/main.rr");

    let fake_rscript = proj_dir.join("fake_rscript.sh");
    fs::write(&fake_rscript, "#!/bin/sh\nprintf '[1] 321\\n'\nexit 0\n")
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
        .expect("failed to run rr run . for managed project");

    assert!(
        output.status.success(),
        "rr run . for managed project failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("[1] 321"),
        "expected fake RRSCRIPT output, got:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[cfg(unix)]
#[test]
fn run_command_requires_main_function_in_project_entry() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_run");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "missing_main_fn");
    fs::create_dir_all(proj_dir.join("src")).expect("failed to create src dir");
    fs::write(
        proj_dir.join("rr.mod"),
        "module example/missing-main\n\nrr 8.0\n",
    )
    .expect("failed to write rr.mod");
    fs::write(
        proj_dir.join("src").join("main.rr"),
        r#"
fn helper() {
  return 1L
}
"#,
    )
    .expect("failed to write src/main.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let output = Command::new(&rr_bin)
        .current_dir(&proj_dir)
        .arg("run")
        .arg(".")
        .output()
        .expect("failed to run rr run .");

    assert!(
        !output.status.success(),
        "rr run . should fail without fn main"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("must define fn main()"),
        "expected missing-main-function diagnostic, got:\n{}",
        stdout
    );
}

#[test]
fn build_command_defaults_to_build_debug_and_incremental_cache() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_build");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "managed_default_build");
    fs::create_dir_all(proj_dir.join("src")).expect("failed to create src dir");
    fs::write(
        proj_dir.join("rr.mod"),
        "module example/build-default\n\nrr 8.0\n",
    )
    .expect("failed to write rr.mod");
    fs::write(
        proj_dir.join("src").join("main.rr"),
        r#"
fn main() {
  print(7L)
}
main()
"#,
    )
    .expect("failed to write src/main.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg("build")
        .arg(&proj_dir)
        .arg("-O0")
        .status()
        .expect("failed to run rr build with default output");
    assert!(status.success(), "rr build default output failed");

    assert!(
        proj_dir
            .join("Build")
            .join("debug")
            .join("src")
            .join("main.R")
            .is_file(),
        "expected Build/debug/src/main.R to be generated"
    );
    assert!(
        proj_dir.join("Build").join("incremental").is_dir(),
        "expected Build/incremental to be created"
    );
}

#[test]
fn watch_command_defaults_to_build_watch_output() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_watch_once");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "managed_watch_default");
    fs::create_dir_all(proj_dir.join("src")).expect("failed to create src dir");
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");

    fs::write(
        proj_dir.join("rr.mod"),
        "module example/watch-default\n\nrr 8.0\n",
    )
    .expect("failed to write rr.mod");
    fs::write(
        proj_dir.join("src").join("main.rr"),
        r#"
fn main() {
  print(42L)
}
main()
"#,
    )
    .expect("failed to write src/main.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg("watch")
        .arg(&proj_dir)
        .arg("--once")
        .arg("--poll-ms")
        .arg("1")
        .status()
        .expect("failed to run rr watch --once with default output");
    assert!(status.success(), "rr watch --once default output failed");

    assert!(
        proj_dir
            .join("Build")
            .join("watch")
            .join("main.R")
            .is_file(),
        "expected Build/watch/main.R to be generated"
    );
    assert!(
        proj_dir.join("Build").join("incremental").is_dir(),
        "expected Build/incremental to be created"
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
