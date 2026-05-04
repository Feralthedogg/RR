#[test]
fn fuzz_harness_uses_explicit_internal_feature() {
    let root_manifest = include_str!("../Cargo.toml");
    let fuzz_manifest = include_str!("../fuzz/Cargo.toml");

    assert!(
        root_manifest.contains("fuzz-internals = []"),
        "root manifest must declare the fuzz-only internal API feature"
    );
    assert!(
        fuzz_manifest.contains("features = [\"fuzz-internals\"]"),
        "fuzz harness must opt into compiler internals explicitly"
    );
}
