use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn floor_index_map_uses_floor_read_helper() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn floor_gather(n) {
  let src = seq_len(n) * 2;
  let idx = seq_len(n) + 0.25;
  let out = seq_len(n);
  for (i in 1..length(out)) {
    out[i] = src[floor(idx[i])];
  }
  return out;
}
"#;

    let rr_path = out_dir.join("floor_gather.rr");
    let out_path = out_dir.join("floor_gather.R");
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
        code.contains("rr_index1_read_vec_floor(")
            || (code.contains("rr_index_vec_floor(") && code.contains("rr_index1_read_vec(")),
        "expected vector floor-gather helper in output"
    );
    assert!(
        code.contains("rr_index1_read_vec_floor(") || code.contains("rr_index1_read_vec("),
        "expected vector read call in output"
    );
    assert!(
        !code.contains("rr_index1_read_vec(src, floor("),
        "expected floor wrapper to be absorbed before vector read"
    );
}
