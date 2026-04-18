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

    let profile_path = proj_dir.join("profile.json");
    let status = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O0")
        .arg("--profile-compile")
        .arg("--profile-compile-out")
        .arg(&profile_path)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to rerun RR CLI with compile profile");
    assert!(status.success(), "RR CLI profiled rerun failed");
    let profile = fs::read_to_string(&profile_path).expect("failed to read incremental profile");
    assert!(profile.contains("\"enabled\": true"));
    assert!(profile.contains("\"phase1_artifact_hit\": true"));
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

#[test]
fn legacy_cli_can_force_cold_compile_without_disturbing_warm_cache() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("cli_incremental_default");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "cold");
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
    let cold_profile = proj_dir.join("cold-profile.json");
    let warm_profile = proj_dir.join("warm-profile.json");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let seed = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O1")
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to seed warm cache");
    assert!(seed.success(), "warm cache seed compile failed");

    let cold = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O1")
        .arg("--cold")
        .arg("--profile-compile")
        .arg("--profile-compile-out")
        .arg(&cold_profile)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run cold compile");
    assert!(cold.success(), "cold compile failed");

    let cold_profile_text =
        fs::read_to_string(&cold_profile).expect("failed to read cold compile profile");
    assert!(cold_profile_text.contains("\"phase1_artifact_hit\": false"));
    assert!(cold_profile_text.contains("\"optimized_mir_cache_hit\": false"));

    let warm = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O1")
        .arg("--profile-compile")
        .arg("--profile-compile-out")
        .arg(&warm_profile)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run warm compile after cold");
    assert!(warm.success(), "warm compile after cold failed");

    let warm_profile_text =
        fs::read_to_string(&warm_profile).expect("failed to read warm compile profile");
    assert!(warm_profile_text.contains("\"phase1_artifact_hit\": true"));
}

#[test]
fn legacy_cli_profile_reports_phase2_emit_hits_and_miss_reasons() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("cli_incremental_default");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "phase2_profile");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");

    let main_path = proj_dir.join("main.rr");
    fs::write(
        &main_path,
        r#"
fn helper(x) {
  return x + 1L
}

fn main() {
  print(helper(1L))
}
main()
"#,
    )
    .expect("failed to write main.rr");
    let out_file = proj_dir.join("out.R");
    let profile_path = proj_dir.join("phase2-profile.json");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let first = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O1")
        .arg("--incremental=2")
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run RR CLI phase2 seed compile");
    assert!(first.success(), "RR CLI phase2 seed compile failed");

    let second = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O1")
        .arg("--incremental=2")
        .arg("--profile-compile")
        .arg("--profile-compile-out")
        .arg(&profile_path)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run RR CLI phase2 profiled compile");
    assert!(second.success(), "RR CLI phase2 profiled compile failed");

    let profile = fs::read_to_string(&profile_path).expect("failed to read phase2 profile json");
    assert!(profile.contains("\"phase2_emit_hits\": "));
    assert!(!profile.contains("\"phase2_emit_hits\": 0"));

    fs::write(
        &main_path,
        r#"
fn helper(x) {
  return x + 2L
}

fn main() {
  print(helper(1L))
}
main()
"#,
    )
    .expect("failed to update main.rr");

    let third = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O1")
        .arg("--incremental=2")
        .arg("--profile-compile")
        .arg("--profile-compile-out")
        .arg(&profile_path)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run RR CLI phase2 miss compile");
    assert!(third.success(), "RR CLI phase2 miss compile failed");
    let miss_profile =
        fs::read_to_string(&profile_path).expect("failed to read phase2 miss profile json");
    assert!(miss_profile.contains("\"miss_reasons\": [\"entry_changed\"]"));
}
