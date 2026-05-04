mod common;

use common::unique_dir;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

fn create_project_src(proj_dir: &Path) -> PathBuf {
    let src_dir = proj_dir.join("src");
    fs::create_dir_all(&src_dir).expect("failed to create project src dir");
    src_dir
}

#[test]
fn watch_once_compiles_and_exits_successfully() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_watch_once");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    let src_dir = create_project_src(&proj_dir);
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = src_dir.join("main.rr");
    let source = r#"
fn main() {
  print(42L)
}
main()
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");

    let out_file = proj_dir.join("watched.R");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(rr_bin)
        .arg("watch")
        .arg(&proj_dir)
        .arg("--once")
        .arg("--poll-ms")
        .arg("1")
        .arg("-o")
        .arg(&out_file)
        .status()
        .expect("failed to run rr watch --once");
    assert!(status.success(), "watch --once command failed");
    assert!(out_file.is_file(), "watch output file was not generated");
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn watch_once_accepts_compiler_parallel_flags() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_watch_once");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "compiler_parallel");
    let src_dir = create_project_src(&proj_dir);
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = src_dir.join("main.rr");
    let source = r#"
fn square(x) {
  return x * x
}

fn main() {
  print(square(5L))
}
main()
"#;
    fs::write(&main_path, source).expect("failed to write main.rr");

    let out_file = proj_dir.join("watched.R");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(rr_bin)
        .arg("watch")
        .arg(&proj_dir)
        .arg("--once")
        .arg("--poll-ms")
        .arg("1")
        .arg("-o")
        .arg(&out_file)
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
        .expect("failed to run rr watch --once with compiler parallel flags");
    assert!(
        status.success(),
        "watch --once with compiler parallel flags failed"
    );
    assert!(out_file.is_file(), "watch output file was not generated");
    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn watch_rebuilds_when_imported_module_changes() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_watch_once");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "import_change");
    let src_dir = create_project_src(&proj_dir);
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = src_dir.join("main.rr");
    let module_path = src_dir.join("module.rr");
    fs::write(
        &main_path,
        r#"
import "./module.rr"

fn main() {
  print(answer())
}
main()
"#,
    )
    .expect("failed to write main.rr");
    fs::write(
        &module_path,
        r#"
fn answer() {
  return 1L
}
"#,
    )
    .expect("failed to write module.rr");

    let out_file = proj_dir.join("watched.R");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let mut child = Command::new(rr_bin)
        .arg("watch")
        .arg(&proj_dir)
        .arg("--poll-ms")
        .arg("25")
        .arg("-o")
        .arg(&out_file)
        .spawn()
        .expect("failed to spawn rr watch");

    let mut wait_for_output = |expected_fragment: &str| {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait().expect("failed to poll rr watch") {
                panic!(
                    "rr watch exited early with status {} while waiting for {:?}",
                    status, expected_fragment,
                );
            }

            if let Ok(code) = fs::read_to_string(&out_file)
                && code.contains(expected_fragment)
            {
                break;
            }

            if Instant::now() >= deadline {
                let _ = child.kill();
                panic!(
                    "timed out waiting for watch output fragment {:?}",
                    expected_fragment,
                );
            }

            thread::sleep(Duration::from_millis(25));
        }
    };

    wait_for_output("print(1L)");

    fs::write(
        &module_path,
        r#"
fn answer() {
  return 2L
}
"#,
    )
    .expect("failed to update module.rr");

    wait_for_output("print(2L)");

    child.kill().expect("failed to stop rr watch");
    let _ = child.wait();

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn watch_once_returns_failure_on_compile_error() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_watch_once");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "compile_error");
    let src_dir = create_project_src(&proj_dir);
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = src_dir.join("main.rr");
    fs::write(
        &main_path,
        r#"
fn main() {
  return missing_name
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let out_file = proj_dir.join("watched.R");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(rr_bin)
        .arg("watch")
        .arg(&proj_dir)
        .arg("--once")
        .arg("--poll-ms")
        .arg("1")
        .arg("-o")
        .arg(&out_file)
        .status()
        .expect("failed to run rr watch --once");
    assert!(
        !status.success(),
        "watch --once should fail on compile error"
    );
    assert!(
        !out_file.exists(),
        "watch --once should not leave an output artifact on compile failure"
    );

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}

#[test]
fn watch_restores_output_when_artifact_is_missing_or_modified_without_source_changes() {
    let env_guard = common::env_lock().lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_watch_once");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "restore_output");
    let src_dir = create_project_src(&proj_dir);
    let cache_dir = proj_dir.join(".rr-cache");
    common::set_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR", &cache_dir);

    let main_path = src_dir.join("main.rr");
    fs::write(
        &main_path,
        r#"
fn main() {
  return 1L
}
"#,
    )
    .expect("failed to write main.rr");

    let out_file = proj_dir.join("watched.R");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let mut child = Command::new(rr_bin)
        .arg("watch")
        .arg(&proj_dir)
        .arg("--poll-ms")
        .arg("25")
        .arg("-o")
        .arg(&out_file)
        .spawn()
        .expect("failed to spawn rr watch");

    let mut wait_for_output = |expected_fragment: &str| {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait().expect("failed to poll rr watch") {
                panic!(
                    "rr watch exited early with status {} while waiting for {:?}",
                    status, expected_fragment,
                );
            }

            if let Ok(code) = fs::read_to_string(&out_file)
                && code.contains(expected_fragment)
            {
                break;
            }

            if Instant::now() >= deadline {
                let _ = child.kill();
                panic!(
                    "timed out waiting for watch output fragment {:?}",
                    expected_fragment,
                );
            }

            thread::sleep(Duration::from_millis(25));
        }
    };

    wait_for_output("return(1L)");

    fs::write(&out_file, "BROKEN\n").expect("failed to corrupt watched output");
    wait_for_output("return(1L)");

    fs::remove_file(&out_file).expect("failed to remove watched output");
    wait_for_output("return(1L)");

    child.kill().expect("failed to stop rr watch");
    let _ = child.wait();

    common::remove_env_var_for_test(&env_guard, "RR_INCREMENTAL_CACHE_DIR");
}
