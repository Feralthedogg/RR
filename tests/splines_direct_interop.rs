mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;

#[test]
fn splines_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping splines direct interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("splines_direct_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "splines"

fn build_basis() -> matrix<float> {
  let x = c(1.0, 1.5, 2.0, 2.5)
  let knots = c(0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 4.0, 4.0, 4.0)
  let b = splines.bs(x)
  let n = splines.ns(x)
  let d = splines.splineDesign(knots, x)
  let interp = splines.interpSpline(x, x)
  let periodic = splines.periodicSpline(x, x)
  let back = splines.backSpline(interp)
  let spline_des = splines.spline.des(knots, x)
  let as_poly = splines.as.polySpline(interp)
  let poly = splines.polySpline(periodic)
  let sk = splines.splineKnots(interp)
  let so = splines.splineOrder(interp)
  let xy = splines.xyVector(x, x)
  let av = splines.asVector(xy)
  print(n)
  print(d)
  print(length(sk))
  print(so)
  print(length(spline_des.knots))
  print(length(spline_des.derivs))
  print(length(as_poly.knots))
  print(length(poly.knots))
  print(length(back))
  print(length(periodic))
  print(length(xy.x))
  print(length(xy.y))
  print(length(av))
  return b
}

print(build_basis())
"#;

    let rr_path = out_dir.join("splines_direct_interop.rr");
    let o0 = out_dir.join("splines_direct_interop_o0.R");
    let o2 = out_dir.join("splines_direct_interop_o2.R");

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
