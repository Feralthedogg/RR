mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use rr::compiler::internal::typeck::{NativeBackend, TypeConfig, TypeMode};
use rr::compiler::{OptLevel, compile_with_config};
use rr::error::RRCode;
use std::fs;
use std::path::PathBuf;

fn strict_type_cfg() -> TypeConfig {
    TypeConfig {
        mode: TypeMode::Strict,
        native_backend: NativeBackend::Off,
    }
}

#[test]
fn stats_direct_calls_have_usable_types_in_strict_mode() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats direct types runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("stats_direct_types");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn center_and_spread(x: vector<float>) -> float {
  let center = stats.median(x)
  let spread = stats.sd(x)
  return center + spread
}

fn quartiles(x: vector<float>) -> vector<float> {
  return stats.quantile(x, probs = c(0.25, 0.75))
}

fn model_views(xs: vector<float>, ys: vector<float>) -> vector<float> {
  let train = base.data.frame(x = xs, y = ys)
  let model = stats.lm(stats.as.formula("y ~ x"), data = train)
  let c = stats.coef(model)
  let f = stats.fitted(model)
  let r = stats.residuals(model)
  print(f)
  print(r)
  return c
}

fn model_mats(xs: vector<float>, ys: vector<float>) -> matrix<float> {
  let train = base.data.frame(x = xs, y = ys)
  let model = stats.lm(stats.as.formula("y ~ x"), data = train)
  let v = stats.vcov(model)
  let ci = stats.confint(model)
  print(ci)
  return v
}

fn design(xs: vector<float>, ys: vector<float>) -> matrix<float> {
  let train = base.data.frame(x = xs, y = ys)
  return stats.model.matrix(stats.as.formula("y ~ x"), data = train)
}

fn model_scores(xs: vector<float>, ys: vector<float>) -> float {
  let train = base.data.frame(x = xs, y = ys)
  let model = stats.lm(stats.as.formula("y ~ x"), data = train)
  let aic = stats.AIC(model)
  let bic = stats.BIC(model)
  let ll = stats.logLik(model)
  let dev = stats.deviance(model)
  let sig = stats.sigma(model)
  let n = stats.nobs(model)
  let df = stats.df.residual(model)
  print(n)
  print(df)
  return aic + bic + ll + dev + sig
}

fn model_meta(xs: vector<float>, ys: vector<float>) -> float {
  let train = base.data.frame(x = xs, y = ys)
  let model = stats.lm(stats.as.formula("y ~ x"), data = train)
  let summary = base.summary(model)
  let table = stats.anova(model)
  let updated = stats.update(model, stats.as.formula("y ~ x"))
  let terms = stats.terms(model)
  print(summary)
  print(table)
  print(updated)
  print(length(terms))
  return stats.AIC(model)
}

print(center_and_spread(c(1.0, 2.0, 3.0, 4.0)))
print(quartiles(c(1.0, 2.0, 3.0, 4.0)))
print(model_views(c(1.0, 2.0, 3.0), c(2.0, 4.0, 6.0)))
print(model_mats(c(1.0, 2.0, 3.0), c(2.0, 4.0, 6.0)))
print(design(c(1.0, 2.0, 3.0), c(2.0, 4.0, 6.0)))
print(model_scores(c(1.0, 2.0, 3.0), c(2.0, 4.0, 6.0)))
print(model_meta(c(1.0, 2.0, 3.0), c(2.0, 4.0, 6.0)))
"#;

    let rr_path = out_dir.join("stats_direct_types.rr");
    let o0 = out_dir.join("stats_direct_types_o0.R");
    let o2 = out_dir.join("stats_direct_types_o2.R");

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
fn stats_median_type_mismatch_is_reported_in_strict_mode() {
    let src = r#"
import r default from "stats"

fn bad(x: vector<float>) -> int {
  return stats.median(x)
}

bad(c(1.0, 2.0, 3.0))
"#;

    let res = compile_with_config(
        "stats_direct_type_mismatch.rr",
        src,
        OptLevel::O1,
        strict_type_cfg(),
    );
    let err = res.expect_err("compile should fail");
    assert!(matches!(err.code, RRCode::E1010));
}
