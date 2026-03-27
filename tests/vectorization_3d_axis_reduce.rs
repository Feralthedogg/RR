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
fn axis_stable_3d_reductions_vectorize_and_preserve_results() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping 3D reduction vectorization test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("vectorization_3d_axis_reduce");
    fs::create_dir_all(&out_dir).expect("failed to create test dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r * as base from "base"

fn reduce_dim1(a) {
  let acc = 0
  let i = 1
  while (i <= 3) {
    acc = acc + a[i, 2, 3]
    i += 1
  }
  return acc
}

fn reduce_dim2(a) {
  let acc = 0
  let j = 1
  while (j <= 3) {
    acc = acc + a[2, j, 3]
    j += 1
  }
  return acc
}

fn reduce_dim3(a) {
  let acc = 0
  let k = 1
  while (k <= 3) {
    acc = acc + a[2, 3, k]
    k += 1
  }
  return acc
}

fn prod_dim1(a) {
  let acc = 1
  let i = 1
  while (i <= 3) {
    acc = acc * a[i, 2, 3]
    i += 1
  }
  return acc
}

fn min_dim2(a) {
  let acc = 999
  let j = 1
  while (j <= 3) {
    acc = min(acc, a[2, j, 3])
    j += 1
  }
  return acc
}

fn max_dim3(a) {
  let acc = -1
  let k = 1
  while (k <= 3) {
    acc = max(acc, a[2, 3, k])
    k += 1
  }
  return acc
}

fn reduce_general_dim1(a, idx_i, idx_j) {
  let acc = 0
  let i = 1
  while (i <= 3) {
    acc = acc + a[idx_i[i], idx_j[i], 3]
    i += 1
  }
  return acc
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
let idx_i = c(3, 2, 1)
let idx_j = c(1, 3, 2)

print(reduce_dim1(a))
print(reduce_dim2(a))
print(reduce_dim3(a))
print(prod_dim1(a))
print(min_dim2(a))
print(max_dim3(a))
print(reduce_general_dim1(a, idx_i, idx_j))
"#;

    let rr_path = out_dir.join("vectorization_3d_axis_reduce.rr");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let o0 = out_dir.join("vectorization_3d_axis_reduce_o0.R");
    let o2 = out_dir.join("vectorization_3d_axis_reduce_o2.R");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let code = fs::read_to_string(&o2).expect("failed to read O2 output");
    assert!(
        code.contains("rr_dim1_sum_range(") || code.contains("sum(rr_dim1_read_values("),
        "expected dim1 3D reduction helper in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_dim2_sum_range(") || code.contains("sum(rr_dim2_read_values("),
        "expected dim2 3D reduction helper in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_dim3_sum_range(") || code.contains("sum(rr_dim3_read_values("),
        "expected dim3 3D reduction helper in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_dim1_reduce_range(") || code.contains("prod(rr_dim1_read_values("),
        "expected dim1 3D product reduction lowering in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_dim2_reduce_range(") || code.contains("min(rr_dim2_read_values("),
        "expected dim2 3D min reduction lowering in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_dim3_reduce_range(") || code.contains("max(rr_dim3_read_values("),
        "expected dim3 3D max reduction lowering in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("sum(rr_array3_gather_values("),
        "expected generic 3D gather reduction lowering in O2 output:\n{}",
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
    assert_eq!(
        normalize(&base.stdout),
        "[1] 69\n[1] 69\n[1] 51\n[1] 12144\n[1] 20\n[1] 26\n[1] 69\n",
        "unexpected 3D reduction baseline output"
    );
}
