mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_fit_internals_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats fit-internals runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_fit_internals_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"
import r * as base from "base"

fn inspect_fit_internals() -> float {
  let x = base.cbind(c(1.0, 1.0, 1.0, 1.0), c(1.0, 2.0, 3.0, 4.0))
  let y = c(1.0, 2.0, 3.0, 4.0)
  let biny = c(0.0, 1.0, 0.0, 1.0)
  let gfit = stats.glm.fit(x = x, y = biny, family = stats.binomial())
  let lfit = stats.lm.fit(x = x, y = y)
  let lwfit = stats.lm.wfit(x = x, y = y, w = c(1.0, 1.0, 1.0, 1.0))
  let xls = base.cbind(c(1.0,2.0,3.0,4.0,5.0,6.0), c(1.0,3.0,2.0,5.0,4.0,6.0))
  let yls = c(1.0,2.0,4.0,3.0,5.0,6.0)
  let ls = stats.lsfit(x = xls, y = yls)
  let lsd = stats.ls.diag(ls)
  let pc = stats.princomp(base.matrix(c(1.0, 2.0, 3.0, 4.0), ncol = 2L))
  let ld = stats.loadings(pc)
  let fit = stats.lm(stats.as.formula("mpg ~ wt + hp"), data = datasets.mtcars)
  let mpc = stats.makepredictcall(stats.coefficients(fit), stats.getCall(fit))
  let nc = stats.na.contiguous(c(1.0, 2.0, NA, 3.0, 4.0))
  print(gfit.coefficients)
  print(gfit.residuals)
  print(gfit.iter)
  print(lfit.coefficients)
  print(lfit.residuals)
  print(lfit.rank)
  print(lwfit.weights)
  print(lwfit.rank)
  print(ls.coefficients)
  print(ls.intercept)
  print(lsd)
  print(lsd.hat)
  print(base.dim(lsd.correlation))
  print(base.dim(ld))
  print(mpc)
  print(nc)
  return gfit.deviance + lfit.rank + lwfit.rank + ls.intercept + base.length(lsd.hat)
}

print(inspect_fit_internals())
"#;

    let rr_path = out_dir.join("stats_fit_internals_interop.rr");
    let o0 = out_dir.join("stats_fit_internals_interop_o0.R");
    let o2 = out_dir.join("stats_fit_internals_interop_o2.R");

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
fn stats_fit_internals_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"
import r * as base from "base"

fn inspect_fit_internals_helpers() -> int {
  let x = base.cbind(c(1.0, 1.0, 1.0, 1.0), c(1.0, 2.0, 3.0, 4.0))
  let y = c(1.0, 2.0, 3.0, 4.0)
  let biny = c(0.0, 1.0, 0.0, 1.0)
  print(stats.glm.fit(x = x, y = biny, family = stats.binomial()))
  print(stats.lm.fit(x = x, y = y))
  print(stats.lm.wfit(x = x, y = y, w = c(1.0, 1.0, 1.0, 1.0)))
  let xls = base.cbind(c(1.0,2.0,3.0,4.0,5.0,6.0), c(1.0,3.0,2.0,5.0,4.0,6.0))
  let yls = c(1.0,2.0,4.0,3.0,5.0,6.0)
  let ls = stats.lsfit(x = xls, y = yls)
  print(ls)
  print(stats.ls.diag(ls))
  let pc = stats.princomp(base.matrix(c(1.0, 2.0, 3.0, 4.0), ncol = 2L))
  print(stats.loadings(pc))
  let fit = stats.lm(stats.as.formula("mpg ~ wt + hp"), data = datasets.mtcars)
  print(stats.makepredictcall(stats.coefficients(fit), stats.getCall(fit)))
  print(stats.na.contiguous(c(1.0, 2.0, NA, 3.0, 4.0)))
  return 1L
}

print(inspect_fit_internals_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_fit_internals_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats fit-internals helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
