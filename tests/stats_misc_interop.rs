mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_misc_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats misc runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("stats_misc_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"
import r * as datasets from "datasets"
import r { png, dev.off } from "grDevices"

fn inspect_misc() -> float {
  let adj = stats.p.adjust(c(0.01, 0.2, 0.5))
  let adj_scalar = stats.p.adjust(0.01)
  let pts = stats.ppoints(4L)
  let den = stats.density(c(1.0, 2.0, 3.0))
  png("stats_misc_qqnorm.png")
  let qq = stats.qqnorm(c(1.0, 2.0, 3.0))
  let qqp = stats.qqplot(c(1.0, 2.0, 3.0, 4.0), c(1.0, 2.0, 4.0, 8.0))
  stats.interaction.plot(
    c("A", "A", "A", "A", "A", "B", "B", "B", "B", "B"),
    c("x1", "x2", "x3", "x4", "x5", "x1", "x2", "x3", "x4", "x5"),
    c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0)
  )
  stats.lag.plot(datasets.AirPassengers)
  stats.monthplot(datasets.AirPassengers)
  stats.scatter.smooth(
    c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0),
    c(1.0, 4.0, 9.0, 16.0, 25.0, 36.0, 49.0, 64.0, 81.0, 100.0),
    span = 0.75
  )
  stats.biplot(stats.prcomp(base.matrix(c(1.0, 2.0, 3.0, 4.0), ncol = 2L)))
  dev.off()
  let d = stats.dist(c(1.0, 2.0, 3.0))
  let cov_xy = stats.cov(c(1.0, 2.0, 3.0), c(1.0, 2.0, 4.0))
  let cor_xy = stats.cor(c(1.0, 2.0, 3.0), c(1.0, 2.0, 4.0))
  let var_m = stats.var(base.matrix(c(1.0, 2.0, 3.0, 4.0), ncol = 2L))
  let iqr_v = stats.IQR(c(1.0, 2.0, 3.0, 4.0))
  let mad_v = stats.mad(c(1.0, 2.0, 3.0, 4.0))
  let poly = stats.poly(c(1.0, 2.0, 3.0), 2L)
  let pc = stats.prcomp(base.matrix(c(1.0, 2.0, 3.0, 4.0), ncol = 2L))
  print(adj)
  print(pts)
  print(den.bw)
  print(den.n)
  print(length(den.x))
  print(length(qq.x))
  print(length(qqp.x))
  print(length(d))
  print(cov_xy)
  print(cor_xy)
  print(base.dim(var_m))
  print(iqr_v)
  print(mad_v)
  print(dim(poly))
  print(pc.sdev)
  print(dim(pc.rotation))
  print(dim(pc.x))
  return adj_scalar + den.bw + cov_xy + cor_xy + iqr_v + mad_v
}

print(inspect_misc())
"#;

    let rr_path = out_dir.join("stats_misc_interop.rr");
    let o0 = out_dir.join("stats_misc_interop_o0.R");
    let o2 = out_dir.join("stats_misc_interop_o2.R");

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
fn stats_misc_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as base from "base"
import r * as datasets from "datasets"
import r { png, dev.off } from "grDevices"
import r { plot } from "graphics"

fn helpers() -> int {
  let f = stats.ecdf(c(1.0, 2.0, 3.0))
  print(f)
  png("stats_misc_qqline.png")
  let qqp = stats.qqplot(c(1.0, 2.0, 3.0, 4.0), c(1.0, 2.0, 4.0, 8.0))
  print(qqp)
  plot(c(1.0, 2.0, 3.0))
  stats.qqline(c(1.0, 2.0, 3.0))
  stats.interaction.plot(
    c("A", "A", "A", "A", "A", "B", "B", "B", "B", "B"),
    c("x1", "x2", "x3", "x4", "x5", "x1", "x2", "x3", "x4", "x5"),
    c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0)
  )
  stats.lag.plot(datasets.AirPassengers)
  stats.monthplot(datasets.AirPassengers)
  stats.scatter.smooth(
    c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0),
    c(1.0, 4.0, 9.0, 16.0, 25.0, 36.0, 49.0, 64.0, 81.0, 100.0),
    span = 0.75
  )
  stats.biplot(stats.prcomp(base.matrix(c(1.0, 2.0, 3.0, 4.0), ncol = 2L)))
  dev.off()
  return 1L
}

print(helpers())
"#;

    let (ok, stdout, stderr) = run_compile_case("stats_misc_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats misc helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
