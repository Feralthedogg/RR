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
fn axis_stable_3d_maps_vectorize_and_preserve_results() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping 3D vectorization test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("vectorization_3d_axis_map");
    fs::create_dir_all(&out_dir).expect("failed to create test dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r * as base from "base"

fn map_dim1(a, b, out) {
  let i = 1
  while (i <= 3) {
    out[i, 2, 3] = a[i, 2, 3] + b[i, 2, 3]
    i += 1
  }
  return out[1, 2, 3] + out[2, 2, 3] + out[3, 2, 3]
}

fn map_dim2(a, b, out) {
  let j = 1
  while (j <= 3) {
    out[2, j, 3] = a[2, j, 3] + b[2, j, 3]
    j += 1
  }
  return out[2, 1, 3] + out[2, 2, 3] + out[2, 3, 3]
}

fn map_dim3(a, b, out) {
  let k = 1
  while (k <= 3) {
    out[2, 3, k] = a[2, 3, k] + b[2, 3, k]
    k += 1
  }
  return out[2, 3, 1] + out[2, 3, 2] + out[2, 3, 3]
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
let b = base.array(seq_len(27) * 2, base.c(3, 3, 3))

print(map_dim1(a, b, base.array(rep.int(0, 27), base.c(3, 3, 3))))
print(map_dim2(a, b, base.array(rep.int(0, 27), base.c(3, 3, 3))))
print(map_dim3(a, b, base.array(rep.int(0, 27), base.c(3, 3, 3))))
"#;

    let rr_path = out_dir.join("vectorization_3d_axis_map.rr");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let o0 = out_dir.join("vectorization_3d_axis_map_o0.R");
    let o2 = out_dir.join("vectorization_3d_axis_map_o2.R");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let code = fs::read_to_string(&o2).expect("failed to read O2 output");
    assert!(
        code.contains("rr_dim1_binop_assign(") || code.contains("rr_dim1_assign_values("),
        "expected dim1 3D map helper in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_dim2_binop_assign(") || code.contains("rr_dim2_assign_values("),
        "expected dim2 3D map helper in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_dim3_binop_assign(") || code.contains("rr_dim3_assign_values("),
        "expected dim3 3D map helper in O2 output:\n{}",
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
        "[1] 207\n[1] 207\n[1] 153\n",
        "unexpected 3D map baseline output"
    );
}
