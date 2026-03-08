use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn o0_codegen_handles_phi_by_running_mandatory_de_ssa() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");
    let rr_src = out_dir.join("o0_codegen_smoke_case.rr");
    let out_path = out_dir.join("benchmark_o0_compiled.R");
    std::fs::create_dir_all(out_path.parent().expect("missing parent"))
        .expect("failed to create target/tests");

    let rr_program = r#"
fn sum_loop(n) {
  let x = seq_len(n)

  let s = 0L

  for (i in 1..length(x)) {
    s = s + x[i]

  }
  return s

}

print(sum_loop(8))

"#;
    fs::write(&rr_src, rr_program).expect("failed to write RR source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_src)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O0")
        .status()
        .expect("failed to run RR compiler");

    assert!(
        status.success(),
        "O0 compile failed for {}",
        rr_src.display()
    );
    let code = std::fs::read_to_string(&out_path).expect("failed to read O0 output");
    assert!(
        code.contains("<- function("),
        "expected at least one lowered function in O0 output"
    );
    assert!(
        !code.contains("Phi should have been eliminated before codegen"),
        "O0 output should not contain unresolved phi diagnostics"
    );
}
