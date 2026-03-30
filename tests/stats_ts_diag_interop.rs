mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_ts_diag_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats ts-diag runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_ts_diag_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let png_path = out_dir.join("stats_tsdiag.png");

    let src = format!(
        r#"
import r default from "stats"
import r default from "grDevices"

fn inspect_ts_diag() -> float {{
  let x = stats.ts(c(1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0), frequency = 4L)
  let fit0 = stats.arima0(x, order = c(1L, 0L, 0L))
  let outfile = "{png}"
  grDevices.png(filename = outfile, width = 400.0, height = 300.0)
  let d = stats.tsdiag(fit0, 5L)
  grDevices.dev.off()
  print(fit0.coef)
  print(fit0.sigma2)
  print(fit0.loglik)
  print(fit0.aic)
  print(fit0.arma)
  print(fit0.residuals)
  print(fit0.code)
  print(d)
  return fit0.sigma2 + fit0.loglik + fit0.aic
}}

print(inspect_ts_diag())
"#,
        png = png_path.display()
    );

    let rr_path = out_dir.join("stats_ts_diag_interop.rr");
    let o0 = out_dir.join("stats_ts_diag_interop_o0.R");
    let o2 = out_dir.join("stats_ts_diag_interop_o2.R");

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
fn stats_ts_diag_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"

fn inspect_ts_diag_helpers() -> int {
  let x = stats.ts(c(1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0), frequency = 4L)
  let fit0 = stats.arima0(x, order = c(1L, 0L, 0L))
  let d = stats.tsdiag(fit0, 5L)
  print(fit0)
  print(d)
  return 1L
}

print(inspect_ts_diag_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_ts_diag_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats ts-diag helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
