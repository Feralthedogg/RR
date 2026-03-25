use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn scalar_floor_index_read_is_lowered_to_idx_helper() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn scalar_idx_kernel(n) {
  let src = seq_len(n) + 0.25

  let x = seq_len(n) * 10

  let out = seq_len(n)

  let i = 1.0

  while (i <= n) {
    let ii = floor(i)

    let j = floor(src[ii])

    out[ii] = x[j]

    i = i + 1.0

  }
  return out

}

print(scalar_idx_kernel(4))
"#;

    let rr_path = out_dir.join("scalar_idx_kernel.rr");
    let out_path = out_dir.join("scalar_idx_kernel.R");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O0")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let code = fs::read_to_string(&out_path).expect("failed to read compiled R");
    assert!(
        code.contains("rr_index1_read_idx("),
        "expected scalar floor-index read helper in output"
    );
}
