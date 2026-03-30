mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_distribution_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats distribution runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_distribution_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"

fn deterministic() -> float {
  print(stats.dnorm(c(0.0, 1.0)))
  print(stats.pnorm(c(0.0, 1.0)))
  print(stats.qnorm(c(0.25, 0.75)))
  print(stats.dbinom(c(0.0, 1.0), 3.0, 0.5))
  print(stats.pbinom(c(0.0, 1.0), 3.0, 0.5))
  print(stats.qbinom(c(0.25, 0.75), 3.0, 0.5))
  print(stats.dpois(c(0.0, 1.0), 2.0))
  print(stats.ppois(c(0.0, 1.0), 2.0))
  print(stats.qpois(c(0.25, 0.75), 2.0))
  print(stats.dunif(c(0.25, 0.75)))
  print(stats.punif(c(0.25, 0.75)))
  print(stats.qunif(c(0.25, 0.75)))
  print(stats.dgamma(c(0.5, 1.5), 2.0))
  print(stats.pgamma(c(0.5, 1.5), 2.0))
  print(stats.qgamma(c(0.25, 0.75), 2.0))
  print(stats.dbeta(c(0.25, 0.75), 2.0, 3.0))
  print(stats.pbeta(c(0.25, 0.75), 2.0, 3.0))
  print(stats.qbeta(c(0.25, 0.75), 2.0, 3.0))
  print(stats.dt(c(0.0, 1.0), 5.0))
  print(stats.pt(c(0.0, 1.0), 5.0))
  print(stats.qt(c(0.25, 0.75), 5.0))
  print(stats.df(c(0.5, 1.5), 5.0, 7.0))
  print(stats.pf(c(0.5, 1.5), 5.0, 7.0))
  print(stats.qf(c(0.25, 0.75), 5.0, 7.0))
  print(stats.dchisq(c(0.5, 1.5), 4.0))
  print(stats.pchisq(c(0.5, 1.5), 4.0))
  print(stats.qchisq(c(0.25, 0.75), 4.0))
  print(stats.dexp(c(0.5, 1.5), 2.0))
  print(stats.pexp(c(0.5, 1.5), 2.0))
  print(stats.qexp(c(0.25, 0.75), 2.0))
  print(stats.dlnorm(c(0.5, 1.5), 0.0, 1.0))
  print(stats.plnorm(c(0.5, 1.5), 0.0, 1.0))
  print(stats.qlnorm(c(0.25, 0.75), 0.0, 1.0))
  print(stats.dweibull(c(0.5, 1.5), 2.0, 1.5))
  print(stats.pweibull(c(0.5, 1.5), 2.0, 1.5))
  print(stats.qweibull(c(0.25, 0.75), 2.0, 1.5))
  print(stats.dcauchy(c(0.5, 1.5), 0.0, 1.0))
  print(stats.pcauchy(c(0.5, 1.5), 0.0, 1.0))
  print(stats.qcauchy(c(0.25, 0.75), 0.0, 1.0))
  print(stats.dgeom(c(0.0, 1.0), 0.5))
  print(stats.pgeom(c(0.0, 1.0), 0.5))
  print(stats.qgeom(c(0.25, 0.75), 0.5))
  print(stats.dhyper(c(0.0, 1.0), 4.0, 6.0, 3.0))
  print(stats.phyper(c(0.0, 1.0), 4.0, 6.0, 3.0))
  print(stats.qhyper(c(0.25, 0.75), 4.0, 6.0, 3.0))
  print(stats.dnbinom(c(0.0, 1.0), 3.0, 0.5))
  print(stats.pnbinom(c(0.0, 1.0), 3.0, 0.5))
  print(stats.qnbinom(c(0.25, 0.75), 3.0, 0.5))
  print(stats.dlogis(c(0.0, 1.0), 0.0, 1.0))
  print(stats.plogis(c(0.0, 1.0), 0.0, 1.0))
  print(stats.qlogis(c(0.25, 0.75), 0.0, 1.0))
  print(stats.pbirthday(23L))
  print(stats.qbirthday(0.5))
  print(stats.ptukey(3.0, 4.0, 10.0))
  print(stats.qtukey(0.95, 4.0, 10.0))
  print(stats.psmirnov(c(0.2, 0.5), c(5L, 7L)))
  print(stats.qsmirnov(c(0.2, 0.8), c(5L, 7L)))
  print(stats.acf2AR(c(0.5, 0.25)))
  print(stats.dsignrank(c(0.0, 1.0), 5.0))
  print(stats.psignrank(c(0.0, 1.0), 5.0))
  print(stats.qsignrank(c(0.25, 0.75), 5.0))
  print(stats.dwilcox(c(0.0, 1.0), 4.0, 5.0))
  print(stats.pwilcox(c(0.0, 1.0), 4.0, 5.0))
  print(stats.qwilcox(c(0.25, 0.75), 4.0, 5.0))
  return stats.qnorm(0.5)
}

print(deterministic())
"#;

    let rr_path = out_dir.join("stats_distribution_interop.rr");
    let o0 = out_dir.join("stats_distribution_interop_o0.R");
    let o2 = out_dir.join("stats_distribution_interop_o2.R");

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
fn stats_random_distribution_helpers_compile_without_opaque_warning() {
    let src = r#"
import r default from "stats"

fn draws() -> int {
  let a = stats.rnorm(3L)
  let b = stats.runif(3L)
  let c = stats.rpois(3L, 2.0)
  let d = stats.rbinom(3L, 4.0, 0.5)
  let e = stats.rgamma(3L, 2.0)
  let f = stats.rbeta(3L, 2.0, 3.0)
  let g = stats.rt(3L, 5.0)
  let h = stats.rf(3L, 5.0, 7.0)
  let i = stats.rchisq(3L, 4.0)
  let j = stats.rexp(3L, 2.0)
  let k = stats.rlnorm(3L, 0.0, 1.0)
  let l = stats.rweibull(3L, 2.0, 1.5)
  let m = stats.rcauchy(3L, 0.0, 1.0)
  let n = stats.rgeom(3L, 0.5)
  let o = stats.rhyper(3L, 4.0, 6.0, 3.0)
  let p = stats.rnbinom(3L, 3.0, 0.5)
  let q = stats.rlogis(3L, 0.0, 1.0)
  let r = stats.rsignrank(3L, 5.0)
  let rs = stats.rsmirnov(3L, c(5L, 7L))
  let s = stats.rwilcox(3L, 4.0, 5.0)
  print(a)
  print(b)
  print(c)
  print(d)
  print(e)
  print(f)
  print(g)
  print(h)
  print(i)
  print(j)
  print(k)
  print(l)
  print(m)
  print(n)
  print(o)
  print(p)
  print(q)
  print(r)
  print(rs)
  print(s)
  return length(a) + length(b) + length(c) + length(d) + length(e) + length(f) + length(g) + length(h) + length(i) + length(j) + length(k) + length(l) + length(m) + length(n) + length(o) + length(p) + length(q) + length(r) + length(rs) + length(s)
}

print(draws())
"#;

    let (ok, stdout, stderr) =
        run_compile_case("stats_distribution_random", src, "case.rr", "-O1", &[]);

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "stats random distribution helpers should stay on direct surface, got stderr:\n{stderr}"
    );
}
