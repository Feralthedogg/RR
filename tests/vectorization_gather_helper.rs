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
fn indirect_param_gather_uses_rr_gather_helper() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("vectorization_gather_helper");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "gather");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_src = r#"
fn gather_map(src, idx) {
  let out = seq_len(length(idx))
  for (i in 1..length(out)) {
    out[i] = src[idx[i]]
  }
  return out
}

print(gather_map(c(10, 20, 30, 40), c(4, 2, 3, 1)))
"#;
    let ref_src = r#"
gather_map <- function(src, idx) {
  out <- seq_len(length(idx))
  for (i in seq_len(length(out))) {
    out[i] <- src[idx[i]]
  }
  out
}

print(gather_map(c(10, 20, 30, 40), c(4, 2, 3, 1)))
"#;

    let rr_path = proj_dir.join("gather.rr");
    let out_path = proj_dir.join("gather.R");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let code = compile_rr(&rr_path, &out_path, "-O1");
    assert!(
        code.contains("rr_gather("),
        "expected indirect gather to lower through rr_gather(...)"
    );

    let Some(rscript) = rscript_path() else {
        return;
    };
    if !rscript_available(&rscript) {
        return;
    }

    let ref_path = proj_dir.join("gather_ref.R");
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
