use super::super::{
    assign_re, assign_slice_re, compile_regex, expr_idents, find_matching_block_end,
    literal_one_re, literal_positive_re, plain_ident_re,
};
use rustc_hash::{FxHashMap, FxHashSet};

pub(in super::super) fn rewrite_temp_minus_one_scaled_to_named_scalar(
    lines: Vec<String>,
) -> Vec<String> {
    let mut out = lines;
    let Some(assign_re) = assign_re() else {
        return out;
    };
    let minus_one_re = compile_regex(r"^\((?P<inner>.+)\s-\s1L?\)$".to_string());

    let mut named_minus_one = FxHashMap::<String, String>::default();
    let mut temp_inner = FxHashMap::<String, String>::default();

    for line in &out {
        let trimmed = line.trim();
        let Some(caps) = assign_re.captures(trimmed) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if let Some(inner) = minus_one_re
            .as_ref()
            .and_then(|re| re.captures(rhs))
            .and_then(|caps| caps.name("inner").map(|m| m.as_str()))
            && plain_ident_re().is_some_and(|re| re.is_match(lhs))
            && !lhs.starts_with('.')
        {
            named_minus_one.insert(inner.to_string(), lhs.to_string());
        } else if lhs.starts_with(".__rr_cse_") {
            temp_inner.insert(lhs.to_string(), rhs.to_string());
        }
    }

    for line in &mut out {
        let mut rewritten = line.clone();
        for (temp, inner) in &temp_inner {
            let Some(name) = named_minus_one.get(inner) else {
                continue;
            };
            let pattern = format!(
                r"\(\(\s*{}\s*-\s*1\s*\)\s*\*\s*([^\)]+)\)",
                regex::escape(temp)
            );
            if let Some(re) = compile_regex(pattern) {
                let replacement = format!("({name} * $1)");
                rewritten = re.replace_all(&rewritten, replacement.as_str()).to_string();
            }
        }
        *line = rewritten;
    }

    out
}

pub(in super::super) fn strip_redundant_nested_temp_reassigns(lines: Vec<String>) -> Vec<String> {
    let out = lines;
    let mut remove = vec![false; out.len()];
    for idx in 0..out.len() {
        let trimmed = out[idx].trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !lhs.starts_with(".__rr_cse_") {
            continue;
        }
        let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
        let cur_indent = out[idx].len() - out[idx].trim_start().len();
        let mut j = idx;
        while j > 0 {
            j -= 1;
            let prev = out[j].trim();
            if prev.is_empty() {
                continue;
            }
            if out[j].contains("<- function")
                || prev == "repeat {"
                || prev.starts_with("while")
                || prev.starts_with("for")
            {
                break;
            }
            if let Some(prev_caps) = assign_re().and_then(|re| re.captures(prev)) {
                let prev_lhs = prev_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let prev_rhs = prev_caps
                    .name("rhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if prev_lhs == lhs {
                    if prev_rhs == lhs {
                        continue;
                    }
                    let prev_indent = out[j].len() - out[j].trim_start().len();
                    if prev_rhs == rhs && prev_indent < cur_indent {
                        remove[idx] = true;
                    }
                    break;
                }
                if deps.contains(prev_lhs) {
                    break;
                }
            }
        }
    }
    out.into_iter()
        .enumerate()
        .filter_map(|(idx, line)| (!remove[idx]).then_some(line))
        .collect()
}

pub(in super::super) fn strip_redundant_tail_assign_slice_return(
    lines: Vec<String>,
) -> Vec<String> {
    let mut out = lines;
    let mut fn_start = 0usize;
    while fn_start < out.len() {
        while fn_start < out.len() && !out[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= out.len() {
            break;
        }
        let Some(fn_end) = find_matching_block_end(&out, fn_start) else {
            break;
        };

        let Some(return_idx) = previous_non_empty_line(&out, fn_end) else {
            fn_start = fn_end + 1;
            continue;
        };
        let return_trimmed = out[return_idx].trim();
        let Some(ret_var) = return_trimmed
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            fn_start = fn_end + 1;
            continue;
        };

        let Some(assign_idx) = previous_non_empty_line(&out, return_idx) else {
            fn_start = fn_end + 1;
            continue;
        };
        let assign_trimmed = out[assign_idx].trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(assign_trimmed)) else {
            fn_start = fn_end + 1;
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if lhs != ret_var {
            fn_start = fn_end + 1;
            continue;
        }

        let Some(assign_caps) = assign_slice_re().and_then(|re| re.captures(rhs)) else {
            fn_start = fn_end + 1;
            continue;
        };
        let dest = assign_caps
            .name("dest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let start = assign_caps
            .name("start")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let end = assign_caps
            .name("end")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let temp = assign_caps
            .name("rest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if dest != ret_var
            || !literal_one_re().is_some_and(|re| re.is_match(start))
            || !plain_ident_re().is_some_and(|re| re.is_match(temp))
        {
            fn_start = fn_end + 1;
            continue;
        }

        if function_has_non_empty_repeat_whole_assign(&out[fn_start..fn_end], ret_var, end, temp)
            || function_has_matching_exprmap_whole_assign(
                &out[fn_start..fn_end],
                ret_var,
                end,
                temp,
            )
        {
            out[assign_idx].clear();
        }

        fn_start = fn_end + 1;
    }
    out
}

fn function_has_matching_exprmap_whole_assign(
    lines: &[String],
    dest_var: &str,
    end_expr: &str,
    temp_var: &str,
) -> bool {
    if !temp_var.starts_with(".tachyon_exprmap") {
        return false;
    }
    let Some(temp_idx) = lines.iter().position(|line| {
        assign_re()
            .and_then(|re| re.captures(line.trim()))
            .is_some_and(|caps| {
                caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim() == temp_var
            })
    }) else {
        return false;
    };
    let Some(temp_rhs) = assign_re()
        .and_then(|re| re.captures(lines[temp_idx].trim()))
        .and_then(|caps| caps.name("rhs").map(|m| m.as_str().trim().to_string()))
    else {
        return false;
    };

    for line in lines.iter().skip(temp_idx + 1) {
        let trimmed = line.trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(rhs)) else {
            continue;
        };
        let dest = slice_caps
            .name("dest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let end = slice_caps
            .name("end")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let rest = slice_caps
            .name("rest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if lhs == dest_var
            && dest == dest_var
            && end == end_expr
            && (rest == temp_rhs || rest == temp_var)
        {
            return true;
        }
    }
    false
}

fn function_has_non_empty_repeat_whole_assign(
    lines: &[String],
    dest_var: &str,
    end_expr: &str,
    temp_var: &str,
) -> bool {
    let debug_tail = std::env::var_os("RR_DEBUG_TAIL").is_some()
        && dest_var == "x"
        && temp_var == ".tachyon_exprmap0_1";
    let Some(temp_idx) = lines.iter().position(|line| {
        assign_re()
            .and_then(|re| re.captures(line.trim()))
            .is_some_and(|caps| {
                caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim() == temp_var
            })
    }) else {
        if debug_tail {
            eprintln!("tail-debug: no temp_idx");
        }
        return false;
    };
    let Some(temp_rhs) = assign_re()
        .and_then(|re| re.captures(lines[temp_idx].trim()))
        .and_then(|caps| caps.name("rhs").map(|m| m.as_str().trim().to_string()))
    else {
        if debug_tail {
            eprintln!("tail-debug: no temp_rhs");
        }
        return false;
    };

    let mut assign_idx = None;
    for idx in temp_idx + 1..lines.len() {
        let trimmed = lines[idx].trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(rhs)) else {
            continue;
        };
        let dest = slice_caps
            .name("dest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let start = slice_caps
            .name("start")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let end = slice_caps
            .name("end")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let rest = slice_caps
            .name("rest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if lhs == dest_var
            && dest == dest_var
            && end == end_expr
            && (rest == temp_rhs || rest == temp_var)
            && plain_ident_re().is_some_and(|re| re.is_match(start))
            && has_assignment_to_one_before(lines, idx, start)
        {
            assign_idx = Some(idx);
            break;
        }
    }
    let Some(assign_idx) = assign_idx else {
        if debug_tail {
            eprintln!("tail-debug: no inner assign match");
        }
        return false;
    };

    let Some(repeat_idx) = (0..assign_idx)
        .rev()
        .find(|idx| lines[*idx].trim() == "repeat {")
    else {
        if debug_tail {
            eprintln!("tail-debug: no repeat_idx");
        }
        return false;
    };
    let Some(guard_idx) = (repeat_idx + 1..assign_idx).find(|idx| {
        lines[*idx].trim().starts_with("if !(") || lines[*idx].trim().starts_with("if (!(")
    }) else {
        if debug_tail {
            eprintln!("tail-debug: no guard_idx");
        }
        return false;
    };
    let guard = lines[guard_idx].trim();
    let Some(inner) = guard
        .strip_prefix("if (!(")
        .and_then(|s| s.strip_suffix(")) break"))
    else {
        if debug_tail {
            eprintln!("tail-debug: guard parse failed: {}", guard);
        }
        return false;
    };
    let Some((iter_var, bound)) = inner.split_once("<=") else {
        if debug_tail {
            eprintln!("tail-debug: split <= failed: {}", inner);
        }
        return false;
    };
    let positive = literal_positive_re().is_some_and(|re| re.is_match(bound.trim()));
    let has_one = has_assignment_to_one_before(lines, guard_idx, iter_var.trim());
    if debug_tail {
        eprintln!(
            "tail-debug: temp_idx={} assign_idx={} repeat_idx={} guard_idx={} inner={} positive={} has_one={}",
            temp_idx, assign_idx, repeat_idx, guard_idx, inner, positive, has_one
        );
    }
    positive && has_one
}

fn previous_non_empty_line(lines: &[String], idx: usize) -> Option<usize> {
    (0..idx).rev().find(|i| !lines[*i].trim().is_empty())
}

fn has_assignment_to_one_before(lines: &[String], idx: usize, var: &str) -> bool {
    (0..idx).rev().any(|i| {
        assign_re()
            .and_then(|re| re.captures(lines[i].trim()))
            .is_some_and(|caps| {
                caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim() == var
                    && literal_one_re().is_some_and(|re| {
                        re.is_match(caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim())
                    })
            })
    })
}
