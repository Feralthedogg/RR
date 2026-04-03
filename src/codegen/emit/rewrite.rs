use super::*;

pub(super) fn rewrite_safe_scalar_loop_index_helpers(output: &mut String) {
    let Some(assign_re) = compile_regex(format!(r"^(?P<lhs>{}) <- (?P<rhs>.+)$", IDENT_PATTERN))
    else {
        return;
    };
    let Some(guard_re) = compile_regex(format!(
        r"^if \(!\((?P<var>{}) (?P<op><|<=) (?P<bound>{})\)\) break$",
        IDENT_PATTERN, IDENT_PATTERN
    )) else {
        return;
    };
    let Some(read_re) = compile_regex(format!(
        r#"rr_index1_read\((?P<base>{}),\s*(?P<idx>\([^)]*\)|{})\s*,\s*(?:"index"|'index')\)"#,
        IDENT_PATTERN, IDENT_PATTERN
    )) else {
        return;
    };
    let Some(write_re) = compile_regex(format!(
        r#"rr_index1_write\((?P<idx>{}),\s*(?:"index"|'index')\)"#,
        IDENT_PATTERN
    )) else {
        return;
    };
    let mut lines: Vec<String> = output.lines().map(str::to_string).collect();
    let mut i = 0usize;
    while i + 3 < lines.len() {
        let init_line = lines[i].trim().to_string();
        let Some(init_caps) = assign_re.captures(&init_line) else {
            i += 1;
            continue;
        };
        let idx_var = init_caps
            .name("lhs")
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        let init_rhs = init_caps
            .name("rhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let Some(start_value) = init_rhs
            .trim_end_matches('L')
            .trim_end_matches('l')
            .parse::<i64>()
            .ok()
        else {
            i += 1;
            continue;
        };
        if start_value < 1 || lines[i + 1].trim() != "repeat {" {
            i += 1;
            continue;
        }
        let Some(guard_caps) = guard_re.captures(lines[i + 2].trim()) else {
            i += 1;
            continue;
        };
        if guard_caps
            .name("var")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim()
            != idx_var
        {
            i += 1;
            continue;
        }
        let allow_plus_one = guard_caps
            .name("op")
            .map(|m| m.as_str())
            .is_some_and(|op| op == "<");
        let mut cursor = i + 3;
        while cursor < lines.len() {
            let trimmed = lines[cursor].trim();
            if trimmed == "}" {
                break;
            }
            let rewritten = read_re
                .replace_all(&lines[cursor], |caps: &Captures<'_>| {
                    let base = caps.name("base").map(|m| m.as_str()).unwrap_or("");
                    let idx_expr = caps.name("idx").map(|m| m.as_str()).unwrap_or("").trim();
                    let compact = idx_expr
                        .chars()
                        .filter(|c| !c.is_whitespace())
                        .collect::<String>();
                    if compact == idx_var {
                        return format!("{base}[{idx_var}]");
                    }
                    let minus_one = format!("({idx_var}-1)");
                    if compact == minus_one && start_value >= 2 {
                        return format!("{base}[({idx_var} - 1)]");
                    }
                    let plus_one = format!("({idx_var}+1)");
                    if compact == plus_one && allow_plus_one {
                        return format!("{base}[({idx_var} + 1)]");
                    }
                    caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                })
                .to_string();
            let rewritten = write_re
                .replace_all(&rewritten, |caps: &Captures<'_>| {
                    let idx_expr = caps.name("idx").map(|m| m.as_str()).unwrap_or("").trim();
                    if idx_expr == idx_var {
                        idx_var.to_string()
                    } else {
                        caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                    }
                })
                .to_string();
            lines[cursor] = rewritten;
            cursor += 1;
        }
        i = cursor.saturating_add(1);
    }
    *output = lines.join("\n");
}

pub(super) fn infer_generated_poly_loop_step(
    lines: &[String],
    body_start: usize,
    body_end: usize,
    var: &str,
) -> i64 {
    if !var.contains("tile_") {
        return 1;
    }
    let Some(step_re) = compile_regex(format!(
        r"\({}\s*\+\s*(?P<delta>[0-9]+)L\)",
        regex::escape(var)
    )) else {
        return 1;
    };
    lines
        .iter()
        .take(body_end)
        .skip(body_start)
        .filter_map(|line| {
            step_re
                .captures(line)
                .and_then(|caps| caps.name("delta"))
                .and_then(|m| m.as_str().parse::<i64>().ok())
        })
        .max()
        .map(|delta| delta + 1)
        .unwrap_or(1)
}

pub(super) fn first_generated_poly_loop_var_in_line(line: &str) -> Option<String> {
    let mut search_from = 0usize;
    while let Some(rel_idx) = line[search_from..].find(GENERATED_POLY_LOOP_IV_PREFIX) {
        let idx = search_from + rel_idx;
        let tail = &line[idx..];
        let len = tail
            .chars()
            .take_while(|ch| RBackend::is_symbol_char(*ch))
            .count();
        if len > 0 {
            return Some(tail[..len].to_string());
        }
        search_from = idx + GENERATED_POLY_LOOP_IV_PREFIX.len();
    }
    None
}

pub(super) fn restore_missing_generated_poly_loop_steps(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(str::to_string).collect();
    if lines.is_empty() {
        return;
    }

    let scope_end = lines.len().saturating_sub(1);
    let mut i = 0usize;
    while i < lines.len() {
        if lines[i].trim() != "repeat {" {
            i += 1;
            continue;
        }

        let Some(loop_end) = RBackend::block_end_for_open_brace(&lines, i, scope_end) else {
            i += 1;
            continue;
        };
        let Some((guard_idx, guard_line)) =
            lines
                .iter()
                .enumerate()
                .skip(i + 1)
                .find_map(|(idx, line)| {
                    let trimmed = line.trim();
                    (!trimmed.is_empty() && trimmed != "# rr-cse-pruned").then_some((idx, line))
                })
        else {
            i = loop_end + 1;
            continue;
        };
        let Some(var) = first_generated_poly_loop_var_in_line(guard_line) else {
            i = loop_end + 1;
            continue;
        };
        let indent = guard_line[..guard_line.len() - guard_line.trim_start().len()].to_string();

        let has_explicit_step = lines.iter().take(loop_end).skip(guard_idx).any(|line| {
            RBackend::extract_plain_assign(line).is_some_and(|(lhs, _, rhs)| {
                lhs == var && RBackend::line_contains_symbol(rhs.as_str(), &var)
            })
        });
        if has_explicit_step {
            i += 1;
            continue;
        }

        let step = infer_generated_poly_loop_step(&lines, guard_idx, loop_end, &var);
        lines.insert(loop_end, format!("{indent}{var} <- ({var} + {step}L)"));
        i += 1;
    }

    let mut rebuilt = lines.join("\n");
    rebuilt.push('\n');
    *output = rebuilt;
}
