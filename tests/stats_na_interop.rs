mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_na_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats NA runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("stats_na_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"
import r * as base from "base"

fn inspect_na() -> float {
  let ws = rep(1.0, 32L)
  let fit = stats.lm(stats.as.formula("mpg ~ wt + offset(hp)"), data = datasets.mtcars, weights = ws)
  let mf = stats.model.frame(fit)
  let w = stats.weights(fit)
  let mw = stats.model.weights(mf)
  let mo = stats.model.offset(mf)
  let off = stats.offset(c(1.0, 2.0, 3.0))
  let omit = stats.na.omit(c(1.0, 2.0, 3.0))
  let excl = stats.na.exclude(c(1.0, NA, 3.0))
  let pass = stats.na.pass(c(1.0, 2.0, 3.0))
  let fail = stats.na.fail(c(1.0, 2.0, 3.0))
  let act = stats.na.action(excl)
  let pred = stats.napredict(act, c(10.0, 30.0))
  let resid = stats.naresid(act, c(10.0, 30.0))
  let resid_mat = stats.naresid(act, base.matrix(c(1.0, 2.0, 3.0, 4.0), ncol = 2L))
  let msg = stats.naprint(act)
  print(w)
  print(mw)
  print(mo)
  print(off)
  print(omit)
  print(excl)
  print(pass)
  print(fail)
  print(act)
  print(pred)
  print(resid)
  print(base.dim(resid_mat))
  print(msg)
  return w[1L] + mw[1L] + mo[1L] + off[1L] + omit[1L] + pass[1L] + pred[1L] + resid[1L]
}

print(inspect_na())
"#;

    let rr_path = out_dir.join("stats_na_interop.rr");
    let o0 = out_dir.join("stats_na_interop_o0.R");
    let o2 = out_dir.join("stats_na_interop_o2.R");

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
fn stats_na_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"
import r * as base from "base"

fn inspect_na_helpers() -> int {
  let ws = rep(1.0, 32L)
  let fit = stats.lm(stats.as.formula("mpg ~ wt + offset(hp)"), data = datasets.mtcars, weights = ws)
  let mf = stats.model.frame(fit)
  let excl = stats.na.exclude(c(1.0, NA, 3.0))
  let act = stats.na.action(excl)
  print(stats.weights(fit))
  print(stats.model.weights(mf))
  print(stats.model.offset(mf))
  print(stats.offset(c(1.0, 2.0, 3.0)))
  print(stats.na.omit(c(1.0, 2.0, 3.0)))
  print(excl)
  print(stats.na.pass(c(1.0, 2.0, 3.0)))
  print(stats.na.fail(c(1.0, 2.0, 3.0)))
  print(act)
  print(stats.napredict(act, c(10.0, 30.0)))
  print(stats.naresid(act, c(10.0, 30.0)))
  print(stats.naresid(act, base.matrix(c(1.0, 2.0, 3.0, 4.0), ncol = 2L)))
  print(stats.naprint(act))
  return 1L
}

print(inspect_na_helpers())
"#;

    let (ok, stdout, stderr) = run_compile_case("stats_na_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats NA helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
