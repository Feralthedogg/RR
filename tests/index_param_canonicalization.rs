use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn floor_index_param_is_canonicalized_once() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn gather(a, idx, n) {
  let out = a

  for (i in 1..n) {
    out[i] = a[floor(idx[i])]

  }
  return out

}

print(gather(seq_len(4), seq_len(4) + 0.25, 4))
"#;

    let rr_path = out_dir.join("index_param_canonicalization.rr");
    let out_path = out_dir.join("index_param_canonicalization.R");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("--preserve-all-defs")
        .arg("-O2")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let code = fs::read_to_string(&out_path).expect("failed to read compiled R");
    let fn_pos = code
        .find("Sym_1 <- function")
        .expect("expected compiled function Sym_1");
    let fn_end = code[fn_pos + 1..]
        .find("\nSym_")
        .map(|idx| fn_pos + 1 + idx)
        .unwrap_or(code.len());
    let fn_code = &code[fn_pos..fn_end];

    assert!(
        fn_code.contains("idx <- rr_index_vec_floor(idx)")
            || fn_code.contains("return(rr_gather(a, rr_index_vec_floor(idx)))")
            || fn_code.contains(".arg_idx <- rr_index_vec_floor(.arg_idx)"),
        "expected one-time index vector canonicalization for floor-index parameter"
    );
    assert_eq!(
        fn_code.matches("rr_index_vec_floor(").count(),
        1,
        "expected exactly one floor-index canonicalization in the preserved gather body"
    );
    assert!(
        !fn_code.contains("rr_index1_read_vec(out, rr_index_vec_floor("),
        "expected floor wrapper to be removed from gather read after canonicalization"
    );
    assert!(
        fn_code.contains("return(rr_gather(a, rr_index_vec_floor(idx)))")
            || fn_code.contains("return(rr_gather(.arg_a, rr_index_vec_floor(.arg_idx)))"),
        "expected gather to lower through a single canonicalized whole-vector gather"
    );
}
