use super::{compile_regex, expr_is_logical_comparison, split_top_level_args};
use regex::{Captures, Regex};
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::OnceLock;

fn same_var_is_na_or_not_finite_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(
            r"is\.na\((?P<lhs>[A-Za-z_][A-Za-z0-9_]*)\)\s*\|\s*\(!\(is\.finite\((?P<rhs>[A-Za-z_][A-Za-z0-9_]*)\)\)\)"
                .to_string(),
        )
    })
    .as_ref()
}

fn wrapped_not_finite_cond_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(r"\(\((?P<inner>!\(is\.finite\([A-Za-z_][A-Za-z0-9_]*\)\))\)\)".to_string())
    })
    .as_ref()
}

fn not_finite_or_zero_guard_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(
            r"\(\(\((?P<inner>!\(is\.finite\((?P<lhs>[A-Za-z_][A-Za-z0-9_]*)\)\))\)\s*\|\s*\((?P<rhs>[A-Za-z_][A-Za-z0-9_]*) == 0\)\)\)".to_string(),
        )
    })
    .as_ref()
}

pub(in super::super) fn simplify_same_var_is_na_or_not_finite_guards(
    lines: Vec<String>,
) -> Vec<String> {
    let Some(re) = same_var_is_na_or_not_finite_re() else {
        return lines;
    };
    lines
        .into_iter()
        .map(|line| {
            re.replace_all(&line, |caps: &Captures<'_>| {
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("");
                let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("");
                if lhs == rhs {
                    format!("!(is.finite({lhs}))")
                } else {
                    caps.get(0)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default()
                }
            })
            .into_owned()
        })
        .collect()
}

pub(in super::super) fn simplify_wrapped_not_finite_parens(lines: Vec<String>) -> Vec<String> {
    let Some(re) = wrapped_not_finite_cond_re() else {
        return lines;
    };
    lines
        .into_iter()
        .map(|line| {
            re.replace_all(&line, |caps: &Captures<'_>| {
                let inner = caps.name("inner").map(|m| m.as_str()).unwrap_or("");
                format!("({inner})")
            })
            .into_owned()
        })
        .collect()
}

pub(in super::super) fn simplify_not_finite_or_zero_guard_parens(
    lines: Vec<String>,
) -> Vec<String> {
    let Some(re) = not_finite_or_zero_guard_re() else {
        return lines;
    };
    lines
        .into_iter()
        .map(|line| {
            re.replace_all(&line, |caps: &Captures<'_>| {
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("");
                let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("");
                let inner = caps.name("inner").map(|m| m.as_str()).unwrap_or("");
                if lhs == rhs {
                    format!("(({inner} | ({rhs} == 0)))")
                } else {
                    caps.get(0)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default()
                }
            })
            .into_owned()
        })
        .collect()
}

pub(in super::super) fn rewrite_guard_truthy_line(
    line: &str,
    no_na_vars: &FxHashSet<String>,
    scalar_consts: &FxHashMap<String, String>,
) -> String {
    let trimmed = line.trim();
    let Some(inner) = trimmed
        .strip_prefix("if (!rr_truthy1(")
        .and_then(|s| s.strip_suffix(")) break"))
    else {
        return line.to_string();
    };
    let Some(parts) = split_top_level_args(inner) else {
        return line.to_string();
    };
    if parts.len() < 2 || !expr_is_logical_comparison(&parts[0], no_na_vars, scalar_consts) {
        return line.to_string();
    }
    let indent_len = line.len() - line.trim_start().len();
    let indent = &line[..indent_len];
    let cond = parts[0].trim();
    let cond = cond
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(cond);
    format!("{indent}if (!({cond})) break")
}

pub(in super::super) fn rewrite_if_truthy_line(
    line: &str,
    no_na_vars: &FxHashSet<String>,
    scalar_consts: &FxHashMap<String, String>,
) -> String {
    let trimmed = line.trim();
    let indent_len = line.len() - line.trim_start().len();
    let indent = &line[..indent_len];

    if let Some(inner) = trimmed
        .strip_prefix("if (rr_truthy1(")
        .and_then(|s| s.strip_suffix(")) {"))
        && let Some(parts) = split_top_level_args(inner)
        && parts.len() >= 2
        && expr_is_logical_comparison(&parts[0], no_na_vars, scalar_consts)
    {
        let cond = parts[0].trim();
        let cond = cond
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(cond);
        return format!("{indent}if (({cond})) {{");
    }

    if let Some(inner) = trimmed
        .strip_prefix("if (!rr_truthy1(")
        .and_then(|s| s.strip_suffix(")) {"))
        && let Some(parts) = split_top_level_args(inner)
        && parts.len() >= 2
        && expr_is_logical_comparison(&parts[0], no_na_vars, scalar_consts)
    {
        let cond = parts[0].trim();
        let cond = cond
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(cond);
        return format!("{indent}if (!({cond})) {{");
    }

    line.to_string()
}
