use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn partial_range_map_vectorizes_with_slice_assign() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn slice_map(n) {
  let x = seq_len(n);
  let y = seq_len(n);
  for (i in 2..n) {
    y[i] = x[i] + 1;
  }
  return y;
}
"#;

    let rr_path = out_dir.join("slice_map.rr");
    let out_path = out_dir.join("slice_map.R");
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
        code.contains("rr_assign_slice("),
        "expected slice assignment helper in vectorized output"
    );
}
