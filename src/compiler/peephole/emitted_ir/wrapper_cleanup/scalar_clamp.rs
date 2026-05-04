use super::*;
pub(crate) fn has_trivial_scalar_clamp_wrapper_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            let significant: Vec<&str> = lines[func.body_start..=func.end]
                .iter()
                .map(|line| line.trim())
                .filter(|trimmed| !trimmed.is_empty() && *trimmed != "{" && *trimmed != "}")
                .collect();
            if significant.len() != 6 {
                return false;
            }
            significant[0].contains(" <- ")
                && significant[1].starts_with("if ((")
                && significant[1].contains(" < ")
                && significant[2].contains(" <- ")
                && significant[3].starts_with("if ((")
                && significant[3].contains(" > ")
                && significant[4].contains(" <- ")
                && significant[5].starts_with("return(")
        })
}

pub(crate) fn apply_collapse_trivial_scalar_clamp_wrappers_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
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
        if significant.len() != 6 {
            continue;
        }
        let Some((tmp, init_expr)) = significant[0]
            .1
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim().to_string(), rhs.trim().to_string()))
        else {
            continue;
        };
        let Some((assign_lo_lhs, lo_expr)) = significant[2]
            .1
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim().to_string(), rhs.trim().to_string()))
        else {
            continue;
        };
        let Some((assign_hi_lhs, hi_expr)) = significant[4]
            .1
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim().to_string(), rhs.trim().to_string()))
        else {
            continue;
        };
        if assign_lo_lhs != tmp
            || assign_hi_lhs != tmp
            || significant[5].1 != format!("return({tmp})")
        {
            continue;
        }
        let first_guard_ok = significant[1].1 == format!("if (({init_expr} < {lo_expr})) {{")
            || significant[1].1 == format!("if (({tmp} < {lo_expr})) {{");
        let second_guard_ok = significant[3].1 == format!("if (({tmp} > {hi_expr})) {{");
        if !first_guard_ok || !second_guard_ok {
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
                "{indent}return(pmin(pmax({init_expr}, {lo_expr}), {hi_expr}))"
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
