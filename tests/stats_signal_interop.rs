mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_signal_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats signal runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_signal_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r { png, dev.off } from "grDevices"

fn inspect_signal() -> float {
  let x = stats.ts(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0), frequency = 4.0)
  let wm = stats.weighted.mean(c(1.0, 2.0, 3.0), c(1.0, 1.0, 2.0))
  let rm = stats.runmed(c(1.0, 3.0, 2.0, 4.0, 5.0), 3L)
  let filt = stats.filter(x, c(0.2, 0.6, 0.2))
  let dec = stats.decompose(x)
  let spec = stats.spectrum(x, plot = false)
  let spg = stats.spec.pgram(x, plot = false)
  let spt = stats.spec.taper(c(1.0, 2.0, 3.0, 4.0))
  let st = stats.stl(stats.ts(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0), frequency = 4.0), "periodic")
  png("stats_signal_spec_plot.png")
  stats.plot.spec.coherency(stats.spec.pgram(matrix(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0), ncol = 2L), plot = false))
  stats.plot.spec.phase(stats.spec.pgram(matrix(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0), ncol = 2L), plot = false))
  dev.off()
  print(wm)
  print(rm)
  print(filt)
  print(dec.x)
  print(dec.trend)
  print(dec.type)
  print(spec.freq)
  print(spec.spec)
  print(spec.method)
  print(spg.freq)
  print(spg.spec)
  print(spg.df)
  print(spg.method)
  print(spt)
  print(st.weights)
  print(st.win)
  print(st.inner)
  print(st.outer)
  return wm + spec.df + spec.bandwidth + spg.df + st.inner + st.outer
}

print(inspect_signal())
"#;

    let rr_path = out_dir.join("stats_signal_interop.rr");
    let o0 = out_dir.join("stats_signal_interop_o0.R");
    let o2 = out_dir.join("stats_signal_interop_o2.R");

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
fn stats_signal_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r { png, dev.off } from "grDevices"

fn inspect_signal() -> int {
  let x = stats.ts(c(1.0, 2.0, 3.0, 4.0), frequency = 4.0)
  let wm = stats.weighted.mean(c(1.0, 2.0, 3.0), c(1.0, 1.0, 2.0))
  let rm = stats.runmed(c(1.0, 3.0, 2.0, 4.0, 5.0), 3L)
  let filt = stats.filter(x, c(0.2, 0.6, 0.2))
  let dec = stats.decompose(stats.ts(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0)))
  let spec = stats.spectrum(x, plot = false)
  let spg = stats.spec.pgram(x, plot = false)
  let spt = stats.spec.taper(c(1.0, 2.0, 3.0, 4.0))
  let st = stats.stl(stats.ts(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0), frequency = 4.0), "periodic")
  png("stats_signal_plot.png")
  print(stats.plot.spec.coherency(stats.spec.pgram(matrix(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0), ncol = 2L), plot = false)))
  print(stats.plot.spec.phase(stats.spec.pgram(matrix(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0), ncol = 2L), plot = false)))
  print(dev.off())
  print(wm)
  print(rm)
  print(filt)
  print(dec)
  print(spec)
  print(spg)
  print(spt)
  print(st)
  return 1L
}

print(inspect_signal())
"#;

    let (ok, stdout, stderr) = run_compile_case("stats_signal_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats signal helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
