mod frames;
mod shaped;
mod structured;

use crate::typeck::term::TypeTerm;

pub(crate) fn infer_datasets_package_binding_term(var: &str) -> Option<TypeTerm> {
    frames::infer_datasets_frame_binding_term(var)
        .or_else(|| shaped::infer_datasets_shaped_binding_term(var))
        .or_else(|| structured::infer_datasets_structured_binding_term(var))
}
