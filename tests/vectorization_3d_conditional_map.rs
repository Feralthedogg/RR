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
fn axis_stable_3d_conditional_maps_vectorize_and_preserve_results() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping 3D conditional map test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("vectorization_3d_conditional_map");
    fs::create_dir_all(&out_dir).expect("failed to create test dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r * as base from "base"

fn cond_dim1(a, b, out, t) {
  let i = 1
  while (i <= 3) {
    if (a[i, 2, 3] > t) {
      out[i, 2, 3] = b[i, 2, 3]
    } else {
      out[i, 2, 3] = 0
    }
    i += 1
  }
  return out[1, 2, 3] + out[2, 2, 3] + out[3, 2, 3]
}

fn cond_dim2(a, b, out, t) {
  let j = 1
  while (j <= 3) {
    if (a[2, j, 3] > t) {
      out[2, j, 3] = b[2, j, 3]
    } else {
      out[2, j, 3] = 0
    }
    j += 1
  }
  return out[2, 1, 3] + out[2, 2, 3] + out[2, 3, 3]
}

fn cond_dim3(a, b, out, t) {
  let k = 1
  while (k <= 3) {
    if (a[2, 3, k] > t) {
      out[2, 3, k] = b[2, 3, k]
    } else {
      out[2, 3, k] = 0
    }
    k += 1
  }
  return out[2, 3, 1] + out[2, 3, 2] + out[2, 3, 3]
}

fn cond_general_dim1(a, b, idx_i, idx_j, out, t) {
  let i = 1
  while (i <= 3) {
    if (a[idx_i[i], idx_j[i], 3] > t) {
      out[i, 1, 3] = b[idx_i[i], idx_j[i], 3]
    } else {
      out[i, 1, 3] = a[idx_i[i], idx_j[i], 3] + 1
    }
    i += 1
  }
  return out[1, 1, 3] + out[2, 1, 3] + out[3, 1, 3]
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
let b = base.array(seq_len(27) * 2, base.c(3, 3, 3))
let idx_i = c(3, 2, 1)
let idx_j = c(1, 3, 2)

print(cond_dim1(a, b, base.array(rep.int(0, 27), base.c(3, 3, 3)), 22))
print(cond_dim2(a, b, base.array(rep.int(0, 27), base.c(3, 3, 3)), 22))
print(cond_dim3(a, b, base.array(rep.int(0, 27), base.c(3, 3, 3)), 16))
print(cond_general_dim1(a, b, idx_i, idx_j, base.array(rep.int(0, 27), base.c(3, 3, 3)), 12))
"#;

    let rr_path = out_dir.join("vectorization_3d_conditional_map.rr");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let o0 = out_dir.join("vectorization_3d_conditional_map_o0.R");
    let o2 = out_dir.join("vectorization_3d_conditional_map_o2.R");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let code = fs::read_to_string(&o2).expect("failed to read O2 output");
    assert!(
        code.contains("rr_dim1_ifelse_assign("),
        "expected dim1 3D conditional helper in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_dim2_ifelse_assign("),
        "expected dim2 3D conditional helper in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_dim3_ifelse_assign("),
        "expected dim3 3D conditional helper in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_array3_gather_values("),
        "expected generalized 3D gather read in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_ifelse_strict(") && code.contains("rr_dim1_assign_values("),
        "expected generalized 3D conditional lowering in O2 output:\n{}",
        code
    );

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
