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
fn axis_stable_3d_scatter_maps_vectorize_and_preserve_results() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping 3D scatter-map test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("vectorization_3d_scatter_map");
    fs::create_dir_all(&out_dir).expect("failed to create test dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r * as base from "base"

fn scatter_dim1(src, idx, out) {
  let i = 1
  while (i <= 4) {
    out[idx[i], 2, 2] = src[i]
    i += 1
  }
  return out[1, 2, 2] + out[2, 2, 2] + out[3, 2, 2] + out[4, 2, 2]
}

fn scatter_dim2(src, idx, out) {
  let j = 1
  while (j <= 4) {
    out[2, idx[j], 2] = src[j]
    j += 1
  }
  return out[2, 1, 2] + out[2, 2, 2] + out[2, 3, 2] + out[2, 4, 2]
}

fn scatter_dim3(src, idx, out) {
  let k = 1
  while (k <= 4) {
    out[2, 2, idx[k]] = src[k]
    k += 1
  }
  return out[2, 2, 1] + out[2, 2, 2] + out[2, 2, 3] + out[2, 2, 4]
}

fn scatter_general(src, idx_i, idx_j, out) {
  let i = 1
  while (i <= 4) {
    out[idx_i[i], idx_j[i], 2] = src[i] + i
    i += 1
  }
  return out[1, 4, 2] + out[2, 3, 2] + out[3, 2, 2] + out[4, 1, 2]
}

let src = c(10, 20, 30, 40)
let idx = c(4, 2, 3, 1)
let idx_i = c(1, 2, 3, 4)
let idx_j = c(4, 3, 2, 1)
let out = base.array(rep.int(0, 64), base.c(4, 4, 4))

print(scatter_dim1(src, idx, out))
print(scatter_dim2(src, idx, out))
print(scatter_dim3(src, idx, out))
print(scatter_general(src, idx_i, idx_j, out))
"#;

    let rr_path = out_dir.join("vectorization_3d_scatter_map.rr");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let o0 = out_dir.join("vectorization_3d_scatter_map_o0.R");
    let o2 = out_dir.join("vectorization_3d_scatter_map_o2.R");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let code = fs::read_to_string(&o2).expect("failed to read O2 output");
    assert!(
        code.contains("rr_dim1_assign_index_values("),
        "expected dim1 3D scatter helper in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_dim2_assign_index_values("),
        "expected dim2 3D scatter helper in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_dim3_assign_index_values("),
        "expected dim3 3D scatter helper in O2 output:\n{}",
        code
    );
    assert!(
        code.contains("rr_array3_assign_gather_values("),
        "expected general 3D scatter helper in O2 output:\n{}",
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
