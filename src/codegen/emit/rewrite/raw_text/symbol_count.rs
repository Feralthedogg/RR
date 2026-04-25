fn count_symbol_occurrences_local(line: &str, symbol: &str) -> usize {
    if line.is_empty() || symbol.is_empty() || !line.contains(symbol) {
        return 0;
    }
    let bytes = line.as_bytes();
    let mut idx = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut count = 0usize;
    while idx < bytes.len() {
        match bytes[idx] {
            b'\'' if !in_double => {
                in_single = !in_single;
                idx += 1;
                continue;
            }
            b'"' if !in_single => {
                in_double = !in_double;
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
            count += 1;
            idx += symbol.len();
            continue;
        }
        let Some(ch) = line[idx..].chars().next() else {
            break;
        };
        idx += ch.len_utf8();
    }
    count
}
