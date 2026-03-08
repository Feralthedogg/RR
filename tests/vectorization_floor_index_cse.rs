use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn floor_index_vectorization_reuses_index_floor() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn floor_idx_cse(a, b, n_l, n_r, n) {
  let out = a

  for (i in 1..n) {
    out[i] = (a[floor(n_r[i])] + b[floor(n_r[i])]) - (a[floor(n_l[i])] + b[floor(n_l[i])])

  }
  return out

}
"#;

    let rr_path = out_dir.join("floor_idx_cse.rr");
    let out_path = out_dir.join("floor_idx_cse.R");
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
    let floor_r = code.matches("n_r <- rr_index_vec_floor(n_r)").count();
    let floor_l = code.matches("n_l <- rr_index_vec_floor(n_l)").count();
    assert_eq!(
        floor_r, 1,
        "expected n_r to be canonicalized once, got {}",
        floor_r
    );
    assert_eq!(
        floor_l, 1,
        "expected n_l to be canonicalized once, got {}",
        floor_l
    );
    assert!(
        code.contains("rr_index1_read_vec("),
        "expected vector gather read calls in output"
    );
    assert!(
        !code.contains("rr_index1_read_vec(.arg_a, rr_index_vec_floor(")
            && !code.contains("rr_index1_read_vec(.arg_b, rr_index_vec_floor("),
        "expected floor wrapper to be removed from gather reads"
    );
}
