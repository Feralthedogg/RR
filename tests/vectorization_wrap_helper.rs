use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn wrap_index_helper_calls_rewrite_to_vector_builtin() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn idx_torus(x, y, w, h) {
  let xx = x;
  let yy = y;
  if (xx < 1) { xx = w; }
  if (xx > w) { xx = 1; }
  if (yy < 1) { yy = h; }
  if (yy > h) { yy = 1; }
  return ((yy - 1) * w) + xx;
}

fn lap_x(field, w, h) {
  let size = w * h;
  let out = seq_len(size);
  for (i in 1..size) {
    let y = floor((i - 1) / w) + 1;
    let x = i - (floor((i - 1) / w) * w);
    let l = field[idx_torus(x - 1, y, w, h)];
    let r = field[idx_torus(x + 1, y, w, h)];
    out[i] = l + r;
  }
  return out;
}
"#;

    let rr_path = out_dir.join("wrap_helper_vec.rr");
    let out_path = out_dir.join("wrap_helper_vec.R");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O2")
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let code = fs::read_to_string(&out_path).expect("failed to read compiled R");
    assert!(
        code.contains("rr_wrap_index_vec_i("),
        "expected wrap helper calls to rewrite to rr_wrap_index_vec_i"
    );
    assert!(
        code.contains("rr_index1_read_vec(") || code.contains("rr_index1_read_vec_floor("),
        "expected lap loop to emit vector gather reads"
    );
    assert!(
        code.contains("rr_assign_slice(")
            || code.contains("out <- (rr_index1_read_vec(")
            || code.contains("out <- (rr_index1_read_vec_floor("),
        "expected vector writeback form (slice assign or whole-array assignment)"
    );
}
