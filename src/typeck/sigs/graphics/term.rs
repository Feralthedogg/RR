#[path = "term/ggplot2.rs"]
mod ggplot2;
#[path = "term/graphics_pkg.rs"]
mod graphics_pkg;
#[path = "term/grdevices.rs"]
mod grdevices;

use crate::typeck::term::TypeTerm;

pub(crate) fn infer_graphics_package_call_term(
    callee: &str,
    arg_terms: &[TypeTerm],
) -> Option<TypeTerm> {
    graphics_pkg::infer_graphics_pkg_term(callee, arg_terms)
        .or_else(|| grdevices::infer_grdevices_term(callee, arg_terms))
        .or_else(|| ggplot2::infer_ggplot2_term(callee, arg_terms))
}
