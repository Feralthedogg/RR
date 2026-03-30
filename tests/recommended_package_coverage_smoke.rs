mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn recommended_package_coverage_script_writes_reports() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("recommended_package_coverage_smoke");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let out_dir = common::unique_dir(&sandbox_root, "out");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");

    let script = root.join("scripts").join("recommended_package_coverage.sh");
    let output = Command::new("bash")
        .arg(&script)
        .arg(&out_dir)
        .env("RR_RECOMMENDED_PACKAGES", "boot,class")
        .output()
        .expect("failed to run recommended package coverage script");
    assert!(
        output.status.success(),
        "recommended package coverage script failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json = fs::read_to_string(out_dir.join("recommended-package-coverage.json"))
        .expect("failed to read recommended package coverage json");
    let md = fs::read_to_string(out_dir.join("recommended-package-coverage.md"))
        .expect("failed to read recommended package coverage markdown");

    assert!(json.contains("\"schema\": \"rr-recommended-package-coverage\""));
    assert!(json.contains("\"package\": \"boot\""));
    assert!(json.contains("\"package\": \"class\""));
    assert!(json.contains("\"totals\""));

    assert!(md.contains("# Recommended Package Coverage"));
    assert!(md.contains("| `boot` |"));
    assert!(md.contains("| `class` |"));
}
