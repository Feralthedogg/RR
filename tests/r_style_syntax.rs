use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
struct RunResult {
    status: i32,
    stdout: String,
    stderr: String,
}

fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
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

fn run_rscript(path: &str, script: &Path) -> RunResult {
    let output = Command::new(path)
        .arg("--vanilla")
        .arg(script)
        .output()
        .expect("failed to execute Rscript");
    RunResult {
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

fn compile_rr(rr_bin: &Path, rr_src: &Path, out_path: &Path, level: &str) {
    let status = Command::new(rr_bin)
        .arg(rr_src)
        .arg("-o")
        .arg(out_path)
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

#[test]
fn r_style_function_assignment_and_operator_aliases_work() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping r_style_syntax test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("r_style_syntax");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/r_style_syntax");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
add <- function(a, b) {
  a + b
}

main <- function() {
  x <- add(1L, 2L)
  y = add(3L, 4L)
  print(x)
  print(y)
  y
}

print(main())
"#;

    let rr_path = out_dir.join("r_style_syntax.rr");
    fs::write(&rr_path, rr_src).expect("failed to write source");

    let o0 = out_dir.join("r_style_syntax_o0.R");
    let o1 = out_dir.join("r_style_syntax_o1.R");
    let o2 = out_dir.join("r_style_syntax_o2.R");

    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o1, "-O1");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let base = run_rscript(&rscript, &o0);
    let run_o1 = run_rscript(&rscript, &o1);
    let run_o2 = run_rscript(&rscript, &o2);

    assert_eq!(base.status, 0, "unexpected O0 failure: {}", base.stderr);
    assert_eq!(base.status, run_o1.status, "status mismatch O0 vs O1");
    assert_eq!(base.status, run_o2.status, "status mismatch O0 vs O2");
    assert_eq!(
        normalize(&base.stdout),
        normalize(&run_o1.stdout),
        "stdout mismatch O0 vs O1"
    );
    assert_eq!(
        normalize(&base.stdout),
        normalize(&run_o2.stdout),
        "stdout mismatch O0 vs O2"
    );
    assert_eq!(
        normalize(&base.stderr),
        normalize(&run_o1.stderr),
        "stderr mismatch O0 vs O1"
    );
    assert_eq!(
        normalize(&base.stderr),
        normalize(&run_o2.stderr),
        "stderr mismatch O0 vs O2"
    );

    let expected = "[1] 3\n[1] 7\n[1] 7\n";
    assert_eq!(
        normalize(&base.stdout),
        expected,
        "unexpected r-style syntax semantics"
    );
}
