use super::{FxHashMap, IDENT_PATTERN, compile_regex, is_control_flow_boundary, plain_ident_re};
use regex::Regex;
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

pub(super) fn rewrite_shifted_square_scalar_reuse(lines: Vec<String>) -> Vec<String> {
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
