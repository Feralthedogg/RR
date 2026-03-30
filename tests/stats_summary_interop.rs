mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_summary_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats summary runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_summary_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"
import r * as base from "base"

fn inspect_summary() -> float {
  let lm_fit = stats.lm(stats.as.formula("mpg ~ wt + hp"), data = datasets.mtcars)
  let glm_fit = stats.glm(stats.as.formula("am ~ mpg"), data = datasets.mtcars, family = stats.binomial())
  let aov_fit = stats.aov(stats.as.formula("mpg ~ factor(cyl)"), data = datasets.mtcars)
  let sf = stats.stepfun(c(1.0, 2.0, 3.0), c(10.0, 20.0, 30.0, 40.0))
  let slm = stats.summary.lm(lm_fit)
  let sglm = stats.summary.glm(glm_fit)
  let saov = stats.summary.aov(aov_fit)
  let sstep = stats.summary.stepfun(sf)
  print(slm.sigma)
  print(base.dim(slm.coefficients))
  print(sglm.family.family)
  print(sglm.family.link)
  print(sglm.deviance)
  print(base.dim(sglm.coefficients))
  print(base.length(saov))
  print(sstep)
  return slm.sigma + sglm.deviance + sglm.aic + base.length(saov)
}

print(inspect_summary())
"#;

    let rr_path = out_dir.join("stats_summary_interop.rr");
    let o0 = out_dir.join("stats_summary_interop_o0.R");
    let o2 = out_dir.join("stats_summary_interop_o2.R");

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
fn stats_summary_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"

fn inspect_summary_helpers() -> int {
  let lm_fit = stats.lm(stats.as.formula("mpg ~ wt + hp"), data = datasets.mtcars)
  let glm_fit = stats.glm(stats.as.formula("am ~ mpg"), data = datasets.mtcars, family = stats.binomial())
  let aov_fit = stats.aov(stats.as.formula("mpg ~ factor(cyl)"), data = datasets.mtcars)
  let sf = stats.stepfun(c(1.0, 2.0, 3.0), c(10.0, 20.0, 30.0, 40.0))
  print(stats.summary.lm(lm_fit))
  print(stats.summary.glm(glm_fit))
  print(stats.summary.aov(aov_fit))
  print(stats.summary.stepfun(sf))
  return 1L
}

print(inspect_summary_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_summary_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats summary helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
