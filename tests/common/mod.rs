#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct RunResult {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
}

pub fn unique_dir(root: &Path, name: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    root.join(format!("{}_{}_{}", name, std::process::id(), ts))
}

pub fn rscript_path() -> Option<String> {
    if let Ok(path) = std::env::var("RRSCRIPT")
        && !path.trim().is_empty() {
            return Some(path);
        }
    Some("Rscript".to_string())
}

pub fn rscript_available(path: &str) -> bool {
    Command::new(path)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn run_rscript(path: &str, script: &Path) -> RunResult {
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

pub fn compile_rr(rr_bin: &Path, rr_src: &Path, out_path: &Path, level: &str) {
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

pub fn run_compile_case(
    suite: &str,
    source: &str,
    file_name: &str,
    level: &str,
    env_kv: &[(&str, &str)],
) -> (bool, String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join(suite);
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let rr_path = proj_dir.join(file_name);
    let out_path = proj_dir.join("out.R");
    fs::write(&rr_path, source).expect("failed to write source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let mut cmd = Command::new(rr_bin);
    cmd.arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg(level);
    for (k, v) in env_kv {
        cmd.env(k, v);
    }
    let output = cmd.output().expect("failed to run RR");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}
