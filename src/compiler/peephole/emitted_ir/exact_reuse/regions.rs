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
