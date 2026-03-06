mod common;

use common::unique_dir;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn watch_once_compiles_and_exits_successfully() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("cli_watch_once");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");
    // SAFETY: This test sets the variable for its own process scope and removes it
    // before exit. No foreign pointers are involved; the operation is bounded to this test.
    unsafe {
        std::env::set_var("RR_INCREMENTAL_CACHE_DIR", &cache_dir);
    }

    let main_path = proj_dir.join("main.rr");
    let source = r#"
fn main() {
  print(42L);
}
main();
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
        .arg("--incremental=all")
        .status()
        .expect("failed to run rr watch --once");
    assert!(status.success(), "watch --once command failed");
    assert!(out_file.is_file(), "watch output file was not generated");
    // SAFETY: Matches the scoped set_var above and restores process env state for this test.
    unsafe {
        std::env::remove_var("RR_INCREMENTAL_CACHE_DIR");
    }
}
