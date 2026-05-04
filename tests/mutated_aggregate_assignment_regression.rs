mod common;

use common::{compile_rr_env_with_args, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::{Path, PathBuf};

fn compile_rr(rr_bin: &Path, rr_src: &Path, out_path: &Path, level: &str) {
    compile_rr_env_with_args(rr_bin, rr_src, out_path, level, &["--no-incremental"], &[]);
}

fn assert_matches_o0(rr_bin: &Path, rscript: &str, rr_path: &Path, out_dir: &Path, name: &str) {
    let o0 = out_dir.join(format!("{name}_o0.R"));
    compile_rr(rr_bin, rr_path, &o0, "-O0");
    let baseline = run_rscript(rscript, &o0);
    assert_eq!(baseline.status, 0, "O0 failed: {}", baseline.stderr);

    for (level, stem) in [("-O2", "o2"), ("-O3", "o3"), ("-Oz", "oz")] {
        let out = out_dir.join(format!("{name}_{stem}.R"));
        compile_rr(rr_bin, rr_path, &out, level);
        let compiled = run_rscript(rscript, &out);
        assert_eq!(compiled.status, 0, "{level} failed: {}", compiled.stderr);
        assert_eq!(
            normalize(&baseline.stdout),
            normalize(&compiled.stdout),
            "{level} changed runtime output for {}",
            rr_path.display()
        );
    }
}

#[test]
fn indexed_store_then_alias_assignment_preserves_mutated_vector_state() {
    let rscript = match rscript_path() {
        Some(path) if rscript_available(&path) => path,
        _ => {
            eprintln!("Skipping mutated aggregate assignment test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("mutated_aggregate_assignment");
    fs::create_dir_all(&out_dir).expect("failed to create mutated aggregate target dir");
    let rr_path = out_dir.join("indexed_store_alias.rr");
    fs::write(
        &rr_path,
        r#"
fn main() {
  let n = 4.0
  let x = seq_len(n)
  let y = seq_len(n)
  let z = rep.int(0.0, n)
  let step = 1.0
  while (step <= 3.0) {
    let i = 1.0
    while (i <= n) {
      z[i] = x[i] + y[i]
      i += 1.0
    }
    let next_x = rep.int(0.0, n)
    let j = 1.0
    while (j <= n) {
      next_x[j] = z[j]
      j += 1.0
    }
    x = next_x
    step += 1.0
  }
  print(z[1.0])
  print(z[n])
  return z[n]
}

print(main())
"#,
    )
    .expect("failed to write indexed store alias case");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    assert_matches_o0(&rr_bin, &rscript, &rr_path, &out_dir, "indexed_store_alias");
}

#[test]
fn vector_fusion_benchmark_preserves_outer_loop_carried_vector_update() {
    let rscript = match rscript_path() {
        Some(path) if rscript_available(&path) => path,
        _ => {
            eprintln!("Skipping vector fusion parity test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let rr_path = root
        .join("example")
        .join("benchmarks")
        .join("vector_fusion_bench.rr");
    let out_dir = root
        .join("target")
        .join("tests")
        .join("mutated_aggregate_assignment_vector_fusion");
    fs::create_dir_all(&out_dir).expect("failed to create vector fusion target dir");

    assert_matches_o0(&rr_bin, &rscript, &rr_path, &out_dir, "vector_fusion");
}
