mod common;

use common::run_rscript;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn rscript_path() -> Option<String> {
    if let Ok(path) = std::env::var("RRSCRIPT")
        && !path.trim().is_empty()
    {
        return Some(path);
    }
    Some("Rscript".to_string())
}

fn rscript_available(path: &str) -> bool {
    Command::new(path)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn p0_base_builtins_compile_and_match_across_opt_levels() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping p0 base builtins runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("base_p0_builtins");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");

    let rr_src = out_dir.join("case.rr");
    let o0_path = out_dir.join("case_o0.R");
    let o2_path = out_dir.join("case_o2.R");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
fn main() {
  let chars = character(3L)
  let flags = logical(4L)
  let ids = integer(2L)
  let vals = double(2L)
  let dup = rep(2L, 4L)
  let keep = any(c(FALSE, TRUE, FALSE))
  let ok = all(c(TRUE, TRUE, TRUE))
  let picks = which(c(FALSE, TRUE, FALSE, TRUE))
  let p = prod(c(2L, 3L, 4L))
  let v = var(c(2.0, 4.0, 6.0))

  print(length(chars))
  print(length(flags))
  print(length(ids))
  print(length(vals))
  print(dup)
  print(keep)
  print(ok)
  print(picks)
  print(p)
  print(v)
}

main()
"#;

    fs::write(&rr_src, src).expect("failed to write RR source");

    let status_o0 = Command::new(&rr_bin)
        .arg(&rr_src)
        .arg("-o")
        .arg(&o0_path)
        .arg("-O0")
        .status()
        .expect("failed to compile O0 case");
    assert!(status_o0.success(), "O0 compile failed");

    let status_o2 = Command::new(&rr_bin)
        .arg(&rr_src)
        .arg("-o")
        .arg(&o2_path)
        .arg("-O2")
        .status()
        .expect("failed to compile O2 case");
    assert!(status_o2.success(), "O2 compile failed");

    let o0 = run_rscript(&rscript, &o0_path);
    let o2 = run_rscript(&rscript, &o2_path);

    assert_eq!(o0.status, 0, "O0 runtime failed:\n{}", o0.stderr);
    assert_eq!(o2.status, 0, "O2 runtime failed:\n{}", o2.stderr);
    assert_eq!(o0.stdout, o2.stdout, "O0/O2 stdout mismatch");
    assert_eq!(o0.stderr, o2.stderr, "O0/O2 stderr mismatch");
}
