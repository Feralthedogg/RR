use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn cube_index_alias_store_vectorizes_to_slice_assign() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn idx_cube(f, x, y, size) {
  let ff = round(f)
  let xx = round(x)
  let yy = round(y)
  let ss = round(size)
  if (ff < 1) { ff = 1 }
  if (ff > 6) { ff = 6 }
  if (xx < 1) { xx = 1 }
  if (xx > ss) { xx = ss }
  if (yy < 1) { yy = 1 }
  if (yy > ss) { yy = ss }
  return ((ff - 1) * ss * ss) + ((xx - 1) * ss) + yy
}

fn fill_row(face, row, size) {
  let total = (6 * size) * size
  let out = rep.int(0, total)
  let y = 1
  while (y <= size) {
    let idx = idx_cube(face, row, y, size)
    out[idx] = idx_cube(face, row, y, size)
    y += 1
  }
  return out
}

print(fill_row(2, 3, 4))
"#;

    let rr_path = out_dir.join("cube_alias_slice.rr");
    let out_path = out_dir.join("cube_alias_slice.R");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("--no-incremental")
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
        code.contains("rr_assign_slice("),
        "expected cube helper alias loop to lower to rr_assign_slice"
    );
    assert!(
        !code.contains("out[rr_index1_write(idx"),
        "expected scalar indexed store to disappear after vectorization"
    );
}
