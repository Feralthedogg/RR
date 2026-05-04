mod common;

use common::{
    compile_rr_env_with_args, normalize, rscript_available, rscript_path, run_rscript, unique_dir,
};
use std::fs;
use std::path::{Path, PathBuf};

fn compile_rr(rr_bin: &Path, rr_src: &Path, out_path: &Path, level: &str) {
    compile_rr_env_with_args(rr_bin, rr_src, out_path, level, &["--no-incremental"], &[]);
}

fn compile_rr_with_env(
    rr_bin: &Path,
    rr_src: &Path,
    out_path: &Path,
    level: &str,
    env_kv: &[(&str, &str)],
) {
    compile_rr_env_with_args(
        rr_bin,
        rr_src,
        out_path,
        level,
        &["--no-incremental"],
        env_kv,
    );
}

#[test]
fn reduction_vectorization_preserves_post_loop_reassignment() {
    let rscript = match rscript_path() {
        Some(path) if rscript_available(&path) => path,
        _ => {
            eprintln!("Skipping reduction rebind test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("vectorization_reduction_rebind");
    fs::create_dir_all(&out_dir).expect("failed to create reduction rebind target dir");
    let rr_path = out_dir.join("reduction_rebind.rr");
    fs::write(
        &rr_path,
        r#"
fn main() {
  let xs = c(1.0, 2.0, 3.0, 4.0)
  let total = 0.0
  let i = 1.0
  while (i <= length(xs)) {
    let err = xs[i] - 1.0
    total += err * err
    i += 1.0
  }
  total = total / length(xs)
  print(total)
  return total
}

print(main())
"#,
    )
    .expect("failed to write reduction rebind case");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let o0 = out_dir.join("reduction_rebind_o0.R");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    let baseline = run_rscript(&rscript, &o0);
    assert_eq!(baseline.status, 0, "O0 failed: {}", baseline.stderr);

    for (level, stem) in [("-O2", "o2"), ("-O3", "o3"), ("-Oz", "oz")] {
        let out = out_dir.join(format!("reduction_rebind_{stem}.R"));
        compile_rr(&rr_bin, &rr_path, &out, level);
        let compiled = run_rscript(&rscript, &out);
        assert_eq!(compiled.status, 0, "{level} failed: {}", compiled.stderr);
        assert_eq!(
            normalize(&baseline.stdout),
            normalize(&compiled.stdout),
            "{level} changed the post-loop reassignment result"
        );
        let code = fs::read_to_string(&out).expect("failed to read optimized R");
        assert!(
            code.contains("/ length(xs)") || code.contains("/ 4"),
            "{level} dropped the post-reduction normalization:\n{code}"
        );
    }
}

#[test]
fn o3_reduction_preserves_live_in_accumulator_from_previous_loop() {
    let rscript = match rscript_path() {
        Some(path) if rscript_available(&path) => path,
        _ => {
            eprintln!("Skipping live-in accumulator reduction test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("vectorization_reduction_rebind");
    fs::create_dir_all(&sandbox_root).expect("failed to create reduction rebind target dir");
    let out_dir = unique_dir(&sandbox_root, "live_in_acc");
    fs::create_dir_all(&out_dir).expect("failed to create live-in accumulator target dir");
    let rr_path = out_dir.join("live_in_accumulator.rr");
    fs::write(
        &rr_path,
        r#"
fn legacy_kernel(n) {
  x <- seq_len(n)
  y <- seq_len(n)
  acc <- 0.0
  i <- 1L
  for (j in 1L..length(x)) {
    y[j] <- (x[j] * 2.0) + 1.0
    acc <- acc + y[j]
  }
  while (i <= n) {
    acc <- acc + i
    i <- i + 1L
  }
  print(y[length(y)])
  return acc
}

print(legacy_kernel(8L))
"#,
    )
    .expect("failed to write live-in accumulator case");

    let legacy_flags = [("RR_STRICT_LET", "off"), ("RR_WARN_IMPLICIT_DECL", "off")];
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let o0 = out_dir.join("live_in_accumulator_o0.R");
    compile_rr_with_env(&rr_bin, &rr_path, &o0, "-O0", &legacy_flags);
    let baseline = run_rscript(&rscript, &o0);
    assert_eq!(baseline.status, 0, "O0 failed: {}", baseline.stderr);

    let o3 = out_dir.join("live_in_accumulator_o3.R");
    compile_rr_with_env(&rr_bin, &rr_path, &o3, "-O3", &legacy_flags);
    let optimized = run_rscript(&rscript, &o3);
    assert_eq!(optimized.status, 0, "O3 failed: {}", optimized.stderr);
    assert_eq!(
        normalize(&baseline.stdout),
        normalize(&optimized.stdout),
        "O3 changed a reduction with a live-in accumulator"
    );
    assert!(
        normalize(&optimized.stdout).contains("[1] 116"),
        "expected live-in accumulator result 116, got:\n{}",
        optimized.stdout
    );

    let code = fs::read_to_string(&o3).expect("failed to read optimized R");
    assert!(
        !code.lines().any(|line| line.trim() == "acc <- sum(1L:n)"),
        "O3 dropped the incoming accumulator:\n{code}"
    );
}
