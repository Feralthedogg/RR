mod common;

use common::unique_dir;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn imported_module_artifact_roundtrips_and_reduces_parsed_modules() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("module_export_cache_roundtrip");
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
  print(square(4L))
}
main()
"#,
    )
    .expect("failed to write main.rr");
    fs::write(
        &dep_path,
        r#"
fn square(x) {
  return x * x
}
"#,
    )
    .expect("failed to write dep.rr");

    let out_file = proj_dir.join("out.R");
    let profile_first = proj_dir.join("first-profile.json");
    let profile_second = proj_dir.join("second-profile.json");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let first = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O1")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&profile_first)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run first compile");
    assert!(first.success(), "first compile failed");

    let module_cache_dir = cache_dir.join("modules");
    let artifacts: Vec<PathBuf> = fs::read_dir(&module_cache_dir)
        .expect("module cache dir should exist")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    assert!(
        !artifacts.is_empty(),
        "expected at least one module export artifact in {}",
        module_cache_dir.display()
    );
    let artifact_text =
        fs::read_to_string(&artifacts[0]).expect("failed to read module artifact json");
    assert!(artifact_text.contains("\"schema\": \"rr-module-artifact\""));
    assert!(artifact_text.contains("\"schema_version\": 2"));
    assert!(artifact_text.contains("\"public_symbols\""));
    assert!(artifact_text.contains("\"public_function_arities\""));
    assert!(artifact_text.contains("\"emit_roots\""));
    assert!(artifact_text.contains("\"source_metadata\""));

    let second = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O1")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&profile_second)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run second compile");
    assert!(second.success(), "second compile failed");

    let first_profile =
        fs::read_to_string(&profile_first).expect("failed to read first compile profile");
    let second_profile =
        fs::read_to_string(&profile_second).expect("failed to read second compile profile");
    assert!(first_profile.contains("\"parsed_modules\": 2"));
    assert!(first_profile.contains("\"cached_modules\": 0"));
    assert!(second_profile.contains("\"parsed_modules\": 1"));
    assert!(second_profile.contains("\"cached_modules\": 1"));
}

#[test]
fn imported_trait_metadata_roundtrips_through_module_artifact() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("module_trait_cache_roundtrip");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let cache_dir = proj_dir.join(".rr-cache");

    let main_path = proj_dir.join("main.rr");
    let dep_path = proj_dir.join("dep.rr");
    fs::write(
        &dep_path,
        r#"
export trait Physical {
  fn energy(self: Self) -> float
}

export impl Physical for Body {
  fn energy(self: Body) -> float {
    self.mass
  }
}

export fn energy_of<T>(x: T) -> float where T: Physical {
  Physical::energy(x)
}
"#,
    )
    .expect("failed to write dep.rr");
    fs::write(
        &main_path,
        r#"
import "./dep.rr"

fn main() {
  let b: Body = {mass: 3.0}
  print(energy_of::<Body>(b))
}
main()
"#,
    )
    .expect("failed to write main.rr");

    let out_file = proj_dir.join("out.R");
    let profile_first = proj_dir.join("first-profile.json");
    let profile_second = proj_dir.join("second-profile.json");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let first = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O1")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&profile_first)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run first compile");
    assert!(first.success(), "first compile failed");

    let module_cache_dir = cache_dir.join("modules");
    let artifacts: Vec<PathBuf> = fs::read_dir(&module_cache_dir)
        .expect("module cache dir should exist")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    assert!(
        !artifacts.is_empty(),
        "expected at least one module export artifact in {}",
        module_cache_dir.display()
    );
    let artifact_text = fs::read_to_string(&artifacts[0]).expect("read module artifact json");
    assert!(artifact_text.contains("\"source_metadata\""));
    assert!(artifact_text.contains("\"schema_version\": 2"));
    assert!(artifact_text.contains("Physical"));
    assert!(artifact_text.contains("energy_of"));

    let second = Command::new(&rr_bin)
        .arg(&main_path)
        .arg("-o")
        .arg(&out_file)
        .arg("-O1")
        .arg("--no-runtime")
        .arg("--no-incremental")
        .arg("--profile-compile-out")
        .arg(&profile_second)
        .env("RR_INCREMENTAL_CACHE_DIR", &cache_dir)
        .status()
        .expect("failed to run second compile");
    assert!(second.success(), "second compile failed");

    let first_profile =
        fs::read_to_string(&profile_first).expect("failed to read first compile profile");
    let second_profile =
        fs::read_to_string(&profile_second).expect("failed to read second compile profile");
    assert!(first_profile.contains("\"parsed_modules\": 2"));
    assert!(first_profile.contains("\"cached_modules\": 0"));
    assert!(second_profile.contains("\"parsed_modules\": 1"));
    assert!(second_profile.contains("\"cached_modules\": 1"));
}
