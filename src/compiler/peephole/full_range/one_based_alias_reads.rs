use super::*;
pub(crate) fn rewrite_one_based_full_range_index_alias_reads(lines: Vec<String>) -> Vec<String> {
    if !has_one_based_full_range_index_alias_read_candidates(&lines) {
        return lines;
    }
    let mut out = Vec::with_capacity(lines.len());
    let mut whole_range_index_aliases: FxHashMap<String, String> = FxHashMap::default();
    for line in lines {
        let trimmed = line.trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            if is_control_flow_boundary(&trimmed) || line.contains("<- function") {
                whole_range_index_aliases.clear();
            }
            out.push(line);
            continue;
        };
        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let mut rewritten_rhs = rhs.to_string();
        if rewritten_rhs.contains("rr_index1_read_vec") {
            rewritten_rhs = rewrite_index1_read_vec_calls(&rewritten_rhs, |base, idx_expr| {
                expr_is_one_based_full_range_alias(idx_expr).then(|| base.to_string())
            });
        }
        for (alias, alias_rhs) in &whole_range_index_aliases {
            if !expr_is_one_based_full_range_alias(alias_rhs) {
                continue;
            }
            let alias_compact = compact_expr(alias);
            rewritten_rhs = rewrite_index1_read_vec_calls(&rewritten_rhs, |base, idx_expr| {
                (compact_expr(idx_expr) == alias_compact).then(|| base.to_string())
            });
        }
        if rewritten_rhs != rhs {
            rewritten_rhs = rewritten_rhs.replace("rr_ifelse_strict(", "ifelse(");
        }
        if lhs.starts_with('.') && expr_is_one_based_full_range_alias(&rewritten_rhs) {
            whole_range_index_aliases.insert(lhs.to_string(), rewritten_rhs.clone());
        } else {
            whole_range_index_aliases.remove(lhs);
        }
        out.push(format!("{indent}{lhs} <- {rewritten_rhs}"));
    }
    out
}
