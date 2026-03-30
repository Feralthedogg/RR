mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_cluster_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats cluster runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_cluster_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"
import r * as graphics from "graphics"
import r * as grDevices from "grDevices"

fn inspect_cluster() -> float {
  let pts = base.matrix(c(1.0, 1.0, 2.0, 2.0, 9.0, 9.0, 10.0, 10.0), ncol = 2L, byrow = true)
  let centers = base.rbind(c(1.0, 1.0), c(10.0, 10.0))
  let km = stats.kmeans(pts, centers)
  let hc = stats.hclust(stats.dist(c(1.0, 2.0, 4.0, 8.0)))
  let ad = stats.as.dist(pts)
  let ah = stats.as.hclust(hc)
  let dend = stats.as.dendrogram(hc)
  let cop = stats.cophenetic(hc)
  let cut = stats.cutree(hc, k = 2L)
  let a = stats.acf(c(1.0, 2.0, 3.0, 4.0), plot = false)
  let p = stats.pacf(c(1.0, 2.0, 3.0, 4.0), plot = false)
  let cc = stats.ccf(c(1.0, 2.0, 3.0, 4.0), c(4.0, 3.0, 2.0, 1.0), plot = false)
  grDevices.png(filename = "target/tests/stats_cluster_interop/rect_hclust.png", width = 320.0, height = 240.0)
  graphics.plot(hc)
  let rects = stats.rect.hclust(hc, k = 2L)
  let closed = grDevices.dev.off()
  print(km.cluster)
  print(km.centers)
  print(km.size)
  print(km.iter)
  print(ad)
  print(ah.order)
  print(ah.height)
  print(dend)
  print(cop)
  print(hc.order)
  print(hc.height)
  print(cut)
  print(rects)
  print(closed)
  print(base.dim(a.lag))
  print(base.dim(a.acf))
  print(a.type)
  print(base.dim(p.lag))
  print(base.dim(p.acf))
  print(p.type)
  print(base.dim(cc.lag))
  print(base.dim(cc.acf))
  print(cc.type)
  return km.totss + base.length(cut)
}

print(inspect_cluster())
"#;

    let rr_path = out_dir.join("stats_cluster_interop.rr");
    let o0 = out_dir.join("stats_cluster_interop_o0.R");
    let o2 = out_dir.join("stats_cluster_interop_o2.R");

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
fn stats_cluster_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as base from "base"
import r * as graphics from "graphics"
import r * as grDevices from "grDevices"

fn cluster_meta() -> int {
  let pts = base.matrix(c(1.0, 1.0, 2.0, 2.0, 9.0, 9.0, 10.0, 10.0), ncol = 2L, byrow = true)
  let km = stats.kmeans(pts, 2L)
  let hc = stats.hclust(stats.dist(c(1.0, 2.0, 4.0, 8.0)))
  let ad = stats.as.dist(pts)
  let ah = stats.as.hclust(hc)
  let dend = stats.as.dendrogram(hc)
  let cop = stats.cophenetic(hc)
  let cut = stats.cutree(hc, k = 2L)
  grDevices.png(filename = "target/tests/stats_cluster_interop/rect_hclust_compile.png", width = 320.0, height = 240.0)
  graphics.plot(hc)
  let rects = stats.rect.hclust(hc, k = 2L)
  let closed = grDevices.dev.off()
  let a = stats.acf(c(1.0, 2.0, 3.0, 4.0), plot = false)
  print(ad)
  print(ah)
  print(dend)
  print(cop)
  print(km.cluster)
  print(hc.order)
  print(cut)
  print(rects)
  print(closed)
  print(a.type)
  return base.length(cut)
}

print(cluster_meta())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_cluster_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats cluster helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
