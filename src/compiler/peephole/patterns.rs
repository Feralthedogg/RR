use regex::Regex;
use std::sync::OnceLock;

pub(crate) const IDENT_PATTERN: &str = r"(?:[A-Za-z_][A-Za-z0-9._]*|\.[A-Za-z_][A-Za-z0-9._]*)";

pub(crate) fn compile_regex(pattern: String) -> Option<Regex> {
    Regex::new(&pattern).ok()
}

pub(crate) fn assign_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^(?P<indent>\s*)(?P<lhs>{}) <- (?P<rhs>.+)$",
            IDENT_PATTERN
        ))
    })
    .as_ref()
}

pub(crate) fn indexed_store_base_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^(?P<indent>\s*)(?P<base>{})\s*\[[^\]]+\]\s*<-\s*.+$",
            IDENT_PATTERN
        ))
    })
    .as_ref()
}

pub(crate) fn range_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^(?P<start>{}|1L?|1(?:\.0+)?)\:(?P<end>{}|\d+(?:\.\d+)?)$",
            IDENT_PATTERN, IDENT_PATTERN
        ))
    })
    .as_ref()
}

pub(crate) fn floor_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"^rr_index_vec_floor\((?P<src>[^\)]*)\)$".to_string()))
        .as_ref()
}

pub(crate) fn nested_index_vec_floor_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"rr_index_vec_floor\(rr_index_vec_floor\((?P<inner>{})\)\)",
            IDENT_PATTERN
        ))
    })
    .as_ref()
}

pub(crate) fn seq_len_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^seq_len\((?P<len>{}|\d+(?:\.\d+)?)\)$",
            IDENT_PATTERN
        ))
    })
    .as_ref()
}

pub(crate) fn rep_int_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^rep\.int\([^,]+,\s*(?P<len>{}|\d+(?:\.\d+)?)\)$",
            IDENT_PATTERN
        ))
    })
    .as_ref()
}

pub(crate) fn length_call_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(format!(r"length\((?P<var>{})\)", IDENT_PATTERN)))
        .as_ref()
}

pub(crate) fn scalar_lit_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"^\d+(?:\.\d+)?L?$".to_string()))
        .as_ref()
}

pub(crate) fn plain_ident_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(format!(r"^{}$", IDENT_PATTERN)))
        .as_ref()
}

pub(crate) fn ident_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(IDENT_PATTERN.to_string()))
        .as_ref()
}

pub(crate) fn call_map_slice_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(
            r"^rr_call_map_slice_auto\((?P<dest>[^,]+),\s*(?P<start>[^,]+),\s*(?P<end>[^,]+),\s*(?P<rest>.+)\)$"
                .to_string(),
        )
    })
    .as_ref()
}

pub(crate) fn assign_slice_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(
            r"^rr_assign_slice\((?P<dest>[^,]+),\s*(?P<start>[^,]+),\s*(?P<end>[^,]+),\s*(?P<rest>.+)\)$"
                .to_string(),
        )
    })
    .as_ref()
}

pub(crate) fn call_map_whole_builtin_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(
            r#"^rr_call_map_whole_auto\((?P<dest>[^,]+),\s*"(?P<callee>abs|sqrt|log|pmax|pmin)",\s*[^,]+,\s*c\((?P<slots>[^\)]*)\),\s*(?P<args>.+)\)$"#.to_string(),
        )
    })
    .as_ref()
}

pub(crate) fn split_top_level_args(expr: &str) -> Option<Vec<String>> {
    let mut args = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (idx, ch) in expr.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                args.push(expr[start..idx].trim().to_string());
                start = idx + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    args.push(expr[start..].trim().to_string());
    Some(args)
}

pub(crate) fn expr_idents(expr: &str) -> Vec<String> {
    let Some(re) = ident_re() else {
        return Vec::new();
    };
    re.find_iter(expr).map(|m| m.as_str().to_string()).collect()
}

pub(crate) fn cse_temp_index_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"\.__rr_cse_(?P<idx>\d+)\b".to_string()))
        .as_ref()
}

pub(crate) fn next_generated_cse_index(lines: &[String]) -> usize {
    lines
        .iter()
        .flat_map(|line| {
            cse_temp_index_re()
                .into_iter()
                .flat_map(|re| re.captures_iter(line))
                .filter_map(|caps| {
                    caps.name("idx")
                        .and_then(|m| m.as_str().parse::<usize>().ok())
                })
        })
        .max()
        .map_or(0, |idx| idx + 1)
}
