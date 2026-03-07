use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
fn dynamic_builtin_marks_hybrid_fallback_and_executes() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn dyn_eval(x) {
  return eval(x);
}

fn plain(x) {
  return x + 1;
}

print(dyn_eval(41));
print(plain(1));
"#;

    let rr_path = out_dir.join("hybrid_fallback.rr");
    let out_path = out_dir.join("hybrid_fallback.R");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
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
        code.contains("rr-hybrid-fallback"),
        "expected hybrid fallback marker in generated R"
    );
    assert!(
        code.contains("eval("),
        "expected dynamic builtin call to remain as eval(...)"
    );

    if let Some(rscript) = rscript_path().filter(|p| rscript_available(p)) {
        let (status, stdout) = run_rscript(&rscript, &out_path);
        assert_eq!(status, 0, "Rscript execution failed");
        assert!(
            stdout.contains("[1] 41"),
            "expected dyn_eval output in stdout, got: {}",
            stdout
        );
        assert!(
            stdout.contains("[1] 2"),
            "expected plain output in stdout, got: {}",
            stdout
        );
    }
}
