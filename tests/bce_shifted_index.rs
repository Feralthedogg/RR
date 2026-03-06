use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn shifted_index_read_is_bce_optimized() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn shifted(x) {
  let y = seq_along(x);
  for (i in 1..(length(x) - 1)) {
    y[i] = x[i + 1];
  }
  return y;
}
"#;

    let rr_path = out_dir.join("shifted_index_bce.rr");
    let out_path = out_dir.join("shifted_index_bce.R");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let code = fs::read_to_string(&out_path).expect("failed to read compiled R");
    assert!(
        code.contains("rr_shift_assign("),
        "expected shifted loop vectorized via rr_shift_assign(...)"
    );
    let mut in_shifted = false;
    let mut has_rr_index_read_call = false;
    for line in code.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("shifted <- function") {
            in_shifted = true;
            continue;
        }
        if in_shifted && trimmed == "}" {
            break;
        }
        if in_shifted && line.contains("rr_index1_read(") {
            has_rr_index_read_call = true;
            break;
        }
    }
    assert!(
        !has_rr_index_read_call,
        "expected no rr_index1_read call sites in shifted vectorized code"
    );
}
