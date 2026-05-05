use super::*;
pub(crate) fn parse_accumulate_product_line_ir(
    line: &str,
    acc: &str,
) -> Option<(String, String, String)> {
    let pattern = format!(
        r"^(?P<lhs>{}) <- \({} \+ \((?P<a>{})\[(?P<idx_a>{})\] \* (?P<b>{})\[(?P<idx_b>{})\]\)\)$",
        IDENT_PATTERN,
        regex::escape(acc),
        IDENT_PATTERN,
        IDENT_PATTERN,
        IDENT_PATTERN,
        IDENT_PATTERN
    );
    let caps = compile_regex(pattern)?.captures(line.trim())?;
    let lhs = caps.name("lhs")?.as_str().trim();
    let lhs_vec = caps.name("a")?.as_str().trim();
    let rhs_vec = caps.name("b")?.as_str().trim();
    let idx_a = caps.name("idx_a")?.as_str().trim();
    let idx_b = caps.name("idx_b")?.as_str().trim();
    if lhs != acc || idx_a != idx_b {
        return None;
    }
    Some((lhs_vec.to_string(), rhs_vec.to_string(), idx_a.to_string()))
}

pub(crate) fn apply_collapse_trivial_dot_product_wrappers_ir(program: &mut EmittedProgram) {
    fn is_zero_literal(expr: &str) -> bool {
        matches!(expr.trim(), "0" | "0L" | "0.0")
    }

    fn is_one_literal(expr: &str) -> bool {
        matches!(expr.trim(), "1" | "1L" | "1.0")
    }

    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((_name, params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        if params.len() != 3 {
            continue;
        }

        let significant: Vec<(usize, String)> = function
            .body
            .iter()
            .enumerate()
            .filter_map(|(idx, stmt)| {
                let trimmed = stmt.text.trim();
                if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                    None
                } else {
                    Some((idx, trimmed.to_string()))
                }
            })
            .collect();
        if significant.len() < 7 {
            continue;
        }

        let mut aliases: FxHashMap<String, String> = params
            .iter()
            .cloned()
            .map(|param| (param.clone(), param))
            .collect();
        let mut idx = 0usize;
        while idx < significant.len() {
            let Some((lhs, rhs)) = significant[idx]
                .1
                .split_once(" <- ")
                .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
            else {
                break;
            };
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                break;
            }
            if params.iter().any(|param| param == rhs) {
                aliases.insert(lhs.to_string(), rhs.to_string());
                idx += 1;
                continue;
            }
            break;
        }

        if idx + 6 >= significant.len() {
            continue;
        }
        let Some((acc, init_expr)) = significant[idx]
            .1
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
        else {
            continue;
        };
        let Some((iter_var, iter_init)) = significant[idx + 1]
            .1
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
        else {
            continue;
        };
        if !plain_ident_re().is_some_and(|re| re.is_match(acc))
            || !plain_ident_re().is_some_and(|re| re.is_match(iter_var))
            || !is_zero_literal(init_expr)
            || !is_one_literal(iter_init)
            || significant[idx + 2].1 != "repeat {"
        {
            continue;
        }

        let guard_line = format!("if (!({iter_var} <= {})) break", params[2]);
        let guard_line_with_alias = aliases.iter().find_map(|(alias, base)| {
            (base == &params[2] && alias != &params[2])
                .then(|| format!("if (!({iter_var} <= {alias})) break"))
        });
        if significant[idx + 3].1 != guard_line
            && guard_line_with_alias.as_deref() != Some(significant[idx + 3].1.as_str())
        {
            continue;
        }

        let mut product_idx = idx + 4;
        let mut index_ref = iter_var.to_string();
        if let Some((alias_lhs, alias_rhs)) = significant[product_idx]
            .1
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
            && alias_rhs == iter_var
            && plain_ident_re().is_some_and(|re| re.is_match(alias_lhs))
        {
            index_ref = alias_lhs.to_string();
            product_idx += 1;
        }
        if product_idx + 2 >= significant.len() {
            continue;
        }

        let Some((lhs_vec, rhs_vec, vec_index_ref)) =
            parse_accumulate_product_line_ir(&significant[product_idx].1, acc)
        else {
            continue;
        };
        let resolved_lhs = aliases
            .get(&lhs_vec)
            .map(String::as_str)
            .unwrap_or(lhs_vec.as_str());
        let resolved_rhs = aliases
            .get(&rhs_vec)
            .map(String::as_str)
            .unwrap_or(rhs_vec.as_str());
        if vec_index_ref != index_ref
            || resolved_lhs != params[0]
            || resolved_rhs != params[1]
            || !matches!(
                significant[product_idx + 1].1.as_str(),
                line if line == format!("{iter_var} <- ({iter_var} + 1)")
                    || line == format!("{iter_var} <- ({iter_var} + 1L)")
                    || line == format!("{iter_var} <- ({iter_var} + 1.0)")
            )
            || significant[product_idx + 2].1 != "next"
            || significant.last().map(|(_, line)| line.as_str()) != Some(&format!("return({acc})"))
            || significant.len() != product_idx + 4
        {
            continue;
        }

        let Some(return_idx) = function
            .body
            .iter()
            .rposition(|stmt| matches!(stmt.kind, EmittedStmtKind::Return))
        else {
            continue;
        };
        let open_idx = function
            .body
            .iter()
            .position(|stmt| stmt.text.trim() == "{")
            .unwrap_or(0);
        let indent = function.body[return_idx].indent();
        if open_idx < function.body.len() {
            function.body[open_idx].replace_text("{".to_string());
        }
        if open_idx + 1 < function.body.len() {
            function.body[open_idx + 1].replace_text(format!(
                "{indent}return(sum(({}[seq_len({})] * {}[seq_len({})])))",
                params[0], params[2], params[1], params[2]
            ));
        }
        let clear_end = function.body.len().saturating_sub(1);
        for stmt in function
            .body
            .iter_mut()
            .skip(open_idx + 2)
            .take(clear_end.saturating_sub(open_idx + 2))
        {
            stmt.clear();
        }
    }
}

pub(crate) fn collapse_trivial_dot_product_wrappers_ir(lines: Vec<String>) -> Vec<String> {
    if !has_trivial_dot_product_wrapper_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_collapse_trivial_dot_product_wrappers_ir(&mut program);
    program.into_lines()
}
