mod common;

use common::unique_dir;
use rr::compiler::{OptLevel, compile};
use std::fs;
use std::path::PathBuf;

#[test]
fn equivalent_import_paths_are_loaded_once() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root.join("target").join("tests").join("import_path_dedup");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");

    let imported_path = proj_dir.join("module.rr");
    let imported_src = r#"
print("DEDUPE_SENTINEL_8J2K")

"#;
    fs::write(&imported_path, imported_src).expect("failed to write module.rr");

    let main_path = proj_dir.join("main.rr");
    let main_src = r#"
import "./module.rr"

import "module.rr"

"#;
    fs::write(&main_path, main_src).expect("failed to write main.rr");

    let (generated, _map) =
        compile(&main_path.to_string_lossy(), main_src, OptLevel::O0).expect("compile failed");

    let count = generated.matches("DEDUPE_SENTINEL_8J2K").count();
    assert_eq!(
        count, 1,
        "imported module appears to be loaded more than once; count={}",
        count
    );
}
