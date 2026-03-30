mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_contrast_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats contrast runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_contrast_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"
import r * as datasets from "datasets"

fn inspect_contrasts() -> int {
  let f = base.factor(c("a", "b", "c"))
  let ct = stats.contr.treatment(3L)
  let cs = stats.contr.sum(3L)
  let ch = stats.contr.helmert(3L)
  let csa = stats.contr.SAS(3L)
  let cp = stats.contr.poly(3L)
  let ctr = stats.contrasts(f)
  let model = stats.glm(stats.as.formula("am ~ mpg"), data = datasets.mtcars, family = stats.binomial())
  let fm = stats.formula(model)
  let rf = stats.reformulate(c("x", "z"), response = "y")
  print(ct)
  print(cs)
  print(ch)
  print(csa)
  print(cp)
  print(ctr)
  print(base.length(fm))
  print(base.length(rf))
  return base.ncol(ct) + base.ncol(ctr)
}

print(inspect_contrasts())
"#;

    let rr_path = out_dir.join("stats_contrast_interop.rr");
    let o0 = out_dir.join("stats_contrast_interop_o0.R");
    let o2 = out_dir.join("stats_contrast_interop_o2.R");

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
fn stats_contrast_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as base from "base"
import r * as datasets from "datasets"

fn inspect_contrast_helpers() -> int {
  let f = base.factor(c("a", "b", "c"))
  let ct = stats.contr.treatment(3L)
  let cs = stats.contr.sum(3L)
  let ch = stats.contr.helmert(3L)
  let csa = stats.contr.SAS(3L)
  let cp = stats.contr.poly(3L)
  let ctr = stats.contrasts(f)
  let model = stats.glm(stats.as.formula("am ~ mpg"), data = datasets.mtcars, family = stats.binomial())
  let fm = stats.formula(model)
  let rf = stats.reformulate(c("x", "z"), response = "y")
  print(ct)
  print(cs)
  print(ch)
  print(csa)
  print(cp)
  print(ctr)
  print(fm)
  print(rf)
  return 1L
}

print(inspect_contrast_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_contrast_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats contrast helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
