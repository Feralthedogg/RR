mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_symbolic_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats symbolic runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_symbolic_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"

fn logis_model(input, Asym, xmid, scal) -> float {
  return Asym / (1.0 + exp((xmid - input) / scal))
}

fn initfun(mCall, LHS, data) {
  return c(10.0, 2.0, 1.0)
}

fn inspect_symbolic() -> float {
  let fm = stats.as.formula("~ x^2 + y")
  let names = c("x", "y")
  let d = stats.deriv(fm, names)
  let d3 = stats.deriv3(fm, names)
  let ss = stats.selfStart(logis_model, initfun, c("Asym", "xmid", "scal"))
  print(d)
  print(d3)
  print(ss(1.0, 10.0, 2.0, 1.0))
  return ss(1.0, 10.0, 2.0, 1.0)
}

print(inspect_symbolic())
"#;

    let rr_path = out_dir.join("stats_symbolic_interop.rr");
    let o0 = out_dir.join("stats_symbolic_interop_o0.R");
    let o2 = out_dir.join("stats_symbolic_interop_o2.R");

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
fn stats_symbolic_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"

fn logis_model(input, Asym, xmid, scal) -> float {
  return Asym / (1.0 + exp((xmid - input) / scal))
}

fn initfun(mCall, LHS, data) {
  return c(10.0, 2.0, 1.0)
}

fn inspect_symbolic_helpers() -> int {
  let fm = stats.as.formula("~ x^2 + y")
  let names = c("x", "y")
  let d = stats.deriv(fm, names)
  let d3 = stats.deriv3(fm, names)
  let ss = stats.selfStart(logis_model, initfun, c("Asym", "xmid", "scal"))
  let nd = stats.numericDeriv(d, names)
  print(d)
  print(d3)
  print(ss)
  print(nd)
  return 1L
}

print(inspect_symbolic_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_symbolic_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats symbolic helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
