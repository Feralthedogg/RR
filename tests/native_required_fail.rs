mod common;

use RR::compiler::{OptLevel, compile_with_config};
use RR::typeck::{NativeBackend, TypeConfig, TypeMode};
use common::{rscript_available, rscript_path, run_rscript};
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
  let x = seq_len(n) - 4;
  let y = seq_len(n);
  for (i in 1..length(x)) {
    y[i] = abs(x[i]);
  }
  return y;
}
print(call_abs(5L));
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

    let res = run_rscript(&rscript, &script);
    assert_ne!(res.status, 0, "expected required mode failure");
    assert!(
        res.stderr.contains("native backend required")
            || res.stdout.contains("native backend required"),
        "expected native-backend failure message\nstdout:\n{}\nstderr:\n{}",
        res.stdout,
        res.stderr
    );
}
