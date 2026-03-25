mod common;

use RR::compiler::{
    CompileOutputOptions, OptLevel, ParallelBackend, ParallelConfig, ParallelMode,
    compile_with_configs_with_options, type_config_from_env,
};
use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;

fn runtime_out_dir() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("runtime_semantics_regression");
    fs::create_dir_all(&out_dir).expect("failed to create runtime semantics output dir");
    out_dir
}

fn run_o0_o2_case(
    case_name: &str,
    source: &str,
    expected_stdout: &str,
) -> Option<(String, String)> {
    let rscript = match rscript_path() {
        Some(path) if rscript_available(&path) => path,
        _ => {
            eprintln!("Skipping {case_name}: Rscript not available.");
            return None;
        }
    };

    let out_dir = runtime_out_dir();
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = out_dir.join(format!("{case_name}.rr"));
    let o0_path = out_dir.join(format!("{case_name}_o0.R"));
    let o2_path = out_dir.join(format!("{case_name}_o2.R"));

    fs::write(&rr_path, source).expect("failed to write RR source");
    compile_rr(&rr_bin, &rr_path, &o0_path, "-O0");
    compile_rr(&rr_bin, &rr_path, &o2_path, "-O2");

    let o0 = run_rscript(&rscript, &o0_path);
    let o2 = run_rscript(&rscript, &o2_path);
    let stdout_o0 = normalize(&o0.stdout);
    let stdout_o2 = normalize(&o2.stdout);
    let stderr_o0 = normalize(&o0.stderr);
    let stderr_o2 = normalize(&o2.stderr);

    assert_eq!(
        o0.status, 0,
        "O0 runtime failed for {case_name}\nstdout:\n{}\nstderr:\n{}",
        stdout_o0, stderr_o0
    );
    assert_eq!(
        o2.status, 0,
        "O2 runtime failed for {case_name}\nstdout:\n{}\nstderr:\n{}",
        stdout_o2, stderr_o2
    );
    assert_eq!(
        stdout_o0, stdout_o2,
        "O0/O2 stdout mismatch for {case_name}\nO0:\n{}\nO2:\n{}",
        stdout_o0, stdout_o2
    );
    assert_eq!(
        stderr_o0, stderr_o2,
        "O0/O2 stderr mismatch for {case_name}\nO0:\n{}\nO2:\n{}",
        stderr_o0, stderr_o2
    );
    assert_eq!(
        stdout_o0, expected_stdout,
        "unexpected runtime output for {case_name}\nstdout:\n{}",
        stdout_o0
    );
    Some((stdout_o0, stdout_o2))
}

#[test]
fn swap_semantics_match_across_opt_levels() {
    let src = r#"
fn main() {
  let u = 1.0
  let u_new = 2.0
  let tmp_u = u
  u = u_new
  u_new = tmp_u
  print(u)
  print(u_new)
}

main()
"#;
    let _ = run_o0_o2_case("swap_semantics", src, "[1] 2\n[1] 1\n");
}

#[test]
fn branch_carried_scalar_semantics_match_across_opt_levels() {
    let src = r#"
fn dot2(a, b) {
  return a + b
}

fn main() {
  let rs_old = 1.0
  let rs_new = 0.0
  let beta = 0.0
  rs_new = dot2(2.0, 3.0)
  if (is.na(rs_new) || !is.finite(rs_new)) {
    rs_new = rs_old
  }
  beta = rs_new / rs_old
  rs_old = rs_new
  print(beta)
  print(rs_old)
}

main()
"#;
    let _ = run_o0_o2_case("branch_carried_scalar_semantics", src, "[1] 5\n[1] 5\n");
}

#[test]
fn particle_index_sampling_semantics_match_across_opt_levels() {
    let src = r#"
fn idx_cube(f, x, y, size) {
  let ff = f
  let xx = x
  let yy = y
  let ss = size
  return ((((ff - 1.0) * ss * ss) + ((xx - 1.0) * ss)) + yy)
}

fn sample_cell(field, px, py, p, f, N) {
  let gx = px[p] * N
  let gy = py[p] * N
  let ix = floor(gx)
  let iy = floor(gy)
  let idx = idx_cube(f, ix, iy, N)
  print(ix)
  print(iy)
  print(field[idx])
}

let field = seq_len(6.0 * 4.0 * 4.0)
let px = c(0.51)
let py = c(0.26)
sample_cell(field, px, py, 1.0, 2.0, 4.0)
"#;
    let _ = run_o0_o2_case(
        "particle_index_sampling_semantics",
        src,
        "[1] 2\n[1] 1\n[1] 21\n",
    );
}

#[test]
fn runtime_artifact_embeds_compile_time_parallel_policy() {
    let out_dir = runtime_out_dir();
    let rr_path = out_dir.join("artifact_policy.rr");
    let src = r#"
fn addv(x: vector<float>, y: vector<float>) -> vector<float> {
  return x + y
}

print(addv(c(1.0, 2.0), c(3.0, 4.0)))
"#;
    fs::write(&rr_path, src).expect("failed to write RR source");

    let compiled = compile_with_configs_with_options(
        rr_path.to_str().expect("non-utf8 path"),
        src,
        OptLevel::O2,
        type_config_from_env(),
        ParallelConfig {
            mode: ParallelMode::Required,
            backend: ParallelBackend::OpenMp,
            threads: 8,
            min_trip: 256,
        },
        CompileOutputOptions {
            inject_runtime: true,
            preserve_all_defs: false,
        },
    )
    .expect("compile should succeed")
    .0;

    assert!(
        compiled.contains("if (!nzchar(Sys.getenv(\"RR_PARALLEL_MODE\", \"\")))"),
        "artifact should gate compile-time parallel defaults on env absence"
    );
    assert!(
        compiled.contains(".rr_env$parallel_mode <- \"required\";"),
        "artifact should embed compile-time parallel mode default"
    );
    assert!(
        compiled.contains(".rr_env$parallel_backend <- \"openmp\";"),
        "artifact should embed compile-time parallel backend default"
    );
    assert!(
        compiled.contains(".rr_env$parallel_threads <- as.integer(8);"),
        "artifact should embed compile-time thread count default"
    );
    assert!(
        compiled.contains(".rr_env$parallel_min_trip <- as.integer(256);"),
        "artifact should embed compile-time min trip default"
    );
    assert!(
        compiled.contains("if (!nzchar(Sys.getenv(\"RR_PARALLEL_MODE\", \"\")))"),
        "artifact should only apply compile-time parallel mode when env override is absent"
    );
}

#[test]
fn runtime_artifact_allows_env_override_of_parallel_policy() {
    let Some(rscript) = rscript_path() else {
        return;
    };
    if !rscript_available(&rscript) {
        return;
    }

    let out_dir = runtime_out_dir();
    let rr_path = out_dir.join("artifact_policy_override.rr");
    let script_path = out_dir.join("artifact_policy_override.R");
    let src = r#"
fn addv(x: vector<float>, y: vector<float>) -> vector<float> {
  return x + y
}

print(addv(c(1.0, 2.0), c(3.0, 4.0)))
"#;
    fs::write(&rr_path, src).expect("failed to write RR source");

    let compiled = compile_with_configs_with_options(
        rr_path.to_str().expect("non-utf8 path"),
        src,
        OptLevel::O2,
        type_config_from_env(),
        ParallelConfig {
            mode: ParallelMode::Required,
            backend: ParallelBackend::OpenMp,
            threads: 8,
            min_trip: 256,
        },
        CompileOutputOptions {
            inject_runtime: true,
            preserve_all_defs: false,
        },
    )
    .expect("compile should succeed")
    .0;
    fs::write(&script_path, compiled).expect("failed to write compiled artifact");

    let missing = out_dir.join(if cfg!(target_os = "macos") {
        "missing_rr_native_override.dylib"
    } else if cfg!(target_os = "windows") {
        "missing_rr_native_override.dll"
    } else {
        "missing_rr_native_override.so"
    });

    let output = std::process::Command::new(&rscript)
        .arg("--vanilla")
        .arg(&script_path)
        .env("RR_PARALLEL_MODE", "off")
        .env("RR_NATIVE_BACKEND", "off")
        .env("RR_NATIVE_LIB", missing.to_string_lossy().to_string())
        .output()
        .expect("failed to execute Rscript");

    let stdout = normalize(&String::from_utf8_lossy(&output.stdout));
    let stderr = normalize(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        output.status.code().unwrap_or(-1),
        0,
        "env override should let artifact fall back to pure-R\nstdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
    );
    assert_eq!(stdout, "[1] 4 6\n");
    assert_eq!(stderr, "");
}
