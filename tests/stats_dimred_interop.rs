mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_dimred_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats dimred runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_dimred_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_dimred() -> float {
  let d = stats.dist(c(1.0, 2.0, 3.0, 4.0))
  let cs = stats.cmdscale(d, 2L)
  let pc = stats.princomp(base.matrix(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), ncol = 2L))
  let cc = stats.cancor(
    base.matrix(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), ncol = 2L),
    base.matrix(c(2.0, 4.0, 6.0, 8.0, 10.0, 12.0), ncol = 2L)
  )
  print(base.dim(cs))
  print(pc.sdev)
  print(base.dim(pc.loadings))
  print(base.dim(pc.scores))
  print(cc.cor)
  print(base.dim(cc.xcoef))
  print(base.dim(cc.ycoef))
  print(cc.xcenter)
  print(cc.ycenter)
  return base.length(pc.sdev) + base.length(cc.cor)
}

print(inspect_dimred())
"#;

    let rr_path = out_dir.join("stats_dimred_interop.rr");
    let o0 = out_dir.join("stats_dimred_interop_o0.R");
    let o2 = out_dir.join("stats_dimred_interop_o2.R");

    fs::write(&rr_path, src).expect("failed to write source");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let run_o0 = run_rscript(&rscript, &o0);
    let run_o2 = run_rscript(&rscript, &o2);

    assert_eq!(run_o0.status, 0, "O0 runtime failed:\n{}", run_o0.stderr);
    assert_eq!(run_o2.status, 0, "O2 runtime failed:\n{}", run_o2.stderr);
    assert_eq!(
        normalize(&run_o0.stdout),
        normalize(&run_o2.stdout),
        "stdout mismatch O0 vs O2"
    );
    assert_eq!(
        normalize(&run_o0.stderr),
        normalize(&run_o2.stderr),
        "stderr mismatch O0 vs O2"
    );
}
