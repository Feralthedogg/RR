use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_for_nested_2d_interchange(name: &str, rr_src: &str) -> (String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");

    let rr_path = out_dir.join(format!("{name}.rr"));
    let out_path = out_dir.join(format!("{name}.R"));
    let stats_path = out_dir.join(format!("{name}_stats.json"));
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
    let emitted = fs::read_to_string(&out_path).expect("failed to read emitted R");
    (stats, emitted)
}

#[test]
fn poly_applies_for_nested_2d_interchange_map() {
    let (stats, emitted) = compile_for_nested_2d_interchange(
        "poly_for_nested_2d_interchange_map",
        r#"
fn poly_for_nested_2d_interchange_map(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let b = matrix(seq_len((n * m)), n, m)
  let out = matrix(seq_len((n * m)), n, m)
  for (r in 1..n) {
    for (c in 1..m) {
      out[r, c] = a[r, c] + b[r, c]
    }
  }
  return out
}

print(poly_for_nested_2d_interchange_map(4, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1")
            && stats.contains("\"poly_schedule_applied_interchange\": 1"),
        "expected one applied interchange poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_matrix_binop_assign("),
        "expected for-loop 2d interchange map to rebuild generic loops without matrix helper, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_for_nested_2d_interchange_fused_minmax_reduction() {
    let (stats, emitted) = compile_for_nested_2d_interchange(
        "poly_for_nested_2d_interchange_multi_minmax_reduce",
        r#"
fn poly_for_nested_2d_interchange_multi_minmax_reduce(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let min_acc = (n * m) + 10
  let max_acc = 0
  for (r in 1..n) {
    for (c in 1..m) {
      min_acc = min(min_acc, a[r, c])
      max_acc = max(max_acc, a[r, c])
    }
  }
  return min_acc + max_acc
}

print(poly_for_nested_2d_interchange_multi_minmax_reduce(4, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1")
            && stats.contains("\"poly_schedule_applied_interchange\": 1"),
        "expected one applied interchange poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_matrix_reduce_rect("),
        "expected fused for-loop 2d interchange minmax reductions to rebuild generic loops without helpers, got:\n{}",
        emitted
    );
    assert!(
        emitted.contains("min(") && emitted.contains("max("),
        "expected min/max aggregation in emitted code, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_for_nested_2d_interchange_fused_reduction() {
    let (stats, emitted) = compile_for_nested_2d_interchange(
        "poly_for_nested_2d_interchange_multi_reduce",
        r#"
fn poly_for_nested_2d_interchange_multi_reduce(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let b = matrix(seq_len((n * m)), n, m)
  let sum_acc = 0
  let prod_acc = 1
  for (r in 1..n) {
    for (c in 1..m) {
      sum_acc = sum_acc + a[r, c]
      prod_acc = prod_acc * b[r, c]
    }
  }
  return sum_acc + prod_acc
}

print(poly_for_nested_2d_interchange_multi_reduce(4, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1")
            && stats.contains("\"poly_schedule_applied_interchange\": 1"),
        "expected one applied interchange poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_matrix_reduce_rect("),
        "expected fused for-loop 2d interchange reductions to rebuild generic loops without helpers, got:\n{}",
        emitted
    );
}
