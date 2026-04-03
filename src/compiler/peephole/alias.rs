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
    let mut out = String::with_capacity(expr.len());
    let bytes = expr.as_bytes();
    let mut idx = 0usize;
    let mut in_single = false;
    let mut in_double = false;

    while idx < bytes.len() {
        match bytes[idx] {
            b'\'' if !in_double => {
                in_single = !in_single;
                out.push('\'');
                idx += 1;
                continue;
            }
            b'"' if !in_single => {
                in_double = !in_double;
                out.push('"');
                idx += 1;
                continue;
            }
            _ => {}
        }

        if !in_single && !in_double && ident_is_start(expr, idx) {
            let end = ident_end(expr, idx);
            let ident = &expr[idx..end];
            if !ident_is_named_label(expr, end)
                && let Some(_) = aliases.get(ident)
            {
                out.push_str(&resolve_alias(ident, aliases));
            } else {
                out.push_str(ident);
            }
            idx = end;
            continue;
        }

        out.push(bytes[idx] as char);
        idx += 1;
    }

    out
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

fn ident_is_start(expr: &str, idx: usize) -> bool {
    let rest = &expr[idx..];
    let mut chars = rest.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first.is_ascii_alphabetic() || first == '_' {
        return true;
    }
    first == '.'
        && chars
            .next()
            .is_some_and(|next| next.is_ascii_alphabetic() || next == '_')
}

fn ident_end(expr: &str, start: usize) -> usize {
    let mut end = start;
    for (off, ch) in expr[start..].char_indices() {
        if !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')) {
            break;
        }
        end = start + off + ch.len_utf8();
    }
    end
}

fn ident_is_named_label(expr: &str, end: usize) -> bool {
    let rest = &expr[end..];
    for (off, ch) in rest.char_indices() {
        if ch.is_ascii_whitespace() {
            continue;
        }
        if ch != '=' {
            return false;
        }
        let tail = &rest[off + ch.len_utf8()..];
        let next_non_ws = tail.chars().find(|ch| !ch.is_ascii_whitespace());
        return next_non_ws != Some('=');
    }
    false
}
