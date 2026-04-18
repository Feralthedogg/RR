mod common;

use RR::compiler::{
    CompileMode, CompileOutputOptions, OptLevel, compile_with_configs_with_options,
    default_parallel_config, default_type_config,
};
use common::unique_dir;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn write_case(dir: &std::path::Path, name: &str, source: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, source).expect("failed to write RR source");
    path
}

#[test]
fn run_cli_defaults_to_fast_dev_profile() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("dev_fast_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "run_default");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_case(
        &proj_dir,
        "main.rr",
        r#"
fn main() {
  print(1L)
}

main()
"#,
    );
    let profile_path = proj_dir.join("run-profile.json");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg("run")
        .arg(&main_path)
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&profile_path)
        .env("RRSCRIPT", "true")
        .status()
        .expect("failed to run RR run");
    assert!(status.success(), "RR run should succeed with RRSCRIPT=true");

    let profile = fs::read_to_string(&profile_path).expect("failed to read run profile");
    assert!(profile.contains("\"compile_mode\": \"fast-dev\""));
    assert!(profile.contains("\"disabled_pass_groups\": [\"poly\", \"vectorize\", \"inline\"]"));
}

#[test]
fn build_o2_defaults_to_standard_profile() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("dev_fast_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "build_o2");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_case(
        &proj_dir,
        "main.rr",
        r#"
fn main() {
  let xs = c(1.0, 2.0, 3.0)
  print(sum(xs))
}

main()
"#,
    );
    let out_dir = proj_dir.join("out");
    let profile_path = proj_dir.join("build-profile.json");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg("build")
        .arg(&main_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("-O2")
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&profile_path)
        .status()
        .expect("failed to run RR build");
    assert!(status.success(), "RR build -O2 should succeed");

    let profile = fs::read_to_string(&profile_path).expect("failed to read build profile");
    assert!(profile.contains("\"compile_mode\": \"standard\""));
    assert!(profile.contains("\"disabled_pass_groups\": []"));
}

#[test]
fn fast_dev_build_disables_structural_pass_groups() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("dev_fast_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "structural");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_case(
        &proj_dir,
        "main.rr",
        r#"
fn main() {
  let n = 16.0
  let xs = rep.int(0.0, n)
  let i = 1.0
  while i <= n {
    xs[i] = i * 2.0
    i += 1.0
  }
  print(sum(xs))
}

main()
"#,
    );
    let fast_out_dir = proj_dir.join("fast-out");
    let standard_out_dir = proj_dir.join("standard-out");
    let fast_profile_path = proj_dir.join("fast-profile.json");
    let standard_profile_path = proj_dir.join("standard-profile.json");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let fast_status = Command::new(&rr_bin)
        .arg("build")
        .arg(&main_path)
        .arg("--out-dir")
        .arg(&fast_out_dir)
        .arg("-O1")
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&fast_profile_path)
        .status()
        .expect("failed to run fast-dev build");
    assert!(fast_status.success(), "fast-dev build should succeed");

    let standard_status = Command::new(&rr_bin)
        .arg("build")
        .arg(&main_path)
        .arg("--out-dir")
        .arg(&standard_out_dir)
        .arg("-O1")
        .arg("--compile-mode")
        .arg("standard")
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&standard_profile_path)
        .status()
        .expect("failed to run standard build");
    assert!(standard_status.success(), "standard build should succeed");

    let fast_profile =
        fs::read_to_string(&fast_profile_path).expect("failed to read fast-dev profile");
    let standard_profile =
        fs::read_to_string(&standard_profile_path).expect("failed to read standard profile");

    assert!(fast_profile.contains("\"compile_mode\": \"fast-dev\""));
    assert!(
        fast_profile.contains("\"disabled_pass_groups\": [\"poly\", \"vectorize\", \"inline\"]")
    );
    assert!(!fast_profile.contains("\"poly\": {"));
    assert!(!fast_profile.contains("\"vectorize\": {"));

    assert!(standard_profile.contains("\"compile_mode\": \"standard\""));
    assert!(standard_profile.contains("\"disabled_pass_groups\": []"));
    assert!(standard_profile.contains("\"poly\": {"));
    assert!(standard_profile.contains("\"vectorize\": {"));
}

#[test]
fn fast_dev_runtime_matches_standard_on_simple_case() {
    let rscript = match common::rscript_path() {
        Some(path) if common::rscript_available(&path) => path,
        _ => {
            eprintln!("Skipping fast-dev runtime equivalence: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("dev_fast_profile_smoke");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let proj_dir = unique_dir(&out_dir, "runtime_equiv");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_path = write_case(
        &proj_dir,
        "main.rr",
        r#"
fn main() {
  let n = 12.0
  let xs = rep.int(0.0, n)
  let i = 1.0
  while i <= n {
    xs[i] = i * 3.0
    i += 1.0
  }
  print(sum(xs))
}

main()
"#,
    );
    let src = fs::read_to_string(&rr_path).expect("failed to read rr source");

    let standard_r = compile_with_configs_with_options(
        rr_path.to_str().expect("non-utf8 path"),
        &src,
        OptLevel::O1,
        default_type_config(),
        default_parallel_config(),
        CompileOutputOptions {
            inject_runtime: true,
            compile_mode: CompileMode::Standard,
            ..Default::default()
        },
    )
    .expect("standard compile should succeed")
    .0;
    let fast_r = compile_with_configs_with_options(
        rr_path.to_str().expect("non-utf8 path"),
        &src,
        OptLevel::O1,
        default_type_config(),
        default_parallel_config(),
        CompileOutputOptions {
            inject_runtime: true,
            compile_mode: CompileMode::FastDev,
            ..Default::default()
        },
    )
    .expect("fast-dev compile should succeed")
    .0;

    let standard_path = proj_dir.join("standard.R");
    let fast_path = proj_dir.join("fast.R");
    fs::write(&standard_path, standard_r).expect("failed to write standard R");
    fs::write(&fast_path, fast_r).expect("failed to write fast-dev R");

    let standard_run = common::run_rscript(&rscript, &standard_path);
    let fast_run = common::run_rscript(&rscript, &fast_path);
    assert_eq!(standard_run.status, 0, "standard runtime failed");
    assert_eq!(fast_run.status, 0, "fast-dev runtime failed");
    assert_eq!(
        common::normalize(&standard_run.stdout),
        common::normalize(&fast_run.stdout),
        "fast-dev runtime stdout should match standard"
    );
    assert_eq!(
        common::normalize(&standard_run.stderr),
        common::normalize(&fast_run.stderr),
        "fast-dev runtime stderr should match standard"
    );
}
