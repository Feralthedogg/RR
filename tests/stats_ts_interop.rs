mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_ts_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats ts runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("stats_ts_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"

fn inspect_ts() -> float {
  let x = stats.ts(c(1.0, 2.0, 3.0, 4.0), start = c(2000.0, 2.0), frequency = 4.0)
  let y = stats.ts(c(4.0, 5.0, 6.0, 7.0), start = c(2000.0, 2.0), frequency = 4.0)
  let xa = stats.as.ts(c(1.0, 2.0, 3.0, 4.0))
  let x2 = stats.window(x)
  let x3 = stats.lag(x, 1.0)
  let ti = stats.ts.intersect(x, y)
  let tu = stats.ts.union(x, y)
  let freq = stats.frequency(x)
  let t = stats.time(x)
  let cyc = stats.cycle(x)
  let its = stats.is.ts(x)
  let imts = stats.is.mts(x)
  let h = stats.hasTsp(x)
  let meta = stats.tsp(x)
  let s = stats.start(x)
  let e = stats.end(x)
  let dt = stats.deltat(x)
  let emb = stats.embed(c(1.0, 2.0, 3.0, 4.0), 2.0)
  print(x)
  print(xa)
  print(x2)
  print(x3)
  print(dim(ti))
  print(dim(tu))
  print(freq)
  print(t)
  print(cyc)
  print(its)
  print(imts)
  print(h)
  print(meta)
  print(s)
  print(e)
  print(dt)
  print(emb)
  return freq + dt + meta[1L] + s[1L] + e[1L] + ti[1L, 1L] + tu[1L, 2L]
}

print(inspect_ts())
"#;

    let rr_path = out_dir.join("stats_ts_interop.rr");
    let o0 = out_dir.join("stats_ts_interop_o0.R");
    let o2 = out_dir.join("stats_ts_interop_o2.R");

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
fn stats_ts_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"

fn inspect_ts() -> int {
  let x = stats.ts(c(1.0, 2.0, 3.0, 4.0), start = c(2000.0, 2.0), frequency = 4.0)
  let y = stats.ts(c(4.0, 5.0, 6.0, 7.0), start = c(2000.0, 2.0), frequency = 4.0)
  let xa = stats.as.ts(c(1.0, 2.0, 3.0, 4.0))
  let x2 = stats.window(x)
  let x3 = stats.lag(x, 1.0)
  let ti = stats.ts.intersect(x, y)
  let tu = stats.ts.union(x, y)
  let freq = stats.frequency(x)
  let t = stats.time(x)
  let cyc = stats.cycle(x)
  let its = stats.is.ts(x)
  let imts = stats.is.mts(x)
  let h = stats.hasTsp(x)
  let meta = stats.tsp(x)
  let s = stats.start(x)
  let e = stats.end(x)
  let dt = stats.deltat(x)
  let emb = stats.embed(c(1.0, 2.0, 3.0, 4.0), 2.0)
  print(xa)
  print(x2)
  print(x3)
  print(ti)
  print(tu)
  print(freq)
  print(t)
  print(cyc)
  print(its)
  print(imts)
  print(h)
  print(meta)
  print(s)
  print(e)
  print(dt)
  print(emb)
  return 1L
}

print(inspect_ts())
"#;

    let (ok, stdout, stderr) = run_compile_case("stats_ts_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats ts helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
