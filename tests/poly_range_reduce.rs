use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn poly_applies_for_partial_contiguous_1d_sum_reduction() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("poly_range_reduce");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_range_reduce");

    let rr_src = r#"
fn poly_range_reduce(n) {
  let x = seq_len(n)
  let s = 0
  let i = 2

  while (i <= (length(x) - 1)) {
    s = s + x[i]
    i += 1
  }

  return s
}

print(poly_range_reduce(8))
"#;

    let rr_path = out_dir.join("poly_range_reduce.rr");
    let out_path = out_dir.join("poly_range_reduce.R");
    let stats_path = out_dir.join("poly_range_reduce_stats.json");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

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
        !emitted.contains("rr_reduce_range(") && !emitted.contains("rr_can_reduce_range("),
        "expected partial-range 1d reduction to lower through generic MIR without reduce helpers, got:\n{}",
        emitted
    );
}
