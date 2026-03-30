mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_formula_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats formula runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_formula_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"
import r * as base from "base"

fn inspect_formula_helpers() -> float {
  let fm = stats.as.formula("mpg ~ wt + hp")
  let mf = stats.model.frame(fm, data = datasets.mtcars)
  let tr = stats.terms.formula(fm)
  let dr = stats.delete.response(tr)
  let gav = stats.get_all_vars(fm, data = datasets.mtcars)
  let mr = stats.model.response(mf)
  let me = stats.model.extract(mf, "response")
  let cn = stats.case.names(mf)
  let cc = stats.complete.cases(gav)
  let fv = stats.fivenum(mr)
  print(base.length(tr))
  print(base.length(dr))
  print(base.length(gav))
  print(mr)
  print(me)
  print(cn)
  print(cc)
  print(fv)
  return mr[1L] + me[1L] + fv[1L]
}

print(inspect_formula_helpers())
"#;

    let rr_path = out_dir.join("stats_formula_interop.rr");
    let o0 = out_dir.join("stats_formula_interop_o0.R");
    let o2 = out_dir.join("stats_formula_interop_o2.R");

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
fn stats_formula_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"

fn inspect_formula_helpers() -> int {
  let fm = stats.as.formula("mpg ~ wt + hp")
  let mf = stats.model.frame(fm, data = datasets.mtcars)
  let tr = stats.terms.formula(fm)
  let dr = stats.delete.response(tr)
  let gav = stats.get_all_vars(fm, data = datasets.mtcars)
  let mr = stats.model.response(mf)
  let me = stats.model.extract(mf, "response")
  let cn = stats.case.names(mf)
  let cc = stats.complete.cases(gav)
  let fv = stats.fivenum(mr)
  print(tr)
  print(dr)
  print(gav)
  print(mr)
  print(me)
  print(cn)
  print(cc)
  print(fv)
  return 1L
}

print(inspect_formula_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_formula_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats formula helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
