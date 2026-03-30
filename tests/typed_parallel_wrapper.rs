mod common;

use RR::compiler::{OptLevel, ParallelBackend, ParallelConfig, ParallelMode, compile_with_configs};
use RR::typeck::{NativeBackend, TypeConfig, TypeMode};
use common::{normalize, rscript_available, rscript_path};
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
        .join("typed_parallel_wrapper");
    let _ = fs::create_dir_all(&root);
    let dir = root.join(format!("{}_{}_{}", name, std::process::id(), seq));
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
        code.contains("__typed_impl <- function(")
            || !code.contains("return(rr_parallel_typed_vec_call("),
        "wrapper should either keep the impl helper or fully prune the unreachable wrapper body"
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
fn typed_vector_parallel_wrapper_preserves_named_vector_metadata() {
    let Some(rscript) = rscript_path() else {
        return;
    };
    if !rscript_available(&rscript) {
        return;
    }

    let src = r#"
import r * as base from "base"

fn keep(a: vector<float>) -> vector<float> {
  return a
}

let a = base.c(alpha = 1.0, beta = 2.0, gamma = 3.0, delta = 4.0)
let out = keep(a)
print(out)
print(base.names(out))
"#;

    let (code, _map) = compile_with_configs(
        "typed_parallel_wrapper_named.rr",
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
a <- c(alpha = 1.0, beta = 2.0, gamma = 3.0, delta = 4.0)
out <- a
print(out)
print(names(out))
"#;

    let tmp = unique_tmp_dir("named");
    let compiled_path = tmp.join("compiled_named.R");
    let ref_path = tmp.join("ref_named.R");
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
fn typed_matrix_parallel_wrapper_preserves_matrix_shape() {
    let Some(rscript) = rscript_path() else {
        return;
    };
    if !rscript_available(&rscript) {
        return;
    }

    let src = r#"
import r * as base from "base"

fn fused_m(a: matrix<float>, b: matrix<float>) -> matrix<float> {
  return (a + b) * 0.5
}

let a = matrix(c(1.0, 2.0, 3.0, 4.0), 2L, 2L)
let b = matrix(c(4.0, 3.0, 2.0, 1.0), 2L, 2L)
let out = fused_m(a, b)
print(out)
print(base.dim(out))
"#;

    let (code, _map) = compile_with_configs(
        "typed_parallel_wrapper_matrix.rr",
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
a <- matrix(c(1.0, 2.0, 3.0, 4.0), 2L, 2L)
b <- matrix(c(4.0, 3.0, 2.0, 1.0), 2L, 2L)
out <- (a + b) * 0.5
print(out)
print(dim(out))
"#;

    let tmp = unique_tmp_dir("matrix");
    let compiled_path = tmp.join("compiled_matrix.R");
    let ref_path = tmp.join("ref_matrix.R");
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
fn typed_matrix_parallel_wrapper_preserves_dimnames() {
    let Some(rscript) = rscript_path() else {
        return;
    };
    if !rscript_available(&rscript) {
        return;
    }

    let src = r#"
import r * as base from "base"

fn keep_m(a: matrix<float>) -> matrix<float> {
  return a
}

let a = base.matrix(
  c(1.0, 2.0, 3.0, 4.0),
  2L,
  2L,
  dimnames = base.list(c("r1", "r2"), c("c1", "c2"))
)
let out = keep_m(a)
print(out)
print(base.dimnames(out))
"#;

    let (code, _map) = compile_with_configs(
        "typed_parallel_wrapper_matrix_dimnames.rr",
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
a <- matrix(
  c(1.0, 2.0, 3.0, 4.0),
  2L,
  2L,
  dimnames = list(c("r1", "r2"), c("c1", "c2"))
)
out <- a
print(out)
print(dimnames(out))
"#;

    let tmp = unique_tmp_dir("matrix_dimnames");
    let compiled_path = tmp.join("compiled_matrix_dimnames.R");
    let ref_path = tmp.join("ref_matrix_dimnames.R");
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
fn typed_matrix_shape_sensitive_kernel_does_not_emit_parallel_wrapper() {
    let src = r#"
fn transpose_m(a: matrix<float>) -> matrix<float> {
  return t(a)
}
"#;

    let (code, _map) = compile_with_configs(
        "typed_parallel_wrapper_matrix_transpose.rr",
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
        "matrix shape-sensitive kernels must not emit typed parallel wrappers"
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
