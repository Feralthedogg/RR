use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn cube_index_helper_calls_rewrite_to_vector_builtin() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn idx_cube(f, x, y, size) {
  let ff = round(f)

  let xx = round(x)

  let yy = round(y)

  let ss = round(size)

  if (ff < 1) { ff = 1
 }
  if (ff > 6) { ff = 6
 }
  if (xx < 1) { xx = 1
 }
  if (xx > ss) { xx = ss
 }
  if (yy < 1) { yy = 1
 }
  if (yy > ss) { yy = ss
 }
  return ((ff - 1) * ss * ss) + ((xx - 1) * ss) + yy

}

fn scatter_face(field, size) {
  let total = (6 * size) * size

  let out = seq_len(total) * 0

  for (i in 1..size) {
    out[idx_cube(1, i, 1, size)] = field[i]

  }
  return out

}
"#;

    let rr_path = out_dir.join("cube_helper_vec.rr");
    let out_path = out_dir.join("cube_helper_vec.R");
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
        code.contains("rr_idx_cube_vec_i("),
        "expected cube helper calls to rewrite to rr_idx_cube_vec_i"
    );
    assert!(
        code.contains("rr_assign_index_vec("),
        "expected indirect scatter loop to lower to rr_assign_index_vec"
    );
}
