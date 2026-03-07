mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;

#[test]
fn user_defined_builtins_shadow_base_r_calls() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping builtin_shadowing test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("builtin_shadowing");
    fs::create_dir_all(&out_dir).expect("failed to create builtin_shadowing dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
fn round(x) {
  r = x % 1.0
  if (r >= 0.5) {
    return (x - r) + 1.0
  }
  return x - r
}

fn length(xs) {
  return 42L
}

fn main() {
  xs = [1L, 2L, 3L]
  print(round(2.5))
  print(length(xs))
  return 0L
}

print(main())
"#;

    let rr_path = out_dir.join("builtin_shadowing.rr");
    fs::write(&rr_path, rr_src).expect("failed to write builtin_shadowing source");

    let o0 = out_dir.join("builtin_shadowing_o0.R");
    let o1 = out_dir.join("builtin_shadowing_o1.R");
    let o2 = out_dir.join("builtin_shadowing_o2.R");

    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o1, "-O1");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let base = run_rscript(&rscript, &o0);
    let run_o1 = run_rscript(&rscript, &o1);
    let run_o2 = run_rscript(&rscript, &o2);

    assert_eq!(base.status, 0, "unexpected O0 failure: {}", base.stderr);
    assert_eq!(base.status, run_o1.status, "status mismatch O0 vs O1");
    assert_eq!(base.status, run_o2.status, "status mismatch O0 vs O2");
    assert_eq!(
        normalize(&base.stdout),
        normalize(&run_o1.stdout),
        "stdout mismatch O0 vs O1"
    );
    assert_eq!(
        normalize(&base.stdout),
        normalize(&run_o2.stdout),
        "stdout mismatch O0 vs O2"
    );
    assert_eq!(
        normalize(&base.stderr),
        normalize(&run_o1.stderr),
        "stderr mismatch O0 vs O1"
    );
    assert_eq!(
        normalize(&base.stderr),
        normalize(&run_o2.stderr),
        "stderr mismatch O0 vs O2"
    );

    let expected = "[1] 3\n[1] 42\n[1] 0\n";
    assert_eq!(
        normalize(&base.stdout),
        expected,
        "user-defined round/length should shadow base R builtins"
    );
}
