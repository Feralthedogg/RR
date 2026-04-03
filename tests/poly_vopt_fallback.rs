use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_with_stats(name: &str, rr_src: &str, extra_env: &[(&str, &str)]) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");

    let rr_path = out_dir.join(format!("{name}.rr"));
    let out_path = out_dir.join(format!("{name}.R"));
    let stats_path = out_dir.join(format!("{name}_stats.json"));
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let mut cmd = Command::new(&rr_bin);
    cmd.arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_PULSE_JSON_PATH", &stats_path);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let status = cmd.status().expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    fs::read_to_string(&stats_path).expect("failed to read pulse stats json")
}

#[test]
fn poly_preempts_legacy_vopt_for_simple_1d_map() {
    let stats = compile_with_stats(
        "poly_preempts_vopt_1d_map",
        r#"
fn poly_preempts_vopt_1d_map(n) {
  let x = seq_len(n)
  let y = x
  for (i in 1..length(y)) {
    y[i] = x[i] + 1
  }
  return y
}

print(poly_preempts_vopt_1d_map(8))
"#,
        &[],
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1")
            && stats.contains("\"vector_applied_total\": 0")
            && stats.contains("\"vector_legacy_poly_fallback_applied_total\": 0"),
        "expected poly to preempt legacy v_opt 1d map path, got:\n{}",
        stats
    );
}

#[test]
fn poly_preempts_legacy_vopt_for_nested_2d_interchange_map() {
    let stats = compile_with_stats(
        "poly_preempts_vopt_2d_interchange",
        r#"
fn poly_preempts_vopt_2d_interchange(n, m) {
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

print(poly_preempts_vopt_2d_interchange(4, 4))
"#,
        &[],
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1")
            && stats.contains("\"vector_applied_total\": 0")
            && stats.contains("\"vector_legacy_poly_fallback_applied_total\": 0"),
        "expected poly to preempt legacy v_opt 2d interchange path, got:\n{}",
        stats
    );
}

#[test]
fn poly_preempts_legacy_vopt_for_nested_3d_tile_map() {
    let stats = compile_with_stats(
        "poly_preempts_vopt_3d_tile_map",
        r#"
import r * as base from "base"
fn poly_preempts_vopt_3d_tile_map(a, b, out) {
  let i = 1
  while (i <= 3) {
    let j = 1
    while (j <= 3) {
      let k = 1
      while (k <= 3) {
        out[i, j, k] = a[i, j, k] + b[i, j, k]
        k += 1
      }
      j += 1
    }
    i += 1
  }
  return out
}
let a = base.array(seq_len(27), base.c(3, 3, 3))
let b = base.array(seq_len(27), base.c(3, 3, 3))
let out = base.array(rep.int(0, 27), base.c(3, 3, 3))
print(poly_preempts_vopt_3d_tile_map(a, b, out))
"#,
        &[
            ("RR_POLY_GENERIC_MIR", "1"),
            ("RR_POLY_TILE_3D", "1"),
            ("RR_POLY_TILE_DEPTH", "2"),
            ("RR_POLY_TILE_ROWS", "2"),
            ("RR_POLY_TILE_COLS", "2"),
        ],
    );
    assert!(
        stats.contains("\"poly_schedule_applied_tile3d\": 1")
            && stats.contains("\"vector_applied_total\": 0")
            && stats.contains("\"vector_legacy_poly_fallback_applied_total\": 0")
            && stats.contains("\"vector_legacy_poly_fallback_candidate_total\": 0"),
        "expected poly to preempt legacy v_opt 3d tile map path, got:\n{}",
        stats
    );
}
