mod common;

use RR::compiler::{OptLevel, ParallelBackend, ParallelConfig, ParallelMode, compile_with_configs};
use RR::typeck::{NativeBackend, TypeConfig, TypeMode};
use common::{normalize, rscript_available, rscript_path};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_tmp_dir(name: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tests")
        .join("typed_parallel_wrapper");
    let _ = fs::create_dir_all(&root);
    let dir = root.join(format!("{}_{}_{}", name, std::process::id(), ts));
    let _ = fs::create_dir_all(&dir);
    dir
}

fn strict_type_cfg() -> TypeConfig {
    TypeConfig {
        mode: TypeMode::Strict,
        native_backend: NativeBackend::Off,
    }
}

#[test]
fn typed_vector_function_emits_parallel_wrapper() {
    let src = r#"
fn fused(a: vector<float>, b: vector<float>) -> vector<float> {
  return (a + b) * 0.5
}
"#;

    let (code, _map) = compile_with_configs(
        "typed_parallel_wrapper_emit.rr",
        src,
        OptLevel::O2,
        strict_type_cfg(),
        ParallelConfig {
            mode: ParallelMode::Optional,
            backend: ParallelBackend::R,
            threads: 2,
            min_trip: 1,
        },
    )
    .expect("compile should succeed");

    assert!(
        code.contains("# rr-typed-parallel-wrapper"),
        "eligible typed vector function should emit a wrapper"
    );
    assert!(
        code.contains("__typed_impl <- function("),
        "wrapper should keep the original body as an impl helper"
    );
    assert!(
        code.contains("return(rr_parallel_typed_vec_call("),
        "wrapper should dispatch through the typed parallel runtime helper"
    );
}

#[test]
fn typed_vector_parallel_wrapper_preserves_semantics() {
    let Some(rscript) = rscript_path() else {
        return;
    };
    if !rscript_available(&rscript) {
        return;
    }

    let src = r#"
fn fused(a: vector<float>, b: vector<float>) -> vector<float> {
  return (a + b) * 0.5
}

print(fused(c(1.0, 2.0, 3.0, 4.0), c(4.0, 3.0, 2.0, 1.0)))
"#;

    let (code, _map) = compile_with_configs(
        "typed_parallel_wrapper_runtime.rr",
        src,
        OptLevel::O2,
        strict_type_cfg(),
        ParallelConfig {
            mode: ParallelMode::Optional,
            backend: ParallelBackend::R,
            threads: 2,
            min_trip: 1,
        },
    )
    .expect("compile should succeed");

    assert!(code.contains("# rr-typed-parallel-wrapper"));

    let ref_src = r#"
print((c(1.0, 2.0, 3.0, 4.0) + c(4.0, 3.0, 2.0, 1.0)) * 0.5)
"#;

    let tmp = unique_tmp_dir("runtime");
    let compiled_path = tmp.join("compiled.R");
    let ref_path = tmp.join("ref.R");
    fs::write(&compiled_path, code).expect("write compiled");
    fs::write(&ref_path, ref_src).expect("write reference");

    let ref_out = Command::new(&rscript)
        .arg("--vanilla")
        .arg(&ref_path)
        .output()
        .expect("run reference");
    assert!(
        ref_out.status.success(),
        "reference failed:\n{}",
        String::from_utf8_lossy(&ref_out.stderr)
    );

    let compiled_out = Command::new(&rscript)
        .arg("--vanilla")
        .arg(&compiled_path)
        .output()
        .expect("run compiled");
    assert!(
        compiled_out.status.success(),
        "compiled failed:\n{}",
        String::from_utf8_lossy(&compiled_out.stderr)
    );

    assert_eq!(
        normalize(&String::from_utf8_lossy(&ref_out.stdout)),
        normalize(&String::from_utf8_lossy(&compiled_out.stdout))
    );
    assert_eq!(
        normalize(&String::from_utf8_lossy(&ref_out.stderr)),
        normalize(&String::from_utf8_lossy(&compiled_out.stderr))
    );
}

#[test]
fn typed_reduction_function_does_not_emit_parallel_wrapper() {
    let src = r#"
fn pick(flag: bool, a: vector<float>, b: vector<float>) -> vector<float> {
  if flag {
    return a
  } else {
    return b
  }
}
"#;

    let (code, _map) = compile_with_configs(
        "typed_parallel_wrapper_reduction.rr",
        src,
        OptLevel::O2,
        strict_type_cfg(),
        ParallelConfig {
            mode: ParallelMode::Optional,
            backend: ParallelBackend::R,
            threads: 2,
            min_trip: 1,
        },
    )
    .expect("compile should succeed");

    assert!(
        !code.contains("# rr-typed-parallel-wrapper"),
        "branchy typed vector functions should stay on the sequential path"
    );
}
