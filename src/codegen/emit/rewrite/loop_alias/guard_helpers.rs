use super::*;
pub(crate) fn rhs_is_simple_scalar_alias_or_literal_local(rhs: &str) -> bool {
    let rhs = strip_outer_parens_local(rhs);
    rhs.chars().all(RBackend::is_symbol_char)
        || rhs.trim_end_matches('L').parse::<f64>().is_ok()
        || matches!(rhs, "TRUE" | "FALSE" | "NA" | "NULL")
}

pub(crate) fn rhs_is_simple_dead_expr_local(rhs: &str) -> bool {
    let rhs = strip_outer_parens_local(rhs);
    !rhs.is_empty()
        && !rhs.contains("<-")
        && !rhs.contains("function(")
        && !rhs.contains("function (")
        && !rhs.contains("tryCatch(")
        && !rhs.contains("print(")
        && !rhs.contains("cat(")
        && !rhs.contains("message(")
        && !rhs.contains("warning(")
        && !rhs.contains("stop(")
        && !rhs.contains("quit(")
        && !rhs.contains('"')
        && !rhs.contains(',')
}

pub(crate) fn parse_repeat_guard_cmp_line_local(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim();
    let inner = if let Some(inner) = trimmed
        .strip_prefix("if (!rr_truthy1(")
        .and_then(|s| s.strip_suffix(")) break"))
    {
        let args = crate::compiler::pipeline::split_top_level_args(inner)?;
        args.first()?.trim().to_string()
    } else {
        trimmed
            .strip_prefix("if (!(")
            .or_else(|| trimmed.strip_prefix("if !("))?
            .strip_suffix(")) break")
            .or_else(|| {
                trimmed
                    .strip_prefix("if (!(")
                    .or_else(|| trimmed.strip_prefix("if !("))
                    .and_then(|s| s.strip_suffix(") break"))
            })?
            .trim()
            .to_string()
    };
    let inner = strip_outer_parens_local(&inner).trim();
    for op in ["<=", "<"] {
        let needle = format!(" {op} ");
        let Some((lhs, rhs)) = inner.split_once(&needle) else {
            continue;
        };
        let lhs = lhs.trim();
        let rhs = rhs.trim();
        if !lhs.is_empty() && !rhs.is_empty() {
            return Some((lhs.to_string(), op.to_string(), rhs.to_string()));
        }
    }
    None
}

pub(crate) fn plain_ident_local(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(RBackend::is_symbol_char)
}

pub(crate) fn raw_enclosing_repeat_guard_mentions_symbol_local(
    lines: &[String],
    idx: usize,
    symbol: &str,
) -> bool {
    for start_idx in (0..idx).rev() {
        if lines[start_idx].trim() != "repeat {" {
            continue;
        }
        let Some(end_idx) = find_block_end_local(lines, start_idx) else {
            continue;
        };
        if idx >= end_idx {
            continue;
        }
        let Some(guard_idx) = ((start_idx + 1)..end_idx)
            .find(|line_idx| parse_repeat_guard_cmp_line_local(lines[*line_idx].trim()).is_some())
        else {
            continue;
        };
        if count_symbol_occurrences_local(lines[guard_idx].trim(), symbol) > 0 {
            return true;
        }
        break;
    }
    false
}

pub(crate) fn raw_enclosing_repeat_body_reads_symbol_before_local(
    lines: &[String],
    idx: usize,
    symbol: &str,
) -> bool {
    for start_idx in (0..idx).rev() {
        if lines[start_idx].trim() != "repeat {" {
            continue;
        }
        let Some(end_idx) = find_block_end_local(lines, start_idx) else {
            continue;
        };
        if idx >= end_idx {
            continue;
        }
        let Some(guard_idx) = ((start_idx + 1)..end_idx)
            .find(|line_idx| parse_repeat_guard_cmp_line_local(lines[*line_idx].trim()).is_some())
        else {
            continue;
        };
        for line in lines.iter().take(idx).skip(guard_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((lhs, rhs)) = parse_local_assign_line(trimmed)
                && lhs == symbol
            {
                if count_symbol_occurrences_local(rhs, symbol) > 0 {
                    return true;
                }
                continue;
            }
            if count_symbol_occurrences_local(trimmed, symbol) > 0 {
                return true;
            }
        }
        break;
    }
    false
}
