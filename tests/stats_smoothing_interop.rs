mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_smoothing_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats smoothing runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_smoothing_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_smoothing() -> float {
  let xs = c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0)
  let ys = c(1.0, 2.0, 1.5, 3.0, 2.5, 3.5, 4.0, 4.5, 5.0, 5.5)
  let a = stats.approx(xs, ys)
  let k = stats.ksmooth(xs, ys, kernel = "normal", bandwidth = 1.0)
  let l = stats.lowess(xs, ys)
  let ls = stats.loess.smooth(xs, ys)
  let s = stats.spline(xs, ys)
  let ss = stats.smooth.spline(xs, ys)
  let sm = stats.supsmu(xs, ys)
  let lo = stats.loess(stats.as.formula("y ~ x"), data = base.data.frame(x = xs, y = ys))
  let lc = stats.loess.control()
  print(a.x)
  print(a.y)
  print(k.x)
  print(k.y)
  print(l.x)
  print(l.y)
  print(ls.x)
  print(ls.y)
  print(s.x)
  print(s.y)
  print(ss.df)
  print(ss.lambda)
  print(length(ss.x))
  print(length(ss.y))
  print(sm.x)
  print(sm.y)
  print(lo.n)
  print(lo.fitted)
  print(lo.s)
  print(lo.divisor)
  print(lo.xnames)
  print(length(lo.weights))
  print(lc.surface)
  print(lc.statistics)
  print(lc.cell)
  print(lc.iterations)
  print(lc.iterTrace)
  return ss.df + ss.lambda + lo.s + lo.divisor + lc.cell
}

print(inspect_smoothing())
"#;

    let rr_path = out_dir.join("stats_smoothing_interop.rr");
    let o0 = out_dir.join("stats_smoothing_interop_o0.R");
    let o2 = out_dir.join("stats_smoothing_interop_o2.R");

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
fn stats_smoothing_function_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_fns() -> int {
  let xs = c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0)
  let ys = c(1.0, 2.0, 1.5, 3.0, 2.5, 3.5, 4.0, 4.5, 5.0, 5.5)
  let af = stats.approxfun(xs, ys)
  let k = stats.ksmooth(xs, ys, kernel = "normal", bandwidth = 1.0)
  let ls = stats.loess.smooth(xs, ys)
  let lo = stats.loess(stats.as.formula("y ~ x"), data = base.data.frame(x = xs, y = ys))
  let lc = stats.loess.control()
  let sf = stats.splinefun(xs, ys)
  let sm = stats.supsmu(xs, ys)
  print(af)
  print(k)
  print(ls)
  print(lo)
  print(lc)
  print(sf)
  print(sm)
  return 1L
}

print(inspect_fns())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_smoothing_functions", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats smoothing function helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
