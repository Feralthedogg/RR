use crate::hir::def::HirTypeRef;

pub(super) fn trait_method_mangle(trait_name: &str, for_ty: &HirTypeRef, method: &str) -> String {
    format!(
        "__rr_trait_{}_{}_{}",
        sanitize_trait_symbol_segment(trait_name),
        sanitize_trait_symbol_segment(&for_ty.key()),
        sanitize_trait_symbol_segment(method)
    )
}

pub(super) fn trait_const_mangle(trait_name: &str, for_ty: &HirTypeRef, name: &str) -> String {
    format!(
        "__rr_trait_const_{}_{}_{}",
        sanitize_trait_symbol_segment(trait_name),
        sanitize_trait_symbol_segment(&for_ty.key()),
        sanitize_trait_symbol_segment(name)
    )
}

fn sanitize_trait_symbol_segment(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out
}
