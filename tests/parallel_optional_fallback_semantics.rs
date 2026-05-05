mod common;

use common::{normalize, rscript_available, rscript_path};
use rr::compiler::internal::typeck::{NativeBackend, TypeConfig, TypeMode};
use rr::compiler::{OptLevel, ParallelBackend, ParallelConfig, ParallelMode, compile_with_configs};
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
        .join("parallel_optional_fallback_semantics");
    let _ = fs::create_dir_all(&root);
    let dir = root.join(format!("{}_{}_{}", name, std::process::id(), seq));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn optional_parallel_openmp_falls_back_without_semantic_change() {
    let Some(rscript) = rscript_path() else {
        return;
    };
    if !rscript_available(&rscript) {
        return;
    }

    let rr_src = r#"
fn addv(x: vector<float>, y: vector<float>) -> vector<float> {
  return x + y

}

print(addv(c(1.0, 2.0, 3.0), c(2.0, 4.0, 8.0)))
print(sum(addv(c(1.0, 2.0, 3.0), c(2.0, 4.0, 8.0))))

"#;

    let ref_src = r#"
z <- c(1.0, 2.0, 3.0) + c(2.0, 4.0, 8.0)
print(z)
print(sum(z))
"#;

    let (compiled, _map) = compile_with_configs(
        "parallel_optional_fallback.rr",
        rr_src,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Gradual,
            native_backend: NativeBackend::Off,
        },
        ParallelConfig {
            mode: ParallelMode::Optional,
            backend: ParallelBackend::OpenMp,
            threads: 4,
            min_trip: 1,
        },
    )
    .expect("compile");

    assert!(
        compiled.contains("rr_parallel_vec_add_f64 <- function"),
        "runtime must define parallel wrapper helper"
    );
    assert!(
        compiled.contains("!identical(.rr_env$parallel_mode, \"off\")"),
        "OpenMP native selection should remain available outside required mode"
    );
    assert!(
        compiled.contains("(.rr_env$parallel_backend %in% c(\"auto\", \"openmp\"))"),
        "OpenMP native selection should cover auto and explicit openmp backends"
    );
    assert!(
        compiled.contains("getNativeSymbolInfo(sym, PACKAGE = .rr_env$native_dll)"),
        "native symbol lookup should stay scoped to the loaded DLL"
    );

    let tmp = unique_tmp_dir("proj");
    let compiled_path = tmp.join("compiled.R");
    let ref_path = tmp.join("ref.R");
    fs::write(&compiled_path, compiled).expect("write compiled");
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

    // Force native OpenMP load failure. Optional mode must fallback safely.
    let bad_lib = tmp.join(if cfg!(target_os = "macos") {
        "does_not_exist_rr_native.dylib"
    } else if cfg!(target_os = "windows") {
        "does_not_exist_rr_native.dll"
    } else {
        "does_not_exist_rr_native.so"
    });
    let compiled_out = Command::new(&rscript)
        .arg("--vanilla")
        .arg(&compiled_path)
        .env("RR_NATIVE_LIB", bad_lib.to_string_lossy().to_string())
        .output()
        .expect("run compiled");
    assert!(
        compiled_out.status.success(),
        "compiled failed:\n{}",
        String::from_utf8_lossy(&compiled_out.stderr)
    );

    let ref_stdout = normalize(&String::from_utf8_lossy(&ref_out.stdout));
    let cmp_stdout = normalize(&String::from_utf8_lossy(&compiled_out.stdout));
    let ref_stderr = normalize(&String::from_utf8_lossy(&ref_out.stderr));
    let cmp_stderr = normalize(&String::from_utf8_lossy(&compiled_out.stderr));
    assert_eq!(ref_stdout, cmp_stdout, "stdout mismatch");
    assert_eq!(ref_stderr, cmp_stderr, "stderr mismatch");
}
