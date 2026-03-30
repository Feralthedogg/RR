mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats4_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats4 direct interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats4_direct_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats4"
import r * as grDevices from "grDevices"

fn nll(mu: float) -> float {
  let xs = c(1.0, 2.0, 3.0)
  let diff = xs - mu
  return sum(diff * diff)
}

fn fit_stats4() -> float {
  let fit = stats4.mle(nll, start = list(mu = 0.0))
  let cf = stats4.coef(fit)
  let vc = stats4.vcov(fit)
  let ci = stats4.confint(fit)
  let ll = stats4.logLik(fit)
  let aic = stats4.AIC(fit)
  let bic = stats4.BIC(fit)
  let n = stats4.nobs(fit)
  let updated = stats4.update(fit, start = list(mu = 1.0))
  let summary = stats4.summary(fit)
  let profile = stats4.profile(fit)
  let outfile = "stats4_profile_plot.pdf"
  grDevices.pdf(outfile)
  let plotted = stats4.plot(profile)
  grDevices.dev.off()
  let shown = stats4.show(fit)
  print(cf)
  print(vc)
  print(ci)
  print(ll)
  print(n)
  print(stats4.coef(updated))
  print(summary)
  print(profile)
  print(plotted)
  print(shown)
  return aic + bic
}

print(fit_stats4())
"#;

    let rr_path = out_dir.join("stats4_direct_interop.rr");
    let o0 = out_dir.join("stats4_direct_interop_o0.R");
    let o2 = out_dir.join("stats4_direct_interop_o2.R");

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

    let pdf_path = std::env::current_dir()
        .expect("cwd")
        .join("stats4_profile_plot.pdf");
    let meta = fs::metadata(&pdf_path).expect("expected stats4 PDF output");
    assert!(meta.len() > 0, "expected non-empty stats4 PDF output");
}
