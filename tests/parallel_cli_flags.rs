use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn unique_tmp_dir(name: &str) -> PathBuf {
    static UNIQUE_TMP_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let seq = UNIQUE_TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tests")
        .join("parallel_cli_flags");
    let _ = fs::create_dir_all(&root);
    let dir = root.join(format!("{}_{}_{}", name, std::process::id(), seq));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn cli_parallel_flags_are_injected_into_runtime_prelude() {
    let tmp = unique_tmp_dir("proj");
    let rr_path = tmp.join("main.rr");
    let out_path = tmp.join("out.R");
    fs::write(
        &rr_path,
        r#"
fn addv(x: vector<float>, y: vector<float>) -> vector<float> {
  return x + y
}

print(addv(c(1.0, 2.0), c(3.0, 4.0)))
"#,
    )
    .expect("write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let output = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("-O1")
        .arg("--parallel-mode")
        .arg("optional")
        .arg("--parallel-backend")
        .arg("r")
        .arg("--parallel-threads")
        .arg("3")
        .arg("--parallel-min-trip")
        .arg("77")
        .output()
        .expect("run rr");
    assert!(
        output.status.success(),
        "rr compile failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let code = fs::read_to_string(&out_path).expect("read out");
    assert!(!code.contains("if (!nzchar(Sys.getenv(\"RR_PARALLEL_MODE\", \"\")))"));
    assert!(code.contains(".rr_env$parallel_mode <- \"optional\";"));
    assert!(code.contains(".rr_env$parallel_backend <- \"r\";"));
    assert!(code.contains(".rr_env$parallel_threads <- as.integer(3);"));
    assert!(code.contains(".rr_env$parallel_min_trip <- as.integer(77);"));
}

#[test]
fn cli_compiler_parallel_flags_are_accepted_without_affecting_runtime_prelude() {
    let tmp = unique_tmp_dir("compiler_parallel");
    let rr_path = tmp.join("main.rr");
    let out_path = tmp.join("out.R");
    fs::write(
        &rr_path,
        r#"
fn addv(x: vector<float>, y: vector<float>) -> vector<float> {
  return x + y
}

print(addv(c(1.0, 2.0), c(3.0, 4.0)))
"#,
    )
    .expect("write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let output = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("-O1")
        .arg("--compiler-parallel-mode")
        .arg("on")
        .arg("--compiler-parallel-threads")
        .arg("2")
        .arg("--compiler-parallel-min-functions")
        .arg("1")
        .arg("--compiler-parallel-min-fn-ir")
        .arg("1")
        .arg("--compiler-parallel-max-jobs")
        .arg("1")
        .output()
        .expect("run rr");
    assert!(
        output.status.success(),
        "rr compile failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let code = fs::read_to_string(&out_path).expect("read out");
    assert!(
        !code.contains(".rr_env$compiler_parallel_mode"),
        "compiler-side scheduling flags must not leak into runtime prelude"
    );
    assert!(
        !code.contains(".rr_env$compiler_parallel_threads"),
        "compiler-side scheduling flags must not leak into runtime prelude"
    );
}
