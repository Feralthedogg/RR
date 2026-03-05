use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn compile_rr(rr_bin: &Path, rr_src: &Path, out: &Path, level: &str) {
    let status = Command::new(rr_bin)
        .arg(rr_src)
        .arg("-o")
        .arg(out)
        .arg("--no-runtime")
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
        && !path.trim().is_empty() {
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

fn run_rscript(path: &str, script: &Path) -> (i32, String, String) {
    let output = Command::new(path)
        .arg("--vanilla")
        .arg(script)
        .output()
        .expect("failed to execute Rscript");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
}

#[test]
fn lambda_and_closure_calls_work_end_to_end() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping lambda closure test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("lambda_closure");
    fs::create_dir_all(&out_dir).expect("failed to create test output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
fn apply_twice(f, x) {
  return f(f(x));
}

fn main() {
  let inc = fn(v) { return v + 1; };
  let seed = 10;
  let add_seed = fn(v) { return v + seed; };
  let r1 = inc(2);
  let r2 = add_seed(5);
  let r3 = apply_twice(inc, 3);
  let r4 = (fn(a) { return a * 2; })(6);
  print(r1);
  print(r2);
  print(r3);
  print(r4);
  return r1 + r2 + r3 + r4;
}

print(main());
"#;

    let rr_path = out_dir.join("lambda_closure.rr");
    fs::write(&rr_path, rr_src).expect("failed to write source");

    let o0 = out_dir.join("lambda_closure_o0.R");
    let o2 = out_dir.join("lambda_closure_o2.R");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let (s0, out0, err0) = run_rscript(&rscript, &o0);
    let (s2, out2, err2) = run_rscript(&rscript, &o2);

    assert_eq!(s0, 0, "O0 execution failed:\n{}", err0);
    assert_eq!(s2, 0, "O2 execution failed:\n{}", err2);
    assert_eq!(
        normalize(&out0),
        normalize(&out2),
        "stdout mismatch O0 vs O2"
    );
    assert_eq!(
        normalize(&err0),
        normalize(&err2),
        "stderr mismatch O0 vs O2"
    );

    let expected = "[1] 3\n[1] 15\n[1] 5\n[1] 12\n[1] 35\n";
    assert_eq!(normalize(&out0), expected, "unexpected runtime output");
}
