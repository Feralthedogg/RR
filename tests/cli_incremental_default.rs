mod common;

use common::unique_dir;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn legacy_cli_seeds_incremental_cache_by_default() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("cli_incremental_default");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "auto");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");

    let main_path = proj_dir.join("main.rr");
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
    let out_file = proj_dir.join("out.R");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O0")
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run RR CLI");
    assert!(status.success(), "RR CLI failed");

    let artifact_dir = cache_dir.join("artifacts");
    let artifact_count = fs::read_dir(&artifact_dir)
        .expect("expected incremental artifact dir to exist")
        .filter_map(|entry| entry.ok())
        .count();
    assert!(
        artifact_count > 0,
        "default CLI compile should seed incremental artifacts"
    );
}

#[test]
fn legacy_cli_can_disable_incremental_cache() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("cli_incremental_default");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "off");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");

    let main_path = proj_dir.join("main.rr");
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
    let out_file = proj_dir.join("out.R");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O0")
        .arg("--no-incremental")
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run RR CLI with --no-incremental");
    assert!(status.success(), "RR CLI with --no-incremental failed");
    assert!(
        !cache_dir.join("artifacts").exists(),
        "--no-incremental should avoid seeding phase1 artifacts"
    );
}
