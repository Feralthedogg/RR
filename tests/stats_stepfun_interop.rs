mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_stepfun_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats stepfun runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_stepfun_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"
import r * as datasets from "datasets"
import r { png, dev.off } from "grDevices"

fn keep(node) {
  return node
}

fn inspect_stepfun() -> int {
  png("stats_stepfun_plot.png")
  let sf = stats.stepfun(c(1.0, 2.0, 3.0), c(10.0, 20.0, 30.0, 40.0))
  let sf2 = stats.as.stepfun(sf)
  let is_sf = stats.is.stepfun(sf)
  let ps = stats.plot.stepfun(sf)
  let pe = stats.plot.ecdf(stats.ecdf(c(1.0, 2.0, 3.0)))
  let pt = stats.plot.ts(datasets.AirPassengers)
  let sc = stats.screeplot(stats.prcomp(base.matrix(c(1.0, 2.0, 3.0, 4.0), ncol = 2L)))
  let dend = stats.as.dendrogram(stats.hclust(stats.dist(base.matrix(c(1.0, 2.0, 3.0, 4.0), ncol = 2L))))
  let dend2 = stats.dendrapply(dend, keep)
  let leaf = stats.is.leaf(dend)
  let ord = stats.order.dendrogram(dend2)
  print(sf)
  print(sf2)
  print(is_sf)
  print(ps.t)
  print(ps.y)
  print(pe)
  print(pt)
  print(sc)
  print(dend)
  print(dend2)
  print(leaf)
  print(ord)
  let closed = dev.off()
  print(closed)
  return base.length(ord) + closed
}

print(inspect_stepfun())
"#;

    let rr_path = out_dir.join("stats_stepfun_interop.rr");
    let o0 = out_dir.join("stats_stepfun_interop_o0.R");
    let o2 = out_dir.join("stats_stepfun_interop_o2.R");

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
fn stats_stepfun_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as base from "base"
import r * as datasets from "datasets"
import r { png, dev.off } from "grDevices"

fn keep(node) {
  return node
}

fn inspect_stepfun_helpers() -> int {
  png("stats_stepfun_helpers_plot.png")
  let sf = stats.stepfun(c(1.0, 2.0, 3.0), c(10.0, 20.0, 30.0, 40.0))
  let sf2 = stats.as.stepfun(sf)
  let dend = stats.as.dendrogram(stats.hclust(stats.dist(base.matrix(c(1.0, 2.0, 3.0, 4.0), ncol = 2L))))
  let dend2 = stats.dendrapply(dend, keep)
  print(sf)
  print(sf2)
  print(stats.plot.stepfun(sf))
  print(stats.plot.ecdf(stats.ecdf(c(1.0, 2.0, 3.0))))
  print(stats.plot.ts(datasets.AirPassengers))
  print(stats.screeplot(stats.prcomp(base.matrix(c(1.0, 2.0, 3.0, 4.0), ncol = 2L))))
  print(stats.is.stepfun(sf))
  print(dend)
  print(dend2)
  print(stats.is.leaf(dend))
  print(stats.order.dendrogram(dend2))
  print(dev.off())
  return 1L
}

print(inspect_stepfun_helpers())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_stepfun_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats stepfun helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
