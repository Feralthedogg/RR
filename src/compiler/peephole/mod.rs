pub(crate) mod alias;
pub(crate) mod patterns;
pub(crate) mod vector;

use rustc_hash::FxHashSet;

pub(crate) fn optimize_emitted_r(code: &str, direct_builtin_call_map: bool) -> String {
    super::r_peephole::optimize_emitted_r(code, direct_builtin_call_map)
}

pub(crate) fn optimize_emitted_r_with_line_map(
    code: &str,
    direct_builtin_call_map: bool,
) -> (String, Vec<u32>) {
    super::r_peephole::optimize_emitted_r_with_line_map(code, direct_builtin_call_map)
}

pub(crate) fn optimize_emitted_r_with_context(
    code: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
) -> (String, Vec<u32>) {
    super::r_peephole::optimize_emitted_r_with_context(
        code,
        direct_builtin_call_map,
        pure_user_calls,
    )
}

pub(crate) fn optimize_emitted_r_with_context_and_fresh(
    code: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
) -> (String, Vec<u32>) {
    super::r_peephole::optimize_emitted_r_with_context_and_fresh(
        code,
        direct_builtin_call_map,
        pure_user_calls,
        fresh_user_calls,
    )
}

pub(crate) fn optimize_emitted_r_with_context_and_fresh_with_options(
    code: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    preserve_all_defs: bool,
) -> (String, Vec<u32>) {
    super::r_peephole::optimize_emitted_r_with_context_and_fresh_with_options(
        code,
        direct_builtin_call_map,
        pure_user_calls,
        fresh_user_calls,
        preserve_all_defs,
    )
}

pub(crate) fn rewrite_selected_simple_expr_helper_calls_in_text(
    code: &str,
    helper_names: &[&str],
) -> String {
    super::r_peephole::rewrite_selected_simple_expr_helper_calls_in_text(code, helper_names)
}

pub(crate) fn simplify_nested_index_vec_floor_calls_in_text(code: &str) -> String {
    super::r_peephole::simplify_nested_index_vec_floor_calls_in_text(code)
}
