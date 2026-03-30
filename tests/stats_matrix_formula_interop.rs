mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_matrix_formula_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats matrix/formula runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_matrix_formula_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_matrix_formula() -> float {
  let x = c(1.0, 2.0, 3.0)
  let y = c(3.0, 2.0, 1.0)
  let tp = stats.toeplitz(x)
  let tp2 = stats.toeplitz2(x, ncol = 2L)
  let di = stats.diffinv(x)
  let pm = stats.polym(x, y)
  let os = stats.asOneSidedFormula(c("x", "z"))
  let vn = stats.variable.names(stats.as.formula("y ~ x + z"))
  print(base.dim(tp))
  print(base.dim(tp2))
  print(di)
  print(base.dim(pm))
  print(os)
  print(vn)
  return base.length(tp) + base.length(tp2) + di[1L] + base.length(pm)
}

print(inspect_matrix_formula())
"#;

    let rr_path = out_dir.join("stats_matrix_formula_interop.rr");
    let o0 = out_dir.join("stats_matrix_formula_interop_o0.R");
    let o2 = out_dir.join("stats_matrix_formula_interop_o2.R");

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
fn stats_matrix_formula_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"

fn inspect_matrix_formula_helpers() -> int {
  let x = c(1.0, 2.0, 3.0)
  let y = c(3.0, 2.0, 1.0)
  print(stats.toeplitz(x))
  print(stats.toeplitz2(x, ncol = 2L))
  print(stats.diffinv(x))
  print(stats.polym(x, y))
  print(stats.asOneSidedFormula(c("x", "z")))
  print(stats.variable.names(stats.as.formula("y ~ x + z")))
  return 1L
}

print(inspect_matrix_formula_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_matrix_formula_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats matrix/formula helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
