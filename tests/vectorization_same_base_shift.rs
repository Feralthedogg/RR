mod common;

use common::{normalize, rscript_available, rscript_path, run_rscript, unique_dir};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn compile_rr(rr_src: &Path, out_path: &Path, level: &str) -> String {
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let output = Command::new(&rr_bin)
        .arg(rr_src)
        .arg("-o")
        .arg(out_path)
        .arg(level)
        .arg("--no-incremental")
        .output()
        .expect("failed to run RR compiler");
    assert!(
        output.status.success(),
        "RR compile failed for {} ({}):\n{}",
        rr_src.display(),
        level,
        String::from_utf8_lossy(&output.stderr)
    );
    fs::read_to_string(out_path).expect("failed to read compiled output")
}

#[test]
fn same_base_forward_shift_vectorizes_via_rr_shift_assign() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("vectorization_same_base_shift");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "forward");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_src = r#"
fn shift_forward(x) {
  for (i in 1..(length(x) - 1)) {
    x[i] = x[i + 1]
  }
  return x
}

print(shift_forward(c(10, 20, 30, 40, 50)))
"#;
    let ref_src = r#"
shift_forward <- function(x) {
  for (i in 1:(length(x) - 1)) {
    x[i] <- x[i + 1]
  }
  x
}

print(shift_forward(c(10, 20, 30, 40, 50)))
"#;

    let rr_path = proj_dir.join("forward.rr");
    let out_path = proj_dir.join("forward.R");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let code = compile_rr(&rr_path, &out_path, "-O1");
    assert!(
        code.contains("rr_shift_assign("),
        "expected same-base forward shift to lower via rr_shift_assign(...)"
    );

    let Some(rscript) = rscript_path() else {
        return;
    };
    if !rscript_available(&rscript) {
        return;
    }

    let ref_path = proj_dir.join("forward_ref.R");
    fs::write(&ref_path, ref_src).expect("failed to write reference");

    let ref_run = run_rscript(&rscript, &ref_path);
    let compiled_run = run_rscript(&rscript, &out_path);
    assert_eq!(ref_run.status, 0, "reference failed: {}", ref_run.stderr);
    assert_eq!(
        compiled_run.status, 0,
        "compiled failed: {}",
        compiled_run.stderr
    );
    assert_eq!(normalize(&ref_run.stdout), normalize(&compiled_run.stdout));
}

#[test]
fn same_base_backward_shift_stays_scalar() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("vectorization_same_base_shift");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "backward");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_src = r#"
fn shift_backward(x) {
  for (i in 1..(length(x) - 1)) {
    x[i + 1] = x[i]
  }
  return x
}

print(shift_backward(c(10, 20, 30, 40, 50)))
"#;
    let ref_src = r#"
shift_backward <- function(x) {
  for (i in 1:(length(x) - 1)) {
    x[i + 1] <- x[i]
  }
  x
}

print(shift_backward(c(10, 20, 30, 40, 50)))
"#;

    let rr_path = proj_dir.join("backward.rr");
    let out_path = proj_dir.join("backward.R");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let code = compile_rr(&rr_path, &out_path, "-O1");
    assert!(
        !code.contains("rr_shift_assign("),
        "backward same-base shift must keep scalar semantics"
    );

    let Some(rscript) = rscript_path() else {
        return;
    };
    if !rscript_available(&rscript) {
        return;
    }

    let ref_path = proj_dir.join("backward_ref.R");
    fs::write(&ref_path, ref_src).expect("failed to write reference");

    let ref_run = run_rscript(&rscript, &ref_path);
    let compiled_run = run_rscript(&rscript, &out_path);
    assert_eq!(ref_run.status, 0, "reference failed: {}", ref_run.stderr);
    assert_eq!(
        compiled_run.status, 0,
        "compiled failed: {}",
        compiled_run.stderr
    );
    assert_eq!(normalize(&ref_run.stdout), normalize(&compiled_run.stdout));
}
