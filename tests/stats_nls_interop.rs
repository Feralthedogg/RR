mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_nls_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats nls runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("stats_nls_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"
import r * as datasets from "datasets"

fn inspect_nls() -> float {
  let df = base.data.frame(x = c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0), y = c(2.4, 3.8, 5.1, 7.0, 9.1, 11.2, 13.1, 15.4))
  let fit = stats.nls(stats.as.formula("y ~ a + b * x"), data = df, start = base.list(a = 1.0, b = 2.0))
  let ctl = stats.nls.control()
  let ini = stats.getInitial(stats.as.formula("rate ~ stats::SSasymp(conc, Asym, R0, lrc)"), data = datasets.Puromycin)
  let cf = stats.coef(fit)
  print(cf)
  print(fit.dataClasses)
  print(length(fit.m))
  print(length(fit.control))
  print(length(fit.convInfo))
  print(ctl.maxiter)
  print(ctl.tol)
  print(ctl.printEval)
  print(ctl.warnOnly)
  print(ctl.scaleOffset)
  print(ctl.nDcentral)
  print(ini)
  return cf[1L] + cf[2L] + ini[1L]
}

print(inspect_nls())
"#;

    let rr_path = out_dir.join("stats_nls_interop.rr");
    let o0 = out_dir.join("stats_nls_interop_o0.R");
    let o2 = out_dir.join("stats_nls_interop_o2.R");

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
fn stats_nls_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as base from "base"
import r * as datasets from "datasets"

fn inspect_nls_helpers() -> int {
  let df = base.data.frame(x = c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0), y = c(2.4, 3.8, 5.1, 7.0, 9.1, 11.2, 13.1, 15.4))
  let fit = stats.nls(stats.as.formula("y ~ a + b * x"), data = df, start = base.list(a = 1.0, b = 2.0))
  let ctl = stats.nls.control()
  let ini = stats.getInitial(stats.as.formula("rate ~ stats::SSasymp(conc, Asym, R0, lrc)"), data = datasets.Puromycin)
  print(fit)
  print(ctl)
  print(ini)
  return 1L
}

print(inspect_nls_helpers())
"#;

    let (ok, stdout, stderr) = run_compile_case("stats_nls_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats nls helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
