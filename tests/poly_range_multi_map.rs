use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn poly_applies_for_partial_contiguous_1d_fused_maps() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("poly_range_multi_map");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_range_multi_map");

    let rr_src = r#"
fn poly_range_multi_map(n) {
  let a = seq_len(n)
  let b = seq_len(n)
  let y = seq_len(n)
  let z = seq_len(n)
  let i = 2

  while (i <= (length(y) - 1)) {
    y[i] = a[i] + 1
    z[i] = b[i] + 2
    i += 1
  }

  return y + z
}

print(poly_range_multi_map(8))
"#;

    let rr_path = out_dir.join("poly_range_multi_map.rr");
    let out_path = out_dir.join("poly_range_multi_map.R");
    let stats_path = out_dir.join("poly_range_multi_map_stats.json");
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
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_assign_slice(")
            && !emitted.contains("rr_can_same_or_scalar(")
            && !emitted.contains("rr_same_len("),
        "expected partial-range fused 1d maps to lower through generic MIR without slice helpers, got:\n{}",
        emitted
    );
}
