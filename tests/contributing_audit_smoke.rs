mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn contributing_audit_reports_static_violations_and_skips_cfg_test_tail() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("contributing_audit_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let sandbox = common::unique_dir(&sandbox_root, "audit");
    fs::create_dir_all(sandbox.join("src")).expect("failed to create sandbox src");

    let bad_file = sandbox.join("src").join("bad.rs");
    fs::write(
        &bad_file,
        r#"
fn bad() {
    let _ = Some(1).expect("boom");
    unsafe { side_effect(); }
}
"#,
    )
    .expect("failed to write bad file");

    let script = root.join("scripts").join("contributing_audit.sh");
    let bad_output = Command::new("bash")
        .arg(&script)
        .arg("--scan-only")
        .arg("--files")
        .arg(&bad_file)
        .output()
        .expect("failed to execute contributing audit script for bad file");
    assert!(
        !bad_output.status.success(),
        "bad audit input should fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&bad_output.stdout),
        String::from_utf8_lossy(&bad_output.stderr)
    );
    let bad_stdout = String::from_utf8_lossy(&bad_output.stdout);
    assert!(bad_stdout.contains("error[production-unwrap]"));
    assert!(bad_stdout.contains("error[unsafe-missing-safety]"));

    let good_file = sandbox.join("src").join("good.rs");
    fs::write(
        &good_file,
        r#"
fn good() -> i32 {
    1
}

struct Parser;

impl Parser {
    fn expect(&self, _token: i32) {}
}

fn parser_style_expect_is_allowed(p: &Parser) {
    p.expect(1);
}

#[cfg(test)]
mod tests {
    #[test]
    fn allows_test_only_unwrap() {
        let _ = Some(1).unwrap();
    }
}
"#,
    )
    .expect("failed to write good file");

    let good_output = Command::new("bash")
        .arg(&script)
        .arg("--scan-only")
        .arg("--files")
        .arg(&good_file)
        .output()
        .expect("failed to execute contributing audit script for good file");
    assert!(
        good_output.status.success(),
        "good audit input should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&good_output.stdout),
        String::from_utf8_lossy(&good_output.stderr)
    );
    let good_stdout = String::from_utf8_lossy(&good_output.stdout);
    assert!(good_stdout.contains("no static findings"));
    assert!(good_stdout.contains("result: PASS (scan-only)"));
}
