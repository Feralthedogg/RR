use super::{FxHashMap, assign_re, is_control_flow_boundary, plain_ident_re};

pub(super) fn collect_prologue_arg_aliases(
    lines: &[String],
    idx: usize,
) -> FxHashMap<String, String> {
    let mut fn_start = 0usize;
    for prev in (0..=idx).rev() {
        if lines[prev].contains("<- function") {
            fn_start = prev + 1;
            break;
        }
    }
    let mut aliases = FxHashMap::default();
    for line in lines.iter().take(idx).skip(fn_start) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if is_control_flow_boundary(trimmed) {
            break;
        }
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if lhs.starts_with(".arg_") && plain_ident_re().is_some_and(|re| re.is_match(rhs)) {
            aliases.insert(lhs.to_string(), rhs.to_string());
        }
    }
    aliases
}

pub(super) fn previous_non_empty_line(lines: &[String], idx: usize) -> Option<usize> {
    (0..idx).rev().find(|i| !lines[*i].trim().is_empty())
}
