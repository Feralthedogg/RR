use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_with_poly(name: &str, rr_src: &str) -> (String, String) {
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
fn poly_applies_for_partial_contiguous_1d_fused_sumprod_reductions() {
    let (stats, emitted) = compile_with_poly(
        "poly_range_multi_reduce_sumprod",
        r#"
fn poly_range_multi_reduce_sumprod(n) {
  let x = seq_len(n)
  let sum_acc = 0
  let prod_acc = 1
  let i = 2

  while (i <= (length(x) - 1)) {
    sum_acc = sum_acc + x[i]
    prod_acc = prod_acc * x[i]
    i += 1
  }

  return sum_acc + prod_acc
}

print(poly_range_multi_reduce_sumprod(8))
"#,
    );

    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_reduce_range(") && !emitted.contains("rr_can_reduce_range("),
        "expected partial fused sum/prod range reductions to lower through generic MIR without helpers, got:\n{}",
        emitted
    );
}

#[test]
fn poly_applies_for_partial_contiguous_1d_fused_minmax_reductions() {
    let (stats, emitted) = compile_with_poly(
        "poly_range_multi_reduce_minmax",
        r#"
fn poly_range_multi_reduce_minmax(n) {
  let x = seq_len(n)
  let min_acc = n + 10
  let max_acc = 0
  let i = 2

  while (i <= (length(x) - 1)) {
    min_acc = min(min_acc, x[i])
    max_acc = max(max_acc, x[i])
    i += 1
  }

  return min_acc + max_acc
}

print(poly_range_multi_reduce_minmax(8))
"#,
    );

    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_reduce_range(") && !emitted.contains("rr_can_reduce_range("),
        "expected partial fused min/max range reductions to lower through generic MIR without helpers, got:\n{}",
        emitted
    );
}
