use super::patterns::ident_re;
use rustc_hash::{FxHashMap, FxHashSet};

pub(crate) fn resolve_alias(name: &str, aliases: &FxHashMap<String, String>) -> String {
    let mut current = name;
    let mut seen: FxHashSet<&str> = FxHashSet::default();
    while let Some(next) = aliases.get(current) {
        if !seen.insert(current) {
            break;
        }
        current = next;
    }
    current.to_string()
}

pub(crate) fn is_peephole_temp(name: &str) -> bool {
    name.starts_with(".__rr_")
        || name.starts_with(".tachyon_")
        || name.starts_with("i_")
        || name.starts_with(".tmp")
}

pub(crate) fn alias_chain_contains(
    name: &str,
    needle: &str,
    aliases: &FxHashMap<String, String>,
) -> bool {
    let mut current = name;
    let mut seen: FxHashSet<&str> = FxHashSet::default();
    while let Some(next) = aliases.get(current) {
        if next == needle {
            return true;
        }
        if !seen.insert(current) {
            break;
        }
        current = next;
    }
    false
}

pub(crate) fn invalidate_aliases_for_write(lhs: &str, aliases: &mut FxHashMap<String, String>) {
    let doomed: Vec<String> = aliases
        .keys()
        .filter(|name| name.as_str() == lhs || alias_chain_contains(name, lhs, aliases))
        .cloned()
        .collect();
    for name in doomed {
        aliases.remove(&name);
    }
}

pub(crate) fn rewrite_known_aliases(expr: &str, aliases: &FxHashMap<String, String>) -> String {
    let Some(re) = ident_re() else {
        return expr.to_string();
    };
    let rewritten = re.replace_all(expr, |caps: &regex::Captures<'_>| {
        let ident = caps.get(0).map(|m| m.as_str()).unwrap_or("");
        aliases
            .get(ident)
            .map(|_| resolve_alias(ident, aliases))
            .unwrap_or_else(|| ident.to_string())
    });
    rewritten.to_string()
}

pub(crate) fn normalize_expr_with_aliases(
    expr: &str,
    aliases: &FxHashMap<String, String>,
) -> String {
    let mut out = rewrite_known_aliases(expr, aliases);
    let mut ordered_aliases: Vec<(&str, &str)> = aliases
        .iter()
        .map(|(alias, target)| (alias.as_str(), target.as_str()))
        .collect();
    ordered_aliases.sort_by(|(lhs_a, _), (lhs_b, _)| {
        lhs_b.len().cmp(&lhs_a.len()).then_with(|| lhs_a.cmp(lhs_b))
    });
    for (alias, target) in ordered_aliases {
        out = out.replace(alias, target);
    }
    out = out.replace(".arg_", "");
    out
}
