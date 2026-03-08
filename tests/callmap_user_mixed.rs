use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn compile_rr(rr_bin: &Path, rr_src: &Path, out: &Path, level: &str) {
    let status = Command::new(rr_bin)
        .arg(rr_src)
        .arg("-o")
        .arg(out)
        .arg(level)
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {} ({})",
        rr_src.display(),
        level
    );
}

fn rscript_path() -> Option<String> {
    if let Ok(path) = std::env::var("RRSCRIPT")
        && !path.trim().is_empty()
    {
        return Some(path);
    }
    Some("Rscript".to_string())
}

fn rscript_available(path: &str) -> bool {
    Command::new(path)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_rscript(path: &str, script: &Path) -> (i32, String) {
    let output = Command::new(path)
        .arg("--vanilla")
        .arg(script)
        .output()
        .expect("failed to execute Rscript");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
    )
}

#[test]
fn callmap_supports_user_and_builtin_mixed_chain() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("callmap_user_mixed");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");

    let rr_src = r#"
fn g(u) {
  return sqrt(u)
}

fn f(a, b) {
  return pmax(log(a), b)
}

fn mixed(n) {
  let x = seq_len(n)
  let z = seq_len(n) + 5
  let y = seq_len(n)
  for (i in 1..length(x)) {
    y[i] = f(x[i] + 1, g(z[i]))
  }
  return y
}

print(mixed(8))
"#;
    let rr_path = out_dir.join("callmap_user_mixed.rr");
    fs::write(&rr_path, rr_src).expect("failed to write source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let o0 = out_dir.join("callmap_user_mixed_o0.R");
    let o1 = out_dir.join("callmap_user_mixed_o1.R");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o1, "-O1");

    let o1_code = fs::read_to_string(&o1).expect("failed to read O1 output");
    assert!(o1_code.contains("pmax("), "expected pmax in generated code");
    assert!(o1_code.contains("log("), "expected log in generated code");
    assert!(o1_code.contains("sqrt("), "expected sqrt in generated code");
    assert!(
        !o1_code.contains("repeat {"),
        "expected loop to be vectorized (no scalar repeat loop)"
    );

    if let Some(rscript) = rscript_path().filter(|p| rscript_available(p)) {
        let (s0, out0) = run_rscript(&rscript, &o0);
        let (s1, out1) = run_rscript(&rscript, &o1);
        assert_eq!(s0, 0, "O0 execution failed");
        assert_eq!(s1, 0, "O1 execution failed");
        assert_eq!(out0.replace("\r\n", "\n"), out1.replace("\r\n", "\n"));
    }
}
