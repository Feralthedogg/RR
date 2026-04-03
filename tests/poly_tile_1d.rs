use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_with_tile(name: &str, rr_src: &str) -> (String, String) {
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
        .env("RR_POLY_TILE_1D", "1")
        .env("RR_POLY_TILE_SIZE", "2")
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
fn poly_tiles_single_1d_map_when_policy_enabled() {
    let (stats, emitted) = compile_with_tile(
        "poly_tile_1d_map",
        r#"
fn poly_tile_1d_map(n) {
  let x = seq_len(n)
  let y = seq_len(n)

  for (i in 1..length(y)) {
    y[i] = x[i] + 1
  }

  return y
}

print(poly_tile_1d_map(8))
"#,
    );

    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_map_range("),
        "expected tiled 1d map to rebuild generic loop without tile helper, got:\n{}",
        emitted
    );
}

#[test]
fn poly_tiles_single_1d_reduction_when_policy_enabled() {
    let (stats, emitted) = compile_with_tile(
        "poly_tile_1d_reduce",
        r#"
fn poly_tile_1d_reduce(n) {
  let x = seq_len(n)
  let s = 0

  for (i in 1..length(x)) {
    s = s + x[i]
  }

  return s
}

print(poly_tile_1d_reduce(8))
"#,
    );

    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_reduce_range("),
        "expected tiled 1d reduction to rebuild generic loop without tile helper, got:\n{}",
        emitted
    );
}

#[test]
fn poly_tiles_fused_1d_maps_when_policy_enabled() {
    let (stats, emitted) = compile_with_tile(
        "poly_tile_1d_multi_map",
        r#"
fn poly_tile_1d_multi_map(n) {
  let a = seq_len(n)
  let b = seq_len(n)
  let y = seq_len(n)
  let z = seq_len(n)

  for (i in 1..length(y)) {
    y[i] = a[i] + 1
    z[i] = b[i] + 2
  }

  return y + z
}

print(poly_tile_1d_multi_map(8))
"#,
    );

    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_map_range("),
        "expected fused tiled 1d maps to rebuild generic loops without tile helpers, got:\n{}",
        emitted
    );
}

#[test]
fn poly_tiles_fused_1d_reductions_when_policy_enabled() {
    let (stats, emitted) = compile_with_tile(
        "poly_tile_1d_multi_reduce",
        r#"
fn poly_tile_1d_multi_reduce(n) {
  let x = seq_len(n)
  let sum_acc = 0
  let prod_acc = 1

  for (i in 1..length(x)) {
    sum_acc = sum_acc + x[i]
    prod_acc = prod_acc * x[i]
  }

  return sum_acc + prod_acc
}

print(poly_tile_1d_multi_reduce(8))
"#,
    );

    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_reduce_range("),
        "expected fused tiled 1d reductions to rebuild generic loops without tile helpers, got:\n{}",
        emitted
    );
}

#[test]
fn poly_tiles_fused_1d_minmax_reductions_when_policy_enabled() {
    let (stats, emitted) = compile_with_tile(
        "poly_tile_1d_multi_minmax_reduce",
        r#"
fn poly_tile_1d_multi_minmax_reduce(n) {
  let x = seq_len(n)
  let min_acc = n + 10
  let max_acc = 0

  for (i in 1..length(x)) {
    min_acc = min(min_acc, x[i])
    max_acc = max(max_acc, x[i])
  }

  return min_acc + max_acc
}

print(poly_tile_1d_multi_minmax_reduce(8))
"#,
    );

    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_reduce_range("),
        "expected fused tiled 1d min/max reductions to rebuild generic loops without tile helpers, got:\n{}",
        emitted
    );
    assert!(
        emitted.contains("min(") && emitted.contains("max("),
        "expected min/max aggregation in emitted code, got:\n{}",
        emitted
    );
}

#[test]
fn poly_tiles_contiguous_2d_column_map_when_policy_enabled() {
    let (stats, emitted) = compile_with_tile(
        "poly_tile_2d_col_map",
        r#"
fn poly_tile_2d_col_map(n) {
  let a = matrix(seq_len((n * 3)), n, 3)
  let b = matrix(seq_len((n * 3)), n, 3)
  let out = matrix(seq_len((n * 3)), n, 3)

  for (i in 1..n) {
    out[i, 2] = a[i, 2] + b[i, 2]
  }

  return out
}

print(poly_tile_2d_col_map(8))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_col_binop_assign("),
        "expected tiled 2d column map to rebuild generic loop without tile helper, got:\n{}",
        emitted
    );
}

#[test]
fn poly_tiles_fused_contiguous_2d_column_reductions_when_policy_enabled() {
    let (stats, emitted) = compile_with_tile(
        "poly_tile_2d_col_multi_reduce",
        r#"
fn poly_tile_2d_col_multi_reduce(n) {
  let a = matrix(seq_len((n * 3)), n, 3)
  let sum_acc = 0
  let prod_acc = 1

  for (i in 1..n) {
    sum_acc = sum_acc + a[i, 2]
    prod_acc = prod_acc * a[i, 3]
  }

  return sum_acc + prod_acc
}

print(poly_tile_2d_col_multi_reduce(8))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_col_reduce_range("),
        "expected fused tiled 2d column reductions to rebuild generic loops without tile helpers, got:\n{}",
        emitted
    );
}

#[test]
fn poly_tiles_contiguous_3d_dim1_map_when_policy_enabled() {
    let (stats, emitted) = compile_with_tile(
        "poly_tile_3d_dim1_map",
        r#"
import r * as base from "base"

fn poly_tile_3d_dim1_map(a, b, out) {
  let i = 1
  while (i <= 3) {
    out[i, 2, 3] = a[i, 2, 3] + b[i, 2, 3]
    i += 1
  }
  return out
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
let b = base.array(seq_len(27), base.c(3, 3, 3))
let out = base.array(rep.int(0, 27), base.c(3, 3, 3))
print(poly_tile_3d_dim1_map(a, b, out))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_dim1_binop_assign("),
        "expected tiled 3d dim1 map to rebuild generic loop without tile helper, got:\n{}",
        emitted
    );
}

#[test]
fn poly_tiles_fused_contiguous_3d_dim1_reductions_when_policy_enabled() {
    let (stats, emitted) = compile_with_tile(
        "poly_tile_3d_dim1_multi_reduce",
        r#"
import r * as base from "base"

fn poly_tile_3d_dim1_multi_reduce(a) {
  let sum_acc = 0
  let prod_acc = 1
  let i = 1

  while (i <= 3) {
    sum_acc = sum_acc + a[i, 2, 3]
    prod_acc = prod_acc * a[i, 1, 2]
    i += 1
  }
  return sum_acc + prod_acc
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
print(poly_tile_3d_dim1_multi_reduce(a))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_dim1_reduce_range("),
        "expected fused tiled 3d dim1 reductions to rebuild generic loops without tile helpers, got:\n{}",
        emitted
    );
}
