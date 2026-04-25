use super::*;

pub(crate) fn prune_unreachable_raw_helper_definitions(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    loop {
        let mut changed = false;
        let mut fn_start = 0usize;
        while fn_start < lines.len() {
            while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
                fn_start += 1;
            }
            if fn_start >= lines.len() {
                break;
            }
            let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
                break;
            };
            let Some((name, _params)) = parse_raw_function_header(&lines[fn_start]) else {
                fn_start = fn_end + 1;
                continue;
            };
            if !name.starts_with("Sym_") || name.starts_with("Sym_top_") {
                fn_start = fn_end + 1;
                continue;
            }

            let mut reachable = false;
            for (line_idx, line) in lines.iter().enumerate() {
                if line_idx >= fn_start && line_idx <= fn_end {
                    continue;
                }
                if find_symbol_call(line, &name, 0).is_some()
                    || line_contains_unquoted_symbol_reference(line, &name)
                {
                    reachable = true;
                    break;
                }
            }
            if reachable {
                fn_start = fn_end + 1;
                continue;
            }

            for line in lines.iter_mut().take(fn_end + 1).skip(fn_start) {
                line.clear();
            }
            changed = true;
            break;
        }
        if !changed {
            break;
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}
