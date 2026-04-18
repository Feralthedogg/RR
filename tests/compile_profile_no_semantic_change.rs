mod common;

use common::unique_dir;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn compile_profile_flag_does_not_change_emitted_r() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("compile_profile_no_semantic_change");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "profile_equiv");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = proj_dir.join("main.rr");
    let out_plain = proj_dir.join("plain.R");
    let out_profiled = proj_dir.join("profiled.R");
    let profile_file = proj_dir.join("compile-profile.json");
    fs::write(
        &main_path,
        r#"
fn add(a, b) {
  return a + b
}

print(add(4.0, 5.0))
"#,
    )
    .expect("failed to write main.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let plain = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_plain)
        .arg("-O1")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .status()
        .expect("failed to run plain RR compile");
    assert!(plain.success(), "plain RR compile failed");

    let profiled = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_profiled)
        .arg("-O1")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("--profile-compile")
        .arg("--profile-compile-out")
        .arg(&profile_file)
        .status()
        .expect("failed to run profiled RR compile");
    assert!(profiled.success(), "profiled RR compile failed");

    let plain_r = fs::read_to_string(&out_plain).expect("failed to read plain R output");
    let profiled_r = fs::read_to_string(&out_profiled).expect("failed to read profiled R output");
    assert_eq!(
        plain_r, profiled_r,
        "compile profiling should not change emitted R"
    );
    assert!(
        profile_file.is_file(),
        "profiled compile should write the requested json file"
    );
}
