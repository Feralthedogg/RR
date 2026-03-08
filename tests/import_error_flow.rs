mod common;

use RR::compiler::{OptLevel, compile};
use RR::error::{RRCode, Stage};
use common::unique_dir;
use std::fs;
use std::path::PathBuf;

#[test]
fn missing_import_returns_error_instead_of_exiting() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("import_error_flow");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let entry_path = proj_dir.join("main.rr");
    let source = r#"
import "missing.rr"
fn main() {
  return 1
}
main()
"#;
    fs::write(&entry_path, source).expect("failed to write main.rr");

    let result = compile(&entry_path.to_string_lossy(), source, OptLevel::O0);

    let err = result.expect_err("expected compile to fail on missing import");
    assert_eq!(err.module, "RR.ParseError");
    assert!(matches!(err.code, RRCode::E0001));
    assert!(matches!(err.stage, Stage::Parse));
    assert!(
        err.message.contains("failed to load imported module"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn conflicting_named_r_import_reports_previous_binding() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("import_error_flow");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let entry_path = proj_dir.join("main.rr");
    let source = r#"
import r { plot } from "graphics"
import r { plot } from "stats"
"#;
    fs::write(&entry_path, source).expect("failed to write main.rr");

    let result = compile(&entry_path.to_string_lossy(), source, OptLevel::O0);

    let err = result.expect_err("expected compile to fail on conflicting named R import");
    assert_eq!(err.module, "RR.SemanticError");
    assert!(matches!(err.code, RRCode::E1002));
    assert!(matches!(err.stage, Stage::Lower));
    assert!(
        err.message
            .contains("R import local 'plot' is already bound to 'graphics::plot'"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn conflicting_namespace_alias_reports_imported_symbol() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("import_error_flow");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let entry_path = proj_dir.join("main.rr");
    let source = r#"
import r { plot as graphics } from "graphics"
import r * as graphics from "grDevices"
"#;
    fs::write(&entry_path, source).expect("failed to write main.rr");

    let result = compile(&entry_path.to_string_lossy(), source, OptLevel::O0);

    let err = result.expect_err("expected compile to fail on conflicting namespace alias");
    assert_eq!(err.module, "RR.SemanticError");
    assert!(matches!(err.code, RRCode::E1002));
    assert!(matches!(err.stage, Stage::Lower));
    assert!(
        err.message.contains(
            "R namespace alias 'graphics' conflicts with imported symbol 'graphics::plot'"
        ),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn package_name_import_conflicts_with_named_symbol_alias() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("import_error_flow");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let entry_path = proj_dir.join("main.rr");
    let source = r#"
import r "graphics"
import r { median as graphics } from "stats"
"#;
    fs::write(&entry_path, source).expect("failed to write main.rr");

    let result = compile(&entry_path.to_string_lossy(), source, OptLevel::O0);

    let err =
        result.expect_err("expected compile to fail on package-name namespace alias conflict");
    assert_eq!(err.module, "RR.SemanticError");
    assert!(matches!(err.code, RRCode::E1002));
    assert!(matches!(err.stage, Stage::Lower));
    assert!(
        err.message.contains(
            "R import local 'graphics' conflicts with namespace alias for package 'graphics'"
        ),
        "unexpected message: {}",
        err.message
    );
}
