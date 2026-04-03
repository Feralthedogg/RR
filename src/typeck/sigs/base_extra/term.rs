mod data_misc;
mod env_io;

use crate::typeck::term::TypeTerm;

pub(crate) fn infer_base_extra_package_call_term(
    callee: &str,
    arg_terms: &[TypeTerm],
) -> Option<TypeTerm> {
    env_io::infer_base_extra_package_call_term_env_io(callee, arg_terms)
        .or_else(|| data_misc::infer_base_extra_package_call_term_data_misc(callee, arg_terms))
}
