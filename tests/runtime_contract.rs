use RR::runtime::R_RUNTIME;

#[test]
fn rr_bool_requires_logical_scalar() {
    assert!(
        R_RUNTIME.contains("if (!is.logical(x)) rr_type_error"),
        "rr_bool should reject non-logical conditions"
    );
}

#[test]
fn strict_index_read_path_exists() {
    assert!(
        R_RUNTIME.contains("rr_index1_read_strict <- function"),
        "strict index-read helper should exist"
    );
    assert!(
        R_RUNTIME.contains(".rr_env$strict_index_read <-"),
        "strict index-read runtime switch should exist"
    );
    assert!(
        R_RUNTIME.contains("if (.rr_env$strict_index_read)"),
        "rr_index1_read should route to strict helper when enabled"
    );
}

#[test]
fn runtime_mode_fast_path_switches_exist() {
    assert!(
        R_RUNTIME.contains(".rr_env$runtime_mode <-"),
        "runtime mode switch should exist"
    );
    assert!(
        R_RUNTIME.contains(".rr_env$fast_runtime <-"),
        "fast-runtime switch should exist"
    );
    assert!(
        R_RUNTIME.contains(".rr_env$enable_marks <-"),
        "mark toggle switch should exist"
    );
    assert!(
        R_RUNTIME.contains("if (!.rr_env$enable_marks) return(invisible(NULL))"),
        "rr_mark should support fast no-op mode"
    );
}
