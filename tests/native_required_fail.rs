mod common;

use RR::compiler::{OptLevel, compile_with_config};
use RR::typeck::{NativeBackend, TypeConfig, TypeMode};
use common::{rscript_available, rscript_path};
use std::fs;
use std::path::PathBuf;

#[test]
fn required_backend_fails_without_native_library() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping required backend runtime test: Rscript unavailable");
            return;
        }
    };

    let src = r#"
fn call_abs(n: int) {
  let x = seq_len(n) - 4
  let y = abs(x)
  return y
}
print(call_abs(5L))

"#;

    let (code, _map) = compile_with_config(
        "native_required.rr",
        src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Required,
        },
    )
    .expect("compile");

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("native_required_fail");
    fs::create_dir_all(&out_dir).expect("mkdir");
    let script = out_dir.join("out.R");
    fs::write(&script, code).expect("write");
    let missing = out_dir.join(if cfg!(target_os = "macos") {
        "missing_rr_native.dylib"
    } else if cfg!(target_os = "windows") {
        "missing_rr_native.dll"
    } else {
        "missing_rr_native.so"
    });

    let output = std::process::Command::new(&rscript)
        .arg("--vanilla")
        .arg(&script)
        .env("RR_NATIVE_AUTOBUILD", "0")
        .env("RR_NATIVE_LIB", missing.to_string_lossy().to_string())
        .output()
        .expect("failed to execute Rscript");
    let res = common::RunResult {
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    };
    assert_ne!(res.status, 0, "expected required mode failure");
    assert!(
        res.stderr.contains("native backend required")
            || res.stdout.contains("native backend required"),
        "expected native-backend failure message\nstdout:\n{}\nstderr:\n{}",
        res.stdout,
        res.stderr
    );
}
