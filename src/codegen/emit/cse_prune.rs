use super::*;

impl RBackend {
    pub(super) fn prune_dead_cse_temps(output: &mut String) {
        let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
        if lines.is_empty() {
            return;
        }
        let function_scope_ends = Self::function_scope_ends(&lines);

        loop {
            let temp_defs: Vec<(usize, String, String)> = lines
                .iter()
                .enumerate()
                .filter_map(|(idx, line)| {
                    let (name, indent) = Self::extract_cse_assign_name(line)?;
                    Some((idx, name, indent))
                })
                .collect();
            if temp_defs.is_empty() {
                break;
            }

            let mut changed = false;
            for (idx, name, indent) in temp_defs {
                let scope_end = function_scope_ends[idx];
                let is_live = lines
                    .iter()
                    .enumerate()
                    .take(scope_end + 1)
                    .skip(idx + 1)
                    .any(|(_, line)| Self::line_contains_symbol(line, &name));
                if !is_live {
                    lines[idx] = format!("{}# rr-cse-pruned", indent);
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        let seed_defs: Vec<(usize, String, String)> = lines
            .iter()
            .enumerate()
            .filter_map(|(idx, line)| {
                let (name, indent, rhs) = Self::extract_dead_loop_seed_assign(line)?;
                if is_generated_poly_loop_var_name(&name) {
                    return None;
                }
                Some((idx, name, indent)).filter(|_| rhs == "1L" || rhs == "1" || rhs == "1.0")
            })
            .collect();
        for (idx, name, indent) in seed_defs {
            let scope_end = function_scope_ends[idx];
            let Some(next_idx) = lines
                .iter()
                .enumerate()
                .take(scope_end + 1)
                .skip(idx + 1)
                .find_map(|(line_idx, line)| {
                    let trimmed = line.trim();
                    (!trimmed.is_empty() && trimmed != "# rr-cse-pruned").then_some(line_idx)
                })
            else {
                continue;
            };
            if Self::line_contains_symbol(&lines[next_idx], &name) {
                continue;
            }
            let is_live_after = lines
                .iter()
                .enumerate()
                .take(scope_end + 1)
                .skip(next_idx + 1)
                .any(|(_, line)| Self::line_contains_symbol(line, &name));
            if !is_live_after {
                lines[idx] = format!("{indent}# rr-cse-pruned");
            }
        }

        loop {
            let init_defs: Vec<(usize, String, String)> = lines
                .iter()
                .enumerate()
                .filter_map(|(idx, line)| {
                    let (name, indent, rhs) = Self::extract_plain_assign(line)?;
                    if is_generated_poly_loop_var_name(&name) {
                        return None;
                    }
                    Self::is_prunable_dead_init_rhs(rhs.as_str()).then_some((idx, name, indent))
                })
                .collect();
            if init_defs.is_empty() {
                break;
            }

            let mut changed = false;
            for (idx, name, indent) in init_defs {
                let scope_end = function_scope_ends[idx];
                let has_later_use = Self::has_later_symbol_use(&lines, idx, scope_end, &name);
                if !has_later_use
                    || Self::is_dead_pre_loop_init_overwritten_before_use(
                        &lines, idx, scope_end, &name,
                    )
                    || Self::find_dead_overwrite_without_intervening_use(
                        &lines, idx, scope_end, &name,
                    )
                    .is_some()
                {
                    lines[idx] = format!("{indent}# rr-cse-pruned");
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        let mut compacted = Vec::with_capacity(lines.len());
        for line in lines {
            let trimmed = line.trim();
            let prev_same_pruned = compacted
                .last()
                .is_some_and(|prev: &String| prev == &line && prev.trim() == "# rr-cse-pruned");
            if trimmed == "# rr-cse-pruned" && prev_same_pruned {
                continue;
            }
            compacted.push(line);
        }

        let mut rebuilt = compacted.join("\n");
        rebuilt.push('\n');
        *output = rebuilt;
    }

    pub(super) fn function_scope_ends(lines: &[String]) -> Vec<usize> {
        let mut ends: Vec<usize> = (0..lines.len()).collect();
        let mut idx = 0usize;
        while idx < lines.len() {
            if !lines[idx].contains(" <- function(") {
                idx += 1;
                continue;
            }
            let start = idx;
            let mut depth = 0isize;
            let mut saw_open = false;
            let mut end = start;
            for (j, line) in lines.iter().enumerate().skip(start) {
                for ch in line.chars() {
                    match ch {
                        '{' => {
                            depth += 1;
                            saw_open = true;
                        }
                        '}' => {
                            depth -= 1;
                        }
                        _ => {}
                    }
                }
                end = j;
                if saw_open && depth <= 0 {
                    break;
                }
            }
            for entry in ends.iter_mut().take(end + 1).skip(start) {
                *entry = end;
            }
            idx = end + 1;
        }
        ends
    }

    pub(super) fn extract_cse_assign_name(line: &str) -> Option<(String, String)> {
        let trimmed = line.trim_start();
        if !(trimmed.starts_with(".__rr_cse_")
            || trimmed.starts_with(".tachyon_callmap_arg")
            || trimmed.starts_with(".tachyon_exprmap"))
        {
            return None;
        }
        let (name, _) = trimmed.split_once(" <- ")?;
        Some((
            name.to_string(),
            line[..line.len() - trimmed.len()].to_string(),
        ))
    }

    pub(super) fn extract_dead_loop_seed_assign(line: &str) -> Option<(String, String, String)> {
        let trimmed = line.trim_start();
        let (name, rhs) = trimmed.split_once(" <- ")?;
        if !is_recognized_loop_index_name(name) {
            return None;
        }
        Some((
            name.to_string(),
            line[..line.len() - trimmed.len()].to_string(),
            rhs.trim().to_string(),
        ))
    }

    pub(super) fn extract_plain_assign(line: &str) -> Option<(String, String, String)> {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        let (name, rhs) = trimmed.split_once(" <- ")?;
        if name.is_empty() || !name.chars().all(Self::is_symbol_char) {
            return None;
        }
        Some((
            name.to_string(),
            line[..line.len() - trimmed.len()].to_string(),
            rhs.trim().to_string(),
        ))
    }

    pub(super) fn is_prunable_dead_init_rhs(rhs: &str) -> bool {
        rhs.starts_with("rep.int(")
            || rhs.starts_with("numeric(")
            || rhs.starts_with("integer(")
            || rhs.starts_with("logical(")
            || rhs.starts_with("character(")
            || rhs.starts_with("vector(")
            || rhs.starts_with("matrix(")
            || rhs.starts_with("Sym_17(")
            || matches!(
                rhs,
                "0" | "0L" | "0.0" | "1" | "1L" | "1.0" | "TRUE" | "FALSE"
            )
    }

    pub(super) fn has_later_symbol_use(
        lines: &[String],
        start_idx: usize,
        scope_end: usize,
        symbol: &str,
    ) -> bool {
        lines
            .iter()
            .enumerate()
            .take(scope_end + 1)
            .skip(start_idx + 1)
            .any(|(_, line)| Self::line_contains_symbol(line, symbol))
    }

    pub(super) fn is_dead_pre_loop_init_overwritten_before_use(
        lines: &[String],
        start_idx: usize,
        scope_end: usize,
        symbol: &str,
    ) -> bool {
        let mut loop_start = None;
        for (idx, line) in lines
            .iter()
            .enumerate()
            .take(scope_end + 1)
            .skip(start_idx + 1)
        {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "# rr-cse-pruned" || trimmed.starts_with('#') {
                continue;
            }
            if trimmed == "repeat {" {
                loop_start = Some(idx);
                break;
            }
            if Self::line_contains_symbol(line, symbol) || Self::line_breaks_straight_line(trimmed)
            {
                return false;
            }
        }
        let Some(loop_start) = loop_start else {
            return false;
        };
        let Some(loop_end) = Self::block_end_for_open_brace(lines, loop_start, scope_end) else {
            return false;
        };
        for line in lines.iter().take(loop_end).skip(loop_start + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "# rr-cse-pruned" || trimmed.starts_with('#') {
                continue;
            }
            if !Self::line_contains_symbol(line, symbol) {
                continue;
            }
            let Some((assigned, _, rhs)) = Self::extract_plain_assign(line) else {
                return false;
            };
            if assigned != symbol || Self::line_contains_symbol(rhs.as_str(), symbol) {
                return false;
            }
            return !Self::has_later_symbol_use(lines, loop_end, scope_end, symbol);
        }
        false
    }

    pub(super) fn block_end_for_open_brace(
        lines: &[String],
        start_idx: usize,
        scope_end: usize,
    ) -> Option<usize> {
        let mut depth = 0isize;
        let mut saw_open = false;
        for (idx, line) in lines.iter().enumerate().take(scope_end + 1).skip(start_idx) {
            for ch in line.chars() {
                match ch {
                    '{' => {
                        depth += 1;
                        saw_open = true;
                    }
                    '}' => depth -= 1,
                    _ => {}
                }
            }
            if saw_open && depth <= 0 {
                return Some(idx);
            }
        }
        None
    }

    pub(super) fn find_dead_overwrite_without_intervening_use(
        lines: &[String],
        start_idx: usize,
        scope_end: usize,
        symbol: &str,
    ) -> Option<usize> {
        for (idx, line) in lines
            .iter()
            .enumerate()
            .take(scope_end + 1)
            .skip(start_idx + 1)
        {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "# rr-cse-pruned" || trimmed.starts_with('#') {
                continue;
            }
            if Self::line_breaks_straight_line(trimmed) {
                return None;
            }
            if !Self::line_contains_symbol(line, symbol) {
                continue;
            }
            let (assigned, _, rhs) = Self::extract_plain_assign(line)?;
            if assigned != symbol {
                return None;
            }
            if Self::line_contains_symbol(rhs.as_str(), symbol) {
                return None;
            }
            return Some(idx);
        }
        None
    }

    pub(super) fn line_breaks_straight_line(trimmed: &str) -> bool {
        trimmed == "{"
            || trimmed == "}"
            || trimmed.starts_with("if ")
            || trimmed.starts_with("if(")
            || trimmed.starts_with("if (")
            || trimmed.starts_with("else")
            || trimmed.starts_with("repeat")
            || trimmed.starts_with("next")
            || trimmed.starts_with("break")
            || trimmed.starts_with("return(")
            || trimmed.starts_with("return (")
            || trimmed.starts_with("return ")
    }

    pub(super) fn line_contains_symbol(line: &str, symbol: &str) -> bool {
        let mut search_from = 0;
        while let Some(rel_idx) = line[search_from..].find(symbol) {
            let idx = search_from + rel_idx;
            let before = line[..idx].chars().next_back();
            let after = line[idx + symbol.len()..].chars().next();
            let boundary_ok = before.is_none_or(|ch| !Self::is_symbol_char(ch))
                && after.is_none_or(|ch| !Self::is_symbol_char(ch));
            if boundary_ok {
                return true;
            }
            search_from = idx + symbol.len();
        }
        false
    }

    pub(super) fn is_symbol_char(ch: char) -> bool {
        ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')
    }
}
