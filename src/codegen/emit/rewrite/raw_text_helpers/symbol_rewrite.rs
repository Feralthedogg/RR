fn replace_symbol_occurrences_local(line: &str, symbol: &str, replacement: &str) -> String {
    if line.is_empty() || symbol.is_empty() || !line.contains(symbol) {
        return line.to_string();
    }
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
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
        if !in_single
            && !in_double
            && line[idx..].starts_with(symbol)
            && line[..idx]
                .chars()
                .next_back()
                .is_none_or(|ch| !RBackend::is_symbol_char(ch))
            && line[idx + symbol.len()..]
                .chars()
                .next()
                .is_none_or(|ch| !RBackend::is_symbol_char(ch))
        {
            out.push_str(replacement);
            idx += symbol.len();
            continue;
        }
        let Some(ch) = line[idx..].chars().next() else {
            break;
        };
        out.push(ch);
        idx += ch.len_utf8();
    }
    out
}

fn unquoted_sym_refs_local(line: &str) -> Vec<String> {
    raw_expr_idents_local(line)
        .into_iter()
        .filter(|ident| ident.starts_with("Sym_"))
        .collect()
}
