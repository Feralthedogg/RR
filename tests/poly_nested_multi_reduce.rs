use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn poly_applies_nested_interchange_for_fused_full_matrix_reductions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("poly_nested_multi_reduce");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_nested_multi_reduce");

    let rr_src = r#"
fn poly_nested_multi_reduce(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let b = matrix(seq_len((n * m)), n, m)
  let sum_acc = 0
  let prod_acc = 1

  let r = 1
  while (r <= n) {
    let c = 1
    while (c <= m) {
      sum_acc = sum_acc + a[r, c]
      prod_acc = prod_acc * b[r, c]
      c += 1
    }
    r += 1
  }
  return sum_acc + prod_acc
}

print(poly_nested_multi_reduce(3, 4))
"#;

    let rr_path = out_dir.join("poly_nested_multi_reduce.rr");
    let out_path = out_dir.join("poly_nested_multi_reduce.R");
    let stats_path = out_dir.join("poly_nested_multi_reduce_stats.json");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_PULSE_JSON_PATH", &stats_path)
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let stats = fs::read_to_string(&stats_path).expect("failed to read pulse stats json");
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );

    let emitted = fs::read_to_string(&out_path).expect("failed to read emitted R");
    assert!(
        !emitted.contains("rr_matrix_reduce_rect("),
        "expected fused matrix reductions to rebuild generic loops without rr_matrix_reduce_rect, got:\n{}",
        emitted
    );
    assert!(
        !emitted.contains("rr_can_matrix_reduce_rect("),
        "expected fused matrix reductions to avoid helper guards, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_nested_interchange_for_fused_full_matrix_minmax_reductions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("poly_nested_multi_minmax_reduce");
    fs::create_dir_all(&out_dir)
        .expect("failed to create target/tests/poly_nested_multi_minmax_reduce");

    let rr_src = r#"
fn poly_nested_multi_minmax_reduce(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let min_acc = (n * m) + 10
  let max_acc = 0

  let r = 1
  while (r <= n) {
    let c = 1
    while (c <= m) {
      min_acc = min(min_acc, a[r, c])
      max_acc = max(max_acc, a[r, c])
      c += 1
    }
    r += 1
  }
  return min_acc + max_acc
}

print(poly_nested_multi_minmax_reduce(3, 4))
"#;

    let rr_path = out_dir.join("poly_nested_multi_minmax_reduce.rr");
    let out_path = out_dir.join("poly_nested_multi_minmax_reduce.R");
    let stats_path = out_dir.join("poly_nested_multi_minmax_reduce_stats.json");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_PULSE_JSON_PATH", &stats_path)
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let stats = fs::read_to_string(&stats_path).expect("failed to read pulse stats json");
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );

    let emitted = fs::read_to_string(&out_path).expect("failed to read emitted R");
    assert!(
        !emitted.contains("rr_matrix_reduce_rect("),
        "expected fused matrix min/max reductions to rebuild generic loops without helpers, got:\n{}",
        emitted
    );
    assert!(
        emitted.contains("min(") && emitted.contains("max("),
        "expected emitted min/max aggregation, got:\n{}",
        emitted
    );
    assert!(
        !emitted.contains("rr_can_matrix_reduce_rect("),
        "expected fused matrix min/max reductions to avoid helper guards, got:\n{}",
        emitted
    );
}
