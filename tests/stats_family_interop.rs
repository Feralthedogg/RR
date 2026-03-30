mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_family_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats family runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_family_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"

fn inspect_family() -> float {
  let fit = stats.glm(stats.as.formula("am ~ mpg"), data = datasets.mtcars, family = stats.binomial())
  let fam = stats.family(fit)
  let lnk = stats.make.link("logit")
  let q = stats.quasi()
  let qb = stats.quasibinomial()
  let qp = stats.quasipoisson()
  let ig = stats.inverse.gaussian()
  print(fam.family)
  print(fam.link)
  print(fam.dispersion)
  print(lnk.name)
  print(q.family)
  print(q.link)
  print(q.varfun)
  print(q.dispersion)
  print(qb.family)
  print(qb.link)
  print(qb.dispersion)
  print(qp.family)
  print(qp.link)
  print(qp.dispersion)
  print(ig.family)
  print(ig.link)
  print(ig.dispersion)
  return fam.dispersion + q.dispersion + qb.dispersion + qp.dispersion + ig.dispersion
}

print(inspect_family())
"#;

    let rr_path = out_dir.join("stats_family_interop.rr");
    let o0 = out_dir.join("stats_family_interop_o0.R");
    let o2 = out_dir.join("stats_family_interop_o2.R");

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
fn stats_family_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"

fn inspect_family_helpers() -> int {
  let fit = stats.glm(stats.as.formula("am ~ mpg"), data = datasets.mtcars, family = stats.binomial())
  let fam = stats.family(fit)
  let lnk = stats.make.link("logit")
  let q = stats.quasi()
  let qb = stats.quasibinomial()
  let qp = stats.quasipoisson()
  let ig = stats.inverse.gaussian()
  print(fit)
  print(fam)
  print(lnk)
  print(q)
  print(qb)
  print(qp)
  print(ig)
  return 1L
}

print(inspect_family_helpers())
"#;

    let (ok, stdout, stderr) = run_compile_case("stats_family_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats family helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
