use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_with_dump(name: &str, rr_src: &str, envs: &[(&str, &str)]) -> Vec<String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");

    let rr_path = out_dir.join(format!("{name}.rr"));
    let out_path = out_dir.join(format!("{name}.R"));
    let dump_dir = out_dir.join("poly_dump");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let mut cmd = Command::new(&rr_bin);
    cmd.arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_GENERIC_MIR", "1")
        .env("RR_POLY_DUMP_DIR", &dump_dir);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let status = cmd.status().expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    fs::read_dir(&dump_dir)
        .expect("failed to read dump dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".poly.txt"))
        .map(|name| {
            fs::read_to_string(dump_dir.join(name)).expect("failed to read poly certificate dump")
        })
        .collect()
}

#[test]
fn poly_dump_marks_fission_split_for_multi_stmt_tile1d_tree() {
    let certs = compile_with_dump(
        "poly_tree_dump_fission_tile1d",
        r#"
fn poly_tree_dump_fission_tile1d(n) {
  let x = seq_len(n)
  let y = x
  let z = x
  for (i in 1..length(x)) {
    y[i] = x[i] + 1
    z[i] = x[i] + 2
  }
  return y
}

print(poly_tree_dump_fission_tile1d(8))
"#,
        &[
            ("RR_POLY_FISSION", "1"),
            ("RR_POLY_TILE_1D", "1"),
            ("RR_POLY_TILE_SIZE", "2"),
        ],
    );
    assert!(
        certs.iter().any(|cert| {
            cert.contains("schedule_tree_split_nodes:")
                && !cert.contains("schedule_tree_split_nodes: 0")
                && cert.contains("fissioned-statement-filter")
        }),
        "expected fission split nodes in some certificate, got:\n{}",
        certs.join("\n---\n")
    );
}

#[test]
fn poly_dump_auto_marks_fused_nodes_for_cross_flow_multi_stmt_tile1d_tree() {
    let certs = compile_with_dump(
        "poly_tree_dump_auto_fuse_tile1d",
        r#"
fn poly_tree_dump_auto_fuse_tile1d(n) {
  let x = seq_len(n)
  let y = x
  let z = x
  for (i in 1..length(x)) {
    y[i] = x[i] + 1
    z[i] = y[i] + 2
  }
  return z
}

print(poly_tree_dump_auto_fuse_tile1d(8))
"#,
        &[("RR_POLY_TILE_1D", "1"), ("RR_POLY_TILE_SIZE", "2")],
    );
    assert!(
        certs.iter().any(|cert| {
            cert.contains("schedule_tree_fuse_nodes:")
                && !cert.contains("schedule_tree_fuse_nodes: 0")
                && cert.contains("auto-fused-statements")
                && cert.contains("schedule_decision_reason:")
        }),
        "expected automatic fused nodes in some certificate, got:\n{}",
        certs.join("\n---\n")
    );
}

#[test]
fn poly_dump_auto_marks_fission_split_for_profitable_multi_stmt_tile1d_tree() {
    let certs = compile_with_dump(
        "poly_tree_dump_auto_fission_tile1d",
        r#"
fn poly_tree_dump_auto_fission_tile1d(n) {
  let x = seq_len(n)
  let y = x
  let z = x
  for (i in 1..length(x)) {
    y[i] = x[i] + 1
    z[i] = x[i] + 2
  }
  return y
}

print(poly_tree_dump_auto_fission_tile1d(8))
"#,
        &[("RR_POLY_TILE_1D", "1"), ("RR_POLY_TILE_SIZE", "2")],
    );
    assert!(
        certs.iter().any(|cert| {
            cert.contains("schedule_tree_split_nodes:")
                && !cert.contains("schedule_tree_split_nodes: 0")
                && cert.contains("auto-fission-split")
                && cert.contains("schedule_decision_reason:")
                && cert.contains("schedule_fission_benefit:")
        }),
        "expected automatic fission split nodes in some certificate, got:\n{}",
        certs.join("\n---\n")
    );
}

#[test]
fn poly_dump_marks_skew_nodes_for_skew2d_tree() {
    let certs = compile_with_dump(
        "poly_tree_dump_skew2d",
        r#"
fn poly_tree_dump_skew2d(n, m) {
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

print(poly_tree_dump_skew2d(4, 4))
"#,
        &[("RR_POLY_SKEW_2D", "1")],
    );
    assert!(
        certs.iter().any(|cert| {
            cert.contains("schedule_tree_skew_nodes:")
                && !cert.contains("schedule_tree_skew_nodes: 0")
                && cert.contains("kind: Skew")
                && cert.contains("schedule_decision_reason:")
        }),
        "expected skew nodes in some certificate, got:\n{}",
        certs.join("\n---\n")
    );
}

#[test]
fn poly_dump_auto_marks_skew_nodes_for_fused_dense_2d_tree() {
    let certs = compile_with_dump(
        "poly_tree_dump_auto_skew2d",
        r#"
fn poly_tree_dump_auto_skew2d(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let b = matrix(seq_len((n * m)), n, m)
  let y = matrix(seq_len((n * m)), n, m)
  let z = matrix(seq_len((n * m)), n, m)
  for (r in 1..n) {
    for (c in 1..m) {
      y[r, c] = a[r, c] + 1
      z[r, c] = b[r, c] + 2
    }
  }
  return y
}

print(poly_tree_dump_auto_skew2d(4, 4))
"#,
        &[],
    );
    assert!(
        certs.iter().any(|cert| {
            cert.contains("schedule_tree_skew_nodes:")
                && !cert.contains("schedule_tree_skew_nodes: 0")
                && cert.contains("kind: Skew")
                && cert.contains("schedule_decision_reason:")
        }),
        "expected automatic skew nodes in some certificate, got:\n{}",
        certs.join("\n---\n")
    );
}
