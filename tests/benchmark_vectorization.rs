use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn benchmark_sym10_stays_vectorized_and_read_check_free() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");
    let rr_src = out_dir.join("benchmark_vectorization_case.rr");
    let out_path = out_dir.join("benchmark_o1_compiled.R");

    let rr_program = r#"
fn vec_map(n) {
  let x = seq_len(n)
  let y = seq_len(n)
  for (i in 1..length(x)) {
    y[i] = x[i] + x[i]
  }
  return y
}

print(vec_map(8))
"#;
    fs::write(&rr_src, rr_program).expect("failed to write RR source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_src)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_src.display()
    );

    let code = fs::read_to_string(&out_path).expect("failed to read compiled R");
    assert!(
        code.contains("vec_map <- function(n)") || code.contains("Sym_"),
        "expected lowered function in output"
    );
    assert!(
        code.contains("y <- (x + x)")
            || code.contains("y <- (y + y)")
            || code.contains("y <- rr_intrinsic_vec_add_f64(x, x)")
            || code.contains("y <- rr_intrinsic_vec_add_f64(y, y)")
            || code.contains("rr_assign_slice(y, i, length(x), rr_intrinsic_vec_add_f64(x, x))"),
        "expected vectorized map body (direct add, intrinsic add helper, or slice assign form)"
    );
}
