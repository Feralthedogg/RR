use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn poly_accepts_empty_guard_if_inside_2d_loop_when_vectorization_is_disabled() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("poly_cfg_guard");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_cfg_guard");

    let rr_src = r#"
fn poly_cfg_guard_2d(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let b = matrix(seq_len((n * m)), n, m)
  let out = matrix(seq_len((n * m)), n, m)
  let r = 1
  while (r <= n) {
    let c = 1
    while (c <= m) {
      if (1 <= n) {
      } else {
      }
      out[r, c] = a[r, c] + b[r, c]
      c += 1
    }
    r += 1
  }
  return out
}

print(poly_cfg_guard_2d(4, 4))
"#;

    let rr_path = out_dir.join("poly_cfg_guard.rr");
    let out_path = out_dir.join("poly_cfg_guard.R");
    let stats_path = out_dir.join("poly_cfg_guard_stats.json");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_GENERIC_MIR", "1")
        .env("RR_DISABLE_VECTORIZE", "1")
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
        "expected guarded 2d loop to be accepted by poly path, got:\n{}",
        stats
    );
}

#[test]
fn poly_accepts_affine_guard_alias_inside_2d_loop_when_vectorization_is_disabled() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("poly_cfg_guard");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_cfg_guard");

    let rr_src = r#"
fn poly_cfg_guard_affine_branch_2d(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let b = matrix(seq_len((n * m)), n, m)
  let out = matrix(seq_len((n * m)), n, m)
  let r = 1
  while (r <= n) {
    let c = 1
    while (c <= m) {
      let rr = r
      if (1 <= n) {
        rr = r
      } else {
        rr = r
      }
      out[rr, c] = a[rr, c] + b[rr, c]
      c += 1
    }
    r += 1
  }
  return out
}

print(poly_cfg_guard_affine_branch_2d(4, 4))
"#;

    let rr_path = out_dir.join("poly_cfg_guard_affine.rr");
    let out_path = out_dir.join("poly_cfg_guard_affine.R");
    let stats_path = out_dir.join("poly_cfg_guard_affine_stats.json");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_GENERIC_MIR", "1")
        .env("RR_DISABLE_VECTORIZE", "1")
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
        "expected affine guarded 2d loop to be accepted by poly path, got:\n{}",
        stats
    );
}

#[test]
fn poly_accepts_affine_guard_preamble_inside_2d_loop_when_vectorization_is_disabled() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("poly_cfg_guard");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_cfg_guard");

    let rr_src = r#"
fn poly_cfg_guard_preamble_2d(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let b = matrix(seq_len((n * m)), n, m)
  let out = matrix(seq_len((n * m)), n, m)
  let r = 1
  while (r <= n) {
    let c = 1
    while (c <= m) {
      let rr = r
      let cc = c
      if (1 <= n) {
        rr = r
      } else {
        rr = r
      }
      out[rr, cc] = a[rr, cc] + b[rr, cc]
      c += 1
    }
    r += 1
  }
  return out
}

print(poly_cfg_guard_preamble_2d(4, 4))
"#;

    let rr_path = out_dir.join("poly_cfg_guard_preamble.rr");
    let out_path = out_dir.join("poly_cfg_guard_preamble.R");
    let stats_path = out_dir.join("poly_cfg_guard_preamble_stats.json");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_GENERIC_MIR", "1")
        .env("RR_DISABLE_VECTORIZE", "1")
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
        "expected affine guard preamble 2d loop to be accepted by poly path, got:\n{}",
        stats
    );
}

#[test]
fn poly_accepts_affine_guarded_2d_reduction_when_vectorization_is_disabled() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("poly_cfg_guard");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_cfg_guard");

    let rr_src = r#"
fn poly_cfg_guard_reduce_2d(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let acc = 0
  let r = 1
  while (r <= n) {
    let c = 1
    while (c <= m) {
      let rr = r
      if (1 <= n) {
        rr = r
      } else {
        rr = r
      }
      acc = acc + a[rr, c]
      c += 1
    }
    r += 1
  }
  return acc
}

print(poly_cfg_guard_reduce_2d(4, 4))
"#;

    let rr_path = out_dir.join("poly_cfg_guard_reduce.rr");
    let out_path = out_dir.join("poly_cfg_guard_reduce.R");
    let stats_path = out_dir.join("poly_cfg_guard_reduce_stats.json");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_GENERIC_MIR", "1")
        .env("RR_DISABLE_VECTORIZE", "1")
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
        "expected guarded 2d reduction to be accepted by poly path, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_reduce_range(")
            && !emitted.contains("rr_matrix_reduce_rect(")
            && emitted.contains(".__poly_gen_iv_")
            && emitted.contains("repeat"),
        "expected helper-free guarded 2d reduction generic loop nest, got:\n{}",
        emitted
    );
}

#[test]
fn poly_accepts_dual_affine_guard_aliases_inside_2d_loop_when_vectorization_is_disabled() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("poly_cfg_guard");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_cfg_guard");

    let rr_src = r#"
fn poly_cfg_guard_dual_alias_2d(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let b = matrix(seq_len((n * m)), n, m)
  let out = matrix(seq_len((n * m)), n, m)
  let r = 1
  while (r <= n) {
    let c = 1
    while (c <= m) {
      let rr = r
      let cc = c
      if (1 <= n) {
        rr = r
        cc = c
      } else {
        rr = r
        cc = c
      }
      out[rr, cc] = a[rr, cc] + b[rr, cc]
      c += 1
    }
    r += 1
  }
  return out
}

print(poly_cfg_guard_dual_alias_2d(4, 4))
"#;

    let rr_path = out_dir.join("poly_cfg_guard_dual_alias.rr");
    let out_path = out_dir.join("poly_cfg_guard_dual_alias.R");
    let stats_path = out_dir.join("poly_cfg_guard_dual_alias_stats.json");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_GENERIC_MIR", "1")
        .env("RR_DISABLE_VECTORIZE", "1")
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
        "expected dual affine guarded 2d loop to be accepted by poly path, got:\n{}",
        stats
    );
}

#[test]
fn poly_accepts_dual_affine_guarded_2d_reduction_when_vectorization_is_disabled() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("poly_cfg_guard");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_cfg_guard");

    let rr_src = r#"
fn poly_cfg_guard_reduce_dual_2d(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let acc = 0
  let r = 1
  while (r <= n) {
    let c = 1
    while (c <= m) {
      let rr = r
      let cc = c
      if (1 <= n) {
        rr = r
        cc = c
      } else {
        rr = r
        cc = c
      }
      acc = acc + a[rr, cc]
      c += 1
    }
    r += 1
  }
  return acc
}

print(poly_cfg_guard_reduce_dual_2d(4, 4))
"#;

    let rr_path = out_dir.join("poly_cfg_guard_reduce_dual.rr");
    let out_path = out_dir.join("poly_cfg_guard_reduce_dual.R");
    let stats_path = out_dir.join("poly_cfg_guard_reduce_dual_stats.json");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_GENERIC_MIR", "1")
        .env("RR_DISABLE_VECTORIZE", "1")
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
        "expected dual guarded 2d reduction to be accepted by poly path, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_reduce_range(")
            && !emitted.contains("rr_matrix_reduce_rect(")
            && emitted.contains("repeat")
            && emitted.contains(".__poly_gen_iv_"),
        "expected helper-free dual guarded 2d reduction loop nest, got:\n{}",
        emitted
    );
}

#[test]
fn poly_accepts_preamble_dual_affine_guarded_2d_reduction_when_vectorization_is_disabled() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("poly_cfg_guard");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_cfg_guard");

    let rr_src = r#"
fn poly_cfg_guard_reduce_preamble_dual_2d(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let acc = 0
  let r = 1
  while (r <= n) {
    let c = 1
    while (c <= m) {
      let rr = r
      let cc = c
      if (1 <= n) {
        rr = r
        cc = c
      } else {
        rr = r
        cc = c
      }
      acc = acc + a[rr, cc]
      c += 1
    }
    r += 1
  }
  return acc
}

print(poly_cfg_guard_reduce_preamble_dual_2d(4, 4))
"#;

    let rr_path = out_dir.join("poly_cfg_guard_reduce_preamble_dual.rr");
    let out_path = out_dir.join("poly_cfg_guard_reduce_preamble_dual.R");
    let stats_path = out_dir.join("poly_cfg_guard_reduce_preamble_dual_stats.json");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_GENERIC_MIR", "1")
        .env("RR_DISABLE_VECTORIZE", "1")
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
        "expected preamble dual guarded 2d reduction to be accepted by poly path, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_reduce_range(")
            && !emitted.contains("rr_matrix_reduce_rect(")
            && emitted.contains("repeat")
            && emitted.contains(".__poly_gen_iv_"),
        "expected helper-free preamble dual guarded 2d reduction loop nest, got:\n{}",
        emitted
    );
}
