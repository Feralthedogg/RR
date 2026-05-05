use super::*;
pub(crate) fn assign_slice_re_local() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^rr_assign_slice\((?P<dest>{}),\s*(?P<start>.+?),\s*(?P<end>.+?),\s*(?P<rest>.+)\)$",
            IDENT_PATTERN
        ))
    })
    .as_ref()
}

pub(crate) fn plain_ident_re_local() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(format!(r"^{}$", IDENT_PATTERN)))
        .as_ref()
}

pub(crate) fn literal_one_re_local() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"^(?:1|1L|1l|1\.0)$".to_string()))
        .as_ref()
}

pub(crate) fn literal_positive_re_local() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(r"^(?:[1-9][0-9]*|[1-9][0-9]*L|[1-9][0-9]*l|[1-9][0-9]*\.0)$".to_string())
    })
    .as_ref()
}
