use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn floor_index_entry_canonicalization_is_skipped_when_callsites_prove_int_vector() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn mk_idx(n) {
  return seq_len(n)

}

fn gather(a, idx, n) {
  let out = a

  for (i in 1..n) {
    out[i] = a[floor(idx[i])]

  }
  return out

}

fn main(n) {
  let a = seq_len(n)

  return gather(a, mk_idx(n), n)

}

print(main(4))
"#;

    let rr_path = out_dir.join("index_param_skip_when_proven.rr");
    let out_path = out_dir.join("index_param_skip_when_proven.R");
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
    let sig_pos = code
        .find("function(a, idx)")
        .expect("expected preserved gather-like function signature");
    let tail = &code[sig_pos..];
    let fn_end = tail.find("\nSym_").unwrap_or(tail.len());
    let fn_code = &tail[..fn_end];

    assert!(
        !fn_code.contains("idx <- rr_index_vec_floor(idx)")
            && !fn_code.contains(".arg_idx <- rr_index_vec_floor(.arg_idx)"),
        "did not expect entry floor canonicalization when callsites already prove int-vector idx"
    );
    assert!(
        fn_code.contains("return(rr_gather(a, idx))")
            || fn_code.contains("return(rr_gather(.arg_a, .arg_idx))"),
        "expected optimized gather path to skip floor canonicalization entirely when idx is proven integer"
    );
    assert!(
        !fn_code.contains("rr_index_vec_floor("),
        "expected floor wrapper to be omitted entirely when idx is already proven int-vector"
    );
}
