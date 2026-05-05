#[test]
fn same_or_scalar_uses_r_recycling_warning_policy() {
    let runtime_src = rr::runtime::R_RUNTIME;
    assert!(
        runtime_src.contains("longer object length is not a multiple of shorter object length"),
        "runtime must include R-compatible recycling warning"
    );
    assert!(
        !runtime_src.contains("RR only allows length-1 recycling"),
        "legacy strict recycling error text should be removed from rr_same_or_scalar"
    );
}
