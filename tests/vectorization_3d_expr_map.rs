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
fn axis_stable_3d_expr_maps_vectorize_and_preserve_results() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping 3D expr-map test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("vectorization_3d_expr_map");
    fs::create_dir_all(&out_dir).expect("failed to create test dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r * as base from "base"

fn expr_dim1(src, idx, out) {
  let i = 1
  while (i <= 4) {
    out[i, 2, 2] = src[idx[i]] + 1
    i += 1
  }
  return out[1, 2, 2] + out[2, 2, 2] + out[3, 2, 2] + out[4, 2, 2]
}

fn expr_dim2(src, idx, out) {
  let j = 1
  while (j <= 4) {
    out[2, j, 2] = src[idx[j]] + 1
    j += 1
  }
  return out[2, 1, 2] + out[2, 2, 2] + out[2, 3, 2] + out[2, 4, 2]
}

fn expr_dim3(src, idx, out) {
  let k = 1
  while (k <= 4) {
    out[2, 2, k] = src[idx[k]] + 1
    k += 1
  }
  return out[2, 2, 1] + out[2, 2, 2] + out[2, 2, 3] + out[2, 2, 4]
}

fn expr3d_dim1(src3, out) {
  let i = 1
  while (i <= 4) {
    out[i, 3, 3] = src3[i, 2, 2] + 5
    i += 1
  }
  return out[1, 3, 3] + out[2, 3, 3] + out[3, 3, 3] + out[4, 3, 3]
}

fn expr3d_dim2(src3, out) {
  let j = 1
  while (j <= 4) {
    out[3, j, 3] = src3[2, j, 2] + 5
    j += 1
  }
  return out[3, 1, 3] + out[3, 2, 3] + out[3, 3, 3] + out[3, 4, 3]
}

fn expr3d_dim3(src3, out) {
  let k = 1
  while (k <= 4) {
    out[3, 3, k] = src3[2, 2, k] + 5
    k += 1
  }
  return out[3, 3, 1] + out[3, 3, 2] + out[3, 3, 3] + out[3, 3, 4]
}

fn expr3d_indirect_dim1(src3, idx, out) {
  let i = 1
  while (i <= 4) {
    out[i, 4, 4] = src3[idx[i], 2, 2] + 7
    i += 1
  }
  return out[1, 4, 4] + out[2, 4, 4] + out[3, 4, 4] + out[4, 4, 4]
}

fn expr3d_indirect_dim2(src3, idx, out) {
  let j = 1
  while (j <= 4) {
    out[4, j, 4] = src3[2, idx[j], 2] + 7
    j += 1
  }
  return out[4, 1, 4] + out[4, 2, 4] + out[4, 3, 4] + out[4, 4, 4]
}

fn expr3d_indirect_dim3(src3, idx, out) {
  let k = 1
  while (k <= 4) {
    out[4, 4, k] = src3[2, 2, idx[k]] + 7
    k += 1
  }
  return out[4, 4, 1] + out[4, 4, 2] + out[4, 4, 3] + out[4, 4, 4]
}

fn expr3d_general_gather_dim1(src3, idx_i, idx_j, out) {
  let i = 1
  while (i <= 4) {
    out[i, 1, 4] = src3[idx_i[i], idx_j[i], 2] + 9
    i += 1
  }
  return out[1, 1, 4] + out[2, 1, 4] + out[3, 1, 4] + out[4, 1, 4]
}

let src = c(10, 20, 30, 40)
let idx = c(4, 2, 3, 1)
let idx_j = c(1, 3, 4, 2)
let out = base.array(rep.int(0, 64), base.c(4, 4, 4))
let src3 = base.array(seq_len(64), base.c(4, 4, 4))

print(expr_dim1(src, idx, out))
print(expr_dim2(src, idx, out))
print(expr_dim3(src, idx, out))
print(expr3d_dim1(src3, out))
print(expr3d_dim2(src3, out))
print(expr3d_dim3(src3, out))
print(expr3d_indirect_dim1(src3, idx, out))
print(expr3d_indirect_dim2(src3, idx, out))
print(expr3d_indirect_dim3(src3, idx, out))
print(expr3d_general_gather_dim1(src3, idx, idx_j, out))
"#;

    let rr_path = out_dir.join("vectorization_3d_expr_map.rr");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let o0 = out_dir.join("vectorization_3d_expr_map_o0.R");
    let o2 = out_dir.join("vectorization_3d_expr_map_o2.R");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let code = fs::read_to_string(&o2).expect("failed to read O2 output");
    assert!(
        code.contains("rr_dim1_assign_values("),
        "expected dim1 3D expr-map helper in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_dim2_assign_values("),
        "expected dim2 3D expr-map helper in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_dim3_assign_values("),
        "expected dim3 3D expr-map helper in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_dim1_read_values(")
            || code.contains("rr_dim2_read_values(")
            || code.contains("rr_dim3_read_values("),
        "expected aligned 3D read helper in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_gather(") || code.contains("rr_index1_read_vec("),
        "expected gather-style vectorized RHS in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_array3_gather_values("),
        "expected general 3D gather helper in O2 output:\n{}",
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
