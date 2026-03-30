mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_model_plumbing_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats model-plumbing runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_model_plumbing_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"
import r * as base from "base"

fn inspect_plumbing() -> float {
  let fm = stats.as.formula("mpg ~ wt + hp")
  let fit = stats.lm(fm, data = datasets.mtcars)
  let mfd = stats.model.frame.default(fm, data = datasets.mtcars)
  let mmd = stats.model.matrix.default(fm, data = datasets.mtcars)
  let mmlm = stats.model.matrix.lm(fit)
  let upd = stats.update.default(fit, stats.as.formula(". ~ ."))
  let upf = stats.update.formula(fm, stats.as.formula(". ~ . + qsec"))
  let upm = stats.model.matrix.default(upf, data = datasets.mtcars)
  let gc = stats.glm.control()
  let dt = stats.drop.terms(stats.terms(fit), 2L, true)
  let empty = stats.lm(stats.as.formula("mpg ~ 1"), data = datasets.mtcars)
  let ie = stats.is.empty.model(stats.terms(empty))
  print(mfd.mpg)
  print(base.dim(mmd))
  print(base.dim(mmlm))
  print(upd.rank)
  print(base.dim(upm))
  print(gc.epsilon)
  print(gc.maxit)
  print(gc.trace)
  print(base.length(dt))
  print(ie)
  return upd.rank + gc.maxit + base.length(upm)
}

print(inspect_plumbing())
"#;

    let rr_path = out_dir.join("stats_model_plumbing_interop.rr");
    let o0 = out_dir.join("stats_model_plumbing_interop_o0.R");
    let o2 = out_dir.join("stats_model_plumbing_interop_o2.R");

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
fn stats_model_plumbing_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"
import r * as base from "base"

fn inspect_plumbing_helpers() -> int {
  let fm = stats.as.formula("mpg ~ wt + hp")
  let fit = stats.lm(fm, data = datasets.mtcars)
  print(stats.model.frame.default(fm, data = datasets.mtcars))
  print(stats.model.matrix.default(fm, data = datasets.mtcars))
  print(stats.model.matrix.lm(fit))
  print(stats.update.default(fit, stats.as.formula(". ~ .")))
  print(stats.update.formula(fm, stats.as.formula(". ~ . + qsec")))
  print(stats.glm.control())
  print(stats.drop.terms(stats.terms(fit), 2L, true))
  print(stats.is.empty.model(stats.terms(stats.lm(stats.as.formula("mpg ~ 1"), data = datasets.mtcars))))
  return 1L
}

print(inspect_plumbing_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_model_plumbing_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats model-plumbing helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
