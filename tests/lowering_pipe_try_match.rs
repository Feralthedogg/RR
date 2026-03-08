mod common;

use common::{rscript_available, rscript_path, unique_dir};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn pipe_try_match_lowering_is_implemented_without_unimplemented_logs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("lowering_pipe_try_match");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "case");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_src = r#"
fn add1(x) {
  return x + 1
}

fn main() {
  let x = 4
  let y = x |> add1()
  let z = (y + 1)?
  let m = match (z) { 6 => 10, _ => 0 }
  print(m)
  return m
}
main()
"#;

    let rr_path = proj_dir.join("case.rr");
    let out_path = proj_dir.join("case.R");
    fs::write(&rr_path, rr_src).expect("failed to write case.rr");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let output = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .output()
        .expect("failed to run RR compiler");

    assert!(
        output.status.success(),
        "compile failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Lowering unimplemented ExprKind"),
        "unexpected HIR lowering fallback log:\n{}",
        stderr
    );
    assert!(
        !stderr.contains("UNIMPLEMENTED Match Arm"),
        "unexpected MIR lowering fallback log:\n{}",
        stderr
    );

    if let Some(rscript) = rscript_path().filter(|p| rscript_available(p)) {
        let run = Command::new(rscript)
            .arg("--vanilla")
            .arg(&out_path)
            .output()
            .expect("failed to execute Rscript");
        assert!(
            run.status.success(),
            "generated R failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        );
        let stdout = String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n");
        assert!(
            stdout.contains("[1] 10"),
            "unexpected runtime output:\n{}",
            stdout
        );
    }
}
