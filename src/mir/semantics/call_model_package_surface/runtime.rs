pub(crate) fn is_runtime_helper(name: &str) -> bool {
    name.starts_with("rr_")
}

pub(crate) fn is_runtime_reserved_symbol(name: &str) -> bool {
    name.starts_with(".phi_")
        || name.starts_with(".tachyon_")
        || name.starts_with("Sym_")
        || name.starts_with("__lambda_")
        || name.starts_with("rr_")
}
