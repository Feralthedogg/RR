mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_model_selection_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats model-selection runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_model_selection_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_selection() -> float {
  let train = base.data.frame(y = c(1.0, 2.0, 4.0, 8.0, 16.0, 32.0), x = c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), z = c(2.0, 1.0, 3.0, 2.0, 4.0, 3.0))
  let fit0 = stats.lm(stats.as.formula("y ~ 1"), data = train)
  let fit1 = stats.lm(stats.as.formula("y ~ x + z"), data = train)
  let a1 = stats.add1(fit0, scope = stats.as.formula("~ x + z"))
  let d1 = stats.drop1(fit1, test = "F")
  let tr1 = stats.terms(fit1)
  let tr2 = stats.terms(stats.as.formula("~ x + z + qsec"))
  let ea = stats.extractAIC(fit1)
  let dc = stats.dummy.coef(fit1)
  let dclm = stats.dummy.coef.lm(fit1)
  let eff = stats.effects(fit1)
  let st = stats.step(fit1, trace = 0.0)
  let sm = base.summary(st)
  let tr = stats.terms(st)
  let mf = stats.model.frame(st)
  print(a1)
  print(d1)
  print(ea)
  print(dc)
  print(dclm)
  print(eff)
  print(st.rank)
  print(st.coefficients)
  print(sm.sigma)
  print(tr.order)
  print(mf.x)
  print(mf.y)
  return st.rank + sm.sigma
}

print(inspect_selection())
"#;

    let rr_path = out_dir.join("stats_model_selection_interop.rr");
    let o0 = out_dir.join("stats_model_selection_interop_o0.R");
    let o2 = out_dir.join("stats_model_selection_interop_o2.R");

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
fn stats_model_selection_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_selection_helpers() -> int {
  let train = base.data.frame(y = c(1.0, 2.0, 4.0, 8.0, 16.0, 32.0), x = c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), z = c(2.0, 1.0, 3.0, 2.0, 4.0, 3.0))
  let fit0 = stats.lm(stats.as.formula("y ~ 1"), data = train)
  let fit1 = stats.lm(stats.as.formula("y ~ x + z"), data = train)
  let a1 = stats.add1(fit0, scope = stats.as.formula("~ x + z"))
  let d1 = stats.drop1(fit1, test = "F")
  let tr1 = stats.terms(fit1)
  let tr2 = stats.terms(stats.as.formula("~ x + z + qsec"))
  let ea = stats.extractAIC(fit1)
  let asc = stats.add.scope(tr1, tr2)
  let dsc = stats.drop.scope(tr1, tr2)
  let fsc = stats.factor.scope(tr1.factors, list(add = tr2.factors))
  let dc = stats.dummy.coef(fit1)
  let dclm = stats.dummy.coef.lm(fit1)
  let eff = stats.effects(fit1)
  let st = stats.step(fit1, trace = 0.0)
  let sm = base.summary(st)
  let tr = stats.terms(st)
  let mf = stats.model.frame(st)
  print(a1)
  print(d1)
  print(ea)
  print(asc)
  print(dsc)
  print(fsc)
  print(dc)
  print(dclm)
  print(eff)
  print(st)
  print(sm)
  print(tr)
  print(mf)
  return 1L
}

print(inspect_selection_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_model_selection_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats model-selection helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
