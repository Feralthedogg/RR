use super::*;
pub(crate) fn is_raw_alloc_like_expr_local(expr: &str) -> bool {
    [
        "rep.int(",
        "numeric(",
        "integer(",
        "logical(",
        "character(",
        "vector(",
        "matrix(",
        "Sym_17(",
    ]
    .iter()
    .any(|prefix| expr.starts_with(prefix))
}

pub(crate) fn is_raw_branch_rebind_candidate_local(expr: &str) -> bool {
    is_raw_alloc_like_expr_local(expr)
        || expr.chars().all(RBackend::is_symbol_char)
        || expr.trim_end_matches('L').parse::<f64>().is_ok()
        || matches!(expr, "TRUE" | "FALSE" | "NA" | "NULL")
}

pub(crate) fn raw_vec_fill_signature_local(expr: &str) -> Option<(String, String)> {
    let expr = strip_outer_parens_local(expr);
    if let Some(inner) = expr
        .strip_prefix("rep.int(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let args = split_top_level_args_local(inner)?;
        if args.len() == 2 {
            return Some((args[1].trim().to_string(), args[0].trim().to_string()));
        }
    }
    if let Some(inner) = expr
        .strip_prefix("Sym_17(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let args = split_top_level_args_local(inner)?;
        if args.len() == 2 {
            return Some((args[0].trim().to_string(), args[1].trim().to_string()));
        }
    }
    None
}

pub(crate) fn raw_branch_rebind_exprs_equivalent_local(prev_rhs: &str, rhs: &str) -> bool {
    let prev_rhs = strip_outer_parens_local(prev_rhs);
    let rhs = strip_outer_parens_local(rhs);
    if prev_rhs == rhs {
        return true;
    }
    raw_vec_fill_signature_local(prev_rhs)
        .zip(raw_vec_fill_signature_local(rhs))
        .is_some_and(|(lhs_sig, rhs_sig)| lhs_sig == rhs_sig)
}

pub(crate) fn is_inlineable_named_scalar_expr_local(rhs: &str) -> bool {
    let rhs = strip_outer_parens_local(rhs);
    if rhs.is_empty()
        || rhs.contains('"')
        || rhs.contains(',')
        || rhs.contains("function(")
        || rhs.contains("function (")
    {
        return false;
    }
    true
}
