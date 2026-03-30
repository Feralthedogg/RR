mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_ts_analysis_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats ts-analysis runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_ts_analysis_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_ts_analysis() -> float {
  let x = stats.ts(c(1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 64.0, 32.0, 16.0, 8.0), frequency = 4L)
  let arf = stats.ar(x)
  let aryw = stats.ar.yw(x)
  let armle = stats.ar.mle(x)
  let arburg = stats.ar.burg(x)
  let arols = stats.ar.ols(x)
  let sim = stats.arima.sim(model = base.list(ar = 0.7), n = 12L)
  let armaacf = stats.ARMAacf(c(0.7, -0.2), c(0.3), 6L)
  let armatoma = stats.ARMAtoMA(c(0.7, -0.2), c(0.3), 6L)
  let sp = stats.spec.ar(x, plot = false)
  print(arf.order)
  print(arf.ar)
  print(arf.aic)
  print(arf.resid)
  print(arf.method)
  print(arf.series)
  print(arf.frequency)
  print(aryw.order)
  print(armle.ar)
  print(arburg.order)
  print(arols.order)
  print(arols.method)
  print(length(sim))
  print(stats.frequency(sim))
  print(armaacf)
  print(armatoma)
  print(sp.freq)
  print(sp.spec)
  print(sp.series)
  print(sp.method)
  return arf.order + arf.frequency + aryw.order + armle.order + arburg.order + arols.order + stats.frequency(sim)
}

print(inspect_ts_analysis())
"#;

    let rr_path = out_dir.join("stats_ts_analysis_interop.rr");
    let o0 = out_dir.join("stats_ts_analysis_interop_o0.R");
    let o2 = out_dir.join("stats_ts_analysis_interop_o2.R");

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
fn stats_ts_analysis_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_ts_analysis_helpers() -> int {
  let x = stats.ts(c(1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 64.0, 32.0, 16.0, 8.0), frequency = 4L)
  let arf = stats.ar(x)
  let aryw = stats.ar.yw(x)
  let armle = stats.ar.mle(x)
  let arburg = stats.ar.burg(x)
  let arols = stats.ar.ols(x)
  let sim = stats.arima.sim(model = base.list(ar = 0.7), n = 12L)
  let armaacf = stats.ARMAacf(c(0.7, -0.2), c(0.3), 6L)
  let armatoma = stats.ARMAtoMA(c(0.7, -0.2), c(0.3), 6L)
  let sp = stats.spec.ar(x, plot = false)
  print(arf)
  print(aryw)
  print(armle)
  print(arburg)
  print(arols)
  print(sim)
  print(armaacf)
  print(armatoma)
  print(sp)
  return 1L
}

print(inspect_ts_analysis_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_ts_analysis_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats ts-analysis helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
