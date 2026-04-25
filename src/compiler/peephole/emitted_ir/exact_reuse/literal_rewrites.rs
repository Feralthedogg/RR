pub(in super::super) fn rewrite_literal_field_get_calls_ir(lines: Vec<String>) -> Vec<String> {
    if !has_literal_field_get_candidates_ir(&lines) {
        return lines;
    }
    let Some(re) = literal_field_get_re() else {
        return lines;
    };
    let mut program = EmittedProgram::parse(&lines);
    for item in &mut program.items {
        match item {
            EmittedItem::Raw(line) => {
                *line = re
                    .replace_all(line, |caps: &Captures<'_>| {
                        let base = caps.name("base").map(|m| m.as_str()).unwrap_or("").trim();
                        let name = caps.name("name").map(|m| m.as_str()).unwrap_or("").trim();
                        format!(r#"{base}[["{name}"]]"#)
                    })
                    .to_string();
            }
            EmittedItem::Function(function) => {
                for stmt in &mut function.body {
                    let rewritten = re
                        .replace_all(&stmt.text, |caps: &Captures<'_>| {
                            let base = caps.name("base").map(|m| m.as_str()).unwrap_or("").trim();
                            let name = caps.name("name").map(|m| m.as_str()).unwrap_or("").trim();
                            format!(r#"{base}[["{name}"]]"#)
                        })
                        .to_string();
                    if rewritten != stmt.text {
                        stmt.replace_text(rewritten);
                    }
                }
            }
        }
    }
    program.into_lines()
}

fn rewrite_literal_named_list_line_ir(line: &str) -> String {
    let mut rewritten = line.to_string();
    let mut search_start = 0usize;
    loop {
        let Some(start) = rewritten[search_start..]
            .find("rr_named_list(")
            .map(|offset| search_start + offset)
        else {
            break;
        };
        let call_start = start + "rr_named_list".len();
        let mut depth = 0i32;
        let mut end = None;
        for (off, ch) in rewritten[call_start..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(call_start + off);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(call_end) = end else {
            break;
        };
        let args_inner = &rewritten[call_start + 1..call_end];
        let Some(args) = split_top_level_args(args_inner) else {
            search_start = call_end + 1;
            continue;
        };
        if args.len() % 2 != 0 {
            search_start = call_end + 1;
            continue;
        }
        let mut fields = Vec::new();
        let mut ok = true;
        for pair in args.chunks(2) {
            let Some(name) = literal_record_field_name(pair[0].trim()) else {
                ok = false;
                break;
            };
            fields.push(format!("{name} = {}", pair[1].trim()));
        }
        if !ok {
            search_start = call_end + 1;
            continue;
        }
        let replacement = if fields.is_empty() {
            "list()".to_string()
        } else {
            format!("list({})", fields.join(", "))
        };
        let replacement_end = start + replacement.len();
        rewritten.replace_range(start..=call_end, &replacement);
        search_start = replacement_end;
    }
    rewritten
}

pub(in super::super) fn rewrite_literal_named_list_calls_ir(lines: Vec<String>) -> Vec<String> {
    if !has_literal_named_list_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    for item in &mut program.items {
        match item {
            EmittedItem::Raw(line) => {
                if !line.contains("rr_named_list <- function") {
                    *line = rewrite_literal_named_list_line_ir(line);
                }
            }
            EmittedItem::Function(function) => {
                for stmt in &mut function.body {
                    let rewritten = rewrite_literal_named_list_line_ir(&stmt.text);
                    if rewritten != stmt.text {
                        stmt.replace_text(rewritten);
                    }
                }
            }
        }
    }
    program.into_lines()
}

pub(in super::super) fn simplify_nested_index_vec_floor_calls_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_nested_index_vec_floor_candidates_ir(&lines) {
        return lines;
    }
    let Some(re) = nested_index_vec_floor_re() else {
        return lines;
    };
    let mut program = EmittedProgram::parse(&lines);
    for item in &mut program.items {
        match item {
            EmittedItem::Raw(line) => {
                let mut rewritten = line.clone();
                loop {
                    let next = re
                        .replace_all(&rewritten, |caps: &Captures<'_>| {
                            format!(
                                "rr_index_vec_floor({})",
                                caps.name("inner").map(|m| m.as_str()).unwrap_or("")
                            )
                        })
                        .to_string();
                    if next == rewritten {
                        break;
                    }
                    rewritten = next;
                }
                *line = rewritten;
            }
            EmittedItem::Function(function) => {
                for stmt in &mut function.body {
                    let mut rewritten = stmt.text.clone();
                    loop {
                        let next = re
                            .replace_all(&rewritten, |caps: &Captures<'_>| {
                                format!(
                                    "rr_index_vec_floor({})",
                                    caps.name("inner").map(|m| m.as_str()).unwrap_or("")
                                )
                            })
                            .to_string();
                        if next == rewritten {
                            break;
                        }
                        rewritten = next;
                    }
                    if rewritten != stmt.text {
                        stmt.replace_text(rewritten);
                    }
                }
            }
        }
    }
    program.into_lines()
}

fn literal_field_read_expr_re() -> Option<&'static regex::Regex> {
    static RE: OnceLock<Option<regex::Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r#"^(?P<base>{})\[\["(?P<field>[A-Za-z_][A-Za-z0-9_]*)"\]\]$"#,
            IDENT_PATTERN
        ))
    })
    .as_ref()
}
