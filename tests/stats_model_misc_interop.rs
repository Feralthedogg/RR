mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_model_misc_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats model-misc runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_model_misc_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"
import r * as base from "base"

fn inspect_model_misc() -> float {
  let mv = stats.manova(stats.as.formula("base::cbind(mpg, disp) ~ factor(cyl)"), data = datasets.mtcars)
  let sm = stats.summary.manova(mv)
  let av = stats.aov(stats.as.formula("mpg ~ factor(cyl)"), data = datasets.mtcars)
  let pj = stats.proj(av)
  let lg = stats.loglin(base.matrix(c(10.0, 5.0, 6.0, 9.0), nrow = 2L), base.list(1L, 2L), fit = true, param = true, print = false)
  print(base.dim(mv.coefficients))
  print(base.dim(mv.residuals))
  print(base.dim(sm.Eigenvalues))
  print(base.dim(sm.stats))
  print(base.dim(pj))
  print(lg.lrt)
  print(lg.pearson)
  print(base.dim(lg.fit))
  return mv.rank + sm.stats[1L, 1L] + pj[1L, 1L] + lg.lrt
}

print(inspect_model_misc())
"#;

    let rr_path = out_dir.join("stats_model_misc_interop.rr");
    let o0 = out_dir.join("stats_model_misc_interop_o0.R");
    let o2 = out_dir.join("stats_model_misc_interop_o2.R");

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
fn stats_model_misc_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"
import r * as base from "base"

fn inspect_model_misc() -> int {
  let mv = stats.manova(stats.as.formula("base::cbind(mpg, disp) ~ factor(cyl)"), data = datasets.mtcars)
  print(mv)
  print(stats.summary.manova(mv))
  let av = stats.aov(stats.as.formula("mpg ~ factor(cyl)"), data = datasets.mtcars)
  print(stats.proj(av))
  print(stats.loglin(base.matrix(c(10.0, 5.0, 6.0, 9.0), nrow = 2L), base.list(1L, 2L), fit = true, param = true, print = false))
  return 1L
}

print(inspect_model_misc())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_model_misc_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats model-misc helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
