use super::*;
use crate::compiler::{OptLevel, compile};
use std::fs;
use std::path::PathBuf;

#[test]
fn parses_manifest_require_block() {
    let manifest = Manifest::parse(
        r#"
module github.com/acme/app

rr 8.0

require (
    github.com/acme/math v1.2.3
    github.com/acme/plot v0.1.0
)
"#,
    )
    .expect("manifest should parse");
    assert_eq!(manifest.module_path, "github.com/acme/app");
    assert_eq!(
        manifest.requires.get("github.com/acme/math"),
        Some(&"v1.2.3".to_string())
    );
}

#[test]
fn compares_semver_tags_in_numeric_order() {
    assert_eq!(compare_versions("v1.10.0", "v1.2.0"), Ordering::Greater);
    assert_eq!(compare_versions("v1.2.0", "v1.2.0"), Ordering::Equal);
}

#[test]
fn module_path_major_rule_matches_go_style_suffixes() {
    assert!(version_matches_module_path(
        "v0.9.0",
        "github.com/acme/mathlib"
    ));
    assert!(version_matches_module_path(
        "v1.2.3",
        "github.com/acme/mathlib"
    ));
    assert!(!version_matches_module_path(
        "v2.0.0",
        "github.com/acme/mathlib"
    ));
    assert!(version_matches_module_path(
        "v2.1.0",
        "github.com/acme/mathlib/v2"
    ));
    assert!(!version_matches_module_path(
        "v1.9.0",
        "github.com/acme/mathlib/v2"
    ));
}

#[test]
fn resolves_local_subpackage_directory_via_synthetic_entry() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("pkg_module");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = sandbox_root.join(format!("proj_{}", std::process::id()));
    let _ = fs::remove_dir_all(&proj_dir);
    fs::create_dir_all(proj_dir.join("src").join("math")).expect("failed to create project dirs");
    fs::write(proj_dir.join("rr.mod"), "module example/app\n\nrr 8.0\n")
        .expect("failed to write rr.mod");
    fs::write(
        proj_dir.join("src").join("main.rr"),
        r#"
import "example/app/math"

fn main() {
  return plus_two(40L)
}
main()
"#,
    )
    .expect("failed to write main.rr");
    fs::write(
        proj_dir.join("src").join("math").join("inc.rr"),
        r#"
fn inc(x) {
  return x + 1L
}
"#,
    )
    .expect("failed to write inc.rr");
    fs::write(
        proj_dir.join("src").join("math").join("plus_two.rr"),
        r#"
fn plus_two(x) {
  return inc(x) + 1L
}
"#,
    )
    .expect("failed to write plus_two.rr");

    let entry_path = proj_dir.join("src").join("main.rr");
    let source = fs::read_to_string(&entry_path).expect("failed to read main.rr");
    let compiled = compile(&entry_path.to_string_lossy(), &source, OptLevel::O0)
        .expect("compile should resolve subpackage import");
    assert!(
        compiled.0.contains("40L + 1L") || compiled.0.contains("41L + 1L"),
        "compiled artifact should include imported subpackage logic"
    );
}
