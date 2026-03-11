use std::fs;
use std::path::PathBuf;

#[test]
fn same_or_scalar_uses_r_recycling_warning_policy() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runtime_src =
        fs::read_to_string(root.join("src").join("runtime").join("runtime_prelude.R"))
            .expect("failed to read src/runtime/runtime_prelude.R");
    assert!(
        runtime_src.contains("longer object length is not a multiple of shorter object length"),
        "runtime must include R-compatible recycling warning"
    );
    assert!(
        !runtime_src.contains("RR only allows length-1 recycling"),
        "legacy strict recycling error text should be removed from rr_same_or_scalar"
    );
}
