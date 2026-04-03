use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_with_tile2d(name: &str, rr_src: &str) -> (String, String) {
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
        .env("RR_POLY_TILE_2D", "1")
        .env("RR_POLY_TILE_ROWS", "2")
        .env("RR_POLY_TILE_COLS", "2")
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
fn poly_tiles_nested_2d_map_when_policy_enabled() {
    let (stats, emitted) = compile_with_tile2d(
        "poly_tile_2d_map",
        r#"
fn poly_tile_2d_map(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let b = matrix(seq_len((n * m)), n, m)
  let out = matrix(seq_len((n * m)), n, m)

  let r = 1
  while (r <= n) {
    let c = 1
    while (c <= m) {
      out[r, c] = a[r, c] + b[r, c]
      c += 1
    }
    r += 1
  }
  return out
}

print(poly_tile_2d_map(4, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_matrix_binop_assign("),
        "expected tiled 2d map to rebuild generic loops without tile helper, got:\n{}",
        emitted
    );
}

#[test]
fn poly_tiles_nested_2d_fused_maps_when_policy_enabled() {
    let (stats, emitted) = compile_with_tile2d(
        "poly_tile_2d_multi_map",
        r#"
fn poly_tile_2d_multi_map(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let b = matrix(seq_len((n * m)), n, m)
  let y = matrix(seq_len((n * m)), n, m)
  let z = matrix(seq_len((n * m)), n, m)

  let r = 1
  while (r <= n) {
    let c = 1
    while (c <= m) {
      y[r, c] = a[r, c] + 1
      z[r, c] = b[r, c] + 2
      c += 1
    }
    r += 1
  }
  return y + z
}

print(poly_tile_2d_multi_map(4, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_matrix_binop_assign("),
        "expected fused tiled 2d maps to rebuild generic loops without tile helpers, got:\n{}",
        emitted
    );
}

#[test]
fn poly_tiles_nested_2d_reduction_when_policy_enabled() {
    let (stats, emitted) = compile_with_tile2d(
        "poly_tile_2d_reduce",
        r#"
fn poly_tile_2d_reduce(n, m) {
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

print(poly_tile_2d_reduce(4, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_matrix_reduce_rect("),
        "expected tiled 2d reduction to rebuild generic loops without tile helper, got:\n{}",
        emitted
    );
}

#[test]
fn poly_tiles_nested_2d_fused_reductions_when_policy_enabled() {
    let (stats, emitted) = compile_with_tile2d(
        "poly_tile_2d_multi_reduce",
        r#"
fn poly_tile_2d_multi_reduce(n, m) {
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

print(poly_tile_2d_multi_reduce(4, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_matrix_reduce_rect("),
        "expected fused tiled 2d reductions to rebuild generic loops without tile helpers, got:\n{}",
        emitted
    );
}

#[test]
fn poly_tiles_nested_2d_fused_minmax_reductions_when_policy_enabled() {
    let (stats, emitted) = compile_with_tile2d(
        "poly_tile_2d_multi_minmax_reduce",
        r#"
fn poly_tile_2d_multi_minmax_reduce(n, m) {
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

print(poly_tile_2d_multi_minmax_reduce(4, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_matrix_reduce_rect("),
        "expected fused tiled 2d minmax reductions to rebuild generic loops without tile helpers, got:\n{}",
        emitted
    );
    assert!(
        emitted.contains("min(") && emitted.contains("max("),
        "expected min/max aggregation in emitted code, got:\n{}",
        emitted
    );
}
