mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_selfstart_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats selfStart runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_selfstart_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"

fn inspect_selfstart() -> float {
  let x = c(1.0, 2.0, 3.0, 4.0)
  let a1 = stats.SSasymp(x, 10.0, 1.0, -1.0)
  let a2 = stats.SSasympOff(x, 10.0, -1.0, 1.0)
  let a3 = stats.SSasympOrig(x, 10.0, -1.0)
  let b = stats.SSbiexp(x, 10.0, -0.5, 5.0, -0.1)
  let fol = stats.SSfol(x, 10.0, -1.0, -0.5, 0.2)
  let fpl = stats.SSfpl(x, 10.0, 1.0, 2.0, 1.0)
  let gom = stats.SSgompertz(x, 10.0, 2.0, 1.0)
  let logis = stats.SSlogis(x, 10.0, 2.0, 1.0)
  let mic = stats.SSmicmen(x, 10.0, 2.0)
  let wei = stats.SSweibull(x, 10.0, 2.0, -1.0, 1.5)
  print(a1)
  print(a2)
  print(a3)
  print(b)
  print(fol)
  print(fpl)
  print(gom)
  print(logis)
  print(mic)
  print(wei)
  return a1[1L] + a2[1L] + a3[1L] + b[1L] + fol[1L] + fpl[1L] + gom[1L] + logis[1L] + mic[1L] + wei[1L]
}

print(inspect_selfstart())
"#;

    let rr_path = out_dir.join("stats_selfstart_interop.rr");
    let o0 = out_dir.join("stats_selfstart_interop_o0.R");
    let o2 = out_dir.join("stats_selfstart_interop_o2.R");

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
fn stats_selfstart_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"

fn inspect_selfstart_helpers() -> int {
  let x = c(1.0, 2.0, 3.0, 4.0)
  print(stats.SSasymp(x, 10.0, 1.0, -1.0))
  print(stats.SSasympOff(x, 10.0, -1.0, 1.0))
  print(stats.SSasympOrig(x, 10.0, -1.0))
  print(stats.SSbiexp(x, 10.0, -0.5, 5.0, -0.1))
  print(stats.SSfol(x, 10.0, -1.0, -0.5, 0.2))
  print(stats.SSfpl(x, 10.0, 1.0, 2.0, 1.0))
  print(stats.SSgompertz(x, 10.0, 2.0, 1.0))
  print(stats.SSlogis(x, 10.0, 2.0, 1.0))
  print(stats.SSmicmen(x, 10.0, 2.0))
  print(stats.SSweibull(x, 10.0, 2.0, -1.0, 1.5))
  return 1L
}

print(inspect_selfstart_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_selfstart_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats selfStart helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
