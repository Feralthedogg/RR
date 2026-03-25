mod common;

use common::{normalize, rscript_available, rscript_path, run_rscript, unique_dir};
use std::fs;
use std::path::PathBuf;

fn strip_trailing_null_line(s: &str) -> String {
    let normalized = normalize(s);
    normalized
        .lines()
        .filter(|line| line.trim() != "NULL")
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[test]
fn loop_carried_scalar_update_survives_r_optimization() {
    let rscript = match rscript_path() {
        Some(path) if rscript_available(&path) => path,
        _ => {
            eprintln!("Skipping loop_carried_state_regression: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("loop_carried_state_regression");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj = unique_dir(&sandbox_root, "bounce");
    fs::create_dir_all(&proj).expect("failed to create project dir");

    let rr_src = r#"
fn main() {
  let y = 50.0
  let vy = 0.0
  let g = -9.81
  let dt = 0.1
  let restitution = 0.8
  let time = 0.0

  while (time <= 5.0) {
    vy = vy + g * dt
    y = y + vy * dt

    if (y < 0.0) {
      y = 0.0
      vy = -vy * restitution
    }

    print(y)
    time = time + dt
  }
}

main()
"#;

    let ref_r = r#"
main <- function() {
  y <- 50.0
  vy <- 0.0
  g <- -9.81
  dt <- 0.1
  restitution <- 0.8
  time <- 0.0

  while (time <= 5.0) {
    vy <- vy + g * dt
    y <- y + vy * dt

    if (y < 0.0) {
      y <- 0.0
      vy <- -vy * restitution
    }

    print(y)
    time <- time + dt
  }
}

main()
"#;

    let rr_path = proj.join("main.rr");
    let out_path = proj.join("main.R");
    let ref_path = proj.join("ref.R");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");
    fs::write(&ref_path, ref_r).expect("failed to write reference R");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    common::compile_rr(&rr_bin, &rr_path, &out_path, "-O2");

    let compiled = fs::read_to_string(&out_path).expect("failed to read compiled R");
    let vy_assign_count = compiled
        .lines()
        .filter(|line| line.trim_start().starts_with("vy <- "))
        .count();
    assert!(
        vy_assign_count >= 2,
        "expected loop-carried vy update plus bounce update to both survive emission:\n{}",
        compiled
    );
    assert!(
        compiled
            .lines()
            .any(|line| line.contains("licm_") && line.contains("(g * dt)")),
        "expected loop-invariant g * dt to be hoisted before the repeat loop:\n{}",
        compiled
    );

    let rr_run = run_rscript(&rscript, &out_path);
    let ref_run = run_rscript(&rscript, &ref_path);

    assert_eq!(ref_run.status, 0, "reference failed: {}", ref_run.stderr);
    assert_eq!(rr_run.status, 0, "compiled RR failed: {}", rr_run.stderr);
    assert_eq!(
        strip_trailing_null_line(&rr_run.stdout),
        strip_trailing_null_line(&ref_run.stdout)
    );
}
