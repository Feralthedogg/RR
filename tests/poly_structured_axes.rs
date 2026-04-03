use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_with_poly(name: &str, rr_src: &str) -> String {
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

    fs::read_to_string(&stats_path).expect("failed to read pulse stats json")
}

fn compile_with_poly_outputs(name: &str, rr_src: &str) -> (String, String) {
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
fn poly_applies_for_contiguous_2d_column_map() {
    let (stats, emitted) = compile_with_poly_outputs(
        "poly_2d_col_map",
        r#"
fn poly_2d_col_map(n) {
  let a = matrix(seq_len((n * 3)), n, 3)

  let b = matrix(seq_len((n * 3)), n, 3)

  let out = matrix(seq_len((n * 3)), n, 3)

  for (i in 1..n) {
    out[i, 2] = a[i, 2] + b[i, 2]

  }
  return out

}

print(poly_2d_col_map(4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_can_same_matrix_shape_or_scalar(")
            && !emitted.contains("rr_matrix_binop_assign("),
        "expected single 2d column map to rebuild generic loop without matrix helper, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_for_fused_contiguous_2d_column_maps() {
    let stats = compile_with_poly(
        "poly_2d_col_multi_map",
        r#"
fn poly_2d_col_multi_map(n) {
  let a = matrix(seq_len((n * 3)), n, 3)
  let b = matrix(seq_len((n * 3)), n, 3)
  let y = matrix(seq_len((n * 3)), n, 3)
  let z = matrix(seq_len((n * 3)), n, 3)

  for (i in 1..n) {
    y[i, 2] = a[i, 2] + 1
    z[i, 3] = b[i, 3] + 2
  }
  return y + z
}

print(poly_2d_col_multi_map(4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
}

#[test]
fn poly_applies_for_contiguous_2d_column_reduction() {
    let (stats, emitted) = compile_with_poly_outputs(
        "poly_2d_col_reduce",
        r#"
fn poly_2d_col_reduce(n) {
  let a = matrix(seq_len((n * 3)), n, 3)

  let acc = 0

  for (i in 1..n) {
    acc = acc + a[i, 2]

  }
  return acc

}

print(poly_2d_col_reduce(4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_can_col_reduce_range(")
            && !emitted.contains("rr_matrix_reduce_rect("),
        "expected single 2d column reduction to rebuild generic loop without helper, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_for_fused_contiguous_2d_column_reductions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("poly_2d_col_multi_reduce");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");
    let rr_path = out_dir.join("poly_2d_col_multi_reduce.rr");
    let out_path = out_dir.join("poly_2d_col_multi_reduce.R");
    let stats_path = out_dir.join("poly_2d_col_multi_reduce_stats.json");
    fs::write(
        &rr_path,
        r#"
fn poly_2d_col_multi_reduce(n) {
  let a = matrix(seq_len((n * 3)), n, 3)
  let sum_acc = 0
  let prod_acc = 1

  for (i in 1..n) {
    sum_acc = sum_acc + a[i, 2]
    prod_acc = prod_acc * a[i, 3]
  }
  return sum_acc + prod_acc
}

print(poly_2d_col_multi_reduce(4))
"#,
    )
    .expect("failed to write RR source");
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
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_can_col_reduce_range(")
            && !emitted.contains("rr_matrix_reduce_rect("),
        "expected fused 2d column reductions to rebuild generic loops without helpers, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_for_fused_contiguous_2d_column_minmax_reductions() {
    let stats = compile_with_poly(
        "poly_2d_col_multi_minmax_reduce",
        r#"
fn poly_2d_col_multi_minmax_reduce(n) {
  let a = matrix(seq_len((n * 3)), n, 3)
  let min_acc = (n * 3) + 10
  let max_acc = 0

  for (i in 1..n) {
    min_acc = min(min_acc, a[i, 2])
    max_acc = max(max_acc, a[i, 3])
  }
  return min_acc + max_acc
}

print(poly_2d_col_multi_minmax_reduce(4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
}

#[test]
fn poly_applies_for_contiguous_3d_dim1_map() {
    let (stats, emitted) = compile_with_poly_outputs(
        "poly_3d_dim1_map",
        r#"
import r * as base from "base"

fn poly_3d_dim1_map(a, b, out) {
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
print(poly_3d_dim1_map(a, b, out))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_can_same_array3_shape_or_scalar(")
            && !emitted.contains("rr_dim1_binop_assign("),
        "expected single 3d dim1 map to rebuild generic loop without dim1 helper, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_for_fused_contiguous_3d_dim1_maps() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("poly_3d_dim1_multi_map");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");

    let rr_src = r#"
import r * as base from "base"

fn poly_3d_dim1_multi_map(a, b, y, z) {
  let i = 1

  while (i <= 3) {
    y[i, 2, 3] = a[i, 2, 3] + 1
    z[i, 1, 2] = b[i, 1, 2] + 2
    i += 1
  }
  return y
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
let b = base.array(seq_len(27), base.c(3, 3, 3))
let y = base.array(rep.int(0, 27), base.c(3, 3, 3))
let z = base.array(rep.int(0, 27), base.c(3, 3, 3))
print(poly_3d_dim1_multi_map(a, b, y, z))
"#;

    let rr_path = out_dir.join("poly_3d_dim1_multi_map.rr");
    let out_path = out_dir.join("poly_3d_dim1_multi_map.R");
    let stats_path = out_dir.join("poly_3d_dim1_multi_map_stats.json");
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
        !emitted.contains("rr_can_same_array3_shape_or_scalar(")
            && !emitted.contains("rr_dim1_binop_assign("),
        "expected fused 3d dim1 maps to rebuild generic loops without dim1 helpers, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_for_contiguous_3d_dim1_reduction() {
    let (stats, emitted) = compile_with_poly_outputs(
        "poly_3d_dim1_reduce",
        r#"
import r * as base from "base"

fn poly_3d_dim1_reduce(a) {
  let acc = 0

  let i = 1

  while (i <= 3) {
    acc = acc + a[i, 2, 3]
    i += 1
  }
  return acc

}

let a = base.array(seq_len(27), base.c(3, 3, 3))
print(poly_3d_dim1_reduce(a))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_can_dim1_reduce_range(")
            && !emitted.contains("rr_array3_reduce_cube("),
        "expected single 3d dim1 reduction to rebuild generic loop without helper, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_for_fused_contiguous_3d_dim1_reductions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("poly_3d_dim1_multi_reduce");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");
    let rr_path = out_dir.join("poly_3d_dim1_multi_reduce.rr");
    let out_path = out_dir.join("poly_3d_dim1_multi_reduce.R");
    let stats_path = out_dir.join("poly_3d_dim1_multi_reduce_stats.json");
    fs::write(
        &rr_path,
        r#"
import r * as base from "base"

fn poly_3d_dim1_multi_reduce(a) {
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
print(poly_3d_dim1_multi_reduce(a))
"#,
    )
    .expect("failed to write RR source");
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
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_can_dim1_reduce_range(")
            && !emitted.contains("rr_array3_reduce_cube("),
        "expected fused 3d dim1 reductions to rebuild generic loops without helpers, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_for_fused_contiguous_3d_dim1_minmax_reductions() {
    let stats = compile_with_poly(
        "poly_3d_dim1_multi_minmax_reduce",
        r#"
import r * as base from "base"

fn poly_3d_dim1_multi_minmax_reduce(a) {
  let min_acc = 100
  let max_acc = 0

  let i = 1

  while (i <= 3) {
    min_acc = min(min_acc, a[i, 2, 3])
    max_acc = max(max_acc, a[i, 1, 2])
    i += 1
  }
  return min_acc + max_acc
}

let a = base.array(seq_len(27), base.c(3, 3, 3))
print(poly_3d_dim1_multi_minmax_reduce(a))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
}
