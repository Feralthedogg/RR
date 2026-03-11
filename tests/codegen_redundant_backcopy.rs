use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_idx_cube(level: &str, out_path: &std::path::Path) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    let rr_src = r#"
fn round(x) {
  let r = x % 1.0
  if (r >= 0.5) {
    return x - r + 1.0
  }
  return x - r
}

fn idx_cube(f, x, y, size) {
  let ff = round(f)
  let xx = round(x)
  let yy = round(y)
  let ss = round(size)

  if (ff < 1.0) {
    ff = 1.0
  }
  if (ff > 6.0) {
    ff = 6.0
  }
  if (xx < 1.0) {
    xx = 1.0
  }
  if (xx > ss) {
    xx = ss
  }
  if (yy < 1.0) {
    yy = 1.0
  }
  if (yy > ss) {
    yy = ss
  }
  return ((ff - 1.0) * ss * ss) + ((xx - 1.0) * ss) + yy
}

print(idx_cube(9.0, 9.0, 9.0, 4.0))
"#;

    let rr_path = out_dir.join(format!(
        "redundant_backcopy_{}.rr",
        level.trim_start_matches('-')
    ));
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(out_path)
        .arg("--no-runtime")
        .arg(level)
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {} ({})",
        rr_path.display(),
        level
    );

    fs::read_to_string(out_path).expect("failed to read emitted R")
}

#[test]
fn clamp_branches_do_not_emit_redundant_back_copies() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests");

    for level in ["-O0", "-O2"] {
        let out_path = out_dir.join(format!(
            "redundant_backcopy_{}.R",
            level.trim_start_matches('-')
        ));
        let code = compile_idx_cube(level, &out_path);
        assert!(
            !code.contains("xx <- ss\n    ss <- xx"),
            "unexpected redundant back-copy for xx clamp in {}:\n{}",
            level,
            code
        );
        assert!(
            !code.contains("yy <- ss\n    ss <- yy"),
            "unexpected redundant back-copy for yy clamp in {}:\n{}",
            level,
            code
        );
    }
}
