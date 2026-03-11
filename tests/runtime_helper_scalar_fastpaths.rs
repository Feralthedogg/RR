mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use common::{normalize, rscript_available, rscript_path, unique_dir};

fn run_rscript_env(
    path: &str,
    script: &std::path::Path,
    env_kv: &[(&str, &str)],
) -> common::RunResult {
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

fn compile_rr_case(rr_source: &str, stem: &str) -> Option<(String, PathBuf)> {
    let rscript = match rscript_path() {
        Some(path) if rscript_available(&path) => path,
        _ => {
            eprintln!("Skipping runtime helper fast-path test: Rscript not available.");
            return None;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("runtime_helper_scalar_fastpaths");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_path = proj_dir.join(format!("{stem}.rr"));
    let out_path = proj_dir.join(format!("{stem}.R"));
    fs::write(&rr_path, rr_source).expect("failed to write RR case");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("-O2")
        .arg("--no-incremental")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    Some((rscript, out_path))
}

fn assert_debug_release_match(rscript: &str, out_path: &std::path::Path) {
    let debug_run = run_rscript_env(rscript, out_path, &[("RR_RUNTIME_MODE", "debug")]);
    assert_eq!(
        debug_run.status, 0,
        "debug runtime failed:\n{}",
        debug_run.stderr
    );

    let release_run = run_rscript_env(rscript, out_path, &[("RR_RUNTIME_MODE", "release")]);
    assert_eq!(
        release_run.status, 0,
        "release runtime failed:\n{}",
        release_run.stderr
    );

    assert_eq!(
        normalize(&debug_run.stdout),
        normalize(&release_run.stdout),
        "stdout mismatch between debug and release runtime\n\
debug:\n{}\n\
release:\n{}",
        debug_run.stdout,
        release_run.stdout
    );
    assert_eq!(
        normalize(&debug_run.stderr),
        normalize(&release_run.stderr),
        "stderr mismatch between debug and release runtime\n\
debug:\n{}\n\
release:\n{}",
        debug_run.stderr,
        release_run.stderr
    );
}

#[test]
fn scalar_index_helpers_match_between_debug_and_release_runtime() {
    let rr_source = r#"
fn idx_cube(f, x, y, size) {
  let ff = round(f)
  let xx = round(x)
  let yy = round(y)
  let ss = round(size)
  if (ff < 1) { ff = 1 }
  if (ff > 6) { ff = 6 }
  if (xx < 1) { xx = 1 }
  if (xx > ss) { xx = ss }
  if (yy < 1) { yy = 1 }
  if (yy > ss) { yy = ss }
  return ((ff - 1) * ss * ss) + ((xx - 1) * ss) + yy
}

fn idx_torus(x, y, w, h) {
  let xx = x
  let yy = y
  if (xx < 1) { xx = w }
  if (xx > w) { xx = 1 }
  if (yy < 1) { yy = h }
  if (yy > h) { yy = 1 }
  return ((yy - 1) * w) + xx
}

fn build_neighbor_table(size) {
  let total = (6 * size) * size
  let out = rep.int(0, total)
  for (i in 1..size) {
    out[idx_cube(1, i, 1, size)] = idx_torus(i - 1, 1, size, size)
  }
  return out
}

print(idx_cube(1.49, 4.51, 0.2, 4))
print(idx_torus(0, 5, 4, 4))
print(build_neighbor_table(4))
"#;

    let Some((rscript, out_path)) = compile_rr_case(rr_source, "case") else {
        return;
    };

    let code = fs::read_to_string(&out_path).expect("failed to read compiled R");
    assert!(
        code.contains("rr_idx_cube_vec_i("),
        "expected cube helper calls to rewrite to rr_idx_cube_vec_i"
    );
    assert!(
        code.contains("rr_wrap_index_vec_i("),
        "expected wrap helper calls to rewrite to rr_wrap_index_vec_i"
    );

    assert_debug_release_match(&rscript, &out_path);
}

#[test]
fn full_slice_helpers_match_between_debug_and_release_runtime() {
    let rr_source = r#"
fn main() {
  let n = 32.0
  let idx = seq_len(n)
  let x = idx * 0.25
  let y = idx * 0.5
  let score = rep.int(0.0, n)
  let clean = rep.int(0.0, n)

  for (i in 1..n) {
    score[i] = pmax(abs(x[i] * 0.65 + y[i] * 0.35 - 0.08), 0.05)
  }
  for (i in 1..n) {
    if (score[i] > 0.40) {
      clean[i] = sqrt(score[i] + 0.10)
    } else {
      clean[i] = score[i] * 0.55 + 0.03
    }
  }
  for (i in 1..n) {
    x[i] = clean[i] + y[i] * 0.15
  }
  for (i in 1..n) {
    y[i] = score[i] * 0.80 + clean[i] * 0.20
  }

  print(score)
  print(clean)
  print(x)
  print(y)
}

main()
"#;

    let Some((rscript, out_path)) = compile_rr_case(rr_source, "full_slice_case") else {
        return;
    };

    let code = fs::read_to_string(&out_path).expect("failed to read compiled R");
    assert!(
        code.contains("pmax(") || code.contains("rr_call_map_slice_auto("),
        "expected whole-slice call-map path to lower through direct pmax(...) or runtime helper"
    );
    assert!(
        code.contains("rr_assign_slice(")
            || code.contains("clean <- rr_ifelse_strict(")
            || code.contains("clean <- ifelse("),
        "expected whole-slice assign path to lower through rr_assign_slice or direct whole-vector assignment"
    );
    assert!(
        code.contains("rr_index1_read_vec(") || code.contains("score <- pmax("),
        "expected whole-slice reads to lower through rr_index1_read_vec or be folded into direct vector expressions"
    );

    assert_debug_release_match(&rscript, &out_path);
}
