use super::*;
pub(crate) fn has_assignment_to_one_before_local(lines: &[String], idx: usize, var: &str) -> bool {
    (0..idx).rev().any(|i| {
        parse_local_assign_line(&lines[i]).is_some_and(|(lhs, rhs)| {
            lhs == var && literal_one_re_local().is_some_and(|re| re.is_match(rhs))
        })
    })
}

pub(crate) fn function_has_matching_exprmap_whole_assign_local(
    lines: &[String],
    dest_var: &str,
    end_expr: &str,
    temp_var: &str,
) -> bool {
    if !temp_var.starts_with(".tachyon_exprmap") {
        return false;
    }
    let Some(temp_idx) = lines
        .iter()
        .position(|line| parse_local_assign_line(line).is_some_and(|(lhs, _)| lhs == temp_var))
    else {
        return false;
    };
    let Some((_, temp_rhs)) = parse_local_assign_line(&lines[temp_idx]) else {
        return false;
    };

    for line in lines.iter().skip(temp_idx + 1) {
        let Some((lhs, rhs)) = parse_local_assign_line(line) else {
            continue;
        };
        let Some(slice_caps) = assign_slice_re_local().and_then(|re| re.captures(rhs)) else {
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

pub(crate) fn function_has_non_empty_repeat_whole_assign_local(
    lines: &[String],
    dest_var: &str,
    end_expr: &str,
    temp_var: &str,
) -> bool {
    let Some(temp_idx) = lines
        .iter()
        .position(|line| parse_local_assign_line(line).is_some_and(|(lhs, _)| lhs == temp_var))
    else {
        return false;
    };
    let Some((_, temp_rhs)) = parse_local_assign_line(&lines[temp_idx]) else {
        return false;
    };

    let mut assign_idx = None;
    for idx in temp_idx + 1..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(&lines[idx]) else {
            continue;
        };
        let Some(slice_caps) = assign_slice_re_local().and_then(|re| re.captures(rhs)) else {
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
            && plain_ident_re_local().is_some_and(|re| re.is_match(start))
            && has_assignment_to_one_before_local(lines, idx, start)
        {
            assign_idx = Some(idx);
            break;
        }
    }
    let Some(assign_idx) = assign_idx else {
        return false;
    };

    let Some(repeat_idx) = (0..assign_idx)
        .rev()
        .find(|idx| lines[*idx].trim() == "repeat {")
    else {
        return false;
    };
    let Some(guard_idx) = (repeat_idx + 1..assign_idx).find(|idx| {
        lines[*idx].trim().starts_with("if !(") || lines[*idx].trim().starts_with("if (!(")
    }) else {
        return false;
    };
    let guard = lines[guard_idx].trim();
    let Some(inner) = guard
        .strip_prefix("if (!(")
        .and_then(|s| s.strip_suffix(")) break"))
    else {
        return false;
    };
    let Some((iter_var, bound)) = inner.split_once("<=") else {
        return false;
    };
    literal_positive_re_local().is_some_and(|re| re.is_match(bound.trim()))
        && has_assignment_to_one_before_local(lines, guard_idx, iter_var.trim())
}
