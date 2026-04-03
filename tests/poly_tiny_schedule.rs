use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_with_env(name: &str, rr_src: &str, envs: &[(&str, &str)]) -> (String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");

    let rr_path = out_dir.join(format!("{name}.rr"));
    let out_path = out_dir.join(format!("{name}.R"));
    let stats_path = out_dir.join(format!("{name}_stats.json"));
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let mut cmd = Command::new(&rr_bin);
    cmd.arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_PULSE_JSON_PATH", &stats_path);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let status = cmd.status().expect("failed to run RR compiler");
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
fn poly_does_not_tile_tiny_constant_1d_loop_even_when_policy_enabled() {
    let (stats, emitted) = compile_with_env(
        "poly_tiny_1d_const_map",
        r#"
fn poly_tiny_1d_const_map() {
  let x = seq_len(2)
  let y = seq_len(2)
  for (i in 1..2) {
    y[i] = x[i] + 1
  }
  return y
}

print(poly_tiny_1d_const_map())
"#,
        &[("RR_POLY_TILE_1D", "1"), ("RR_POLY_TILE_SIZE", "8")],
    );
    assert!(
        stats.contains("\"poly_schedule_applied_identity\": 1")
            && stats.contains("\"poly_schedule_applied_tile1d\": 0"),
        "expected identity, not tile1d, for tiny constant 1d loop, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_map_range("),
        "did not expect tiled 1d helper for tiny constant loop, got:\n{}",
        emitted
    );
}

#[test]
fn poly_does_not_tile_tiny_constant_3d_loop_even_when_policy_enabled() {
    let (stats, emitted) = compile_with_env(
        "poly_tiny_3d_const_map",
        r#"
import r * as base from "base"

fn poly_tiny_3d_const_map(a, b, out) {
  let i = 1
  while (i <= 1) {
    let j = 1
    while (j <= 1) {
      let k = 1
      while (k <= 1) {
        out[i, j, k] = a[i, j, k] + b[i, j, k]
        k += 1
      }
      j += 1
    }
    i += 1
  }
  return out
}

let a = base.array(seq_len(1), base.c(1, 1, 1))
let b = base.array(seq_len(1), base.c(1, 1, 1))
let out = base.array(rep.int(0, 1), base.c(1, 1, 1))
print(poly_tiny_3d_const_map(a, b, out))
"#,
        &[
            ("RR_POLY_TILE_3D", "1"),
            ("RR_POLY_TILE_DEPTH", "4"),
            ("RR_POLY_TILE_ROWS", "4"),
            ("RR_POLY_TILE_COLS", "4"),
        ],
    );
    assert!(
        stats.contains("\"poly_schedule_applied_tile3d\": 0"),
        "did not expect tile3d for tiny constant 3d loop, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_tile_array3_binop_cube_assign(")
            && !emitted.contains("rr_array3_binop_cube_assign(")
            && emitted.contains(".__poly_gen_iv_2_i <- (.__poly_gen_iv_2_i + 1L)"),
        "expected tiny 3d constant loop to rebuild generic loops without helpers, got:\n{}",
        emitted
    );
}
