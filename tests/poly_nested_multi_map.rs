use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn poly_applies_nested_interchange_for_fused_full_matrix_maps() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("poly_nested_multi_map");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_nested_multi_map");

    let rr_src = r#"
fn poly_nested_multi_map(n, m) {
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

print(poly_nested_multi_map(3, 4))
"#;

    let rr_path = out_dir.join("poly_nested_multi_map.rr");
    let out_path = out_dir.join("poly_nested_multi_map.R");
    let stats_path = out_dir.join("poly_nested_multi_map_stats.json");
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
        !emitted.contains("rr_matrix_binop_assign("),
        "expected fused nested maps to rebuild generic loops without matrix helpers, got:\n{}",
        emitted
    );
    assert!(
        !emitted.contains("rr_can_same_matrix_shape_or_scalar("),
        "expected fused nested maps to avoid matrix helper guards, got:\n{}",
        emitted
    );
}
