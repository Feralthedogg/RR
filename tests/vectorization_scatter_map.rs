use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn indirect_scatter_map_vectorizes_to_index_assign() {
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

fn scatter_shift(field, w, h) {
  let size = w * h;
  let out = seq_len(size) * 0;
  for (i in 1..size) {
    let y = floor((i - 1) / w) + 1;
    let x = i - (floor((i - 1) / w) * w);
    out[idx_torus(x + 1, y, w, h)] = field[i];
  }
  return out;
}
"#;

    let rr_path = out_dir.join("scatter_map.rr");
    let out_path = out_dir.join("scatter_map.R");
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
        "expected wrap helper rewrite in scatter loop"
    );
    assert!(
        code.contains("rr_assign_index_vec("),
        "expected indirect scatter loop to lower to rr_assign_index_vec"
    );
}
