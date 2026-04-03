use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn poly_backend_isl_applies_schedule_for_row_major_2d_map() {
    if std::env::var("RR_HAS_ISL").ok().as_deref() != Some("1") {
        return;
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("poly_isl_schedule");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_isl_schedule");

    let rr_src = r#"
fn poly_isl_schedule_2d(n, m) {
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

print(poly_isl_schedule_2d(4, 4))
"#;

    let rr_path = out_dir.join("poly_isl_schedule.rr");
    let out_path = out_dir.join("poly_isl_schedule.R");
    let stats_path = out_dir.join("poly_isl_schedule_stats.json");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_BACKEND", "isl")
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
        stats.contains("\"poly_schedule_applied\": 1")
            && (stats.contains("\"poly_schedule_applied_identity\": 1")
                || stats.contains("\"poly_schedule_applied_interchange\": 1")
                || stats.contains("\"poly_schedule_applied_tile2d\": 1")),
        "expected isl backend to apply a poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_matrix_binop_assign("),
        "expected isl backend schedule to rebuild generic loops without matrix helper, got:\n{}",
        emitted
    );
    assert!(
        (emitted.contains(".__poly_gen_iv_2_r <- (.__poly_gen_iv_2_r + 1L)")
            || emitted.contains(".__poly_gen_iv_tile_2_r <- (.__poly_gen_iv_tile_2_r + 8L)"))
            && (emitted.contains(".__poly_gen_iv_2_c <- (.__poly_gen_iv_2_c + 1L)")
                || emitted.contains(".__poly_gen_iv_tile_2_c <- (.__poly_gen_iv_tile_2_c + 8L)")),
        "expected generic isl schedule to keep row/col loop increments in emitted code, got:\n{}",
        emitted
    );
}
