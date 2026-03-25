mod common;

use common::compile_rr;
use std::fs;
use std::path::PathBuf;

#[test]
fn scalar_non_na_comparison_condition_skips_truthy_wrapper() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("condition_truthy_elision");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/condition_truthy_elision");

    let rr_path = out_dir.join("main.rr");
    let out_path = out_dir.join("main.R");
    fs::write(
        &rr_path,
        r#"
fn main() {
  let pass = 1L
  while (pass <= 3L) {
    print(pass)
    pass = pass + 1L
  }
}

main()
"#,
    )
    .expect("failed to write RR source");

    compile_rr(&rr_bin, &rr_path, &out_path, "-O2");
    let code = fs::read_to_string(&out_path).expect("failed to read generated R");

    assert!(
        code.contains("if (!(pass <= 3L)) break") || code.contains("for (pass in seq_len(3L)) {"),
        "expected scalar comparison loop guard to emit directly without truthy helper"
    );
    assert!(
        !code.contains("rr_truthy1((pass <= 3L), \"condition\")"),
        "unexpected rr_truthy1 wrapper remained for scalar non-NA comparison"
    );
}

#[test]
fn branch_joined_scalar_comparison_condition_skips_truthy_wrapper() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("condition_truthy_elision");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/condition_truthy_elision");

    let rr_path = out_dir.join("joined_main.rr");
    let out_path = out_dir.join("joined_main.R");
    fs::write(
        &rr_path,
        r#"
fn main() {
  let f_curr = 5L
  let f = 0L
  if (f_curr == 5L) {
    f = f_curr
  } else {
    f = f_curr
  }
  if (f < 6L) {
    print(f)
  }
}

main()
"#,
    )
    .expect("failed to write RR source");

    compile_rr(&rr_bin, &rr_path, &out_path, "-O2");
    let code = fs::read_to_string(&out_path).expect("failed to read generated R");

    assert!(
        code.contains("if ((f < 6L)) {") || code.contains("print(f)"),
        "expected joined scalar comparison to emit directly or fold away entirely"
    );
    assert!(
        !code.contains("if (rr_truthy1((f < 6L), \"condition\")) {")
            && !code.contains("rr_truthy1(("),
        "unexpected rr_truthy1 wrapper remained for joined scalar comparison"
    );
}
