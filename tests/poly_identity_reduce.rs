use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_with_poly_outputs(name: &str, rr_src: &str) -> (String, String) {
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
fn poly_identity_schedule_applies_for_simple_sum_reduction() {
    let (stats, emitted) = compile_with_poly_outputs(
        "poly_identity_reduce",
        r#"
fn poly_reduce(n) {
  let x = seq_len(n)

  let s = 0

  for (i in 1..length(x)) {
    s = s + x[i]

  }
  return s

}

print(poly_reduce(6))
"#,
    );
    assert!(
        !stats.contains("\"poly_scops_detected\": 0"),
        "expected at least one detected poly SCoP, got:\n{}",
        stats
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_reduce_range(") && !emitted.contains("rr_can_reduce_range("),
        "expected 1d sum reduction to rebuild generic loop without range helper, got:\n{}",
        emitted
    );
}

#[test]
fn poly_identity_schedule_applies_for_simple_minmax_reduction() {
    let (stats, emitted) = compile_with_poly_outputs(
        "poly_identity_minmax_reduce",
        r#"
fn poly_minmax_reduce(n) {
  let x = seq_len(n)
  let min_acc = n + 10
  let max_acc = 0

  for (i in 1..length(x)) {
    min_acc = min(min_acc, x[i])
    max_acc = max(max_acc, x[i])
  }

  return min_acc + max_acc
}

print(poly_minmax_reduce(6))
"#,
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_reduce_range(") && !emitted.contains("rr_can_reduce_range("),
        "expected 1d min/max reduction to rebuild generic loops without range helpers, got:\n{}",
        emitted
    );
}
