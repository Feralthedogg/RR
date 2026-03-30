mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_table_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats table runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_table_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"
import r * as base from "base"

fn inspect_table() -> float {
  let mat = base.matrix(c(1.0, 2.0, 3.0, 4.0), nrow = 2L)
  let am = stats.addmargins(mat)
  let ft = stats.ftable(datasets.Titanic)
  let df = base.data.frame(
    Freq = c(1.0, 2.0, 3.0, 4.0),
    Class = c("A", "A", "B", "B"),
    Survived = c("Yes", "No", "Yes", "No")
  )
  let xt = stats.xtabs(stats.as.formula("Freq ~ Class + Survived"), data = df)
  let iso = stats.isoreg(c(3.0, 1.0, 2.0, 5.0))
  let sm = stats.smooth(c(1.0, 2.0, 3.0, 4.0, 5.0))
  let se = stats.smoothEnds(c(1.0, 2.0, 3.0, 4.0, 5.0))
  let ln = stats.line(c(1.0, 2.0, 3.0, 4.0, 5.0))
  let vx = stats.varimax(mat)
  let px = stats.promax(mat)
  print(base.dim(am))
  print(base.dim(ft))
  print(base.dim(xt))
  print(iso.x)
  print(iso.yf)
  print(iso.iKnots)
  print(iso.isOrd)
  print(sm)
  print(se)
  print(ln.coefficients)
  print(ln.residuals)
  print(base.dim(vx.loadings))
  print(base.dim(vx.rotmat))
  print(base.dim(px.loadings))
  print(base.dim(px.rotmat))
  return base.length(am) + base.length(ft) + base.length(xt) + sm[1L] + se[1L]
}

print(inspect_table())
"#;

    let rr_path = out_dir.join("stats_table_interop.rr");
    let o0 = out_dir.join("stats_table_interop_o0.R");
    let o2 = out_dir.join("stats_table_interop_o2.R");

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
fn stats_table_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"
import r * as base from "base"

fn inspect_table_helpers() -> int {
  let mat = base.matrix(c(1.0, 2.0, 3.0, 4.0), nrow = 2L)
  print(stats.addmargins(mat))
  print(stats.ftable(datasets.Titanic))
  let df = base.data.frame(
    Freq = c(1.0, 2.0, 3.0, 4.0),
    Class = c("A", "A", "B", "B"),
    Survived = c("Yes", "No", "Yes", "No")
  )
  print(stats.xtabs(stats.as.formula("Freq ~ Class + Survived"), data = df))
  print(stats.isoreg(c(3.0, 1.0, 2.0, 5.0)))
  print(stats.smooth(c(1.0, 2.0, 3.0, 4.0, 5.0)))
  print(stats.smoothEnds(c(1.0, 2.0, 3.0, 4.0, 5.0)))
  print(stats.line(c(1.0, 2.0, 3.0, 4.0, 5.0)))
  print(stats.varimax(mat))
  print(stats.promax(mat))
  return 1L
}

print(inspect_table_helpers())
"#;

    let (ok, stdout, stderr) = run_compile_case("stats_table_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats table helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
