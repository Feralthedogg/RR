mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_density_bw_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats density/bw runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_density_bw_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r * as base from "base"
import r default from "stats"

fn inspect_density_bw() -> float {
  let x = c(0.1, 0.3, 0.6, 1.0, 1.5, 2.1, 2.8, 3.6, 4.5, 5.5, 6.6, 7.8, 9.1, 10.5, 12.0, 13.6, 15.3, 17.1, 19.0, 21.0)
  let d = stats.density.default(x)
  let b1 = stats.bw.nrd(x)
  let b2 = stats.bw.nrd0(x)
  print(d.x)
  print(d.y)
  print(d.bw)
  print(d.n)
  print(base.getElement(d, "old.coords"))
  print(base.getElement(d, "data.name"))
  print(base.getElement(d, "has.na"))
  print(b1)
  print(b2)
  return d.bw + b1 + b2
}

print(inspect_density_bw())
"#;

    let rr_path = out_dir.join("stats_density_bw_interop.rr");
    let o0 = out_dir.join("stats_density_bw_interop_o0.R");
    let o2 = out_dir.join("stats_density_bw_interop_o2.R");

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
fn stats_density_bw_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"

fn inspect_density_bw_helpers() -> int {
  let x = c(0.1, 0.3, 0.6, 1.0, 1.5, 2.1, 2.8, 3.6, 4.5, 5.5, 6.6, 7.8, 9.1, 10.5, 12.0, 13.6, 15.3, 17.1, 19.0, 21.0)
  let d = stats.density.default(x)
  let b1 = stats.bw.nrd(x)
  let b2 = stats.bw.nrd0(x)
  let b3 = stats.bw.ucv(x)
  let b4 = stats.bw.bcv(x)
  let b5 = stats.bw.SJ(x)
  print(d)
  print(b1)
  print(b2)
  print(b3)
  print(b4)
  print(b5)
  return 1L
}

print(inspect_density_bw_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_density_bw_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats density/bw helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
