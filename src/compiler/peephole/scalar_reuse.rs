use super::{
    FxHashMap, IDENT_PATTERN, assign_re, compile_regex,
    has_one_based_full_range_index_alias_read_candidates, is_control_flow_boundary, plain_ident_re,
};
use regex::{Captures, Regex};
use std::sync::OnceLock;

fn shifted_minus_one_temp_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^\(\((?P<a>{}) \+ (?P<b>{})\) - 1\)$",
            IDENT_PATTERN, IDENT_PATTERN
        ))
    })
    .as_ref()
}

fn shifted_minus_one_temp_name(rhs: &str) -> Option<String> {
    let caps = shifted_minus_one_temp_re().and_then(|re| re.captures(rhs.trim()))?;
    let a = caps.name("a").map(|m| m.as_str()).unwrap_or("").trim();
    let b = caps.name("b").map(|m| m.as_str()).unwrap_or("").trim();
    (a == b).then(|| a.to_string())
}

fn doubled_temp_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^\((?P<a>{}) \+ (?P<b>{})\)$",
            IDENT_PATTERN, IDENT_PATTERN
        ))
    })
    .as_ref()
}

fn doubled_temp_name(rhs: &str) -> Option<String> {
    let caps = doubled_temp_re().and_then(|re| re.captures(rhs.trim()))?;
    let a = caps.name("a").map(|m| m.as_str()).unwrap_or("").trim();
    let b = caps.name("b").map(|m| m.as_str()).unwrap_or("").trim();
    (a == b).then(|| a.to_string())
}

fn compact_expr_local(expr: &str) -> String {
    expr.chars().filter(|c| !c.is_whitespace()).collect()
}

fn expr_is_one_based_full_range_alias_local(expr: &str) -> bool {
    let expr = compact_expr_local(expr);
    let starts = ["1L", "1", "1.0", "1.0L"];
    starts.iter().any(|start_expr| {
        expr.starts_with(&format!("{}:", start_expr))
            || expr.starts_with(&format!("rr_index_vec_floor({}:", start_expr))
    })
}

fn minus_one_named_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"^\((?P<inner>.+)\s-\s1L?\)$".to_string()))
        .as_ref()
}

fn collect_temp_minus_one_pairs(lines: &[String]) -> Vec<(String, String)> {
    let Some(assign_re) = assign_re() else {
        return Vec::new();
    };
    let mut named_minus_one = FxHashMap::<String, String>::default();
    let mut temp_inner = FxHashMap::<String, String>::default();

    for line in lines {
        let trimmed = line.trim();
        let Some(caps) = assign_re.captures(trimmed) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if let Some(inner) = minus_one_named_re()
            .and_then(|re| re.captures(rhs))
            .and_then(|caps| caps.name("inner").map(|m| m.as_str()))
            && plain_ident_re().is_some_and(|re| re.is_match(lhs))
            && !lhs.starts_with('.')
        {
            named_minus_one.insert(inner.to_string(), lhs.to_string());
        } else if lhs.starts_with(".__rr_cse_") {
            temp_inner.insert(lhs.to_string(), rhs.to_string());
        }
    }

    temp_inner
        .into_iter()
        .filter_map(|(temp, inner)| {
            named_minus_one
                .get(&inner)
                .cloned()
                .map(|name| (temp, name))
        })
        .collect()
}

fn apply_temp_minus_one_scaled_to_named_scalar_text(
    text: &str,
    temp_minus_one_pairs: &[(String, String)],
) -> String {
    let mut rewritten = text.to_string();
    for (temp, name) in temp_minus_one_pairs {
        let pattern = format!(
            r"\(\(\s*{}\s*-\s*1\s*\)\s*\*\s*([^\)]+)\)",
            regex::escape(temp)
        );
        if let Some(re) = compile_regex(pattern) {
            let replacement = format!("({name} * $1)");
            rewritten = re.replace_all(&rewritten, replacement.as_str()).to_string();
        }
    }
    rewritten
}

pub(super) fn run_secondary_exact_local_scalar_bundle(lines: Vec<String>) -> Vec<String> {
    let has_one_based_index_alias_reads =
        has_one_based_full_range_index_alias_read_candidates(&lines);
    if !lines.iter().any(|line| line.contains(".__rr_cse_"))
        || !lines.iter().any(|line| line.contains("- 1"))
    {
        if !has_one_based_index_alias_reads {
            return lines;
        }
    }

    let temp_minus_one_pairs = collect_temp_minus_one_pairs(&lines);
    let needs_shifted_square = lines.iter().any(|line| line.contains("* ("));
    let needs_one_based_index_alias_reads = has_one_based_index_alias_reads;
    if temp_minus_one_pairs.is_empty()
        && !needs_shifted_square
        && !needs_one_based_index_alias_reads
    {
        return lines;
    }

    let Some(assign_re) = assign_re() else {
        return lines;
    };
    let mut out = lines;
    let mut base_to_named = FxHashMap::<String, String>::default();
    let mut doubled_to_named = FxHashMap::<String, String>::default();
    let mut whole_range_index_aliases = FxHashMap::<String, String>::default();
    let read_vec_re = if needs_one_based_index_alias_reads {
        compile_regex(format!(
            r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*(?P<idx>{}|rr_index_vec_floor\([^\)]*\)|[^,\)]*:[^\)]*)\)",
            IDENT_PATTERN, IDENT_PATTERN
        ))
    } else {
        None
    };

    for idx in 0..out.len() {
        let original_line = out[idx].clone();
        let trimmed = original_line.trim().to_string();
        if trimmed.contains("<- function") {
            base_to_named.clear();
            doubled_to_named.clear();
            whole_range_index_aliases.clear();
        } else if is_control_flow_boundary(&trimmed)
            && trimmed != "}"
            && !trimmed.starts_with("if ")
            && !trimmed.starts_with("else")
            && !trimmed.starts_with("} else")
        {
            base_to_named.clear();
            doubled_to_named.clear();
            whole_range_index_aliases.clear();
        }

        let mut rewritten_line =
            if !temp_minus_one_pairs.is_empty() && original_line.contains("- 1") {
                apply_temp_minus_one_scaled_to_named_scalar_text(
                    original_line.as_str(),
                    &temp_minus_one_pairs,
                )
            } else {
                original_line.clone()
            };

        let Some(caps) = assign_re.captures(&trimmed) else {
            if rewritten_line != original_line {
                out[idx] = rewritten_line;
            }
            continue;
        };

        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let mut rewritten_rhs = rhs.to_string();

        if needs_one_based_index_alias_reads
            && rewritten_rhs.contains("rr_index1_read_vec")
            && let Some(re) = read_vec_re.as_ref()
        {
            rewritten_rhs = re
                .replace_all(&rewritten_rhs, |caps: &Captures<'_>| {
                    let idx_expr = caps.name("idx").map(|m| m.as_str()).unwrap_or("").trim();
                    if expr_is_one_based_full_range_alias_local(idx_expr) {
                        caps.name("base")
                            .map(|m| m.as_str())
                            .unwrap_or("")
                            .to_string()
                    } else {
                        caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                    }
                })
                .to_string();
            for (alias, alias_rhs) in &whole_range_index_aliases {
                if !expr_is_one_based_full_range_alias_local(alias_rhs) {
                    continue;
                }
                let pattern = format!(
                    r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*{}\s*\)",
                    IDENT_PATTERN,
                    regex::escape(alias),
                );
                let Some(alias_re) = compile_regex(pattern) else {
                    continue;
                };
                rewritten_rhs = alias_re
                    .replace_all(&rewritten_rhs, |caps: &Captures<'_>| {
                        caps.name("base")
                            .map(|m| m.as_str())
                            .unwrap_or("")
                            .to_string()
                    })
                    .to_string();
            }
            if rewritten_rhs != rhs {
                rewritten_rhs = rewritten_rhs.replace("rr_ifelse_strict(", "ifelse(");
                let indent_len = original_line.len() - original_line.trim_start().len();
                let indent = &original_line[..indent_len];
                rewritten_line = format!("{indent}{lhs} <- {rewritten_rhs}");
            }
        }

        if plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
            if let Some(temp) = shifted_minus_one_temp_name(rhs) {
                base_to_named.insert(temp.to_string(), lhs.to_string());
            } else if let Some(temp) = doubled_temp_name(rhs)
                && let Some(named) = base_to_named.get(&temp).cloned()
            {
                doubled_to_named.insert(lhs.to_string(), named);
            }
        }

        if needs_shifted_square && rhs.contains("* (") && rewritten_line.contains("- 1") {
            let indent_len = original_line.len() - original_line.trim_start().len();
            let indent = &original_line[..indent_len];
            let mut shifted_rhs = if rewritten_rhs != rhs {
                rewritten_rhs.clone()
            } else if !temp_minus_one_pairs.is_empty() {
                apply_temp_minus_one_scaled_to_named_scalar_text(rhs, &temp_minus_one_pairs)
            } else {
                rhs.to_string()
            };
            for (temp, named) in &doubled_to_named {
                let needle = format!("(({temp} - 1) * ({temp} - 1))");
                let replacement = format!("({named} * {named})");
                shifted_rhs = shifted_rhs.replace(&needle, &replacement);
            }
            if shifted_rhs != rhs {
                rewritten_line = format!("{indent}{lhs} <- {shifted_rhs}");
                rewritten_rhs = shifted_rhs;
            }
        }

        if lhs.starts_with('.') && expr_is_one_based_full_range_alias_local(&rewritten_rhs) {
            whole_range_index_aliases.insert(lhs.to_string(), rewritten_rhs.clone());
        } else {
            whole_range_index_aliases.remove(lhs);
        }

        if rewritten_line != original_line {
            out[idx] = rewritten_line;
        }
    }

    out
}

pub(super) fn rewrite_shifted_square_scalar_reuse(lines: Vec<String>) -> Vec<String> {
    if !lines.iter().any(|line| line.contains(".__rr_cse_"))
        || !lines.iter().any(|line| line.contains("- 1"))
        || !lines.iter().any(|line| line.contains("* ("))
    {
        return lines;
    }
    let mut out = lines;
    let mut base_to_named = FxHashMap::<String, String>::default();
    let mut doubled_to_named = FxHashMap::<String, String>::default();
    let mut idx = 0usize;
    while idx < out.len() {
        let trimmed = out[idx].trim().to_string();
        if trimmed.contains("<- function") {
            base_to_named.clear();
            doubled_to_named.clear();
            idx += 1;
            continue;
        }
        if is_control_flow_boundary(&trimmed)
            && trimmed != "}"
            && !trimmed.starts_with("if ")
            && !trimmed.starts_with("else")
            && !trimmed.starts_with("} else")
        {
            base_to_named.clear();
            doubled_to_named.clear();
        }
        let Some(caps) = super::assign_re().and_then(|re| re.captures(&trimmed)) else {
            idx += 1;
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
            if let Some(temp) = shifted_minus_one_temp_name(rhs) {
                base_to_named.insert(temp.to_string(), lhs.to_string());
            } else if let Some(temp) = doubled_temp_name(rhs)
                && let Some(named) = base_to_named.get(&temp).cloned()
            {
                doubled_to_named.insert(lhs.to_string(), named);
            }
        }
        if !rhs.contains("* (") || !rhs.contains("- 1") {
            idx += 1;
            continue;
        }
        let mut rewritten_rhs = rhs.to_string();
        for (temp, named) in &doubled_to_named {
            let needle = format!("(({temp} - 1) * ({temp} - 1))");
            let replacement = format!("({named} * {named})");
            rewritten_rhs = rewritten_rhs.replace(&needle, &replacement);
        }
        if rewritten_rhs != rhs {
            let indent_len = out[idx].len() - out[idx].trim_start().len();
            let indent = &out[idx][..indent_len];
            out[idx] = format!("{indent}{lhs} <- {rewritten_rhs}");
        }
        idx += 1;
    }
    out
}
