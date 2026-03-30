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
fn p0_metadata_and_ordering_builtins_match_across_opt_levels() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping p0 metadata runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("base_p0_metadata");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");

    let rr_src = out_dir.join("case.rr");
    let o0_path = out_dir.join("case_o0.R");
    let o2_path = out_dir.join("case_o2.R");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r * as base from "base"
import r { match as base_match } from "base"

fn main() {
  let named = c(alpha = 3L, beta = 1L, gamma = 2L, delta = 1L)
  let mat = matrix(
    c(1.0, 2.0, 3.0, 4.0),
    2L,
    2L,
    dimnames = list(c("r1", "r2"), c("c1", "c2"))
  )

  print(base.names(named))
  print(base.rownames(mat))
  print(base.colnames(mat))
  print(base.sort(c(3L, 1L, 2L, 1L)))
  print(base.order(c(3L, 1L, 2L, 1L)))
  print(base.match(c(3L, 1L), c(1L, 2L, 3L)))
  print(base_match(c(3L, 1L), c(1L, 2L, 3L)))
  print(base.unique(c(1L, 2L, 1L, 2L)))
  print(base.duplicated(c(1L, 2L, 1L, 2L)))
  print(base.anyDuplicated(c(1L, 2L, 1L, 2L)))
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
