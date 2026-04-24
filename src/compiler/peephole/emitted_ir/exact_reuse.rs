fn has_exact_expr_candidates_ir(lines: &[String]) -> bool {
    lines.iter().any(|line| {
        assign_re()
            .and_then(|re| re.captures(line.trim()))
            .and_then(|caps| caps.name("rhs").map(|m| m.as_str()))
            .is_some_and(expr_is_exact_reusable_scalar)
    })
}

#[derive(Default)]
pub(in super::super) struct ExactPreBundleProfile {
    pub(in super::super) pre_elapsed_ns: u128,
    pub(in super::super) cleanup_elapsed_ns: u128,
}

pub(in super::super) fn run_exact_pre_full_ir_bundle(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> (Vec<String>, ExactPreBundleProfile) {
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs_dead_eval = scan.needs_dead_eval;
    let needs_noop_assign = scan.needs_noop_assign;
    let needs_nested_temp = scan.needs_nested_temp;
    let needs_exact_expr = has_exact_expr_candidates_ir(&lines);
    if !needs_exact_expr && !needs_dead_eval && !needs_noop_assign && !needs_nested_temp {
        return (lines, ExactPreBundleProfile::default());
    }

    let mut profile = ExactPreBundleProfile::default();
    let mut program = EmittedProgram::parse(&lines);

    let started = std::time::Instant::now();
    if needs_exact_expr {
        apply_rewrite_forward_exact_expr_reuse_ir(&mut program);
        apply_strip_redundant_identical_pure_rebinds_ir(&mut program, pure_user_calls);
    }
    profile.pre_elapsed_ns = started.elapsed().as_nanos();

    let started = std::time::Instant::now();
    if needs_dead_eval {
        apply_strip_dead_simple_eval_lines_ir(&mut program);
    }
    if needs_noop_assign {
        apply_strip_noop_self_assignments_ir(&mut program);
    }
    if needs_nested_temp {
        apply_strip_redundant_nested_temp_reassigns_ir(&mut program);
    }
    profile.cleanup_elapsed_ns = started.elapsed().as_nanos();

    (program.into_lines(), profile)
}

pub(in super::super) fn run_secondary_exact_expr_bundle_ir(lines: Vec<String>) -> Vec<String> {
    let needs_exact_expr = has_exact_expr_candidates_ir(&lines);
    let needs_noop_assign = scan_basic_cleanup_candidates_ir(&lines).needs_noop_assign;
    if !needs_exact_expr && !needs_noop_assign {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_exact_expr {
        apply_rewrite_forward_exact_expr_reuse_ir(&mut program);
    }
    if needs_noop_assign {
        apply_strip_noop_self_assignments_ir(&mut program);
    }
    program.into_lines()
}

pub(in super::super) fn run_secondary_exact_bundle_ir(lines: Vec<String>) -> Vec<String> {
    let needs_dead_zero = has_dead_zero_loop_seed_candidates_ir(&lines);
    let needs_terminal_next = has_terminal_repeat_next_candidates_ir(&lines);
    let needs_exact_expr = has_exact_expr_candidates_ir(&lines);
    let needs_noop_assign = scan_basic_cleanup_candidates_ir(&lines).needs_noop_assign;
    if !needs_dead_zero && !needs_terminal_next && !needs_exact_expr && !needs_noop_assign {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_dead_zero {
        apply_rewrite_dead_zero_loop_seeds_before_for_ir(&mut program);
    }
    if needs_terminal_next {
        apply_strip_terminal_repeat_nexts_ir(&mut program);
    }
    if needs_exact_expr {
        apply_rewrite_forward_exact_expr_reuse_ir(&mut program);
    }
    if needs_noop_assign {
        apply_strip_noop_self_assignments_ir(&mut program);
    }
    run_secondary_exact_local_scalar_bundle(program.into_lines())
}

pub(in super::super) fn run_exact_finalize_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs_dead_eval = scan.needs_dead_eval;
    let needs_noop_assign = scan.needs_noop_assign;
    let needs_nested_temp = scan.needs_nested_temp;
    let needs_tail_assign = has_tail_assign_slice_return_candidates_ir(&lines);
    if !needs_dead_eval && !needs_noop_assign && !needs_nested_temp && !needs_tail_assign {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_dead_eval {
        apply_strip_dead_simple_eval_lines_ir(&mut program);
    }
    if needs_noop_assign {
        apply_strip_noop_self_assignments_ir(&mut program);
    }
    if needs_nested_temp {
        apply_strip_redundant_nested_temp_reassigns_ir(&mut program);
    }
    if needs_tail_assign {
        apply_strip_redundant_tail_assign_slice_return_ir(&mut program);
    }
    program.into_lines()
}

pub(in super::super) fn collapse_identical_if_else_tail_assignments_late_ir(
    lines: Vec<String>,
) -> Vec<String> {
    let mut program = EmittedProgram::parse(&lines);
    apply_collapse_identical_if_else_tail_assignments_late_ir(&mut program);
    program.into_lines()
}

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
    loop {
        let Some(start) = rewritten.find("rr_named_list(") else {
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
            break;
        };
        if args.len() % 2 != 0 {
            break;
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
            break;
        }
        let replacement = if fields.is_empty() {
            "list()".to_string()
        } else {
            format!("list({})", fields.join(", "))
        };
        rewritten.replace_range(start..=call_end, &replacement);
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

fn is_identical_pure_rebind_candidate_ir(
    lhs: &str,
    rhs: &str,
    pure_user_calls: &FxHashSet<String>,
) -> bool {
    let lhs = lhs.trim();
    let rhs = rhs.trim();
    let is_pure_call = rhs.contains('(') && expr_has_only_pure_calls(rhs, pure_user_calls);
    let is_literal_field_read = literal_field_read_expr_re().is_some_and(|re| re.is_match(rhs));
    plain_ident_re().is_some_and(|re| re.is_match(lhs))
        && !lhs.starts_with(".arg_")
        && !lhs.starts_with(".__rr_cse_")
        && (is_pure_call || is_literal_field_read)
}

pub(in super::super) fn strip_redundant_identical_pure_rebinds_ir(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    if !lines.iter().any(|line| line.contains("<-")) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_redundant_identical_pure_rebinds_ir(&mut program, pure_user_calls);
    program.into_lines()
}

fn apply_strip_redundant_identical_pure_rebinds_ir(
    program: &mut EmittedProgram,
    pure_user_calls: &FxHashSet<String>,
) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut removable = vec![false; function.body.len()];
        for idx in 0..function.body.len() {
            let EmittedStmtKind::Assign { lhs, rhs } = &function.body[idx].kind else {
                continue;
            };
            if !is_identical_pure_rebind_candidate_ir(lhs, rhs, pure_user_calls) {
                continue;
            }
            let rhs_canonical = strip_redundant_outer_parens(rhs);
            let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
            let cur_indent = function.body[idx].indent().len();
            let mut depth = 0usize;
            let mut crossed_enclosing_if_boundary = false;
            let mut found = false;
            for prev_idx in (0..idx).rev() {
                let prev_stmt = &function.body[prev_idx];
                match &prev_stmt.kind {
                    EmittedStmtKind::Blank => continue,
                    EmittedStmtKind::BlockClose => {
                        depth += 1;
                        continue;
                    }
                    EmittedStmtKind::IfOpen | EmittedStmtKind::ElseOpen => {
                        if depth > 0 {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                            continue;
                        }
                        if matches!(prev_stmt.kind, EmittedStmtKind::IfOpen)
                            && !crossed_enclosing_if_boundary
                        {
                            crossed_enclosing_if_boundary = true;
                            continue;
                        }
                        break;
                    }
                    EmittedStmtKind::RepeatOpen
                    | EmittedStmtKind::ForSeqLen { .. }
                    | EmittedStmtKind::ForOpen
                    | EmittedStmtKind::WhileOpen
                    | EmittedStmtKind::OtherOpen => {
                        if depth > 0 {
                            depth -= 1;
                        }
                        continue;
                    }
                    EmittedStmtKind::Assign {
                        lhs: prev_lhs,
                        rhs: prev_rhs,
                    } => {
                        if prev_lhs == lhs {
                            if depth == 0 {
                                let prev_indent = prev_stmt.indent().len();
                                let same_scope_rebind = prev_indent == cur_indent;
                                let enclosing_if_rebind = crossed_enclosing_if_boundary;
                                if strip_redundant_outer_parens(prev_rhs) == rhs_canonical
                                    && (same_scope_rebind || enclosing_if_rebind)
                                {
                                    found = true;
                                }
                            }
                            break;
                        }
                        if deps.contains(prev_lhs) {
                            break;
                        }
                    }
                    _ => {
                        if let Some(base) = indexed_store_base_re()
                            .and_then(|re| re.captures(prev_stmt.text.trim()))
                            .and_then(|caps| {
                                caps.name("base").map(|m| m.as_str().trim().to_string())
                            })
                        {
                            if base == *lhs || deps.contains(&base) {
                                break;
                            }
                        }
                    }
                }
            }
            removable[idx] = found;
        }
        function.body = function
            .body
            .drain(..)
            .enumerate()
            .filter_map(|(idx, stmt)| (!removable[idx]).then_some(stmt))
            .collect();
    }
}

fn straight_line_region_end(body: &[EmittedStmt], idx: usize) -> usize {
    let candidate_indent = body[idx].indent().len();
    let mut line_no = idx + 1;
    while line_no < body.len() {
        let trimmed = body[line_no].text.trim();
        let next_indent = body[line_no].indent().len();
        if matches!(body[line_no].kind, EmittedStmtKind::RepeatOpen)
            || matches!(body[line_no].kind, EmittedStmtKind::WhileOpen)
            || matches!(body[line_no].kind, EmittedStmtKind::ForSeqLen { .. })
            || matches!(body[line_no].kind, EmittedStmtKind::ForOpen)
            || (!trimmed.is_empty() && next_indent < candidate_indent)
        {
            break;
        }
        line_no += 1;
    }
    line_no
}

fn collect_assign_line_indices(body: &[EmittedStmt]) -> FxHashMap<String, Vec<usize>> {
    let mut defs = FxHashMap::default();
    for (idx, stmt) in body.iter().enumerate() {
        if let Some((lhs, _)) = stmt.assign_parts() {
            defs.entry(lhs.to_string())
                .or_insert_with(Vec::new)
                .push(idx);
        }
    }
    defs
}

fn next_assign_line_before(
    defs: &FxHashMap<String, Vec<usize>>,
    lhs: &str,
    after_idx: usize,
    region_end: usize,
) -> usize {
    let Some(lines) = defs.get(lhs) else {
        return region_end;
    };
    let start = lines.partition_point(|line_idx| *line_idx <= after_idx);
    match lines.get(start).copied() {
        Some(next_idx) if next_idx < region_end => next_idx,
        _ => region_end,
    }
}

fn prev_assign_line_before(
    defs: &FxHashMap<String, Vec<usize>>,
    lhs: &str,
    before_idx: usize,
) -> Option<usize> {
    let lines = defs.get(lhs)?;
    let end = lines.partition_point(|line_idx| *line_idx < before_idx);
    end.checked_sub(1).and_then(|idx| lines.get(idx)).copied()
}

fn compute_straight_line_region_ends(body: &[EmittedStmt]) -> Vec<usize> {
    (0..body.len())
        .map(|idx| straight_line_region_end(body, idx))
        .collect()
}

pub(in super::super) fn rewrite_forward_exact_pure_call_reuse_ir(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    if !lines
        .iter()
        .any(|line| line.contains("<-") && line.contains('('))
    {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_forward_exact_pure_call_reuse_ir(&mut program, pure_user_calls);
    program.into_lines()
}

fn apply_rewrite_forward_exact_pure_call_reuse_ir(
    program: &mut EmittedProgram,
    pure_user_calls: &FxHashSet<String>,
) {
    let debug = std::env::var_os("RR_DEBUG_IR_PURE_CALL").is_some();
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let straight_region_ends = compute_straight_line_region_ends(&function.body);
        let assign_line_indices = collect_assign_line_indices(&function.body);
        for idx in 0..function.body.len() {
            let Some((lhs, rhs)) = function.body[idx].assign_parts() else {
                continue;
            };
            let lhs = lhs.to_string();
            let rhs = rhs.to_string();
            if !rhs.contains('(') {
                continue;
            }
            if !plain_ident_re().is_some_and(|re| re.is_match(&lhs))
                || lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || !expr_has_only_pure_calls(&rhs, pure_user_calls)
            {
                continue;
            }
            let deps: FxHashSet<String> = expr_idents(&rhs).into_iter().collect();
            if deps.contains(&lhs) {
                continue;
            }
            let straight_region_end = straight_region_ends[idx];
            let next_lhs_def =
                next_assign_line_before(&assign_line_indices, &lhs, idx, straight_region_end);
            let lhs_reassigned_later = next_lhs_def < straight_region_end;
            let scan_end = next_lhs_def;
            if scan_end <= idx + 1 {
                continue;
            }
            if !function.body[(idx + 1)..scan_end]
                .iter()
                .any(|stmt| stmt.text.contains(&rhs))
            {
                continue;
            }
            let mut line_no = idx + 1;
            while line_no < scan_end {
                let mut should_break = false;
                let mut should_continue = false;
                let current_text = function.body[line_no].text.clone();
                let assign_parts = function.body[line_no]
                    .assign_parts()
                    .map(|(lhs, rhs)| (lhs.to_string(), rhs.to_string()));
                if let Some((next_lhs, next_rhs)) = assign_parts {
                    if next_lhs == lhs {
                        should_break = true;
                    } else {
                        if next_rhs.contains(&rhs) {
                            if lhs_reassigned_later {
                                should_continue = true;
                            } else {
                                if let Some(new_text) = replace_exact_rhs_occurrence(
                                    &function.body[line_no],
                                    &rhs,
                                    &lhs,
                                ) {
                                    if debug {
                                        eprintln!(
                                            "RR_DEBUG_IR_PURE_CALL rewrite cand_line={} lhs=`{}` rhs=`{}` target_line={} before=`{}` after=`{}`",
                                            idx + 1,
                                            lhs,
                                            rhs,
                                            line_no + 1,
                                            function.body[line_no].text.trim(),
                                            new_text.trim()
                                        );
                                    }
                                    function.body[line_no].replace_text(new_text);
                                }
                            }
                        }
                        if deps.contains(&next_lhs) {
                            should_break = true;
                        }
                    }
                } else {
                    let line_trimmed = current_text.trim().to_string();
                    if line_trimmed.contains(&rhs) {
                        if let Some(new_text) =
                            replace_exact_rhs_occurrence(&function.body[line_no], &rhs, &lhs)
                        {
                            if debug {
                                eprintln!(
                                    "RR_DEBUG_IR_PURE_CALL rewrite cand_line={} lhs=`{}` rhs=`{}` target_line={} before=`{}` after=`{}`",
                                    idx + 1,
                                    lhs,
                                    rhs,
                                    line_no + 1,
                                    function.body[line_no].text.trim(),
                                    new_text.trim()
                                );
                            }
                            function.body[line_no].replace_text(new_text);
                        }
                    }
                    if line_trimmed == "return(NULL)"
                        || (line_trimmed.starts_with("return(") && line_trimmed.ends_with(')'))
                    {
                        should_break = true;
                    }
                }
                if should_break {
                    break;
                }
                if should_continue {
                    line_no += 1;
                    continue;
                }
                line_no += 1;
            }
        }
    }
}

pub(in super::super) fn rewrite_forward_exact_expr_reuse_ir(lines: Vec<String>) -> Vec<String> {
    if !lines.iter().any(|line| line.contains("<-")) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_forward_exact_expr_reuse_ir(&mut program);
    program.into_lines()
}

#[derive(Default)]
pub(in super::super) struct ExactReuseBundleProfile {
    pub(in super::super) pure_call_elapsed_ns: u128,
    pub(in super::super) expr_elapsed_ns: u128,
    pub(in super::super) rebind_elapsed_ns: u128,
}

pub(in super::super) fn run_exact_reuse_ir_bundle(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> (Vec<String>, ExactReuseBundleProfile) {
    if !lines.iter().any(|line| line.contains("<-")) {
        return (lines, ExactReuseBundleProfile::default());
    }
    let mut profile = ExactReuseBundleProfile::default();
    let mut program = EmittedProgram::parse(&lines);

    let started = std::time::Instant::now();
    apply_rewrite_forward_exact_pure_call_reuse_ir(&mut program, pure_user_calls);
    profile.pure_call_elapsed_ns = started.elapsed().as_nanos();

    let started = std::time::Instant::now();
    apply_rewrite_forward_exact_expr_reuse_ir(&mut program);
    profile.expr_elapsed_ns = started.elapsed().as_nanos();

    let started = std::time::Instant::now();
    apply_strip_redundant_identical_pure_rebinds_ir(&mut program, pure_user_calls);
    profile.rebind_elapsed_ns = started.elapsed().as_nanos();

    (program.into_lines(), profile)
}

fn apply_rewrite_forward_exact_expr_reuse_ir(program: &mut EmittedProgram) {
    let debug = std::env::var_os("RR_DEBUG_IR_EXACT_EXPR").is_some();
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let straight_region_ends = compute_straight_line_region_ends(&function.body);
        let assign_line_indices = collect_assign_line_indices(&function.body);
        let mut function_lines = None;
        let candidate_snapshots = function
            .body
            .iter()
            .enumerate()
            .filter_map(|(idx, stmt)| {
                let (lhs, rhs) = stmt.assign_parts()?;
                Some((idx, lhs.to_string(), rhs.to_string()))
            })
            .collect::<Vec<_>>();
        for (idx, lhs, rhs) in candidate_snapshots {
            if idx >= function.body.len() {
                continue;
            }
            let ident_count = expr_idents(&rhs).len();
            let replacement_symbol = prefer_smaller_cse_symbol(&lhs, &rhs);
            if !plain_ident_re().is_some_and(|re| re.is_match(&lhs))
                || !expr_is_exact_reusable_scalar(&rhs)
                || (lhs.starts_with(".__rr_cse_") && ident_count > 2 && replacement_symbol == lhs)
            {
                continue;
            }
            let straight_region_end = straight_region_ends[idx];
            let next_lhs_def =
                next_assign_line_before(&assign_line_indices, &lhs, idx, straight_region_end);
            let lhs_reassigned_later = next_lhs_def < straight_region_end;
            let scan_end = next_lhs_def;
            if scan_end <= idx + 1 {
                continue;
            }
            if !function.body[(idx + 1)..scan_end]
                .iter()
                .any(|stmt| stmt.text.contains(&rhs))
            {
                continue;
            }
            let deps: FxHashSet<String> = expr_idents(&rhs).into_iter().collect();
            let mut prologue_arg_aliases = None;
            for line_no in idx + 1..scan_end {
                let mut should_break = false;
                let mut should_continue = false;
                let current_text = function.body[line_no].text.clone();
                let assign_parts = function.body[line_no]
                    .assign_parts()
                    .map(|(lhs, rhs)| (lhs.to_string(), rhs.to_string()));
                if let Some((next_lhs, next_rhs)) = assign_parts {
                    if next_lhs == lhs {
                        should_break = true;
                    } else {
                        if next_rhs.contains(&rhs) {
                            if lhs_reassigned_later {
                                should_continue = true;
                            } else {
                                if let Some(new_text) = replace_exact_rhs_occurrence(
                                    &function.body[line_no],
                                    &rhs,
                                    &replacement_symbol,
                                ) {
                                    if debug {
                                        eprintln!(
                                            "RR_DEBUG_IR_EXACT_EXPR rewrite cand_line={} lhs=`{}` rhs=`{}` target_line={} before=`{}` after=`{}`",
                                            idx + 1,
                                            lhs,
                                            rhs,
                                            line_no + 1,
                                            function.body[line_no].text.trim(),
                                            new_text.trim()
                                        );
                                    }
                                    function.body[line_no].replace_text(new_text);
                                }
                            }
                        }
                        if deps.contains(&next_lhs) {
                            let mut same_rhs_as_previous = false;
                            if let Some(prev_idx) =
                                prev_assign_line_before(&assign_line_indices, &next_lhs, line_no)
                            {
                                let Some((_, prev_rhs)) = function.body[prev_idx].assign_parts()
                                else {
                                    should_break = true;
                                    if should_break {
                                        break;
                                    }
                                    continue;
                                };
                                let aliases = prologue_arg_aliases.get_or_insert_with(|| {
                                    let lines = function_lines.get_or_insert_with(|| {
                                        function
                                            .body
                                            .iter()
                                            .map(|stmt| stmt.text.clone())
                                            .collect::<Vec<_>>()
                                    });
                                    collect_prologue_arg_aliases(lines, idx)
                                });
                                let prev_norm = normalize_expr_with_aliases(prev_rhs, aliases);
                                let next_norm = normalize_expr_with_aliases(&next_rhs, aliases);
                                if prev_norm == next_norm {
                                    same_rhs_as_previous = true;
                                }
                            }
                            if same_rhs_as_previous {
                                should_continue = true;
                            } else {
                                should_break = true;
                            }
                        }
                    }
                } else {
                    let line_trimmed = current_text.trim().to_string();
                    if line_trimmed.contains(&rhs) {
                        if let Some(new_text) = replace_exact_rhs_occurrence(
                            &function.body[line_no],
                            &rhs,
                            &replacement_symbol,
                        ) {
                            if debug {
                                eprintln!(
                                    "RR_DEBUG_IR_EXACT_EXPR rewrite cand_line={} lhs=`{}` rhs=`{}` target_line={} before=`{}` after=`{}`",
                                    idx + 1,
                                    lhs,
                                    rhs,
                                    line_no + 1,
                                    function.body[line_no].text.trim(),
                                    new_text.trim()
                                );
                            }
                            function.body[line_no].replace_text(new_text);
                        }
                    }
                    if line_trimmed == "return(NULL)"
                        || (line_trimmed.starts_with("return(") && line_trimmed.ends_with(')'))
                    {
                        should_break = true;
                    }
                }
                if should_break {
                    break;
                }
                if should_continue {
                    continue;
                }
            }
        }
    }
}

pub(in super::super) fn run_exact_pre_ir_bundle(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    if !lines.iter().any(|line| line.contains("<-")) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_forward_exact_expr_reuse_ir(&mut program);
    apply_strip_redundant_identical_pure_rebinds_ir(&mut program, pure_user_calls);
    program.into_lines()
}
