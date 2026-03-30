mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_htest_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats htest runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_htest_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"
import r default from "datasets"

fn inspect_tests() -> float {
  let tt = stats.t.test(c(1.0, 2.0, 3.0), c(2.0, 3.0, 4.0))
  let wt = stats.wilcox.test(c(1.0, 2.0, 3.0), c(2.0, 3.0, 4.0), exact = false)
  let bt = stats.binom.test(3L, 5L)
  let pt = stats.prop.test(c(3.0, 4.0), c(5.0, 6.0))
  let pot = stats.poisson.test(c(3.0, 4.0), c(1.0, 2.0))
  let ct = stats.chisq.test(base.matrix(c(10.0, 20.0, 20.0, 30.0), nrow = 2L))
  let ft = stats.fisher.test(base.matrix(c(10.0, 20.0, 20.0, 30.0), nrow = 2L))
  let crt = stats.cor.test(c(1.0, 2.0, 3.0, 4.0), c(2.0, 4.0, 6.0, 8.0))
  let kst = stats.ks.test(c(1.0, 2.0, 3.0, 4.0), c(2.0, 3.0, 4.0, 5.0))
  let sht = stats.shapiro.test(c(1.0, 2.0, 3.0, 4.0))
  let ant = stats.ansari.test(c(1.0, 2.0, 3.0), c(2.0, 3.0, 4.0), exact = false)
  let bart = stats.bartlett.test(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), base.factor(c("a", "a", "b", "b", "c", "c")))
  let mlt = stats.lm(stats.as.formula("base::cbind(mpg, disp) ~ factor(cyl)"), data = datasets.mtcars)
  let mct2 = stats.mauchly.test(mlt)
  let boxt = stats.Box.test(stats.ts(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0)), lag = 1L, type = "Ljung-Box")
  let flt = stats.fligner.test(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), base.factor(c("a", "a", "b", "b", "c", "c")))
  let frd = stats.friedman.test(base.matrix(c(1.0, 2.0, 3.0, 2.0, 3.0, 4.0), nrow = 3L))
  let kru = stats.kruskal.test(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), base.factor(c("a", "a", "b", "b", "c", "c")))
  let mht = stats.mantelhaen.test(datasets.UCBAdmissions)
  let mct = stats.mcnemar.test(base.matrix(c(10.0, 20.0, 20.0, 30.0), nrow = 2L))
  let mdt = stats.mood.test(c(1.0, 2.0, 3.0), c(2.0, 3.0, 4.0), exact = false)
  let owt = stats.oneway.test(stats.as.formula("y ~ grp"), data = base.data.frame(y = c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), grp = base.factor(c("a", "a", "b", "b", "c", "c"))))
  let trend = stats.prop.trend.test(c(3.0, 4.0, 5.0), c(5.0, 6.0, 7.0))
  let qdt = stats.quade.test(base.matrix(c(1.0, 2.0, 3.0, 2.0, 3.0, 4.0), nrow = 3L))
  let vtt = stats.var.test(c(1.0, 2.0, 3.0), c(2.0, 3.0, 4.0))
  let ptt = stats.pairwise.t.test(c(1.0, 2.0, 3.0, 4.0), c("a", "a", "b", "b"), "holm")
  let pwt = stats.pairwise.wilcox.test(c(1.0, 2.0, 3.0, 4.0), c("a", "a", "b", "b"), "holm")
  let ppt = stats.pairwise.prop.test(c(3.0, 4.0, 5.0), c(5.0, 6.0, 7.0), "holm")
  print(tt.statistic)
  print(tt.estimate)
  print(tt.method)
  print(wt.statistic)
  print(wt.method)
  print(bt.statistic)
  print(bt.estimate)
  print(bt.method)
  print(pt.statistic)
  print(pt.estimate)
  print(pt.method)
  print(pot.statistic)
  print(pot.estimate)
  print(pot.method)
  print(ct.statistic)
  print(ct.method)
  print(ft.estimate)
  print(ft.method)
  print(crt.statistic)
  print(crt.estimate)
  print(crt.method)
  print(kst.statistic)
  print(kst.method)
  print(sht.statistic)
  print(sht.method)
  print(ant.statistic)
  print(ant.method)
  print(bart.statistic)
  print(bart.method)
  print(mct2.statistic)
  print(mct2.method)
  print(boxt.statistic)
  print(boxt.method)
  print(flt.statistic)
  print(flt.method)
  print(frd.statistic)
  print(frd.method)
  print(kru.statistic)
  print(kru.parameter)
  print(mht.estimate)
  print(mht.method)
  print(mct.statistic)
  print(mct.method)
  print(mdt.statistic)
  print(mdt.method)
  print(owt.statistic)
  print(owt.parameter)
  print(trend.statistic)
  print(qdt.statistic)
  print(qdt.parameter)
  print(vtt.estimate)
  print(vtt.method)
  print(ptt.method)
  print(pwt.method)
  print(ppt.method)
  return tt.statistic + bt.statistic + pt.statistic + pot.statistic + ct.statistic + ft.estimate + crt.statistic + kst.statistic + sht.statistic + ant.statistic + bart.statistic + mct2.statistic + boxt.statistic + flt.statistic + frd.statistic + kru.statistic + mht.estimate + mct.statistic + mdt.statistic + owt.statistic + trend.statistic + qdt.statistic + vtt.estimate
}

print(inspect_tests())
"#;

    let rr_path = out_dir.join("stats_htest_interop.rr");
    let o0 = out_dir.join("stats_htest_interop_o0.R");
    let o2 = out_dir.join("stats_htest_interop_o2.R");

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
fn stats_htest_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"
import r * as base from "base"
import r default from "datasets"

fn inspect_pairwise() -> int {
  let ptt = stats.pairwise.t.test(c(1.0, 2.0, 3.0, 4.0), c("a", "a", "b", "b"), "holm")
  let pwt = stats.pairwise.wilcox.test(c(1.0, 2.0, 3.0, 4.0), c("a", "a", "b", "b"), "holm")
  let ppt = stats.pairwise.prop.test(c(3.0, 4.0, 5.0), c(5.0, 6.0, 7.0), "holm")
  let ct = stats.chisq.test(base.matrix(c(10.0, 20.0, 20.0, 30.0), nrow = 2L))
  let ft = stats.fisher.test(base.matrix(c(10.0, 20.0, 20.0, 30.0), nrow = 2L))
  let crt = stats.cor.test(c(1.0, 2.0, 3.0, 4.0), c(2.0, 4.0, 6.0, 8.0))
  let kst = stats.ks.test(c(1.0, 2.0, 3.0, 4.0), c(2.0, 3.0, 4.0, 5.0))
  let sht = stats.shapiro.test(c(1.0, 2.0, 3.0, 4.0))
  let ant = stats.ansari.test(c(1.0, 2.0, 3.0), c(2.0, 3.0, 4.0), exact = false)
  let bart = stats.bartlett.test(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), base.factor(c("a", "a", "b", "b", "c", "c")))
  let mlt = stats.lm(stats.as.formula("base::cbind(mpg, disp) ~ factor(cyl)"), data = datasets.mtcars)
  let mct2 = stats.mauchly.test(mlt)
  let boxt = stats.Box.test(stats.ts(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0)), lag = 1L, type = "Ljung-Box")
  let flt = stats.fligner.test(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), base.factor(c("a", "a", "b", "b", "c", "c")))
  let frd = stats.friedman.test(base.matrix(c(1.0, 2.0, 3.0, 2.0, 3.0, 4.0), nrow = 3L))
  let kru = stats.kruskal.test(c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), base.factor(c("a", "a", "b", "b", "c", "c")))
  let mht = stats.mantelhaen.test(datasets.UCBAdmissions)
  let mct = stats.mcnemar.test(base.matrix(c(10.0, 20.0, 20.0, 30.0), nrow = 2L))
  let mdt = stats.mood.test(c(1.0, 2.0, 3.0), c(2.0, 3.0, 4.0), exact = false)
  let owt = stats.oneway.test(stats.as.formula("y ~ grp"), data = base.data.frame(y = c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), grp = base.factor(c("a", "a", "b", "b", "c", "c"))))
  let trend = stats.prop.trend.test(c(3.0, 4.0, 5.0), c(5.0, 6.0, 7.0))
  let qdt = stats.quade.test(base.matrix(c(1.0, 2.0, 3.0, 2.0, 3.0, 4.0), nrow = 3L))
  let vtt = stats.var.test(c(1.0, 2.0, 3.0), c(2.0, 3.0, 4.0))
  print(ptt)
  print(pwt)
  print(ppt)
  print(ct)
  print(ft)
  print(crt)
  print(kst)
  print(sht)
  print(ant)
  print(bart)
  print(mct2)
  print(boxt)
  print(flt)
  print(frd)
  print(kru)
  print(mht)
  print(mct)
  print(mdt)
  print(owt)
  print(trend)
  print(qdt)
  print(vtt)
  return 1L
}

print(inspect_pairwise())
"#;

    let (ok, stdout, stderr) = run_compile_case("stats_htest_helpers", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats htest helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
