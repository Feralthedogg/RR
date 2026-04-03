use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_nested_reduce(name: &str, rr_src: &str) -> (String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create nested poly reduction test dir");

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

fn assert_nested_reduce_lowered(name: &str, rr_src: &str) {
    let (stats, emitted) = compile_nested_reduce(name, rr_src);
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_matrix_reduce_rect("),
        "expected nested reduction lowering to rebuild generic loops without rr_matrix_reduce_rect, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_nested_interchange_for_full_matrix_sum_reduction() {
    assert_nested_reduce_lowered(
        "poly_nested_reduce_sum",
        r#"
fn poly_nested_matrix_reduce_sum(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let acc = 0

  let r = 1
  while (r <= n) {
    let c = 1
    while (c <= m) {
      acc = acc + a[r, c]
      c += 1
    }
    r += 1
  }
  return acc
}

print(poly_nested_matrix_reduce_sum(3, 4))
"#,
    );
}

#[test]
fn poly_applies_nested_interchange_for_full_matrix_prod_reduction() {
    assert_nested_reduce_lowered(
        "poly_nested_reduce_prod",
        r#"
fn poly_nested_matrix_reduce_prod(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let acc = 1

  let r = 1
  while (r <= n) {
    let c = 1
    while (c <= m) {
      acc = acc * a[r, c]
      c += 1
    }
    r += 1
  }
  return acc
}

print(poly_nested_matrix_reduce_prod(3, 4))
"#,
    );
}

#[test]
fn poly_applies_nested_interchange_for_full_matrix_minmax_reduction() {
    assert_nested_reduce_lowered(
        "poly_nested_reduce_min",
        r#"
fn poly_nested_matrix_reduce_min(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let acc = (n * m) + 10

  let r = 1
  while (r <= n) {
    let c = 1
    while (c <= m) {
      acc = min(acc, a[r, c])
      c += 1
    }
    r += 1
  }
  return acc
}

print(poly_nested_matrix_reduce_min(3, 4))
"#,
    );

    assert_nested_reduce_lowered(
        "poly_nested_reduce_max",
        r#"
fn poly_nested_matrix_reduce_max(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let acc = 0

  let r = 1
  while (r <= n) {
    let c = 1
    while (c <= m) {
      acc = max(acc, a[r, c])
      c += 1
    }
    r += 1
  }
  return acc
}

print(poly_nested_matrix_reduce_max(3, 4))
"#,
    );
}
