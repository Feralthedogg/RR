mod common;

use common::unique_dir;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn leaf_import_change_only_rebuilds_invalidated_modules() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("module_export_cache_invalidation");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");

    let main_path = proj_dir.join("main.rr");
    let mid_path = proj_dir.join("mid.rr");
    let leaf_path = proj_dir.join("leaf.rr");
    fs::write(
        &main_path,
        r#"
import "./mid.rr"

fn main() {
  print(mid_value())
}
main()
"#,
    )
    .expect("failed to write main.rr");
    fs::write(
        &mid_path,
        r#"
import "./leaf.rr"

fn mid_value() {
  return leaf_value()
}
"#,
    )
    .expect("failed to write mid.rr");
    fs::write(
        &leaf_path,
        r#"
fn leaf_value() {
  return 10L
}
"#,
    )
    .expect("failed to write leaf.rr");

    let out_file = proj_dir.join("out.R");
    let profile_seed = proj_dir.join("seed-profile.json");
    let profile_cached = proj_dir.join("cached-profile.json");
    let profile_invalidated = proj_dir.join("invalidated-profile.json");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    for profile in [&profile_seed, &profile_cached] {
        let status = Command::new(&rr_bin)
            .arg(&main_path)
            .arg("-o")
            .arg(&out_file)
            .arg("-O1")
            .arg("--no-runtime")
            .arg("--no-incremental")
            .arg("--profile-compile-out")
            .arg(profile)
            .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
            .status()
            .expect("failed to run module export cache compile");
        assert!(status.success(), "module export cache compile failed");
    }

    let cached_profile =
        fs::read_to_string(&profile_cached).expect("failed to read cached profile");
    assert!(cached_profile.contains("\"parsed_modules\": 1"));
    assert!(cached_profile.contains("\"cached_modules\": 2"));

    fs::write(
        &leaf_path,
        r#"
fn leaf_value() {
  return 11L
}
"#,
    )
    .expect("failed to update leaf.rr");

    let status = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O1")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&profile_invalidated)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run invalidated compile");
    assert!(status.success(), "invalidated compile failed");

    let invalidated_profile =
        fs::read_to_string(&profile_invalidated).expect("failed to read invalidated profile");
    assert!(invalidated_profile.contains("\"parsed_modules\": 2"));
    assert!(invalidated_profile.contains("\"cached_modules\": 1"));
}
