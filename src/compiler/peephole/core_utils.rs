use super::{
    FxHashMap, assign_re, find_matching_block_end, is_control_flow_boundary, plain_ident_re,
};

#[derive(Debug, Clone)]
pub(super) struct IndexedFunction {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) body_start: usize,
    pub(super) return_idx: Option<usize>,
    pub(super) name: Option<String>,
    pub(super) params: Vec<String>,
}

pub(super) fn build_function_text_index(
    lines: &[String],
    parse_header: impl Fn(&str) -> Option<(String, Vec<String>)>,
) -> Vec<IndexedFunction> {
    let mut out = Vec::new();
    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_matching_block_end(lines, fn_start) else {
            break;
        };
        let (name, params) = parse_header(&lines[fn_start])
            .map(|(name, params)| (Some(name), params))
            .unwrap_or_else(|| (None, Vec::new()));
        out.push(IndexedFunction {
            start: fn_start,
            end: fn_end,
            body_start: fn_start + 1,
            return_idx: previous_non_empty_line(lines, fn_end),
            name,
            params,
        });
        fn_start = fn_end + 1;
    }
    out
}

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
