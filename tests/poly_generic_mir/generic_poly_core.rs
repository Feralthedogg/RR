use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub(crate) fn compile_generic_poly(name: &str, rr_src: &str) -> (String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");

    let rr_path = out_dir.join(format!("{name}.rr"));
    let out_path = out_dir.join(format!("{name}.R"));
    let stats_path = out_dir.join(format!("{name}_stats.json"));
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_BACKEND", "heuristic")
        .env("RR_POLY_GENERIC_MIR", "1")
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

pub(crate) fn compile_generic_tiled_poly(name: &str, rr_src: &str) -> (String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");

    let rr_path = out_dir.join(format!("{name}.rr"));
    let out_path = out_dir.join(format!("{name}.R"));
    let stats_path = out_dir.join(format!("{name}_stats.json"));
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_BACKEND", "heuristic")
        .env("RR_POLY_GENERIC_MIR", "1")
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

pub(crate) fn compile_generic_tiled2d_poly(name: &str, rr_src: &str) -> (String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");

    let rr_path = out_dir.join(format!("{name}.rr"));
    let out_path = out_dir.join(format!("{name}.R"));
    let stats_path = out_dir.join(format!("{name}_stats.json"));
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_BACKEND", "heuristic")
        .env("RR_POLY_GENERIC_MIR", "1")
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

pub(crate) fn compile_generic_tiled3d_poly(name: &str, rr_src: &str) -> (String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");

    let rr_path = out_dir.join(format!("{name}.rr"));
    let out_path = out_dir.join(format!("{name}.R"));
    let stats_path = out_dir.join(format!("{name}_stats.json"));
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_BACKEND", "heuristic")
        .env("RR_POLY_GENERIC_MIR", "1")
        .env("RR_POLY_TILE_3D", "1")
        .env("RR_POLY_TILE_DEPTH", "2")
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

pub(crate) fn compile_generic_fission_tiled_poly(
    name: &str,
    rr_src: &str,
    envs: &[(&str, &str)],
) -> (String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");

    let rr_path = out_dir.join(format!("{name}.rr"));
    let out_path = out_dir.join(format!("{name}.R"));
    let stats_path = out_dir.join(format!("{name}_stats.json"));
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let mut cmd = Command::new(&rr_bin);
    cmd.arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_BACKEND", "heuristic")
        .env("RR_POLY_GENERIC_MIR", "1")
        .env("RR_POLY_GENERIC_FISSION", "1")
        .env("RR_PULSE_JSON_PATH", &stats_path);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let status = cmd.status().expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let stats = fs::read_to_string(&stats_path).expect("failed to read pulse stats json");
    let emitted = fs::read_to_string(&out_path).expect("failed to read emitted R");
    (stats, emitted)
}

pub(crate) fn compile_default_fission_poly(
    name: &str,
    rr_src: &str,
    envs: &[(&str, &str)],
) -> (String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");

    let rr_path = out_dir.join(format!("{name}.rr"));
    let out_path = out_dir.join(format!("{name}.R"));
    let stats_path = out_dir.join(format!("{name}_stats.json"));
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let mut cmd = Command::new(&rr_bin);
    cmd.arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_BACKEND", "heuristic")
        .env("RR_POLY_FISSION", "1")
        .env("RR_PULSE_JSON_PATH", &stats_path);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let status = cmd.status().expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let stats = fs::read_to_string(&stats_path).expect("failed to read pulse stats json");
    let emitted = fs::read_to_string(&out_path).expect("failed to read emitted R");
    (stats, emitted)
}

pub(crate) fn compile_generic_skew2d_poly(
    name: &str,
    rr_src: &str,
    fission: bool,
) -> (String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");

    let rr_path = out_dir.join(format!("{name}.rr"));
    let out_path = out_dir.join(format!("{name}.R"));
    let stats_path = out_dir.join(format!("{name}_stats.json"));
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let mut cmd = Command::new(&rr_bin);
    cmd.arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_BACKEND", "heuristic")
        .env("RR_POLY_GENERIC_MIR", "1")
        .env("RR_POLY_SKEW_2D", "1")
        .env("RR_PULSE_JSON_PATH", &stats_path);
    if fission {
        cmd.env("RR_POLY_GENERIC_FISSION", "1");
    }
    let status = cmd.status().expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let stats = fs::read_to_string(&stats_path).expect("failed to read pulse stats json");
    let emitted = fs::read_to_string(&out_path).expect("failed to read emitted R");
    (stats, emitted)
}

pub(crate) fn compile_default_skew2d_poly(name: &str, rr_src: &str) -> (String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");

    let rr_path = out_dir.join(format!("{name}.rr"));
    let out_path = out_dir.join(format!("{name}.R"));
    let stats_path = out_dir.join(format!("{name}_stats.json"));
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_BACKEND", "heuristic")
        .env("RR_POLY_SKEW_2D", "1")
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

pub(crate) fn compile_generic_auto_skew2d_poly(name: &str, rr_src: &str) -> (String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");

    let rr_path = out_dir.join(format!("{name}.rr"));
    let out_path = out_dir.join(format!("{name}.R"));
    let stats_path = out_dir.join(format!("{name}_stats.json"));
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_BACKEND", "heuristic")
        .env("RR_POLY_GENERIC_MIR", "1")
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
pub(crate) fn poly_generic_mir_rebuilds_1d_identity_loop_without_helper_fast_path() {
    let (stats, emitted) = compile_generic_poly(
        "poly_generic_identity_map",
        r#"
fn poly_generic_map(n) {
  let x = seq_len(n)
  let y = x
  for (i in 1..length(y)) {
    y[i] = x[i] + 1
  }
  return y
}

print(poly_generic_map(6))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_identity\": 1"),
        "expected generic poly identity apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_can_same_or_scalar(") && !emitted.contains("rr_assign_slice("),
        "expected helper-free poly lowering after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_2d_interchange_loop_without_matrix_helper() {
    let (stats, emitted) = compile_generic_poly(
        "poly_generic_interchange_map",
        r#"
fn poly_generic_interchange(n, m) {
  let a = matrix(seq_len(n * m), n, m)
  let b = a
  let out = b
    for (r in 1..n) {
      for (c in 1..m) {
        out[r, c] = a[r, c] + b[r, c]
      }
    }
  return out
}

print(poly_generic_interchange(4, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_interchange\": 1"),
        "expected generic poly interchange apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_matrix_binop_assign(")
            && !emitted.contains("rr_can_same_matrix_shape_or_scalar(")
            && !emitted.contains("rr_same_len("),
        "expected helper-free poly lowering after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_2d_identity_map_without_matrix_helper() {
    let (stats, emitted) = compile_generic_poly(
        "poly_generic_identity_2d_map",
        r#"
fn poly_generic_identity_2d_map(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let b = matrix(seq_len((n * m)), n, m)
  let out = matrix(seq_len((n * m)), n, m)
  for (c in 1..m) {
    for (r in 1..n) {
      out[r, c] = a[r, c] + b[r, c]
    }
  }
  return out
}

print(poly_generic_identity_2d_map(4, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_identity\": 1"),
        "expected generic 2d identity map apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_matrix_binop_assign("),
        "expected helper-free 2d identity map after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_1d_sum_reduction_without_reduce_helper() {
    let (stats, emitted) = compile_generic_poly(
        "poly_generic_identity_reduce",
        r#"
fn poly_generic_reduce(n) {
  let x = seq_len(n)
  let s = 0
  for (i in 1..length(x)) {
    s = s + x[i]
  }
  return s
}

print(poly_generic_reduce(6))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_identity\": 1"),
        "expected generic poly identity reduce apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_reduce_range(") && !emitted.contains("rr_can_reduce_range("),
        "expected helper-free poly reduction after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_1d_fused_minmax_reduction_without_reduce_helper() {
    let (stats, emitted) = compile_generic_poly(
        "poly_generic_multi_minmax_reduce",
        r#"
fn poly_generic_multi_minmax_reduce(n) {
  let x = seq_len(n)
  let min_acc = n + 10
  let max_acc = 0
  for (i in 1..length(x)) {
    min_acc = min(min_acc, x[i])
    max_acc = max(max_acc, x[i])
  }
  return min_acc + max_acc
}

print(poly_generic_multi_minmax_reduce(6))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_identity\": 1"),
        "expected generic poly fused reduction apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_reduce_range(") && !emitted.contains("rr_can_reduce_range("),
        "expected helper-free fused poly reduction after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_2d_interchange_sum_reduction_without_matrix_reduce_helper()
{
    let (stats, emitted) = compile_generic_poly(
        "poly_generic_interchange_reduce",
        r#"
fn poly_generic_interchange_reduce(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let acc = 0
    for (r in 1..n) {
      for (c in 1..m) {
        acc = acc + a[r, c]
      }
    }
  return acc
}

print(poly_generic_interchange_reduce(4, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_interchange\": 1"),
        "expected generic poly interchange reduction apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_matrix_reduce_rect(")
            && !emitted.contains("rr_can_matrix_reduce_rect("),
        "expected helper-free 2d reduction after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_2d_identity_sum_reduction_without_matrix_reduce_helper() {
    let (stats, emitted) = compile_generic_poly(
        "poly_generic_identity_2d_reduce",
        r#"
fn poly_generic_identity_2d_reduce(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let acc = 0
  for (c in 1..m) {
    for (r in 1..n) {
      acc = acc + a[r, c]
    }
  }
  return acc
}

print(poly_generic_identity_2d_reduce(4, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_identity\": 1"),
        "expected generic 2d identity reduction apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_matrix_reduce_rect("),
        "expected helper-free 2d identity reduction after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_2d_interchange_fused_minmax_reduction_without_matrix_reduce_helper()
 {
    let (stats, emitted) = compile_generic_poly(
        "poly_generic_interchange_multi_minmax_reduce",
        r#"
fn poly_generic_interchange_multi_minmax_reduce(n, m) {
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

print(poly_generic_interchange_multi_minmax_reduce(4, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_interchange\": 1"),
        "expected generic poly fused interchange reduction apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_matrix_reduce_rect(")
            && !emitted.contains("rr_can_matrix_reduce_rect("),
        "expected helper-free fused 2d reduction after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_1d_tile_map_without_tile_helper() {
    let (stats, emitted) = compile_generic_tiled_poly(
        "poly_generic_tile1d_map",
        r#"
fn poly_generic_tile1d_map(n) {
  let x = seq_len(n)
  let y = x
  for (i in 1..length(y)) {
    y[i] = x[i] + 1
  }
  return y
}

print(poly_generic_tile1d_map(8))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_tile1d\": 1"),
        "expected generic poly tile1d map apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_map_range(") && !emitted.contains("rr_can_same_or_scalar("),
        "expected helper-free tile1d map after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_1d_tile_reduce_without_tile_helper() {
    let (stats, emitted) = compile_generic_tiled_poly(
        "poly_generic_tile1d_reduce",
        r#"
fn poly_generic_tile1d_reduce(n) {
  let x = seq_len(n)
  let s = 0
  for (i in 1..length(x)) {
    s = s + x[i]
  }
  return s
}

print(poly_generic_tile1d_reduce(8))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_tile1d\": 1"),
        "expected generic poly tile1d reduction apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_reduce_range(") && !emitted.contains("rr_can_reduce_range("),
        "expected helper-free tile1d reduction after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_2d_tile_map_without_tile_helper() {
    let (stats, emitted) = compile_generic_tiled2d_poly(
        "poly_generic_tile2d_map",
        r#"
fn poly_generic_tile2d_map(n, m) {
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

print(poly_generic_tile2d_map(4, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_tile2d\": 1"),
        "expected generic poly tile2d map apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_matrix_binop_assign("),
        "expected helper-free tile2d map after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_2d_tile_reduce_without_tile_helper() {
    let (stats, emitted) = compile_generic_tiled2d_poly(
        "poly_generic_tile2d_reduce",
        r#"
fn poly_generic_tile2d_reduce(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let acc = 0
  for (r in 1..n) {
    for (c in 1..m) {
      acc = acc + a[r, c]
    }
  }
  return acc
}

print(poly_generic_tile2d_reduce(4, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_tile2d\": 1"),
        "expected generic poly tile2d reduction apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_matrix_reduce_rect("),
        "expected helper-free tile2d reduction after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_3d_interchange_map_without_cube_helper() {
    let (stats, emitted) = compile_generic_poly(
        "poly_generic_interchange_3d_map",
        r#"
import r * as base from "base"
fn poly_generic_interchange_3d_map(a, b, out) {
  let i = 1
  while (i <= 3) {
    let j = 1
    while (j <= 3) {
      let k = 1
      while (k <= 3) {
        out[i, j, k] = a[i, j, k] + b[i, j, k]
        k += 1
      }
      j += 1
    }
    i += 1
  }
  return out
}
let a = base.array(seq_len(27), base.c(3, 3, 3))
let b = base.array(seq_len(27), base.c(3, 3, 3))
let out = base.array(rep.int(0, 27), base.c(3, 3, 3))
print(poly_generic_interchange_3d_map(a, b, out))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_interchange\": 1"),
        "expected generic poly 3d interchange map apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_array3_binop_cube_assign(")
            && !emitted.contains("rr_same_array3_shape_or_scalar("),
        "expected helper-free 3d interchange map after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_3d_interchange_reduce_without_cube_reduce_helper() {
    let (stats, emitted) = compile_generic_poly(
        "poly_generic_interchange_3d_reduce",
        r#"
import r * as base from "base"
fn poly_generic_interchange_3d_reduce(a) {
  let acc = 0
  let i = 1
  while (i <= 3) {
    let j = 1
    while (j <= 3) {
      let k = 1
      while (k <= 3) {
        acc = acc + a[i, j, k]
        k += 1
      }
      j += 1
    }
    i += 1
  }
  return acc
}
let a = base.array(seq_len(27), base.c(3, 3, 3))
print(poly_generic_interchange_3d_reduce(a))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_interchange\": 1"),
        "expected generic poly 3d interchange reduction apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_array3_reduce_cube("),
        "expected helper-free 3d interchange reduction after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_3d_tile_map_without_tile_cube_helper() {
    let (stats, emitted) = compile_generic_tiled3d_poly(
        "poly_generic_tile3d_map",
        r#"
import r * as base from "base"
fn poly_generic_tile3d_map(a, b, out) {
  let i = 1
  while (i <= 3) {
    let j = 1
    while (j <= 3) {
      let k = 1
      while (k <= 3) {
        out[i, j, k] = a[i, j, k] + b[i, j, k]
        k += 1
      }
      j += 1
    }
    i += 1
  }
  return out
}
let a = base.array(seq_len(27), base.c(3, 3, 3))
let b = base.array(seq_len(27), base.c(3, 3, 3))
let out = base.array(rep.int(0, 27), base.c(3, 3, 3))
print(poly_generic_tile3d_map(a, b, out))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_tile3d\": 1"),
        "expected generic poly tile3d map apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_array3_binop_cube_assign("),
        "expected helper-free tile3d map after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_3d_tile_reduce_without_tile_cube_helper() {
    let (stats, emitted) = compile_generic_tiled3d_poly(
        "poly_generic_tile3d_reduce",
        r#"
import r * as base from "base"
fn poly_generic_tile3d_reduce(a) {
  let acc = 0
  let i = 1
  while (i <= 3) {
    let j = 1
    while (j <= 3) {
      let k = 1
      while (k <= 3) {
        acc = acc + a[i, j, k]
        k += 1
      }
      j += 1
    }
    i += 1
  }
  return acc
}
let a = base.array(seq_len(27), base.c(3, 3, 3))
print(poly_generic_tile3d_reduce(a))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_tile3d\": 1"),
        "expected generic poly tile3d reduction apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_array3_reduce_cube("),
        "expected helper-free tile3d reduction after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_2d_column_identity_map_without_column_helper() {
    let (stats, emitted) = compile_generic_poly(
        "poly_generic_column_identity_map",
        r#"
fn poly_generic_column_identity_map(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let out = a
  for (r in 1..n) {
    out[r, 2] = a[r, 2] + 1
  }
  return out
}

print(poly_generic_column_identity_map(6, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_identity\": 1"),
        "expected generic structured-axis identity map apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_col_binop_assign(")
            && !emitted.contains("rr_same_matrix_shape_or_scalar("),
        "expected helper-free structured-axis 2d map after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_2d_column_identity_reduce_without_column_reduce_helper() {
    let (stats, emitted) = compile_generic_poly(
        "poly_generic_column_identity_reduce",
        r#"
fn poly_generic_column_identity_reduce(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let acc = 0
  for (r in 1..n) {
    acc = acc + a[r, 2]
  }
  return acc
}

print(poly_generic_column_identity_reduce(6, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_identity\": 1"),
        "expected generic structured-axis identity reduction apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_col_reduce_range("),
        "expected helper-free structured-axis 2d reduction after generic MIR regeneration, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_offset_2d_column_identity_map_without_column_helper() {
    let (stats, emitted) = compile_generic_poly(
        "poly_generic_column_offset_identity_map",
        r#"
fn poly_generic_column_offset_identity_map(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let out = a
  let n1 = n - 1
  for (r in 1..n1) {
    out[r + 1, 2] = a[r + 1, 2] + 1
  }
  return out
}

print(poly_generic_column_offset_identity_map(6, 4))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_identity\": 1"),
        "expected generic structured-axis offset identity map apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_col_binop_assign("),
        "expected helper-free structured-axis offset 2d map, got:\n{}",
        emitted
    );
}

#[test]
pub(crate) fn poly_generic_mir_rebuilds_3d_dim1_tile_map_without_dim1_helper() {
    let (stats, emitted) = compile_generic_tiled_poly(
        "poly_generic_dim1_tile_map",
        r#"
import r * as base from "base"
fn poly_generic_dim1_tile_map(a, b, out) {
  for (i in 1..3) {
    out[i, 2, 3] = a[i, 2, 3] + b[i, 2, 3]
  }
  return out
}
let a = base.array(seq_len(27), base.c(3, 3, 3))
let b = base.array(seq_len(27), base.c(3, 3, 3))
let out = base.array(rep.int(0, 27), base.c(3, 3, 3))
print(poly_generic_dim1_tile_map(a, b, out))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied_tile1d\": 1"),
        "expected generic structured-axis tile1d map apply, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_dim1_binop_assign("),
        "expected helper-free structured-axis 3d map after generic MIR regeneration, got:\n{}",
        emitted
    );
}
