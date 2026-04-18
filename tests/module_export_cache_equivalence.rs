mod common;

use common::unique_dir;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn cached_module_artifacts_preserve_emitted_r_output() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("module_export_cache_equivalence");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");

    let main_path = proj_dir.join("main.rr");
    let dep_path = proj_dir.join("dep.rr");
    fs::write(
        &main_path,
        r#"
import "./dep.rr"

fn main() {
  print(dep_sum(5L))
}
main()
"#,
    )
    .expect("failed to write main.rr");
    fs::write(
        &dep_path,
        r#"
fn dep_sum(n) {
  let acc = 0L
  let i = 1L
  while (i <= n) {
    acc = acc + i
    i = i + 1L
  }
  return acc
}
"#,
    )
    .expect("failed to write dep.rr");

    let uncached_out = proj_dir.join("uncached.R");
    let cached_out = proj_dir.join("cached.R");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let uncached = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&uncached_out)
        .arg("-O1")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run uncached compile");
    assert!(uncached.success(), "uncached compile failed");

    let cached = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&cached_out)
        .arg("-O1")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run cached compile");
    assert!(cached.success(), "cached compile failed");

    let uncached_r = fs::read_to_string(&uncached_out).expect("failed to read uncached output");
    let cached_r = fs::read_to_string(&cached_out).expect("failed to read cached output");
    assert_eq!(
        uncached_r, cached_r,
        "module artifact reuse must preserve emitted R"
    );
}
