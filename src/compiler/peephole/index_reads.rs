use super::*;

pub(super) fn literal_field_get_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r#"rr_field_get\((?P<base>{}),\s*"(?P<name>[A-Za-z_][A-Za-z0-9_]*)"\)"#,
            IDENT_PATTERN
        ))
    })
    .as_ref()
}

pub(super) fn rewrite_literal_field_get_calls(lines: Vec<String>) -> Vec<String> {
    rewrite_literal_field_get_calls_ir(lines)
}

pub(super) fn literal_record_field_name(arg: &str) -> Option<String> {
    let trimmed = arg.trim();
    let inner = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| {
            trimmed
                .strip_prefix('\'')
                .and_then(|s| s.strip_suffix('\''))
        })?;
    plain_ident_re()
        .is_some_and(|re| re.is_match(inner))
        .then(|| inner.to_string())
}

pub(super) fn rewrite_literal_named_list_calls(lines: Vec<String>) -> Vec<String> {
    rewrite_literal_named_list_calls_ir(lines)
}

pub(super) fn expr_is_safe_scalar_index_source(expr: &str) -> bool {
    let expr = expr.trim();
    expr.starts_with("rr_idx_cube_vec_i(")
        || expr.starts_with("rr_wrap_index_vec_i(")
        || expr_is_floor_clamped_scalar_index_source(expr)
}

pub(super) fn expr_is_floor_clamped_scalar_index_source(expr: &str) -> bool {
    let mut compact = compact_expr(expr);
    if compact.starts_with('(') && compact.ends_with(')') {
        compact = compact[1..compact.len() - 1].to_string();
    }
    compact.starts_with("pmin(pmax((1+floor(")
        || compact.starts_with("pmin(pmax(1+floor(")
        || compact.starts_with("pmin(pmax((1.0+floor(")
        || compact.starts_with("pmin(pmax(1.0+floor(")
}

fn rewrite_index_access_calls_in_line(
    mut rewritten: String,
    safe_index_vars: &FxHashSet<String>,
    scalar_positive: &FxHashSet<String>,
    vector_lens: &FxHashMap<String, String>,
    prev_lines: &[String],
    current_idx: usize,
) -> String {
    loop {
        let read_pos = rewritten.find("rr_index1_read(");
        let write_pos = rewritten.find("rr_index1_write(");
        let (callee, start) = match (read_pos, write_pos) {
            (Some(r), Some(w)) if r <= w => ("rr_index1_read", r),
            (Some(r), Some(_)) => ("rr_index1_read", r),
            (Some(r), None) => ("rr_index1_read", r),
            (None, Some(w)) => ("rr_index1_write", w),
            (None, None) => break,
        };
        let call_start = start + callee.len();
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
            break;
        };
        let replacement = match callee {
            "rr_index1_read" if args.len() >= 2 => {
                let base = args[0].trim();
                let idx = args[1].trim();
                let ctx_ok = args.len() == 2 || is_literal_index_ctx(&args[2]);
                if !plain_ident_re().is_some_and(|re| re.is_match(base)) || !ctx_ok {
                    None
                } else if idx.starts_with("rr_wrap_index_vec_i(") {
                    Some(format!("{base}[{idx}]"))
                } else if safe_index_vars.contains(idx) || expr_is_safe_scalar_index_source(idx) {
                    Some(format!("{base}[{idx}]"))
                } else if plain_ident_re().is_some_and(|re| re.is_match(idx))
                    && scalar_positive.contains(idx)
                    && vector_lens.get(base).map(String::as_str) == Some(idx)
                {
                    Some(format!("{base}[{idx}]"))
                } else if let Some((outer, bound, inner)) = parse_flat_positive_loop_index_expr(idx)
                {
                    (var_has_known_positive_progression_before(prev_lines, current_idx, &outer)
                        && var_has_known_positive_progression_before(
                            prev_lines,
                            current_idx,
                            &inner,
                        )
                        && positive_guard_for_var_before(prev_lines, current_idx, &inner, &bound))
                    .then(|| format!("{base}[{idx}]"))
                } else {
                    None
                }
            }
            "rr_index1_write" if !args.is_empty() => {
                let idx = args[0].trim();
                let ctx_ok = args.len() == 1 || is_literal_index_ctx(&args[1]);
                (ctx_ok && idx.starts_with("rr_wrap_index_vec_i(")).then(|| idx.to_string())
            }
            _ => None,
        };
        let Some(replacement) = replacement else {
            break;
        };
        rewritten.replace_range(start..=call_end, &replacement);
    }
    rewritten
}

pub(super) fn rewrite_index_access_patterns(lines: Vec<String>) -> Vec<String> {
    if !lines.iter().any(|line| {
        line.contains("rr_index1_read(")
            || (line.contains("rr_index1_write(") && line.contains("rr_wrap_index_vec_i("))
    }) {
        return lines;
    }
    let mut out = Vec::with_capacity(lines.len());
    let mut safe_index_vars = FxHashSet::<String>::default();
    let mut scalar_positive = FxHashSet::<String>::default();
    let mut vector_lens = FxHashMap::<String, String>::default();

    for line in lines {
        if line.contains("<- function") {
            safe_index_vars.clear();
            scalar_positive.clear();
            vector_lens.clear();
            out.push(line);
            continue;
        }

        let trimmed = line.trim().to_string();
        if let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) {
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                safe_index_vars.remove(lhs);
                if expr_is_safe_scalar_index_source(rhs) {
                    safe_index_vars.insert(lhs.to_string());
                }
            }
        }

        let rewritten = rewrite_index_access_calls_in_line(
            line,
            &safe_index_vars,
            &scalar_positive,
            &vector_lens,
            &out,
            out.len(),
        );

        if is_control_flow_boundary(&trimmed) {
            if trimmed == "repeat {"
                || trimmed == "}"
                || trimmed.starts_with("} else")
                || trimmed.starts_with("else")
            {
                safe_index_vars.clear();
            }
            if trimmed.starts_with("} else") || trimmed.starts_with("else") {
                scalar_positive.clear();
                vector_lens.clear();
            }
            out.push(rewritten);
            continue;
        }

        if let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) {
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                let inferred_len = infer_same_len_expr(rhs, &vector_lens);
                scalar_positive.remove(lhs);
                vector_lens.remove(lhs);
                if literal_integer_value(rhs).is_some_and(|value| value >= 1) {
                    scalar_positive.insert(lhs.to_string());
                }
                if let Some(len) = inferred_len {
                    vector_lens.insert(lhs.to_string(), len);
                }
            }
        }

        out.push(rewritten);
    }
    out
}

pub(super) fn rewrite_safe_named_index_read_calls(lines: Vec<String>) -> Vec<String> {
    if !lines.iter().any(|line| line.contains("rr_index1_read(")) {
        return lines;
    }
    let mut out = Vec::with_capacity(lines.len());
    let mut safe_index_vars = FxHashSet::<String>::default();
    for line in lines {
        if line.contains("<- function") {
            safe_index_vars.clear();
            out.push(line);
            continue;
        }
        let trimmed = line.trim().to_string();
        if is_control_flow_boundary(&trimmed) {
            if trimmed == "repeat {"
                || trimmed == "}"
                || trimmed.starts_with("} else")
                || trimmed.starts_with("else")
            {
                safe_index_vars.clear();
            }
            out.push(line);
            continue;
        }

        let mut rewritten = line;
        if let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) {
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                safe_index_vars.remove(lhs);
                if expr_is_safe_scalar_index_source(rhs) {
                    safe_index_vars.insert(lhs.to_string());
                }
            }
        }

        loop {
            let Some(start) = rewritten.find("rr_index1_read(") else {
                break;
            };
            let call_start = start + "rr_index1_read".len();
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
                break;
            };
            if args.len() < 2 {
                break;
            }
            let base = args[0].trim();
            let idx = args[1].trim();
            if !plain_ident_re().is_some_and(|re| re.is_match(base))
                || !(safe_index_vars.contains(idx) || expr_is_safe_scalar_index_source(idx))
                || !(args.len() == 2 || is_literal_index_ctx(&args[2]))
            {
                break;
            }
            rewritten.replace_range(start..=call_end, &format!("{base}[{idx}]"));
        }

        out.push(rewritten);
    }
    out
}

pub(super) fn parse_flat_positive_loop_index_expr(expr: &str) -> Option<(String, String, String)> {
    let mut compact = compact_expr(expr);
    if compact.starts_with('(') && compact.ends_with(')') {
        compact = compact[1..compact.len() - 1].to_string();
    }
    let re = compile_regex(format!(
        r"^\(\((?P<outer>{})-1(?:L|\.0+)?\)\*(?P<bound>{})\)\+(?P<inner>{})$",
        IDENT_PATTERN, IDENT_PATTERN, IDENT_PATTERN
    ))?;
    let caps = re.captures(&compact)?;
    Some((
        caps.name("outer")?.as_str().to_string(),
        caps.name("bound")?.as_str().to_string(),
        caps.name("inner")?.as_str().to_string(),
    ))
}

pub(super) fn rewrite_safe_flat_loop_index_read_calls(lines: Vec<String>) -> Vec<String> {
    if !lines.iter().any(|line| line.contains("rr_index1_read(")) {
        return lines;
    }
    let mut out = lines;
    for idx in 0..out.len() {
        if out[idx].contains("<- function") {
            continue;
        }
        let mut rewritten = out[idx].clone();
        loop {
            let Some(start) = rewritten.find("rr_index1_read(") else {
                break;
            };
            let call_start = start + "rr_index1_read".len();
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
                break;
            };
            if args.len() < 2 {
                break;
            }
            let base = args[0].trim();
            let index_expr = args[1].trim();
            let Some((outer, bound, inner)) = parse_flat_positive_loop_index_expr(index_expr)
            else {
                break;
            };
            if !plain_ident_re().is_some_and(|re| re.is_match(base))
                || !var_has_known_positive_progression_before(&out, idx, &outer)
                || !var_has_known_positive_progression_before(&out, idx, &inner)
                || !positive_guard_for_var_before(&out, idx, &inner, &bound)
                || !(args.len() == 2 || is_literal_index_ctx(&args[2]))
            {
                break;
            }
            rewritten.replace_range(start..=call_end, &format!("{base}[{index_expr}]"));
        }
        out[idx] = rewritten;
    }
    out
}

pub(super) fn is_length_preserving_call(name: &str) -> bool {
    matches!(
        name,
        "abs"
            | "sqrt"
            | "log"
            | "log10"
            | "log2"
            | "exp"
            | "sign"
            | "floor"
            | "ceiling"
            | "trunc"
            | "pmin"
            | "pmax"
            | "ifelse"
            | "rr_ifelse_strict"
    )
}

pub(super) fn expr_is_length_preserving_shape(expr: &str) -> bool {
    let expr = expr.trim();
    if expr.is_empty() || expr.contains("<-") || expr.contains("function(") {
        return false;
    }
    let Some(re) = compile_regex(format!(r"(?P<callee>{})\s*\(", IDENT_PATTERN)) else {
        return false;
    };
    re.captures_iter(expr).all(|caps| {
        let callee = caps.name("callee").map(|m| m.as_str()).unwrap_or("").trim();
        is_length_preserving_call(callee)
    })
}

pub(super) fn infer_same_len_expr(
    expr: &str,
    vector_lens: &FxHashMap<String, String>,
) -> Option<String> {
    let mut expr = expr.trim();
    loop {
        if !(expr.starts_with('(') && expr.ends_with(')')) {
            break;
        }
        let mut depth = 0i32;
        let mut wraps = true;
        for (idx, ch) in expr.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 && idx + ch.len_utf8() < expr.len() {
                        wraps = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !wraps {
            break;
        }
        expr = expr[1..expr.len() - 1].trim();
    }

    if let Some(inner) = expr
        .strip_prefix("seq_len(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return Some(inner.trim().to_string());
    }
    if let Some(inner) = expr
        .strip_prefix("rep.int(")
        .and_then(|s| s.strip_suffix(')'))
        && let Some(args) = split_top_level_args(inner)
        && args.len() == 2
    {
        return Some(args[1].trim().to_string());
    }

    let vector_idents: Vec<String> = expr_idents(expr)
        .into_iter()
        .filter_map(|ident| vector_lens.get(&ident).cloned())
        .collect();
    if vector_idents.is_empty() || !expr_is_length_preserving_shape(expr) {
        return None;
    }
    let first = vector_idents[0].clone();
    vector_idents
        .iter()
        .all(|len| len == &first)
        .then_some(first)
}

pub(super) fn rewrite_same_len_scalar_tail_reads(lines: Vec<String>) -> Vec<String> {
    if !lines.iter().any(|line| line.contains("rr_index1_read(")) {
        return lines;
    }
    let mut out = Vec::with_capacity(lines.len());
    let mut scalar_positive = FxHashSet::<String>::default();
    let mut vector_lens = FxHashMap::<String, String>::default();

    for line in lines {
        if line.contains("<- function") {
            scalar_positive.clear();
            vector_lens.clear();
            out.push(line);
            continue;
        }

        let trimmed = line.trim().to_string();
        let mut rewritten = line;

        loop {
            let Some(start) = rewritten.find("rr_index1_read(") else {
                break;
            };
            let call_start = start + "rr_index1_read".len();
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
                break;
            };
            if args.len() < 2 {
                break;
            }
            let base = args[0].trim();
            let idx = args[1].trim();
            if !plain_ident_re().is_some_and(|re| re.is_match(base))
                || !plain_ident_re().is_some_and(|re| re.is_match(idx))
                || !scalar_positive.contains(idx)
                || vector_lens.get(base).map(String::as_str) != Some(idx)
                || !(args.len() == 2 || is_literal_index_ctx(&args[2]))
            {
                break;
            }
            rewritten.replace_range(start..=call_end, &format!("{base}[{idx}]"));
        }

        if is_control_flow_boundary(&trimmed) {
            if trimmed.starts_with("} else") || trimmed.starts_with("else") {
                scalar_positive.clear();
                vector_lens.clear();
            }
            out.push(rewritten);
            continue;
        }

        if let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) {
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                let inferred_len = infer_same_len_expr(rhs, &vector_lens);
                scalar_positive.remove(lhs);
                vector_lens.remove(lhs);
                if literal_integer_value(rhs).is_some_and(|value| value >= 1) {
                    scalar_positive.insert(lhs.to_string());
                }
                if let Some(len) = inferred_len {
                    vector_lens.insert(lhs.to_string(), len);
                }
            }
        }

        out.push(rewritten);
    }
    out
}

pub(super) fn is_literal_index_ctx(arg: &str) -> bool {
    matches!(arg.trim(), "\"index\"" | "'index'")
}

pub(super) fn rewrite_wrap_index_scalar_access_helpers(lines: Vec<String>) -> Vec<String> {
    if !lines.iter().any(|line| {
        line.contains("rr_wrap_index_vec_i(")
            && (line.contains("rr_index1_read(") || line.contains("rr_index1_write("))
    }) {
        return lines;
    }
    lines
        .into_iter()
        .map(|line| {
            if line.contains("<- function") {
                return line;
            }
            let mut rewritten = line;
            loop {
                let mut changed = false;
                for callee in ["rr_index1_read", "rr_index1_write"] {
                    let Some(start) = rewritten.find(&format!("{callee}(")) else {
                        continue;
                    };
                    let call_start = start + callee.len();
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
                        continue;
                    };
                    let args_inner = &rewritten[call_start + 1..call_end];
                    let Some(args) = split_top_level_args(args_inner) else {
                        continue;
                    };
                    let replacement = match callee {
                        "rr_index1_read" if args.len() >= 2 => {
                            let base = args[0].trim();
                            let idx = args[1].trim();
                            if plain_ident_re().is_some_and(|re| re.is_match(base))
                                && idx.starts_with("rr_wrap_index_vec_i(")
                                && (args.len() == 2 || is_literal_index_ctx(&args[2]))
                            {
                                Some(format!("{base}[{idx}]"))
                            } else {
                                None
                            }
                        }
                        "rr_index1_write" if !args.is_empty() => {
                            let idx = args[0].trim();
                            if idx.starts_with("rr_wrap_index_vec_i(")
                                && (args.len() == 1 || is_literal_index_ctx(&args[1]))
                            {
                                Some(idx.to_string())
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                    let Some(replacement) = replacement else {
                        continue;
                    };
                    rewritten.replace_range(start..=call_end, &replacement);
                    changed = true;
                    break;
                }
                if !changed {
                    break;
                }
            }
            rewritten
        })
        .collect()
}
