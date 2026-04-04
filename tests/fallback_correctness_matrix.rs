mod common;

use RR::compiler::{
    OptLevel, ParallelBackend, ParallelConfig, ParallelMode, compile_with_config,
    compile_with_configs,
};
use RR::typeck::{NativeBackend, TypeConfig, TypeMode};
use common::{normalize, rscript_available, rscript_path, run_rscript, unique_dir};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn fallback_paths_preserve_reference_behavior() {
    let Some(rscript) = rscript_path() else {
        return;
    };
    if !rscript_available(&rscript) {
        return;
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("fallback_correctness_matrix");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj = unique_dir(&sandbox_root, "matrix");
    fs::create_dir_all(&proj).expect("failed to create project dir");

    let parallel_rr = r#"
fn addv(x: vector<float>, y: vector<float>) -> vector<float> {
  return x + y
}

let z = addv(c(1.0, 2.0, 3.0), c(2.0, 4.0, 8.0))
print(z)
print(sum(z))
"#;
    let parallel_ref = r#"
z <- c(1.0, 2.0, 3.0) + c(2.0, 4.0, 8.0)
print(z)
print(sum(z))
"#;
    let parallel_compiled = compile_with_configs(
        "parallel_optional_fallback_matrix.rr",
        parallel_rr,
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
    .expect("parallel compile should succeed")
    .0;
    let parallel_script = proj.join("parallel_compiled.R");
    let parallel_ref_script = proj.join("parallel_ref.R");
    fs::write(&parallel_script, &parallel_compiled)
        .expect("failed to write parallel compiled artifact");
    fs::write(&parallel_ref_script, parallel_ref).expect("failed to write parallel reference");
    let bad_parallel_lib = proj.join(if cfg!(target_os = "macos") {
        "missing_rr_parallel.dylib"
    } else if cfg!(target_os = "windows") {
        "missing_rr_parallel.dll"
    } else {
        "missing_rr_parallel.so"
    });
    let parallel_out = Command::new(&rscript)
        .arg("--vanilla")
        .arg(&parallel_script)
        .env(
            "RR_NATIVE_LIB",
            bad_parallel_lib.to_string_lossy().to_string(),
        )
        .output()
        .expect("failed to run parallel compiled script");
    let parallel_ref_out = Command::new(&rscript)
        .arg("--vanilla")
        .arg(&parallel_ref_script)
        .output()
        .expect("failed to run parallel reference script");
    assert!(
        parallel_out.status.success(),
        "parallel fallback run failed:\n{}",
        String::from_utf8_lossy(&parallel_out.stderr)
    );
    assert!(
        parallel_ref_out.status.success(),
        "parallel reference run failed:\n{}",
        String::from_utf8_lossy(&parallel_ref_out.stderr)
    );
    assert_eq!(
        normalize(&String::from_utf8_lossy(&parallel_ref_out.stdout)),
        normalize(&String::from_utf8_lossy(&parallel_out.stdout)),
        "parallel optional fallback changed observable stdout"
    );

    let native_rr = r#"
fn call_abs(n: int) {
  let x = seq_len(n) - 4
  let y = seq_len(n)
  for (i in 1..length(x)) {
    y[i] = abs(x[i])
  }
  return y
}

print(call_abs(5L))
"#;
    let native_ref = "print(abs(seq_len(5L) - 4L))\n";
    let native_compiled = compile_with_config(
        "native_optional_matrix.rr",
        native_rr,
        OptLevel::O2,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Optional,
        },
    )
    .expect("native optional compile should succeed")
    .0;
    let native_script = proj.join("native_compiled.R");
    let native_ref_script = proj.join("native_ref.R");
    fs::write(&native_script, &native_compiled).expect("failed to write native compiled artifact");
    fs::write(&native_ref_script, native_ref).expect("failed to write native reference");
    let bad_native_lib = proj.join(if cfg!(target_os = "macos") {
        "missing_rr_native.dylib"
    } else if cfg!(target_os = "windows") {
        "missing_rr_native.dll"
    } else {
        "missing_rr_native.so"
    });
    let native_out = Command::new(&rscript)
        .arg("--vanilla")
        .arg(&native_script)
        .env(
            "RR_NATIVE_LIB",
            bad_native_lib.to_string_lossy().to_string(),
        )
        .output()
        .expect("failed to run native compiled script");
    let native_ref_out = Command::new(&rscript)
        .arg("--vanilla")
        .arg(&native_ref_script)
        .output()
        .expect("failed to run native reference script");
    assert!(
        native_out.status.success(),
        "native fallback run failed:\n{}",
        String::from_utf8_lossy(&native_out.stderr)
    );
    assert!(
        native_ref_out.status.success(),
        "native reference run failed:\n{}",
        String::from_utf8_lossy(&native_ref_out.stderr)
    );
    assert_eq!(
        normalize(&String::from_utf8_lossy(&native_ref_out.stdout)),
        normalize(&String::from_utf8_lossy(&native_out.stdout)),
        "native optional fallback changed observable stdout"
    );

    let hybrid_rr = r#"
fn dyn_eval(x) {
  return eval(x)
}

fn plain(x) {
  return x + 1
}

print(dyn_eval(41))
print(plain(1))
"#;
    let hybrid_ref = "print(eval(41))\nprint(2)\n";
    let hybrid_rr_path = proj.join("hybrid.rr");
    let hybrid_out_path = proj.join("hybrid.R");
    let hybrid_ref_path = proj.join("hybrid_ref.R");
    fs::write(&hybrid_rr_path, hybrid_rr).expect("failed to write hybrid RR source");
    fs::write(&hybrid_ref_path, hybrid_ref).expect("failed to write hybrid reference source");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&hybrid_rr_path)
        .arg("-o")
        .arg(&hybrid_out_path)
        .arg("-O1")
        .status()
        .expect("failed to compile hybrid fallback case");
    assert!(status.success(), "hybrid compile failed");
    let hybrid_code = fs::read_to_string(&hybrid_out_path).expect("failed to read hybrid artifact");
    assert!(
        hybrid_code.contains("rr-hybrid-fallback"),
        "expected hybrid fallback marker"
    );
    let hybrid_run = run_rscript(&rscript, &hybrid_out_path);
    let hybrid_ref_run = run_rscript(&rscript, &hybrid_ref_path);
    assert_eq!(
        hybrid_run.status, 0,
        "hybrid compiled script failed:\n{}",
        hybrid_run.stderr
    );
    assert_eq!(
        hybrid_ref_run.status, 0,
        "hybrid reference script failed:\n{}",
        hybrid_ref_run.stderr
    );
    assert_eq!(
        normalize(&hybrid_ref_run.stdout),
        normalize(&hybrid_run.stdout),
        "hybrid fallback changed observable stdout"
    );
}
