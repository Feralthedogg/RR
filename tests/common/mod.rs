#![allow(dead_code)]

pub mod random_error_cases;
pub mod random_rr;

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

#[derive(Clone, Debug)]
pub struct RunResult {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
}

pub fn unique_dir(root: &Path, name: &str) -> PathBuf {
    static UNIQUE_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let seq = UNIQUE_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    root.join(format!("{}_{}_{}", name, std::process::id(), seq))
}

pub fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub fn set_env_var_for_test(_guard: &MutexGuard<'_, ()>, key: &str, value: impl AsRef<OsStr>) {
    // SAFETY: Tests call this only while holding env_lock(), which serializes
    // process-environment mutation for the current test process. Safe
    // alternatives are insufficient because std::env provides no scoped,
    // test-local override API.
    unsafe {
        std::env::set_var(key, value);
    }
}

pub fn remove_env_var_for_test(_guard: &MutexGuard<'_, ()>, key: &str) {
    // SAFETY: Tests call this only while holding env_lock(), which serializes
    // process-environment mutation for the current test process. Safe
    // alternatives are insufficient because std::env provides no scoped,
    // test-local restore API.
    unsafe {
        std::env::remove_var(key);
    }
}

pub struct ScopedCurrentDir {
    prev: PathBuf,
}

impl Drop for ScopedCurrentDir {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.prev).expect("failed to restore current dir");
    }
}

pub fn set_current_dir_for_test(_guard: &MutexGuard<'_, ()>, path: &Path) -> ScopedCurrentDir {
    let prev = std::env::current_dir().expect("failed to read current dir");
    std::env::set_current_dir(path).expect("failed to set current dir for test");
    ScopedCurrentDir { prev }
}

pub fn rscript_path() -> Option<String> {
    if let Ok(path) = std::env::var("RRSCRIPT")
        && !path.trim().is_empty()
    {
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
    compile_rr_env(rr_bin, rr_src, out_path, level, &[]);
}

pub fn compile_rr_env(
    rr_bin: &Path,
    rr_src: &Path,
    out_path: &Path,
    level: &str,
    env_kv: &[(&str, &str)],
) {
    compile_rr_env_with_args(rr_bin, rr_src, out_path, level, &[], env_kv);
}

pub fn compile_rr_env_with_args(
    rr_bin: &Path,
    rr_src: &Path,
    out_path: &Path,
    level: &str,
    extra_args: &[&str],
    env_kv: &[(&str, &str)],
) {
    let mut cmd = Command::new(rr_bin);
    cmd.arg(rr_src)
        .arg("-o")
        .arg(out_path)
        .arg(level)
        .arg("--cold");
    for arg in extra_args {
        cmd.arg(arg);
    }
    for arg in compile_env_args(env_kv) {
        cmd.arg(arg);
    }
    for (k, v) in env_kv {
        if !is_compile_config_env(k) {
            cmd.env(k, v);
        }
    }
    let status = cmd.status().expect("failed to run RR compiler");
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
    cmd.arg(&rr_path).arg("-o").arg(&out_path).arg(level);
    for arg in compile_env_args(env_kv) {
        cmd.arg(arg);
    }
    for (k, v) in env_kv {
        if !is_compile_config_env(k) {
            cmd.env(k, v);
        }
    }
    let output = cmd.output().expect("failed to run RR");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn is_compile_config_env(key: &str) -> bool {
    const TYPE_MODE_ENV: &str = concat!("RR_", "TYPE_MODE");
    const NATIVE_BACKEND_ENV: &str = concat!("RR_", "NATIVE_BACKEND");
    const PARALLEL_MODE_ENV: &str = concat!("RR_", "PARALLEL_MODE");
    const PARALLEL_BACKEND_ENV: &str = concat!("RR_", "PARALLEL_BACKEND");
    const PARALLEL_THREADS_ENV: &str = concat!("RR_", "PARALLEL_THREADS");
    const PARALLEL_MIN_TRIP_ENV: &str = concat!("RR_", "PARALLEL_MIN_TRIP");
    const STRICT_LET_ENV: &str = concat!("RR_", "STRICT_LET");
    const STRICT_ASSIGN_ENV: &str = concat!("RR_", "STRICT_ASSIGN");
    const WARN_IMPLICIT_DECL_ENV: &str = concat!("RR_", "WARN_IMPLICIT_DECL");
    matches!(
        key,
        TYPE_MODE_ENV
            | NATIVE_BACKEND_ENV
            | PARALLEL_MODE_ENV
            | PARALLEL_BACKEND_ENV
            | PARALLEL_THREADS_ENV
            | PARALLEL_MIN_TRIP_ENV
            | STRICT_LET_ENV
            | STRICT_ASSIGN_ENV
            | WARN_IMPLICIT_DECL_ENV
    )
}

fn compile_env_args<'a>(env_kv: &'a [(&'a str, &'a str)]) -> Vec<&'a str> {
    const TYPE_MODE_ENV: &str = concat!("RR_", "TYPE_MODE");
    const NATIVE_BACKEND_ENV: &str = concat!("RR_", "NATIVE_BACKEND");
    const PARALLEL_MODE_ENV: &str = concat!("RR_", "PARALLEL_MODE");
    const PARALLEL_BACKEND_ENV: &str = concat!("RR_", "PARALLEL_BACKEND");
    const PARALLEL_THREADS_ENV: &str = concat!("RR_", "PARALLEL_THREADS");
    const PARALLEL_MIN_TRIP_ENV: &str = concat!("RR_", "PARALLEL_MIN_TRIP");
    const STRICT_LET_ENV: &str = concat!("RR_", "STRICT_LET");
    const STRICT_ASSIGN_ENV: &str = concat!("RR_", "STRICT_ASSIGN");
    const WARN_IMPLICIT_DECL_ENV: &str = concat!("RR_", "WARN_IMPLICIT_DECL");
    let mut args = Vec::new();
    for (key, value) in env_kv {
        match *key {
            TYPE_MODE_ENV => {
                args.push("--type-mode");
                args.push(*value);
            }
            NATIVE_BACKEND_ENV => {
                args.push("--native-backend");
                args.push(*value);
            }
            PARALLEL_MODE_ENV => {
                args.push("--parallel-mode");
                args.push(*value);
            }
            PARALLEL_BACKEND_ENV => {
                args.push("--parallel-backend");
                args.push(*value);
            }
            PARALLEL_THREADS_ENV => {
                args.push("--parallel-threads");
                args.push(*value);
            }
            PARALLEL_MIN_TRIP_ENV => {
                args.push("--parallel-min-trip");
                args.push(*value);
            }
            STRICT_LET_ENV | STRICT_ASSIGN_ENV => {
                args.push("--strict-let");
                args.push(*value);
            }
            WARN_IMPLICIT_DECL_ENV => {
                args.push("--warn-implicit-decl");
                args.push(*value);
            }
            _ => {}
        }
    }
    args
}
