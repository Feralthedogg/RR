mod common;

use RR::compiler::{OptLevel, compile_with_config};
use RR::typeck::{NativeBackend, TypeConfig, TypeMode};
use common::{RunResult, normalize, rscript_available, rscript_path, run_rscript, unique_dir};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn compile_code(
    tag: &str,
    src: &str,
    opt: OptLevel,
    mode: TypeMode,
    native: NativeBackend,
) -> String {
    let cfg = TypeConfig {
        mode,
        native_backend: native,
    };
    let (code, _map) = compile_with_config(tag, src, opt, cfg)
        .unwrap_or_else(|e| panic!("compile failed for {}: {:?}", tag, e));
    code
}

fn run_rscript_with_env(path: &str, script: &Path, env_kv: &[(&str, &str)]) -> RunResult {
    let mut cmd = Command::new(path);
    cmd.arg("--vanilla").arg(script);
    for (k, v) in env_kv {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("failed to execute Rscript");
    RunResult {
        status: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    }
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

#[test]
fn typed_condition_elides_truthy_wrapper_and_preserves_output() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping typed condition audit: Rscript unavailable.");
            return;
        }
    };

    let typed_src = r#"
fn choose(flag: bool, a: int, b: int) -> int {
  if (flag) {
    return a

  } else {
    return b

  }
}
print(choose(TRUE, 11L, 7L))

"#;
    let untyped_src = r#"
fn choose(flag, a, b) {
  if (flag) {
    return a

  } else {
    return b

  }
}
print(choose(TRUE, 11L, 7L))

"#;
    let ref_r = r#"
choose <- function(flag, a, b) {
  if (flag) {
    return(a)
  } else {
    return(b)
  }
}
print(choose(TRUE, 11L, 7L))
"#;

    let typed_code = compile_code(
        "typed_truthy_audit.rr",
        typed_src,
        OptLevel::O2,
        TypeMode::Strict,
        NativeBackend::Off,
    );
    let untyped_code = compile_code(
        "untyped_truthy_audit.rr",
        untyped_src,
        OptLevel::O2,
        TypeMode::Strict,
        NativeBackend::Off,
    );

    assert!(
        !typed_code.contains("if (rr_truthy1("),
        "typed branch should not require rr_truthy1 wrapper in generated code"
    );
    assert!(
        untyped_code.contains("if (rr_truthy1("),
        "untyped branch should keep rr_truthy1 wrapper in generated code"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("r_output_optimization_audit");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj = unique_dir(&sandbox_root, "truthy");
    fs::create_dir_all(&proj).expect("failed to create project dir");

    let typed_path = proj.join("typed.R");
    let untyped_path = proj.join("untyped.R");
    let ref_path = proj.join("ref.R");
    fs::write(&typed_path, typed_code).expect("failed to write typed output");
    fs::write(&untyped_path, untyped_code).expect("failed to write untyped output");
    fs::write(&ref_path, ref_r).expect("failed to write reference R");

    let typed_run = run_rscript(&rscript, &typed_path);
    let untyped_run = run_rscript(&rscript, &untyped_path);
    let ref_run = run_rscript(&rscript, &ref_path);

    assert_eq!(ref_run.status, 0, "reference failed: {}", ref_run.stderr);
    assert_eq!(typed_run.status, 0, "typed failed: {}", typed_run.stderr);
    assert_eq!(
        untyped_run.status, 0,
        "untyped failed: {}",
        untyped_run.stderr
    );
    assert_eq!(normalize(&ref_run.stdout), normalize(&typed_run.stdout));
    assert_eq!(normalize(&ref_run.stdout), normalize(&untyped_run.stdout));
    assert_eq!(normalize(&ref_run.stderr), normalize(&typed_run.stderr));
    assert_eq!(normalize(&ref_run.stderr), normalize(&untyped_run.stderr));
}

#[test]
fn intrinsic_emission_and_optional_native_fallback_are_verified() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping intrinsic/native audit: Rscript unavailable.");
            return;
        }
    };

    let rr_src = r#"
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
    let ref_r = r#"
call_abs <- function(n) {
  x <- seq_len(n) - 4
  y <- seq_len(n)
  for (i in 1:length(x)) {
    y[i] <- abs(x[i])
  }
  y
}
print(call_abs(5L))
"#;

    let off_code = compile_code(
        "intrinsic_audit_off.rr",
        rr_src,
        OptLevel::O2,
        TypeMode::Strict,
        NativeBackend::Off,
    );
    let opt_code = compile_code(
        "intrinsic_audit_optional.rr",
        rr_src,
        OptLevel::O2,
        TypeMode::Strict,
        NativeBackend::Optional,
    );

    for code in [&off_code, &opt_code] {
        assert!(
            code.contains("rr_intrinsic_vec_abs_f64("),
            "expected intrinsic abs call in generated code"
        );
        assert!(
            code.contains("rr_intrinsic_vec_abs_f64(")
                || code.contains("rr_intrinsic_vec_sum_f64(")
                || code.contains("rr_intrinsic_vec_mean_f64("),
            "expected intrinsic helper call in generated code"
        );
    }
    assert!(
        off_code.contains("rr_set_native_backend(\"off\");"),
        "missing native backend off marker"
    );
    assert!(
        opt_code.contains("rr_set_native_backend(\"optional\");"),
        "missing native backend optional marker"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("r_output_optimization_audit");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj = unique_dir(&sandbox_root, "intrinsic");
    fs::create_dir_all(&proj).expect("failed to create project dir");

    let off_path = proj.join("off.R");
    let opt_path = proj.join("optional.R");
    let ref_path = proj.join("ref.R");
    fs::write(&off_path, off_code).expect("failed to write off output");
    fs::write(&opt_path, opt_code).expect("failed to write optional output");
    fs::write(&ref_path, ref_r).expect("failed to write reference R");

    let bad_native = proj.join("missing_native_library.so");
    let bad_native_str = bad_native.to_string_lossy().to_string();

    let off_run = run_rscript(&rscript, &off_path);
    let opt_run = run_rscript_with_env(
        &rscript,
        &opt_path,
        &[("RR_NATIVE_LIB", bad_native_str.as_str())],
    );
    let ref_run = run_rscript(&rscript, &ref_path);

    assert_eq!(ref_run.status, 0, "reference failed: {}", ref_run.stderr);
    assert_eq!(off_run.status, 0, "off failed: {}", off_run.stderr);
    assert_eq!(opt_run.status, 0, "optional failed: {}", opt_run.stderr);
    assert_eq!(normalize(&ref_run.stdout), normalize(&off_run.stdout));
    assert_eq!(normalize(&ref_run.stdout), normalize(&opt_run.stdout));
    assert_eq!(normalize(&off_run.stdout), normalize(&opt_run.stdout));
    assert_eq!(normalize(&ref_run.stderr), normalize(&off_run.stderr));
    assert_eq!(normalize(&ref_run.stderr), normalize(&opt_run.stderr));
}

#[test]
fn o2_reduces_index_guard_calls_and_matches_reference_semantics() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping index-guard audit: Rscript unavailable.");
            return;
        }
    };

    let rr_src = r#"
fn map_err(n: int) {
  let x = seq_len(n)

  let y = seq_len(n)

  for (i in 1L..length(x)) {
    y[i] = (x[i] * 2L) + 1L

  }
  let target = (x * 2L) + 1L

  return sum(abs(y - target))

}
print(map_err(30L))

"#;
    let ref_r = r#"
map_err <- function(n) {
  x <- seq_len(n)
  y <- seq_len(n)
  for (i in 1L:length(x)) {
    y[i] <- (x[i] * 2L) + 1L
  }
  target <- (x * 2L) + 1L
  sum(abs(y - target))
}
print(map_err(30L))
"#;

    let o0_code = compile_code(
        "index_guard_o0.rr",
        rr_src,
        OptLevel::O0,
        TypeMode::Strict,
        NativeBackend::Off,
    );
    let o2_code = compile_code(
        "index_guard_o2.rr",
        rr_src,
        OptLevel::O2,
        TypeMode::Strict,
        NativeBackend::Off,
    );

    let o0_read = count_occurrences(&o0_code, "rr_index1_read(");
    let o0_write = count_occurrences(&o0_code, "rr_index1_write(");
    let o2_read = count_occurrences(&o2_code, "rr_index1_read(");
    let o2_write = count_occurrences(&o2_code, "rr_index1_write(");
    let o0_total = o0_read + o0_write;
    let o2_total = o2_read + o2_write;

    assert!(
        o0_total > 0,
        "O0 should contain index guard wrappers for index-heavy source"
    );
    assert!(
        o2_total < o0_total,
        "O2 should reduce index guard calls. O0={}, O2={}",
        o0_total,
        o2_total
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("r_output_optimization_audit");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj = unique_dir(&sandbox_root, "index_guard");
    fs::create_dir_all(&proj).expect("failed to create project dir");

    let o0_path = proj.join("o0.R");
    let o2_path = proj.join("o2.R");
    let ref_path = proj.join("ref.R");
    fs::write(&o0_path, o0_code).expect("failed to write O0 output");
    fs::write(&o2_path, o2_code).expect("failed to write O2 output");
    fs::write(&ref_path, ref_r).expect("failed to write reference R");

    let o0_run = run_rscript(&rscript, &o0_path);
    let o2_run = run_rscript(&rscript, &o2_path);
    let ref_run = run_rscript(&rscript, &ref_path);

    assert_eq!(ref_run.status, 0, "reference failed: {}", ref_run.stderr);
    assert_eq!(o0_run.status, 0, "O0 failed: {}", o0_run.stderr);
    assert_eq!(o2_run.status, 0, "O2 failed: {}", o2_run.stderr);
    assert_eq!(normalize(&ref_run.stdout), normalize(&o0_run.stdout));
    assert_eq!(normalize(&ref_run.stdout), normalize(&o2_run.stdout));
    assert_eq!(normalize(&o0_run.stdout), normalize(&o2_run.stdout));
    assert_eq!(normalize(&ref_run.stderr), normalize(&o0_run.stderr));
    assert_eq!(normalize(&ref_run.stderr), normalize(&o2_run.stderr));
}

#[test]
fn nested_generic_type_hint_program_matches_reference_r() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping nested-generic audit: Rscript unavailable.");
            return;
        }
    };

    let rr_src = r#"
fn count_nested(xs: list<vector<float>>) -> int {
  return length(xs)

}

fn main() -> int {
  let xs = list(c(1.0, 2.0, 3.0), c(4.0))

  let n = count_nested(xs)

  print(n)

  return n

}

print(main())

"#;
    let ref_r = r#"
count_nested <- function(xs) {
  length(xs)
}

main <- function() {
  xs <- list(c(1.0, 2.0, 3.0), c(4.0))
  n <- count_nested(xs)
  print(n)
  n
}

print(main())
"#;

    let code = compile_code(
        "nested_generic_runtime_audit.rr",
        rr_src,
        OptLevel::O2,
        TypeMode::Strict,
        NativeBackend::Off,
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("r_output_optimization_audit");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj = unique_dir(&sandbox_root, "nested_generic");
    fs::create_dir_all(&proj).expect("failed to create project dir");

    let compiled_path = proj.join("compiled.R");
    let ref_path = proj.join("ref.R");
    fs::write(&compiled_path, code).expect("failed to write compiled output");
    fs::write(&ref_path, ref_r).expect("failed to write reference output");

    let compiled = run_rscript(&rscript, &compiled_path);
    let reference = run_rscript(&rscript, &ref_path);

    assert_eq!(
        reference.status, 0,
        "reference failed: {}",
        reference.stderr
    );
    assert_eq!(compiled.status, 0, "compiled failed: {}", compiled.stderr);
    assert_eq!(normalize(&reference.stdout), normalize(&compiled.stdout));
    assert_eq!(normalize(&reference.stderr), normalize(&compiled.stderr));
}
