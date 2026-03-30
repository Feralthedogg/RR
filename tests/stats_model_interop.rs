mod common;

use common::{
    compile_rr, normalize, rscript_available, rscript_path, run_compile_case, run_rscript,
};
use std::fs;
use std::path::PathBuf;

#[test]
fn stats_lm_and_predict_work_in_strict_rr_code() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats model interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_model_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn fit_and_predict() -> vector<float> {
  let train = base.data.frame(
    x = c(1.0, 2.0, 3.0),
    y = c(2.0, 4.0, 6.0)
  )
  let model = stats.lm(stats.as.formula("y ~ x"), data = train)
  let grid = base.data.frame(x = c(4.0, 5.0))
  return stats.predict(model, newdata = grid)
}

print(fit_and_predict())
"#;

    let rr_path = out_dir.join("stats_model_interop.rr");
    let o0 = out_dir.join("stats_model_interop_o0.R");
    let o2 = out_dir.join("stats_model_interop_o2.R");

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
fn stats_glm_and_predict_response_work_in_rr_code() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats glm interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_model_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn fit_and_predict_glm() -> vector<float> {
  let train = base.data.frame(
    x = c(1.0, 2.0, 3.0, 4.0),
    y = c(0.0, 0.0, 1.0, 1.0)
  )
  let model = stats.glm(
    stats.as.formula("y ~ x"),
    data = train,
    family = stats.binomial()
  )
  let grid = base.data.frame(x = c(1.5, 3.5))
  return stats.predict(model, newdata = grid, type = "response")
}

print(fit_and_predict_glm())
"#;

    let rr_path = out_dir.join("stats_glm_interop.rr");
    let o0 = out_dir.join("stats_glm_interop_o0.R");
    let o2 = out_dir.join("stats_glm_interop_o2.R");

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
fn stats_model_view_helpers_work_in_rr_code() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats model view interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_model_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_model() -> vector<float> {
  let train = base.data.frame(
    x = c(1.0, 2.0, 3.0),
    y = c(2.0, 4.0, 6.0)
  )
  let model = stats.lm(stats.as.formula("y ~ x"), data = train)
  print(stats.coef(model))
  print(stats.fitted(model))
  print(stats.residuals(model))
  return stats.coef(model)
}

print(inspect_model())
"#;

    let rr_path = out_dir.join("stats_model_views.rr");
    let o0 = out_dir.join("stats_model_views_o0.R");
    let o2 = out_dir.join("stats_model_views_o2.R");

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
fn stats_model_matrix_helpers_work_in_rr_code() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats model matrix interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_model_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_mats() -> matrix<float> {
  let train = base.data.frame(
    x = c(1.0, 2.0, 3.0),
    z = c(3.0, 2.0, 1.0),
    y = c(2.0, 4.0, 6.0)
  )
  let model = stats.lm(stats.as.formula("y ~ x"), data = train)
  let updated = stats.update(model, stats.as.formula("y ~ x"))
  let glm_model = stats.glm(
    stats.as.formula("y ~ x"),
    data = train,
    family = stats.gaussian()
  )
  print(stats.confint(model))
  print(stats.model.matrix(stats.as.formula("y ~ x"), data = train))
  print(stats.model.matrix(model))
  print(stats.model.matrix(updated))
  print(stats.model.matrix(glm_model))
  print(ncol(stats.model.matrix(stats.as.formula("y ~ x"), data = train)))
  print(ncol(stats.model.matrix(stats.as.formula("y ~ x + z"), data = train)))
  print(ncol(stats.model.matrix(stats.as.formula("y ~ 0 + x + z"), data = train)))
  print(ncol(stats.model.matrix(stats.as.formula("y ~ -1 + x + z"), data = train)))
  print(ncol(stats.model.matrix(stats.as.formula("y ~ x + 0 + z"), data = train)))
  print(ncol(stats.model.matrix(stats.as.formula("y ~ x + z - 1"), data = train)))
  print(ncol(stats.model.matrix(stats.as.formula("y ~ x + z - 0"), data = train)))
  print(ncol(stats.model.matrix(stats.as.formula("y ~ -0 + x + z"), data = train)))
  print(ncol(stats.model.matrix(stats.as.formula("y ~ -1 + 1 + x + z"), data = train)))
  print(ncol(stats.model.matrix(stats.as.formula("y ~ x + z - 1 + 1"), data = train)))
  print(ncol(stats.model.matrix(model)))
  print(ncol(stats.model.matrix(updated)))
  print(ncol(stats.model.matrix(glm_model)))
  return stats.vcov(model)
}

print(inspect_mats())
"#;

    let rr_path = out_dir.join("stats_model_mats.rr");
    let o0 = out_dir.join("stats_model_mats_o0.R");
    let o2 = out_dir.join("stats_model_mats_o2.R");

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
fn stats_model_score_helpers_work_in_rr_code() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats model score interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_model_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_scores() -> float {
  let train = base.data.frame(
    x = c(1.0, 2.0, 3.0),
    y = c(2.0, 4.0, 6.0)
  )
  let model = stats.lm(stats.as.formula("y ~ x"), data = train)
  print(stats.AIC(model))
  print(stats.BIC(model))
  print(stats.logLik(model))
  print(stats.deviance(model))
  print(stats.sigma(model))
  print(stats.nobs(model))
  print(stats.df.residual(model))
  return stats.AIC(model)
}

print(inspect_scores())
"#;

    let rr_path = out_dir.join("stats_model_scores.rr");
    let o0 = out_dir.join("stats_model_scores_o0.R");
    let o2 = out_dir.join("stats_model_scores_o2.R");

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
fn stats_model_meta_helpers_work_in_rr_code() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats model meta interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_model_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_meta() -> float {
  let train = base.data.frame(
    x = c(1.0, 2.0, 3.0),
    y = c(2.0, 4.0, 6.0)
  )
  let model = stats.lm(stats.as.formula("y ~ x"), data = train)
  let summary = base.summary(model)
  print(summary.sigma)
  print(summary.coefficients)
  print(summary.residuals)
  print(summary.terms.order)
  print(summary.terms.dataClasses)
  print(length(summary.terms.variables))
  print(stats.anova(model))
  let updated = stats.update(model, stats.as.formula("y ~ x"))
  let updated_summary = base.summary(updated)
  let updated_terms = stats.terms(updated)
  print(updated_summary.sigma)
  print(updated_terms.order)
  print(updated_terms.factors)
  print(updated_terms.dataClasses)
  print(length(updated_terms.predvars))
  return stats.AIC(model)
}

print(inspect_meta())
"#;

    let rr_path = out_dir.join("stats_model_meta.rr");
    let o0 = out_dir.join("stats_model_meta_o0.R");
    let o2 = out_dir.join("stats_model_meta_o2.R");

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
fn stats_model_frame_preserves_visible_dataframe_shape() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats model.frame interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_model_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_model_frame() -> float {
  let train = base.data.frame(
    x = c(1.0, 2.0, 3.0),
    y = c(2.0, 4.0, 6.0)
  )
  let frame = stats.model.frame(stats.as.formula("y ~ x"), data = train)
  print(frame.x)
  print(frame.y)
  return sum(frame.x)
}

print(inspect_model_frame())
"#;

    let rr_path = out_dir.join("stats_model_frame.rr");
    let o0 = out_dir.join("stats_model_frame_o0.R");
    let o2 = out_dir.join("stats_model_frame_o2.R");

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
fn stats_model_frame_from_model_preserves_visible_dataframe_shape() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats model.frame(model) runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_model_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_model_frame_from_model() -> float {
  let train = base.data.frame(
    x = c(1.0, 2.0, 3.0),
    y = c(2.0, 4.0, 6.0)
  )
  let model = stats.lm(stats.as.formula("y ~ x"), data = train)
  let frame = stats.model.frame(model)
  print(frame.x)
  print(frame.y)
  return sum(frame.y)
}

print(inspect_model_frame_from_model())
"#;

    let rr_path = out_dir.join("stats_model_frame_from_model.rr");
    let o0 = out_dir.join("stats_model_frame_from_model_o0.R");
    let o2 = out_dir.join("stats_model_frame_from_model_o2.R");

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
fn stats_model_frame_from_glm_preserves_visible_dataframe_shape() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats model.frame(glm) runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_model_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_model_frame_from_glm() -> float {
  let train = base.data.frame(
    x = c(1.0, 2.0, 3.0, 4.0),
    y = c(0.0, 0.0, 1.0, 1.0)
  )
  let model = stats.glm(
    stats.as.formula("y ~ x"),
    data = train,
    family = stats.binomial()
  )
  let frame = stats.model.frame(model)
  print(frame.x)
  print(frame.y)
  return sum(frame.y)
}

print(inspect_model_frame_from_glm())
"#;

    let rr_path = out_dir.join("stats_model_frame_from_glm.rr");
    let o0 = out_dir.join("stats_model_frame_from_glm_o0.R");
    let o2 = out_dir.join("stats_model_frame_from_glm_o2.R");

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
fn stats_model_frame_from_updated_model_preserves_visible_dataframe_shape() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!(
                "Skipping stats model.frame(update(model)) runtime test: Rscript unavailable."
            );
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_model_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_model_frame_from_updated() -> float {
  let train = base.data.frame(
    x = c(1.0, 2.0, 3.0),
    y = c(2.0, 4.0, 6.0)
  )
  let model = stats.lm(stats.as.formula("y ~ x"), data = train)
  let updated = stats.update(model, stats.as.formula("y ~ x"))
  let frame = stats.model.frame(updated)
  print(frame.x)
  print(frame.y)
  return sum(frame.x)
}

print(inspect_model_frame_from_updated())
"#;

    let rr_path = out_dir.join("stats_model_frame_from_updated.rr");
    let o0 = out_dir.join("stats_model_frame_from_updated_o0.R");
    let o2 = out_dir.join("stats_model_frame_from_updated_o2.R");

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
fn stats_glm_summary_fields_work_in_rr_code() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping stats glm summary interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("stats_model_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "stats"
import r * as base from "base"

fn inspect_glm_meta() -> float {
  let train = base.data.frame(
    x = c(1.0, 2.0, 3.0, 4.0),
    y = c(0.0, 0.0, 1.0, 1.0)
  )
  let model = stats.glm(
    stats.as.formula("y ~ x"),
    data = train,
    family = stats.binomial()
  )
  let summary = base.summary(model)
  print(summary.dispersion)
  print(summary.family.family)
  print(summary.family.link)
  print(summary.coefficients)
  print(summary.df)
  print(summary.terms.order)
  print(length(summary.terms.variables))
  let updated = stats.update(model, stats.as.formula("y ~ x"))
  let updated_summary = base.summary(updated)
  let updated_terms = stats.terms(updated)
  print(updated_summary.dispersion)
  print(updated_terms.order)
  print(length(updated_terms.predvars))
  return stats.AIC(model)
}

print(inspect_glm_meta())
"#;

    let rr_path = out_dir.join("stats_glm_summary.rr");
    let o0 = out_dir.join("stats_glm_summary_o0.R");
    let o2 = out_dir.join("stats_glm_summary_o2.R");

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
fn stats_family_helpers_compile_on_direct_surface_without_opaque_warning() {
    let src = r#"
import r default from "stats"

fn family_marker() -> float {
  let bin = stats.binomial()
  let gau = stats.gaussian()
  let poi = stats.poisson()
  print(bin.family)
  print(gau.link)
  print(poi.family)
  let sink = list(bin, gau, poi)
  print(sink)
  return 0.0
}

print(family_marker())
"#;

    let (ok, stdout, stderr) = run_compile_case(
        "stats_family_helpers",
        src,
        "stats_family_helpers.rr",
        "-O1",
        &[],
    );

    assert!(ok, "compile failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        !stderr.contains("Opaque interop enabled"),
        "family helpers should stay on the direct surface, got stderr:\n{stderr}"
    );
}
