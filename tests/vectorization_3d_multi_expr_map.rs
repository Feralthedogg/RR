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
fn multi_output_3d_expr_maps_vectorize_and_preserve_results() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping 3D multi-expr-map test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("vectorization_3d_multi_expr_map");
    fs::create_dir_all(&out_dir).expect("failed to create test dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r * as base from "base"

fn pair_dim1(a, y, z) {
  let i = 1
  while (i <= 3) {
    y[i, 2, 2] = a[i, 1, 1] + 1
    z[i, 3, 3] = a[i, 2, 2] * 2
    i += 1
  }
  return y[1, 2, 2] + y[2, 2, 2] + y[3, 2, 2] + z[1, 3, 3] + z[2, 3, 3] + z[3, 3, 3]
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
let y = base.array(rep.int(0, 27), base.c(3, 3, 3))
let z = base.array(rep.int(0, 27), base.c(3, 3, 3))

print(pair_dim1(a, y, z))
"#;

    let rr_path = out_dir.join("vectorization_3d_multi_expr_map.rr");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let o0 = out_dir.join("vectorization_3d_multi_expr_map_o0.R");
    let o2 = out_dir.join("vectorization_3d_multi_expr_map_o2.R");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let code = fs::read_to_string(&o2).expect("failed to read O2 output");
    assert!(
        code.matches("rr_dim1_assign_values(").count() >= 2,
        "expected multiple 3D expr-map assignments in O2 output:\n{}",
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
