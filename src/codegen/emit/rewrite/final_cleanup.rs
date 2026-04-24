pub(super) fn rewrite_hoisted_loop_counter_aliases(output: &mut String) {
    fn extract_loop_counter_step(rhs: &str) -> Option<String> {
        let rhs = strip_outer_parens_local(rhs).trim();
        let caps = compile_regex(r"^([A-Za-z_][A-Za-z0-9_\.]*)\s*\+\s*(1L?|1\.0)$".to_string())?
            .captures(rhs)?;
        let var = caps.get(1)?.as_str();
        let step = caps.get(2)?.as_str();
        Some(format!("({var} + {step})"))
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_block_end_local(&lines, fn_start) else {
            break;
        };

        for idx in (fn_start + 1)..fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                continue;
            };
            let lhs = lhs.to_string();
            let rhs = rhs.to_string();
            if !lhs.starts_with("licm_") {
                continue;
            }
            let Some(replacement) = extract_loop_counter_step(rhs.as_str()) else {
                continue;
            };
            let Some(var) = strip_outer_parens_local(rhs.as_str())
                .split('+')
                .next()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
            else {
                continue;
            };

            let mut use_lines = Vec::new();
            let mut valid = true;
            for later_idx in (idx + 1)..=fn_end {
                let trimmed = lines[later_idx].trim();
                if trimmed.is_empty() {
                    continue;
                }
                let occurrences = count_symbol_occurrences_local(trimmed, lhs.as_str());
                if occurrences == 0 {
                    continue;
                }
                let Some((later_lhs, later_rhs)) = parse_local_assign_line(trimmed) else {
                    valid = false;
                    break;
                };
                if later_lhs != var || strip_outer_parens_local(later_rhs).trim() != lhs {
                    valid = false;
                    break;
                }
                use_lines.push(later_idx);
            }
            if !valid || use_lines.is_empty() {
                continue;
            }

            for use_idx in use_lines {
                let indent = lines[use_idx]
                    .chars()
                    .take_while(|ch| ch.is_ascii_whitespace())
                    .collect::<String>();
                lines[use_idx] = format!("{indent}{var} <- {replacement}");
            }
            lines[idx].clear();
        }

        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

fn split_top_level_compare_local(expr: &str) -> Option<(&str, &str, &str)> {
    let mut depth = 0i32;
    let bytes = expr.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() {
        match bytes[idx] as char {
            '(' => depth += 1,
            ')' => depth -= 1,
            '>' | '<' | '=' | '!' if depth == 0 => {
                let rest = &expr[idx..];
                for op in ["<=", ">=", "==", "!=", "<", ">"] {
                    if rest.starts_with(op) {
                        let lhs = expr[..idx].trim();
                        let rhs = expr[idx + op.len()..].trim();
                        return Some((lhs, op, rhs));
                    }
                }
            }
            _ => {}
        }
        idx += 1;
    }
    None
}

fn extract_ifelse_range_expr_local(line: &str) -> Option<String> {
    let start = line.find("rr_ifelse_strict((")? + "rr_ifelse_strict((".len();
    let rest = &line[start..];
    let mut depth = 0i32;
    let mut idx = 0usize;
    while idx < rest.len() {
        let ch = rest.as_bytes()[idx] as char;
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            '<' | '>' | '=' | '!' if depth == 0 => {
                for op in ["<=", ">=", "==", "!=", "<", ">"] {
                    if rest[idx..].starts_with(op) {
                        let lhs = rest[..idx].trim();
                        if lhs.contains(':') && !lhs.contains(".__rr_cse_") {
                            return Some(lhs.to_string());
                        }
                        return None;
                    }
                }
            }
            _ => {}
        }
        idx += 1;
    }
    None
}

pub(super) fn repair_missing_cse_range_aliases(output: &mut String) {
    let Some(floor_temp_re) = compile_regex(r"rr_index_vec_floor\(\.__rr_cse_\d+\)".to_string())
    else {
        return;
    };

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for line in &mut lines {
        if !line.contains("rr_ifelse_strict(") || !line.contains("rr_index_vec_floor(.__rr_cse_") {
            continue;
        }
        let Some(range) = extract_ifelse_range_expr_local(line.as_str()) else {
            continue;
        };
        *line = floor_temp_re
            .replace_all(line, format!("rr_index_vec_floor({range})"))
            .to_string();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn restore_constant_one_guard_repeat_loop_counters(output: &mut String) {
    fn parse_constant_guard_local(line: &str) -> Option<(String, String, String)> {
        let trimmed = line.trim();
        let inner = trimmed
            .strip_prefix("if (!(")
            .or_else(|| trimmed.strip_prefix("if !("))?
            .strip_suffix(")) break")
            .or_else(|| {
                trimmed
                    .strip_prefix("if (!(")
                    .or_else(|| trimmed.strip_prefix("if !("))
                    .and_then(|s| s.strip_suffix(") break"))
            })?
            .trim();
        for op in ["<=", "<"] {
            let needle = format!(" {op} ");
            let Some((lhs, rhs)) = inner.split_once(&needle) else {
                continue;
            };
            let lhs = crate::compiler::pipeline::strip_redundant_outer_parens(lhs.trim());
            let rhs = rhs.trim();
            if lhs.is_empty() || rhs.is_empty() {
                continue;
            }
            let numeric = lhs.trim_end_matches('L').trim_end_matches('l');
            if numeric.parse::<f64>().ok().is_some() {
                return Some((lhs.to_string(), op.to_string(), rhs.to_string()));
            }
        }
        None
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        if lines[idx].trim() != "repeat {" {
            idx += 1;
            continue;
        }
        let Some(loop_end) = find_block_end_local(&lines, idx) else {
            idx += 1;
            continue;
        };
        let Some(guard_idx) = ((idx + 1)..loop_end).find(|line_idx| {
            let trimmed = lines[*line_idx].trim();
            (trimmed.starts_with("if !(") || trimmed.starts_with("if (!("))
                && trimmed.ends_with("break")
        }) else {
            idx = loop_end + 1;
            continue;
        };
        let Some((start_lit, cmp, bound)) = parse_constant_guard_local(&lines[guard_idx]) else {
            idx = loop_end + 1;
            continue;
        };
        let idx_var = ".__rr_i";
        if lines
            .iter()
            .take(loop_end)
            .skip(guard_idx + 1)
            .any(|line| count_symbol_occurrences_local(line.trim(), idx_var) > 0)
        {
            idx = loop_end + 1;
            continue;
        }

        let indent = lines[guard_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let repeat_indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines.insert(idx, format!("{repeat_indent}{idx_var} <- {start_lit}"));
        lines[guard_idx + 1] = if cmp == "<=" {
            format!("{indent}if (!({idx_var} <= {bound})) break")
        } else {
            format!("{indent}if (!({idx_var} < {bound})) break")
        };
        let one = if start_lit.contains('.') {
            "1.0"
        } else if start_lit.ends_with('L') || start_lit.ends_with('l') {
            "1L"
        } else {
            "1"
        };
        lines.insert(
            loop_end + 1,
            format!("{indent}{idx_var} <- ({idx_var} + {one})"),
        );
        idx = loop_end + 3;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn strip_redundant_branch_local_vec_fill_rebinds(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            continue;
        };
        let Some(sig) = raw_vec_fill_signature_local(rhs) else {
            continue;
        };
        let Some(branch_start) = enclosing_branch_start_local(&lines, idx) else {
            continue;
        };
        if branch_body_writes_symbol_before_local(&lines, branch_start + 1, idx, lhs) {
            continue;
        }

        let mut prev_match = None;
        for prev_idx in (0..branch_start).rev() {
            let trimmed = lines[prev_idx].trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            if trimmed == "repeat {"
                || trimmed.starts_with("while ")
                || trimmed.starts_with("while(")
                || trimmed.starts_with("for ")
                || trimmed.starts_with("for(")
                || trimmed.contains("<- function")
            {
                break;
            }
            let Some((prev_lhs, prev_rhs)) = parse_local_assign_line(trimmed) else {
                continue;
            };
            if prev_lhs == lhs {
                prev_match = Some(prev_rhs.to_string());
                break;
            }
        }
        let Some(prev_rhs) = prev_match else {
            continue;
        };
        if raw_vec_fill_signature_local(prev_rhs.as_str()).is_some_and(|prev_sig| prev_sig == sig) {
            lines[idx].clear();
        }
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn strip_unused_raw_arg_aliases(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_block_end_local(&lines, fn_start) else {
            break;
        };
        let mut idx = fn_start + 1;
        while idx < fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                idx += 1;
                continue;
            };
            if !lhs.starts_with(".arg_") || !rhs.chars().all(RBackend::is_symbol_char) {
                idx += 1;
                continue;
            }
            let used_later = lines
                .iter()
                .take(fn_end + 1)
                .skip(idx + 1)
                .any(|line| count_symbol_occurrences_local(line.trim(), lhs) > 0);
            if !used_later {
                lines[idx].clear();
            }
            idx += 1;
        }
        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_readonly_raw_arg_aliases(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_block_end_local(&lines, fn_start) else {
            break;
        };

        let mut aliases = Vec::new();
        for idx in (fn_start + 1)..fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                continue;
            };
            if !lhs.starts_with(".arg_") || !rhs.chars().all(RBackend::is_symbol_char) {
                continue;
            }
            let reassigned_later = lines
                .iter()
                .take(fn_end + 1)
                .skip(idx + 1)
                .filter_map(|line| parse_local_assign_line(line.trim()))
                .any(|(later_lhs, _)| later_lhs == rhs);
            if reassigned_later {
                continue;
            }
            aliases.push((idx, lhs.to_string(), rhs.to_string()));
        }

        for (alias_idx, alias, target) in aliases {
            for line in lines.iter_mut().take(fn_end + 1).skip(alias_idx + 1) {
                *line = replace_symbol_occurrences_local(line, &alias, &target);
            }
            lines[alias_idx].clear();
        }

        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
