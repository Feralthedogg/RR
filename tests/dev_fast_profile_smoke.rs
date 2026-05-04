mod common;

use common::unique_dir;
use rr::compiler::{
    CompileMode, CompileOutputOptions, OptLevel, compile_with_configs_with_options,
    default_parallel_config, default_type_config,
};
use serde_json::Value;
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
    assert!(profile.contains("\"disabled_pass_groups\": [\"poly\"]"));
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
fn fast_dev_build_keeps_poly_disabled_and_can_still_vectorize_tiny_loops() {
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
    assert!(fast_profile.contains("\"disabled_pass_groups\": [\"poly\"]"));
    assert!(!fast_profile.contains("\"poly\": {"));
    assert!(fast_profile.contains("\"vectorize\": {"));

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

#[test]
fn fast_dev_profile_runs_inline_on_small_helpers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("dev_fast_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "fast_dev_inline");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_case(
        &proj_dir,
        "main.rr",
        r#"
fn add1(x) {
  let y = x + 1L
  return y
}

fn pair_sum(a, b) {
  let lhs = add1(a)
  let rhs = add1(b)
  return lhs + rhs
}

fn main() {
  print(pair_sum(4L, 6L))
}

main()
"#,
    );
    let out_dir = proj_dir.join("out");
    let profile_path = proj_dir.join("inline-profile.json");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg("build")
        .arg(&main_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("-O1")
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&profile_path)
        .status()
        .expect("failed to run fast-dev inline profile build");
    assert!(
        status.success(),
        "fast-dev inline profile build should succeed"
    );

    let profile = fs::read_to_string(&profile_path).expect("failed to read inline profile");
    let parsed: Value =
        serde_json::from_str(&profile).expect("compile profile should be valid json");
    assert_eq!(parsed["compile_mode"].as_str(), Some("fast-dev"));
    assert_eq!(
        parsed["tachyon"]["disabled_pass_groups"],
        serde_json::json!(["poly"])
    );
    assert!(
        parsed["tachyon"]["pulse_stats"]["inline_rounds"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "expected fast-dev to record at least one inline round"
    );
    assert!(
        parsed["tachyon"]["passes"]["inline"]["changed_invocations"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "expected fast-dev inline pass to change the tiny helper fixture"
    );
}

#[test]
fn fast_dev_profile_vectorizes_tiny_loops_even_with_poly_disabled() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("dev_fast_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "fast_dev_vectorize");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_case(
        &proj_dir,
        "main.rr",
        r#"
fn main() {
  let n = 8.0
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
    let out_dir = proj_dir.join("out");
    let profile_path = proj_dir.join("vectorize-profile.json");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg("build")
        .arg(&main_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("-O1")
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&profile_path)
        .status()
        .expect("failed to run fast-dev vectorize profile build");
    assert!(
        status.success(),
        "fast-dev vectorize profile build should succeed"
    );

    let profile = fs::read_to_string(&profile_path).expect("failed to read vectorize profile");
    let parsed: Value =
        serde_json::from_str(&profile).expect("compile profile should be valid json");
    assert_eq!(parsed["compile_mode"].as_str(), Some("fast-dev"));
    assert_eq!(
        parsed["tachyon"]["disabled_pass_groups"],
        serde_json::json!(["poly"])
    );
    assert!(
        parsed["tachyon"]["passes"]["vectorize"]["changed_invocations"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "expected fast-dev vectorize pass to change the tiny loop fixture"
    );
    assert!(
        parsed["tachyon"]["pulse_stats"]["vector_applied_total"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "expected fast-dev to apply at least one vectorization on the tiny loop fixture"
    );
}

#[test]
fn fast_dev_profile_vectorizes_tiny_conditional_loops_when_poly_stays_disabled() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("dev_fast_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "fast_dev_vectorize_conditional");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_case(
        &proj_dir,
        "main.rr",
        r#"
fn main() {
  let xs = c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0)
  let ys = rep.int(0.0, length(xs))
  let i = 1.0
  while i <= length(xs) {
    if xs[i] > 4.0 {
      ys[i] = xs[i] * 2.0
    } else {
      ys[i] = xs[i] + 3.0
    }
    i += 1.0
  }
  print(sum(ys))
}

main()
"#,
    );
    let out_dir = proj_dir.join("out");
    let profile_path = proj_dir.join("vectorize-conditional-profile.json");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg("build")
        .arg(&main_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("-O1")
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&profile_path)
        .status()
        .expect("failed to run fast-dev conditional vectorize profile build");
    assert!(
        status.success(),
        "fast-dev conditional vectorize profile build should succeed"
    );

    let profile =
        fs::read_to_string(&profile_path).expect("failed to read conditional vectorize profile");
    let parsed: Value =
        serde_json::from_str(&profile).expect("compile profile should be valid json");
    assert_eq!(parsed["compile_mode"].as_str(), Some("fast-dev"));
    assert_eq!(
        parsed["tachyon"]["disabled_pass_groups"],
        serde_json::json!(["poly"])
    );
    assert!(
        parsed["tachyon"]["passes"]["vectorize"]["changed_invocations"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "expected fast-dev vectorize pass to change the tiny conditional loop fixture"
    );
    assert!(
        parsed["tachyon"]["pulse_stats"]["vector_applied_total"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "expected fast-dev to apply at least one vectorization on the tiny conditional loop fixture"
    );
}

#[test]
fn fast_dev_profile_vectorizes_tiny_call_map_loops_when_poly_stays_disabled() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("dev_fast_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "fast_dev_vectorize_callmap");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_case(
        &proj_dir,
        "main.rr",
        r#"
fn main() {
  let xs = c(1.0, -2.0, 3.0, -4.0, 5.0, -6.0, 7.0, -8.0)
  let ys = rep.int(0.0, length(xs))
  let i = 1.0
  while i <= length(xs) {
    ys[i] = abs(xs[i])
    i += 1.0
  }
  print(sum(ys))
}

main()
"#,
    );
    let out_dir = proj_dir.join("out");
    let profile_path = proj_dir.join("vectorize-callmap-profile.json");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg("build")
        .arg(&main_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("-O1")
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&profile_path)
        .status()
        .expect("failed to run fast-dev call-map vectorize profile build");
    assert!(
        status.success(),
        "fast-dev call-map vectorize profile build should succeed"
    );

    let profile =
        fs::read_to_string(&profile_path).expect("failed to read call-map vectorize profile");
    let parsed: Value =
        serde_json::from_str(&profile).expect("compile profile should be valid json");
    assert_eq!(parsed["compile_mode"].as_str(), Some("fast-dev"));
    assert_eq!(
        parsed["tachyon"]["disabled_pass_groups"],
        serde_json::json!(["poly"])
    );
    assert!(
        parsed["tachyon"]["passes"]["vectorize"]["changed_invocations"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "expected fast-dev vectorize pass to change the tiny call-map loop fixture"
    );
    assert!(
        parsed["tachyon"]["pulse_stats"]["vector_applied_total"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "expected fast-dev to apply at least one vectorization on the tiny call-map loop fixture"
    );
}

#[test]
fn fast_dev_profile_vectorizes_tiny_expr_map_loops_when_poly_stays_disabled() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("dev_fast_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "fast_dev_vectorize_exprmap");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_case(
        &proj_dir,
        "main.rr",
        r#"
fn main() {
  let xs = c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0)
  let ys = rep.int(0.0, length(xs))
  let i = 1.0
  while i <= length(xs) {
    let a = xs[i] + 1.0
    let b = a * 2.0
    ys[i] = b - 3.0
    i += 1.0
  }
  print(sum(ys))
}

main()
"#,
    );
    let out_dir = proj_dir.join("out");
    let profile_path = proj_dir.join("vectorize-exprmap-profile.json");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg("build")
        .arg(&main_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("-O1")
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&profile_path)
        .status()
        .expect("failed to run fast-dev expr-map vectorize profile build");
    assert!(
        status.success(),
        "fast-dev expr-map vectorize profile build should succeed"
    );

    let profile =
        fs::read_to_string(&profile_path).expect("failed to read expr-map vectorize profile");
    let parsed: Value =
        serde_json::from_str(&profile).expect("compile profile should be valid json");
    assert_eq!(parsed["compile_mode"].as_str(), Some("fast-dev"));
    assert_eq!(
        parsed["tachyon"]["disabled_pass_groups"],
        serde_json::json!(["poly"])
    );
    assert!(
        parsed["tachyon"]["passes"]["vectorize"]["changed_invocations"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "expected fast-dev vectorize pass to change the tiny expr-map loop fixture"
    );
    assert!(
        parsed["tachyon"]["pulse_stats"]["vector_applied_total"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "expected fast-dev to apply at least one vectorization on the tiny expr-map loop fixture"
    );
}

#[test]
fn fast_dev_profile_vectorizes_tiny_multi_store_loops_when_poly_stays_disabled() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("dev_fast_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "fast_dev_vectorize_multi_store");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_case(
        &proj_dir,
        "main.rr",
        r#"
fn main() {
  let n = 16.0
  let xs = rep.int(0.0, n)
  let ys = rep.int(0.0, n)
  let i = 1.0
  while i <= n {
    xs[i] = i * 2.0
    ys[i] = i + 5.0
    i += 1.0
  }
  print(sum(xs) + sum(ys))
}

main()
"#,
    );
    let out_dir = proj_dir.join("out");
    let profile_path = proj_dir.join("vectorize-multi-store-profile.json");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg("build")
        .arg(&main_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("-O1")
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&profile_path)
        .status()
        .expect("failed to run fast-dev multi-store vectorize profile build");
    assert!(
        status.success(),
        "fast-dev multi-store vectorize profile build should succeed"
    );

    let profile =
        fs::read_to_string(&profile_path).expect("failed to read multi-store vectorize profile");
    let parsed: Value =
        serde_json::from_str(&profile).expect("compile profile should be valid json");
    assert_eq!(parsed["compile_mode"].as_str(), Some("fast-dev"));
    assert_eq!(
        parsed["tachyon"]["disabled_pass_groups"],
        serde_json::json!(["poly"])
    );
    assert!(
        parsed["tachyon"]["passes"]["vectorize"]["changed_invocations"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "expected fast-dev vectorize pass to change the tiny multi-store loop fixture"
    );
    assert!(
        parsed["tachyon"]["pulse_stats"]["vector_applied_total"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "expected fast-dev to apply at least one vectorization on the tiny multi-store loop fixture"
    );
}

#[test]
fn fast_dev_profile_vectorizes_tiny_gather_loops_when_poly_stays_disabled() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("dev_fast_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "fast_dev_vectorize_gather");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_case(
        &proj_dir,
        "main.rr",
        r#"
fn main() {
  let xs = c(10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0)
  let idx = c(8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0)
  let ys = rep.int(0.0, length(xs))
  let i = 1.0
  while i <= length(xs) {
    ys[i] = xs[idx[i]]
    i += 1.0
  }
  print(sum(ys))
}

main()
"#,
    );
    let out_dir = proj_dir.join("out");
    let profile_path = proj_dir.join("vectorize-gather-profile.json");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg("build")
        .arg(&main_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("-O1")
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&profile_path)
        .status()
        .expect("failed to run fast-dev gather vectorize profile build");
    assert!(
        status.success(),
        "fast-dev gather vectorize profile build should succeed"
    );

    let profile =
        fs::read_to_string(&profile_path).expect("failed to read gather vectorize profile");
    let parsed: Value =
        serde_json::from_str(&profile).expect("compile profile should be valid json");
    assert_eq!(parsed["compile_mode"].as_str(), Some("fast-dev"));
    assert_eq!(
        parsed["tachyon"]["disabled_pass_groups"],
        serde_json::json!(["poly"])
    );
    assert!(
        parsed["tachyon"]["passes"]["vectorize"]["changed_invocations"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "expected fast-dev vectorize pass to change the tiny gather loop fixture"
    );
    assert!(
        parsed["tachyon"]["pulse_stats"]["vector_applied_total"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "expected fast-dev to apply at least one vectorization on the tiny gather loop fixture"
    );
}

#[test]
fn fast_dev_profile_vectorizes_tiny_scatter_loops_when_poly_stays_disabled() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("dev_fast_profile_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "fast_dev_vectorize_scatter");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let main_path = write_case(
        &proj_dir,
        "main.rr",
        r#"
fn main() {
  let xs = c(10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0)
  let idx = c(8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0)
  let ys = rep.int(0.0, length(xs))
  let i = 1.0
  while i <= length(xs) {
    ys[idx[i]] = xs[i]
    i += 1.0
  }
  print(sum(ys))
}

main()
"#,
    );
    let out_dir = proj_dir.join("out");
    let profile_path = proj_dir.join("vectorize-scatter-profile.json");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg("build")
        .arg(&main_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("-O1")
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&profile_path)
        .status()
        .expect("failed to run fast-dev scatter vectorize profile build");
    assert!(
        status.success(),
        "fast-dev scatter vectorize profile build should succeed"
    );

    let profile =
        fs::read_to_string(&profile_path).expect("failed to read scatter vectorize profile");
    let parsed: Value =
        serde_json::from_str(&profile).expect("compile profile should be valid json");
    assert_eq!(parsed["compile_mode"].as_str(), Some("fast-dev"));
    assert_eq!(
        parsed["tachyon"]["disabled_pass_groups"],
        serde_json::json!(["poly"])
    );
    assert!(
        parsed["tachyon"]["passes"]["vectorize"]["changed_invocations"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "expected fast-dev vectorize pass to change the tiny scatter loop fixture"
    );
    assert!(
        parsed["tachyon"]["pulse_stats"]["vector_applied_total"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "expected fast-dev to apply at least one vectorization on the tiny scatter loop fixture"
    );
}
