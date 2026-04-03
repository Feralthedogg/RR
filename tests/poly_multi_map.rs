use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn poly_applies_for_fused_multi_store_vector_map() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("poly_multi_map");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_multi_map");

    let rr_src = r#"
fn poly_multi_map(n) {
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

print(poly_multi_map(6))
"#;

    let rr_path = out_dir.join("poly_multi_map.rr");
    let out_path = out_dir.join("poly_multi_map.R");
    let stats_path = out_dir.join("poly_multi_map_stats.json");
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
        emitted.contains("y[.__poly_gen_iv_2_i] <- (a[.__poly_gen_iv_2_i] + 1L)")
            && emitted.contains("z[.__poly_gen_iv_2_i] <- (b[.__poly_gen_iv_2_i] + 2L)"),
        "expected emitted code to retain both fused indexed map expressions, got:\n{}",
        emitted
    );
    assert!(
        !emitted.contains("rr_assign_slice(") && !emitted.contains("rr_can_same_or_scalar("),
        "expected fused 1d maps to rebuild generic loops without slice helpers, got:\n{}",
        emitted
    );
}
