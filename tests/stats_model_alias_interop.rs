mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_model_alias_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats model-alias runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_model_alias_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"
import r * as base from "base"
import r { set.seed } from "base"

fn inspect_model_alias() -> float {
  set.seed(1L)
  let stats_ns = base.asNamespace("stats")
  let has_qr_influence = base.exists("qr.influence", envir = stats_ns, inherits = false)
  let lm_fit = stats.lm(stats.as.formula("mpg ~ wt + hp"), data = datasets.mtcars)
  let glm_fit = stats.glm(stats.as.formula("am ~ mpg"), data = datasets.mtcars, family = stats.binomial())
  let sim_lm = stats.simulate(lm_fit, nsim = 2L)
  let sim_glm = stats.simulate(glm_fit, nsim = 2L)
  let co = stats.coefficients(lm_fit)
  let fv = stats.fitted.values(lm_fit)
  let resid = stats.resid(lm_fit)
  let plm = stats.predict.lm(lm_fit)
  let pglm = stats.predict.glm(glm_fit)
  let rlm = stats.residuals.lm(lm_fit)
  let rglm = stats.residuals.glm(glm_fit)
  let clm = stats.confint.lm(lm_fit)
  let cdef = stats.confint.default(glm_fit)
  let gc = stats.getCall(lm_fit)
  let hat = stats.hatvalues(lm_fit)
  let h = stats.hat(stats.model.matrix.lm(lm_fit))
  let cd = stats.cooks.distance(lm_fit)
  let cr = stats.covratio(lm_fit)
  let dfb = stats.dfbeta(lm_fit)
  let dfbs = stats.dfbetas(lm_fit)
  let dff = stats.dffits(lm_fit)
  let rs = stats.rstandard(lm_fit)
  let rst = stats.rstudent(lm_fit)
  let wr = stats.weighted.residuals(lm_fit)
  let infl = stats.influence(lm_fit)
  let infm = stats.influence.measures(lm_fit)
  let li = stats.lm.influence(lm_fit)
  print(base.dim(sim_lm))
  print(base.dim(sim_glm))
  print(sim_lm)
  print(sim_glm)
  print(co)
  print(fv)
  print(resid)
  print(plm)
  print(pglm)
  print(rlm)
  print(rglm)
  print(clm)
  print(cdef)
  print(gc)
  print(hat)
  print(h)
  print(cd)
  print(cr)
  print(dfb)
  print(dfbs)
  print(dff)
  print(rs)
  print(rst)
  print(wr)
  print(infl.hat)
  print(infl.coefficients)
  print(infl.sigma)
  print(infm)
  if (has_qr_influence) {
    let qri = stats.qr.influence(lm_fit.qr, stats.residuals.lm(lm_fit))
    print(qri.hat)
    print(qri.sigma)
  } else {
    print(li.hat)
    print(li.sigma)
  }
  print(li.hat)
  print(li.coefficients)
  print(li.sigma)
  print(li)
  return co[1L] + fv[1L] + resid[1L] + plm[1L] + pglm[1L] + hat[1L] + h[1L] + cd[1L] + cr[1L] + dff[1L] + rs[1L] + rst[1L] + wr[1L] + sim_lm[1L, 1L]
}

print(inspect_model_alias())
"#;

    let rr_path = out_dir.join("stats_model_alias_interop.rr");
    let o0 = out_dir.join("stats_model_alias_interop_o0.R");
    let o2 = out_dir.join("stats_model_alias_interop_o2.R");

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
fn stats_model_alias_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"

fn inspect_model_alias_helpers() -> int {
  let lm_fit = stats.lm(stats.as.formula("mpg ~ wt + hp"), data = datasets.mtcars)
  let glm_fit = stats.glm(stats.as.formula("am ~ mpg"), data = datasets.mtcars, family = stats.binomial())
  print(stats.simulate(lm_fit, nsim = 2L))
  print(stats.simulate(glm_fit, nsim = 2L))
  print(stats.coefficients(lm_fit))
  print(stats.fitted.values(lm_fit))
  print(stats.resid(lm_fit))
  print(stats.predict.lm(lm_fit))
  print(stats.predict.glm(glm_fit))
  print(stats.residuals.lm(lm_fit))
  print(stats.residuals.glm(glm_fit))
  print(stats.confint.lm(lm_fit))
  print(stats.confint.default(glm_fit))
  print(stats.getCall(lm_fit))
  print(stats.hatvalues(lm_fit))
  print(stats.hat(stats.model.matrix.lm(lm_fit)))
  print(stats.cooks.distance(lm_fit))
  print(stats.covratio(lm_fit))
  print(stats.dfbeta(lm_fit))
  print(stats.dfbetas(lm_fit))
  print(stats.dffits(lm_fit))
  print(stats.rstandard(lm_fit))
  print(stats.rstudent(lm_fit))
  print(stats.weighted.residuals(lm_fit))
  print(stats.influence(lm_fit))
  print(stats.influence.measures(lm_fit))
  print(stats.qr.influence(lm_fit.qr, stats.residuals.lm(lm_fit)))
  print(stats.lm.influence(lm_fit))
  return 1L
}

print(inspect_model_alias_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_model_alias_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats model-alias helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
