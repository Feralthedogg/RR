mod common;

use common::{normalize, rscript_available, rscript_path, unique_dir};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn run_rscript_env(path: &str, script: &Path, env_kv: &[(&str, &str)]) -> common::RunResult {
    let mut cmd = Command::new(path);
    cmd.arg("--vanilla").arg(script);
    for (k, v) in env_kv {
        cmd.env(k, v);
    }
    let output = cmd.output().expect("failed to execute Rscript");
    common::RunResult {
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

fn compile_rr(rr_bin: &Path, rr_src: &Path, out: &Path) -> String {
    let output = Command::new(rr_bin)
        .arg(rr_src)
        .arg("-o")
        .arg(out)
        .arg("-O2")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .output()
        .expect("failed to run RR compiler");
    assert!(
        output.status.success(),
        "RR compile failed for {}:\n{}",
        rr_src.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::read_to_string(out).expect("failed to read generated R")
}

#[test]
fn helper_heavy_whole_call_map_uses_runtime_profit_guard() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("vectorization_callmap_profit_guard");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "whole");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_src = r#"
fn gather_abs(src, idx) {
  let out = seq_len(length(idx))
  for (i in 1..length(out)) {
    out[i] = abs(src[idx[i]])
  }
  return out
}

print(gather_abs(c(10, -20, 30, -40, 50, -60), c(6, 5, 4, 3, 2, 1)))
"#;
    let rr_path = proj_dir.join("whole.rr");
    let out_path = proj_dir.join("whole.R");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let code = compile_rr(&rr_bin, &rr_path, &out_path);
    assert!(
        code.contains("rr_call_map_whole_auto("),
        "expected helper-heavy whole-destination call-map to lower through rr_call_map_whole_auto(...)"
    );
    assert!(
        code.contains("rr_gather("),
        "expected gather helper in the mapped argument"
    );

    let Some(rscript) = rscript_path().filter(|p| rscript_available(p)) else {
        return;
    };

    let scalar_run = run_rscript_env(
        &rscript,
        &out_path,
        &[
            ("RR_VECTOR_FALLBACK_BASE_TRIP", "9999"),
            ("RR_VECTOR_FALLBACK_HELPER_SCALE", "0"),
        ],
    );
    assert_eq!(
        scalar_run.status, 0,
        "scalar-fallback forced run failed:\n{}",
        scalar_run.stderr
    );

    let vector_run = run_rscript_env(
        &rscript,
        &out_path,
        &[
            ("RR_VECTOR_FALLBACK_BASE_TRIP", "0"),
            ("RR_VECTOR_FALLBACK_HELPER_SCALE", "0"),
        ],
    );
    assert_eq!(
        vector_run.status, 0,
        "vector-path forced run failed:\n{}",
        vector_run.stderr
    );

    assert_eq!(
        normalize(&scalar_run.stdout),
        normalize(&vector_run.stdout),
        "scalar/vector runtime guard paths diverged\nscalar:\n{}\nvector:\n{}",
        scalar_run.stdout,
        vector_run.stdout
    );
    assert_eq!(
        normalize(&scalar_run.stderr),
        normalize(&vector_run.stderr),
        "scalar/vector runtime guard stderr diverged\nscalar:\n{}\nvector:\n{}",
        scalar_run.stderr,
        vector_run.stderr
    );
}

#[test]
fn helper_heavy_slice_call_map_uses_runtime_profit_guard() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("vectorization_callmap_profit_guard");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "slice");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_src = r#"
fn gather_abs_slice(src, idx) {
  let out = seq_len(length(idx))
  for (i in 2..length(out)) {
    out[i] = abs(src[idx[i]])
  }
  return out
}

print(gather_abs_slice(c(10, -20, 30, -40, 50, -60), c(6, 5, 4, 3, 2, 1)))
"#;
    let rr_path = proj_dir.join("slice.rr");
    let out_path = proj_dir.join("slice.R");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let code = compile_rr(&rr_bin, &rr_path, &out_path);
    assert!(
        code.contains("rr_call_map_slice_auto("),
        "expected helper-heavy slice call-map to lower through rr_call_map_slice_auto(...)"
    );
    assert!(
        code.contains("rr_gather("),
        "expected gather helper in the mapped argument"
    );

    let Some(rscript) = rscript_path().filter(|p| rscript_available(p)) else {
        return;
    };

    let scalar_run = run_rscript_env(
        &rscript,
        &out_path,
        &[
            ("RR_VECTOR_FALLBACK_BASE_TRIP", "9999"),
            ("RR_VECTOR_FALLBACK_HELPER_SCALE", "0"),
        ],
    );
    assert_eq!(
        scalar_run.status, 0,
        "scalar-fallback forced run failed:\n{}",
        scalar_run.stderr
    );

    let vector_run = run_rscript_env(
        &rscript,
        &out_path,
        &[
            ("RR_VECTOR_FALLBACK_BASE_TRIP", "0"),
            ("RR_VECTOR_FALLBACK_HELPER_SCALE", "0"),
        ],
    );
    assert_eq!(
        vector_run.status, 0,
        "vector-path forced run failed:\n{}",
        vector_run.stderr
    );

    assert_eq!(
        normalize(&scalar_run.stdout),
        normalize(&vector_run.stdout),
        "scalar/vector runtime guard paths diverged\nscalar:\n{}\nvector:\n{}",
        scalar_run.stdout,
        vector_run.stdout
    );
    assert_eq!(
        normalize(&scalar_run.stderr),
        normalize(&vector_run.stderr),
        "scalar/vector runtime guard stderr diverged\nscalar:\n{}\nvector:\n{}",
        scalar_run.stderr,
        vector_run.stderr
    );
}
