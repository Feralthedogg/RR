mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_structure_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats structure runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_structure_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"
import r * as base from "base"

fn inspect_structure() -> float {
  let mp = stats.medpolish(base.matrix(c(1.0, 2.0, 3.0, 4.0), nrow = 2L), false)
  let sm = stats.symnum(base.matrix(c(1.0, 0.5, 0.2, 0.5), ncol = 2L))
  let rp = stats.replications(stats.as.formula("yield ~ block + N*P*K"), datasets.npk)
  let df = base.data.frame(
    id = c(1L, 1L, 2L, 2L),
    time = c(1L, 2L, 1L, 2L),
    y = c(10.0, 11.0, 20.0, 21.0)
  )
  let rs = stats.reshape(df, idvar = "id", timevar = "time", direction = "wide")
  print(mp.overall)
  print(mp.row)
  print(mp.col)
  print(mp.residuals)
  print(mp.name)
  print(sm)
  print(dim(sm))
  print(rp)
  print(rs)
  return mp.overall + rp[1L]
}

print(inspect_structure())
"#;

    let rr_path = out_dir.join("stats_structure_interop.rr");
    let o0 = out_dir.join("stats_structure_interop_o0.R");
    let o2 = out_dir.join("stats_structure_interop_o2.R");

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
fn stats_structure_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as datasets from "datasets"
import r * as base from "base"

fn inspect_structure() -> int {
  print(stats.medpolish(base.matrix(c(1.0, 2.0, 3.0, 4.0), nrow = 2L), false))
  print(stats.symnum(base.matrix(c(1.0, 0.5, 0.2, 0.5), ncol = 2L)))
  print(stats.replications(stats.as.formula("yield ~ block + N*P*K"), datasets.npk))
  let df = base.data.frame(
    id = c(1L, 1L, 2L, 2L),
    time = c(1L, 2L, 1L, 2L),
    y = c(10.0, 11.0, 20.0, 21.0)
  )
  print(stats.reshape(df, idvar = "id", timevar = "time", direction = "wide"))
  return 1L
}

print(inspect_structure())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_structure_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats structure helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
