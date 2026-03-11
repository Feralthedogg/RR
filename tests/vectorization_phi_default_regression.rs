mod common;

use common::{normalize, rscript_available, rscript_path, run_rscript, unique_dir};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn compile_rr(rr_bin: &Path, rr_src: &Path, out_path: &Path, level: &str) {
    let status = Command::new(rr_bin)
        .arg(rr_src)
        .arg("-o")
        .arg(out_path)
        .arg(level)
        .arg("--no-incremental")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {} ({})",
        rr_src.display(),
        level
    );
}

#[test]
fn independent_if_chain_keeps_default_phi_value_under_vectorization() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping phi default regression: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("phi_default_regression");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_src = r#"
fn barrier_alloc(n, val, depth) {
  rep.int(val, n)
}

fn vector_kernel(field_u, field_v, n_l, r_l, size) {
  let out = barrier_alloc(size, 0.0, 2.0)
  let i = 1.0
  let ii = 0.0
  let idx = 0.0
  let rot = 0.0
  let comp_u = 0.0
  let comp_v = 0.0
  let u_rot = 0.0
  let uc = 0.0

  while (i <= size) {
    ii = floor(i)
    uc = field_u[ii]
    idx = floor(n_l[ii])
    rot = r_l[ii]
    comp_u = field_u[idx]
    comp_v = field_v[idx]
    u_rot = comp_u
    if (rot == 1.0) {
      u_rot = 0.0 - comp_v
    }
    if (rot == 2.0) {
      u_rot = 0.0 - comp_u
    }
    if (rot == 3.0) {
      u_rot = comp_v
    }
    out[ii] = 4.0 * uc - u_rot
    i += 1.0
  }
  return out
}

let field_u = [10.0, 10.0, 10.0, 10.0]
let field_v = [7.0, 7.0, 7.0, 7.0]
let n_l = [1.0, 1.0, 1.0, 1.0]
let r_l = [0.0, 1.0, 2.0, 3.0]
print(vector_kernel(field_u, field_v, n_l, r_l, 4.0))
"#;

    let rr_path = proj_dir.join("phi_default_regression.rr");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let o0 = proj_dir.join("out_o0.R");
    let o2 = proj_dir.join("out_o2.R");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let run_o0 = run_rscript(&rscript, &o0);
    let run_o2 = run_rscript(&rscript, &o2);
    let stdout_o0 = normalize(&run_o0.stdout);
    let stdout_o2 = normalize(&run_o2.stdout);
    let stderr_o0 = normalize(&run_o0.stderr);
    let stderr_o2 = normalize(&run_o2.stderr);

    assert!(
        run_o0.status == 0,
        "O0 run failed:\nstdout={stdout_o0}\nstderr={stderr_o0}"
    );
    assert!(
        run_o2.status == 0,
        "O2 run failed:\nstdout={stdout_o2}\nstderr={stderr_o2}"
    );
    assert_eq!(
        stdout_o2, stdout_o0,
        "O2 changed independent-if default semantics:\nO0 stdout={stdout_o0}\nO2 stdout={stdout_o2}"
    );
    assert!(
        stdout_o2.contains("[1] 30 47 50 33"),
        "unexpected kernel output:\nstdout={stdout_o2}"
    );
}
