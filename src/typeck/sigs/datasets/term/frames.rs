mod primary;
mod secondary;

use crate::typeck::term::TypeTerm;

pub(crate) fn infer_datasets_frame_binding_term(var: &str) -> Option<TypeTerm> {
    primary::infer_datasets_frame_binding_term_primary(var)
        .or_else(|| secondary::infer_datasets_frame_binding_term_secondary(var))
}
