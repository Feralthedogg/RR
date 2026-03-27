use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn wrap_index_helper_calls_rewrite_to_vector_builtin() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn idx_torus(x, y, w, h) {
  let xx = x

  let yy = y

  if (xx < 1) { xx = w
 }
  if (xx > w) { xx = 1
 }
  if (yy < 1) { yy = h
 }
  if (yy > h) { yy = 1
 }
  return ((yy - 1) * w) + xx

}

fn lap_x(field, w, h) {
  let size = w * h

  let out = seq_len(size)

  for (i in 1..size) {
    let y = floor((i - 1) / w) + 1

    let x = i - (floor((i - 1) / w) * w)

    let l = field[idx_torus(x - 1, y, w, h)]

    let r = field[idx_torus(x + 1, y, w, h)]

    out[i] = l + r

  }
  return out

}
"#;

    let rr_path = out_dir.join("wrap_helper_vec.rr");
    let out_path = out_dir.join("wrap_helper_vec.R");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("--preserve-all-defs")
        .arg("-O2")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let code = fs::read_to_string(&out_path).expect("failed to read compiled R");
    assert!(
        code.contains("rr_wrap_index_vec_i("),
        "expected wrap helper calls to rewrite to rr_wrap_index_vec_i"
    );
    assert!(
        code.contains("rr_gather(")
            || code.contains("rr_index1_read_vec(")
            || code.contains("rr_index1_read_vec_floor("),
        "expected lap loop to emit vector gather reads"
    );
    assert!(
        code.contains("rr_assign_slice(")
            || code.contains("out <- (rr_gather(")
            || code.contains("out <- (rr_index1_read_vec(")
            || code.contains("out <- (rr_index1_read_vec_floor("),
        "expected vector writeback form (slice assign or whole-array assignment)"
    );
}

#[test]
fn periodic_1d_helper_calls_rewrite_to_wrap_index_builtin() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn periodic_left(i, n) {
  if (i <= 1) {
    return n
  }
  return i - 1
}

fn periodic_right(i, n) {
  if (i >= n) {
    return 1
  }
  return i + 1
}

fn lap_1d(field) {
  let n = length(field)
  let out = seq_len(n)
  for (i in 1..n) {
    let l = field[periodic_left(i, n)]
    let r = field[periodic_right(i, n)]
    out[i] = l + r
  }
  return out
}
"#;

    let rr_path = out_dir.join("periodic_wrap_helper_vec.rr");
    let out_path = out_dir.join("periodic_wrap_helper_vec.R");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("--preserve-all-defs")
        .arg("-O2")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let code = fs::read_to_string(&out_path).expect("failed to read compiled R");
    assert!(
        code.contains("rr_wrap_index_vec_i("),
        "expected periodic 1d helper calls to rewrite to rr_wrap_index_vec_i"
    );
    assert!(
        code.contains("rr_gather(")
            || code.contains("rr_index1_read_vec(")
            || code.contains("rr_index1_read_vec_floor("),
        "expected periodic 1d helper loop to emit vector gather reads"
    );
}

#[test]
fn trivial_abs_helper_calls_rewrite_to_builtin_abs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn abs1(x) {
  if (x < 0) {
    return 0 - x
  }
  return x
}

fn vector_abs_sum(xs) {
  let s = 0
  for (i in 1..length(xs)) {
    s += abs1(xs[i])
  }
  return s
}

print(vector_abs_sum(c(-1, 2, -3, 4)))
"#;

    let rr_path = out_dir.join("abs_helper_vec.rr");
    let out_path = out_dir.join("abs_helper_vec.R");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("-O2")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let code = fs::read_to_string(&out_path).expect("failed to read compiled R");
    assert!(
        code.contains("abs("),
        "expected trivial abs helper calls to rewrite to builtin abs"
    );
}

#[test]
fn unit_index_helper_calls_rewrite_to_clamped_floor_expr() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn floor1(x) {
  return x - (x % 1)
}

fn unit_index(u, n) {
  let idx = 1 + floor1(u * n)
  if (idx < 1) {
    idx = 1
  }
  if (idx > n) {
    idx = n
  }
  return idx
}

fn main(draws, n) {
  let out = 0
  for (i in 1..length(draws)) {
    out += unit_index(draws[i], n)
  }
  return out
}

print(main(c(0.1, 0.7, 1.2), 10))
"#;

    let rr_path = out_dir.join("unit_index_helper_vec.rr");
    let out_path = out_dir.join("unit_index_helper_vec.R");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("-O2")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let code = fs::read_to_string(&out_path).expect("failed to read compiled R");
    assert!(
        code.contains("pmin(pmax(") || (code.contains("pmin(") && code.contains("pmax(")),
        "expected unit_index helper calls to rewrite to clamped floor expression"
    );
    assert!(
        !code.contains("unit_index("),
        "expected unit_index helper to inline away from generated code"
    );
}

#[test]
fn trivial_minmax_helper_calls_rewrite_to_builtin_pmin_pmax() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn min1(a, b) {
  if (a < b) {
    return a
  }
  return b
}

fn max1(a, b) {
  if (a > b) {
    return a
  }
  return b
}

fn main(xs) {
  let acc = 0
  for (i in 1..length(xs)) {
    let x = xs[i]
    acc += min1(x, max1(2, x))
  }
  return acc
}

print(main(c(1, 3, 5)))
"#;

    let rr_path = out_dir.join("minmax_helper_vec.rr");
    let out_path = out_dir.join("minmax_helper_vec.R");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("-O2")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let code = fs::read_to_string(&out_path).expect("failed to read compiled R");
    assert!(
        code.contains("pmin(") || code.contains("pmax("),
        "expected trivial min/max helpers to rewrite to builtin pmin/pmax"
    );
    assert!(
        !code.contains("min1(") && !code.contains("max1("),
        "expected min1/max1 helpers to inline away from generated code"
    );
}
