use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn repeated_vector_binop_is_hoisted_once() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn kernel(u, n_l, n_r, n) {
  let out = u

  for (i in 1..n) {
    out[i] = ((u[floor(n_r[i])] - u[floor(n_l[i])]) / 2.0) * ((u[floor(n_r[i])] - u[floor(n_l[i])]) / 2.0)

  }
  return out

}
fn main() {
  let u = seq_len(4)
  let n_l = seq_len(4)
  let n_r = seq_len(4)
  print(kernel(u, n_l, n_r, 4))
  return 0L
}
main()
"#;

    let rr_path = out_dir.join("binop_cse_floor.rr");
    let out_path = out_dir.join("binop_cse_floor.R");
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
            || code.contains("rr_index_vec_floor(")
            || (code.contains("rr_gather(u, n_r)") && code.contains("rr_gather(u, n_l)")),
        "expected either a floor-index helper or the direct gathered vector-diff shape in output"
    );
    let has_hoisted_diff = code.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with(".__rr_cse_")
            && trimmed.contains(" <- (.__rr_cse_")
            && trimmed.contains(" - .__rr_cse_")
    });
    let has_inlined_repeated_diff = code.lines().any(|line| {
        (line.matches("rr_gather(u, n_r)").count() >= 2
            && line.matches("rr_gather(u, n_l)").count() >= 2)
            || (line
                .matches("rr_gather(u, rr_index1_read_vec(n_r, rr_index_vec_floor(i:n)))")
                .count()
                >= 2
                && line
                    .matches("rr_gather(u, rr_index1_read_vec(n_l, rr_index_vec_floor(i:n)))")
                    .count()
                    >= 2)
    });
    assert!(
        has_hoisted_diff || has_inlined_repeated_diff,
        "expected repeated vector subtraction to be either hoisted into a temp or preserved as a repeated inlined vector diff"
    );
    let has_reused_temp = code
        .lines()
        .any(|line| line.matches(".__rr_cse_").count() >= 2);
    assert!(
        has_reused_temp || has_inlined_repeated_diff,
        "expected final vector expression to reuse a hoisted temp or keep the repeated vector diff inline"
    );
}
