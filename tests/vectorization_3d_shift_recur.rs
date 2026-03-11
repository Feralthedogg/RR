mod common;

use common::{normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn compile_rr(rr_bin: &Path, rr_src: &Path, out: &Path, level: &str) {
    let status = Command::new(rr_bin)
        .arg(rr_src)
        .arg("-o")
        .arg(out)
        .arg("--no-incremental")
        .arg(level)
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
fn shifted_and_recurrence_3d_vectorize_and_preserve_results() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping 3D shift/recur test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("vectorization_3d_shift_recur");
    fs::create_dir_all(&out_dir).expect("failed to create test dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r * as base from "base"

fn shift_dim1(src, out) {
  let i = 1
  while (i <= 3) {
    out[i, 2, 2] = src[i + 1, 2, 2]
    i += 1
  }
  return out[1, 2, 2] + out[2, 2, 2] + out[3, 2, 2]
}

fn shift_dim2(src, out) {
  let j = 1
  while (j <= 3) {
    out[2, j, 2] = src[2, j + 1, 2]
    j += 1
  }
  return out[2, 1, 2] + out[2, 2, 2] + out[2, 3, 2]
}

fn shift_dim3(src, out) {
  let k = 1
  while (k <= 3) {
    out[2, 2, k] = src[2, 2, k + 1]
    k += 1
  }
  return out[2, 2, 1] + out[2, 2, 2] + out[2, 2, 3]
}

fn recur_dim1(out) {
  out[1, 3, 3] = 10
  let i = 2
  while (i <= 4) {
    out[i, 3, 3] = out[i - 1, 3, 3] + 2
    i += 1
  }
  return out[1, 3, 3] + out[2, 3, 3] + out[3, 3, 3] + out[4, 3, 3]
}

fn recur_dim2(out) {
  out[3, 1, 3] = 5
  let j = 2
  while (j <= 4) {
    out[3, j, 3] = out[3, j - 1, 3] + 3
    j += 1
  }
  return out[3, 1, 3] + out[3, 2, 3] + out[3, 3, 3] + out[3, 4, 3]
}

fn recur_dim3(out) {
  out[3, 3, 1] = 7
  let k = 2
  while (k <= 4) {
    out[3, 3, k] = out[3, 3, k - 1] + 4
    k += 1
  }
  return out[3, 3, 1] + out[3, 3, 2] + out[3, 3, 3] + out[3, 3, 4]
}

let src = base.array(seq_len(64), base.c(4, 4, 4))
let out = base.array(rep.int(0, 64), base.c(4, 4, 4))

print(shift_dim1(src, out))
print(shift_dim2(src, out))
print(shift_dim3(src, out))
print(recur_dim1(base.array(rep.int(0, 64), base.c(4, 4, 4))))
print(recur_dim2(base.array(rep.int(0, 64), base.c(4, 4, 4))))
print(recur_dim3(base.array(rep.int(0, 64), base.c(4, 4, 4))))
"#;

    let rr_path = out_dir.join("vectorization_3d_shift_recur.rr");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let o0 = out_dir.join("vectorization_3d_shift_recur_o0.R");
    let o2 = out_dir.join("vectorization_3d_shift_recur_o2.R");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let code = fs::read_to_string(&o2).expect("failed to read O2 output");
    assert!(code.contains("rr_dim1_shift_assign("));
    assert!(code.contains("rr_dim2_shift_assign("));
    assert!(code.contains("rr_dim3_shift_assign("));

    let base = run_rscript(&rscript, &o0);
    let opt = run_rscript(&rscript, &o2);
    assert_eq!(base.status, 0, "O0 execution failed:\n{}", base.stderr);
    assert_eq!(opt.status, 0, "O2 execution failed:\n{}", opt.stderr);
    assert_eq!(
        normalize(&base.stdout),
        normalize(&opt.stdout),
        "stdout mismatch between O0 and O2"
    );
}
