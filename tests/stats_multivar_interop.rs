mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_multivar_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats multivar runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_multivar_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let png_path = out_dir.join("stats_heatmap.png");

    let src = format!(
        r#"
import r default from "stats"
import r default from "grDevices"
import r * as base from "base"
import r * as datasets from "datasets"

fn inspect_multivar() -> float {{
  let fa = stats.factanal(datasets.USArrests, factors = 1L)
  let mat = base.matrix(c(1.0, 2.0, 3.0, 4.0), nrow = 2L)
  let outfile = "{png}"
  grDevices.png(filename = outfile, width = 400.0, height = 300.0)
  let hm = stats.heatmap(mat)
  grDevices.dev.off()
  print(fa.converged)
  print(fa.factors)
  print(fa.method)
  print(fa.STATISTIC)
  print(fa.PVAL)
  print(length(fa.loadings))
  print(length(fa.uniquenesses))
  print(dim(fa.correlation))
  print(hm.rowInd)
  print(hm.colInd)
  print(length(hm.Rowv))
  print(length(hm.Colv))
  return fa.STATISTIC + fa.PVAL + fa.factors
}}

print(inspect_multivar())
"#,
        png = png_path.display()
    );

    let rr_path = out_dir.join("stats_multivar_interop.rr");
    let o0 = out_dir.join("stats_multivar_interop_o0.R");
    let o2 = out_dir.join("stats_multivar_interop_o2.R");

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
fn stats_multivar_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as base from "base"
import r * as datasets from "datasets"

fn inspect_multivar_helpers() -> int {
  let fa = stats.factanal(datasets.USArrests, factors = 1L)
  let hm = stats.heatmap(base.matrix(c(1.0, 2.0, 3.0, 4.0), nrow = 2L))
  print(fa)
  print(hm)
  return 1L
}

print(inspect_multivar_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_multivar_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats multivar helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
