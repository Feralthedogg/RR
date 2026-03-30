mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_multivar_math_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats multivar-math runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_multivar_math_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_multivar_math() -> float {
  let m = base.matrix(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), nrow = 3L)
  let cv = stats.cov.wt(m)
  let c2 = stats.cov2cor(base.matrix(c(2.0, 1.0, 1.0, 3.0), nrow = 2L))
  let mh = stats.mahalanobis(base.matrix(c(1.0, 2.0, 3.0, 4.0), nrow = 2L), center = c(0.0, 0.0), cov = base.diag(2L))
  let rw = stats.rWishart(2L, 5.0, base.diag(2L))
  let r2 = stats.r2dtable(2L, c(5.0, 7.0), c(6.0, 6.0))
  let dm = stats.dmultinom(c(2.0, 1.0, 1.0), prob = c(0.2, 0.3, 0.5))
  let rm = stats.rmultinom(3L, 4.0, c(0.2, 0.3, 0.5))
  print(cv.cov)
  print(cv.center)
  print(cv)
  print(c2)
  print(mh)
  print(dim(rw))
  print(length(r2))
  print(dm)
  return dm + mh[1L]
}

print(inspect_multivar_math())
"#;

    let rr_path = out_dir.join("stats_multivar_math_interop.rr");
    let o0 = out_dir.join("stats_multivar_math_interop_o0.R");
    let o2 = out_dir.join("stats_multivar_math_interop_o2.R");

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

#[test]
fn stats_multivar_math_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_multivar_math_helpers() -> int {
  let m = base.matrix(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), nrow = 3L)
  let cv = stats.cov.wt(m)
  let c2 = stats.cov2cor(base.matrix(c(2.0, 1.0, 1.0, 3.0), nrow = 2L))
  let mh = stats.mahalanobis(base.matrix(c(1.0, 2.0, 3.0, 4.0), nrow = 2L), center = c(0.0, 0.0), cov = base.diag(2L))
  let rw = stats.rWishart(2L, 5.0, base.diag(2L))
  let r2 = stats.r2dtable(2L, c(5.0, 7.0), c(6.0, 6.0))
  let dm = stats.dmultinom(c(2.0, 1.0, 1.0), prob = c(0.2, 0.3, 0.5))
  let rm = stats.rmultinom(3L, 4.0, c(0.2, 0.3, 0.5))
  print(cv)
  print(c2)
  print(mh)
  print(rw)
  print(r2)
  print(dm)
  print(rm)
  return 1L
}

print(inspect_multivar_math_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_multivar_math_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats multivar-math helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
