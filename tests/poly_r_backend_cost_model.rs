use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn rr_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_RR"))
}

fn output_text(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn assert_success(output: Output, ctx: &str) {
    assert!(
        output.status.success(),
        "{ctx} failed\n{}",
        output_text(&output)
    );
}

fn write_dense_2d_case(path: &Path) {
    fs::write(
        path,
        r#"
fn dense2d(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let b = matrix(seq_len((n * m)), n, m)
  let out = matrix(rep.int(0, (n * m)), n, m)

  for (r in 1..n) {
    for (c in 1..m) {
      out[r, c] = a[r, c] + b[r, c]
    }
  }
  return out[n, m]
}

print(dense2d(120, 120))
"#,
    )
    .expect("failed to write dense2d source");
}

#[test]
fn r_backend_cost_model_suppresses_auto_tile_but_allows_explicit_tile() {
    if std::env::var("RR_HAS_ISL").ok().as_deref() != Some("1") {
        return;
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("poly_r_backend_cost_model");
    fs::create_dir_all(&out_dir).expect("failed to create poly cost model test dir");
    let rr_path = out_dir.join("dense2d.rr");
    write_dense_2d_case(&rr_path);

    let auto_out = out_dir.join("auto.R");
    let auto_stats = out_dir.join("auto_stats.json");
    let output = Command::new(rr_bin())
        .arg(&rr_path)
        .arg("-o")
        .arg(&auto_out)
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("-O3")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_BACKEND", "isl")
        .env("RR_POLY_GENERIC_MIR", "1")
        .env("RR_PULSE_JSON_PATH", &auto_stats)
        .output()
        .expect("failed to run RR auto poly compile");
    assert_success(output, "auto poly compile");

    let auto_stats_text = fs::read_to_string(&auto_stats).expect("failed to read auto stats");
    let auto_emitted = fs::read_to_string(&auto_out).expect("failed to read auto emitted R");
    assert!(
        auto_stats_text.contains("\"poly_schedule_applied\": 1")
            && auto_stats_text.contains("\"poly_schedule_applied_tile2d\": 0"),
        "expected R cost model to choose a non-tile poly schedule, got:\n{}",
        auto_stats_text
    );
    assert!(
        !auto_emitted.contains(".__poly_gen_iv_tile_"),
        "auto R backend cost model should avoid tile loop code growth, got:\n{}",
        auto_emitted
    );

    let forced_out = out_dir.join("forced_tile.R");
    let forced_stats = out_dir.join("forced_tile_stats.json");
    let output = Command::new(rr_bin())
        .arg(&rr_path)
        .arg("-o")
        .arg(&forced_out)
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("-O3")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_BACKEND", "heuristic")
        .env("RR_POLY_GENERIC_MIR", "1")
        .env("RR_POLY_TILE_2D", "1")
        .env("RR_POLY_TILE_ROWS", "2")
        .env("RR_POLY_TILE_COLS", "2")
        .env("RR_PULSE_JSON_PATH", &forced_stats)
        .output()
        .expect("failed to run RR forced tile poly compile");
    assert_success(output, "forced tile poly compile");

    let forced_stats_text = fs::read_to_string(&forced_stats).expect("failed to read forced stats");
    let forced_emitted = fs::read_to_string(&forced_out).expect("failed to read forced emitted R");
    assert!(
        forced_stats_text.contains("\"poly_schedule_applied_tile2d\": 1"),
        "explicit RR_POLY_TILE_2D must override the R code-size gate, got:\n{}",
        forced_stats_text
    );
    assert!(
        forced_emitted.contains(".__poly_gen_iv_tile_"),
        "forced tile lowering should emit tile loop controls, got:\n{}",
        forced_emitted
    );
}
