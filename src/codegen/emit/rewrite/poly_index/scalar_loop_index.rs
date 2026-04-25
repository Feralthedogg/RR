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
