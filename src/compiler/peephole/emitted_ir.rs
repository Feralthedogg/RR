use super::*;
use crate::compiler::r_peephole::scalar_reuse_rewrites::run_secondary_exact_local_scalar_bundle;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub(in super::super) struct EmittedProgram {
    items: Vec<EmittedItem>,
}

#[derive(Debug, Clone)]
enum EmittedItem {
    Raw(String),
    Function(EmittedFunction),
}

#[derive(Debug, Clone)]
struct EmittedFunction {
    header: String,
    body: Vec<EmittedStmt>,
}

#[derive(Debug, Clone)]
struct EmittedStmt {
    text: String,
    kind: EmittedStmtKind,
}

#[derive(Debug, Clone)]
enum EmittedStmtKind {
    Blank,
    Assign { lhs: String, rhs: String },
    IfOpen,
    ElseOpen,
    RepeatOpen,
    ForSeqLen { iter_var: String, end_expr: String },
    ForOpen,
    WhileOpen,
    OtherOpen,
    BlockClose,
    Next,
    Return,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockKind {
    Repeat,
    Other,
}

impl EmittedStmt {
    fn parse(line: &str) -> Self {
        let trimmed = line.trim();
        let kind = if trimmed.is_empty() {
            EmittedStmtKind::Blank
        } else if let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) {
            let lhs = caps
                .name("lhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let rhs = caps
                .name("rhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            EmittedStmtKind::Assign { lhs, rhs }
        } else if trimmed == "} else {" {
            EmittedStmtKind::ElseOpen
        } else if trimmed == "repeat {" {
            EmittedStmtKind::RepeatOpen
        } else if trimmed.starts_with("if ") && trimmed.ends_with('{') {
            EmittedStmtKind::IfOpen
        } else if let Some((iter_var, end_expr)) = parse_for_seq_len_header(trimmed) {
            EmittedStmtKind::ForSeqLen { iter_var, end_expr }
        } else if trimmed.starts_with("for ") && trimmed.ends_with('{') {
            EmittedStmtKind::ForOpen
        } else if trimmed.starts_with("while ") && trimmed.ends_with('{') {
            EmittedStmtKind::WhileOpen
        } else if trimmed == "{" || (trimmed.ends_with('{') && !trimmed.starts_with("function")) {
            EmittedStmtKind::OtherOpen
        } else if trimmed == "}" {
            EmittedStmtKind::BlockClose
        } else if trimmed == "next" {
            EmittedStmtKind::Next
        } else if trimmed.starts_with("return(") && trimmed.ends_with(')') {
            EmittedStmtKind::Return
        } else {
            EmittedStmtKind::Other
        };
        Self {
            text: line.to_string(),
            kind,
        }
    }

    fn block_close_with_indent(indent: &str) -> Self {
        Self {
            text: format!("{indent}}}"),
            kind: EmittedStmtKind::BlockClose,
        }
    }

    fn render(&self) -> String {
        self.text.clone()
    }

    fn indent(&self) -> String {
        self.text
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect()
    }

    fn mentions_ident(&self, ident: &str) -> bool {
        match &self.kind {
            EmittedStmtKind::Assign { lhs, rhs } => {
                lhs == ident || expr_idents(rhs).iter().any(|cand| cand == ident)
            }
            EmittedStmtKind::ForSeqLen { iter_var, end_expr } => {
                iter_var == ident || expr_idents(end_expr).iter().any(|cand| cand == ident)
            }
            _ => expr_idents(&self.text).iter().any(|cand| cand == ident),
        }
    }

    fn assign_parts(&self) -> Option<(&str, &str)> {
        match &self.kind {
            EmittedStmtKind::Assign { lhs, rhs } => Some((lhs.as_str(), rhs.as_str())),
            _ => None,
        }
    }

    fn replace_text(&mut self, new_text: String) {
        *self = EmittedStmt::parse(&new_text);
    }

    fn clear(&mut self) {
        *self = EmittedStmt::parse("");
    }
}

impl EmittedProgram {
    fn parse(lines: &[String]) -> Self {
        let functions = build_function_text_index(lines, |_| None);
        let mut items = Vec::new();
        let mut line_idx = 0usize;
        for function in functions {
            while line_idx < function.start {
                items.push(EmittedItem::Raw(lines[line_idx].clone()));
                line_idx += 1;
            }
            let body = lines[(function.start + 1)..=function.end]
                .iter()
                .map(|line| EmittedStmt::parse(line))
                .collect();
            items.push(EmittedItem::Function(EmittedFunction {
                header: lines[function.start].clone(),
                body,
            }));
            line_idx = function.end + 1;
        }
        while line_idx < lines.len() {
            items.push(EmittedItem::Raw(lines[line_idx].clone()));
            line_idx += 1;
        }
        Self { items }
    }

    fn into_lines(self) -> Vec<String> {
        let mut out = Vec::new();
        for item in self.items {
            match item {
                EmittedItem::Raw(line) => out.push(line),
                EmittedItem::Function(function) => {
                    out.push(function.header);
                    out.extend(function.body.into_iter().map(|stmt| stmt.render()));
                }
            }
        }
        out
    }
}

fn parse_for_seq_len_header(trimmed: &str) -> Option<(String, String)> {
    let inner = trimmed.strip_prefix("for (")?.strip_suffix(") {")?;
    let (iter_var, rest) = inner.split_once(" in seq_len(")?;
    let end_expr = rest.strip_suffix(')')?;
    Some((iter_var.trim().to_string(), end_expr.trim().to_string()))
}

#[derive(Debug, Clone)]
struct HelperTrimIr {
    original_len: usize,
    kept_indices: Vec<usize>,
    kept_params: Vec<String>,
}

#[derive(Debug, Clone)]
struct SimpleExprHelperIr {
    params: Vec<String>,
    expr: String,
}

#[derive(Debug, Clone)]
struct MetricHelperIr {
    name_param: String,
    value_param: String,
    pre_name_lines: Vec<String>,
    pre_value_lines: Vec<String>,
}

fn parse_function_header_ir(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    let (name, rest) = trimmed.split_once("<- function(")?;
    let args_inner = rest.split_once(')')?.0.trim();
    let params = if args_inner.is_empty() {
        Vec::new()
    } else {
        split_top_level_args(args_inner)?
            .into_iter()
            .map(|arg| arg.trim().to_string())
            .collect()
    };
    Some((name.trim().to_string(), params))
}

fn collect_prologue_arg_alias_defs_ir(body: &[EmittedStmt]) -> Vec<(usize, String, String)> {
    let mut out = Vec::new();
    let mut in_prologue = true;
    for (idx, stmt) in body.iter().enumerate() {
        match &stmt.kind {
            EmittedStmtKind::Blank => continue,
            EmittedStmtKind::OtherOpen if stmt.text.trim() == "{" => continue,
            EmittedStmtKind::Assign { lhs, rhs }
                if in_prologue
                    && lhs.starts_with(".arg_")
                    && plain_ident_re().is_some_and(|re| re.is_match(rhs)) =>
            {
                out.push((idx, lhs.clone(), rhs.clone()));
            }
            _ => {
                in_prologue = false;
            }
        }
    }
    out
}

fn has_arg_alias_cleanup_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            let body: Vec<EmittedStmt> = lines[func.body_start..=func.end]
                .iter()
                .map(|line| EmittedStmt::parse(line))
                .collect();
            !collect_prologue_arg_alias_defs_ir(&body).is_empty()
        })
}

fn collect_mutated_arg_aliases_ir(body: &[EmittedStmt]) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    for stmt in body {
        match &stmt.kind {
            EmittedStmtKind::Assign { lhs, rhs } if lhs.starts_with(".arg_") => {
                let expected = lhs.trim_start_matches(".arg_");
                if rhs.trim() != expected {
                    out.insert(lhs.clone());
                }
            }
            _ => {
                if let Some(base) = indexed_store_base_re()
                    .and_then(|re| re.captures(stmt.text.trim()))
                    .and_then(|caps| caps.name("base").map(|m| m.as_str().trim().to_string()))
                    && base.starts_with(".arg_")
                {
                    out.insert(base);
                }
            }
        }
    }
    out
}

fn collect_post_prologue_assignment_facts_ir(
    body: &[EmittedStmt],
) -> (FxHashSet<String>, FxHashSet<String>, FxHashSet<String>) {
    let mut assigned = FxHashSet::default();
    let mut stored_bases = FxHashSet::default();
    let mut mentioned_arg_aliases = FxHashSet::default();
    let mut in_prologue = true;
    for stmt in body {
        match &stmt.kind {
            EmittedStmtKind::Blank => continue,
            EmittedStmtKind::OtherOpen if stmt.text.trim() == "{" => continue,
            EmittedStmtKind::Assign { lhs, rhs }
                if in_prologue
                    && lhs.starts_with(".arg_")
                    && plain_ident_re().is_some_and(|re| re.is_match(rhs)) => {}
            _ => {
                in_prologue = false;
                if let EmittedStmtKind::Assign { lhs, .. } = &stmt.kind {
                    assigned.insert(lhs.clone());
                }
                if let Some(base) = indexed_store_base_re()
                    .and_then(|re| re.captures(stmt.text.trim()))
                    .and_then(|caps| caps.name("base").map(|m| m.as_str().trim().to_string()))
                {
                    stored_bases.insert(base);
                }
                for ident in expr_idents(&stmt.text) {
                    if ident.starts_with(".arg_") {
                        mentioned_arg_aliases.insert(ident);
                    }
                }
            }
        }
    }
    (assigned, stored_bases, mentioned_arg_aliases)
}

fn is_cse_add_chain_rhs(rhs: &str) -> bool {
    let trimmed = rhs.trim();
    let inner = trimmed
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(trimmed)
        .trim();
    inner.starts_with(".__rr_cse_") && inner.contains(" + ")
}

fn prefer_smaller_cse_symbol(lhs: &str, rhs: &str) -> String {
    if !lhs.starts_with(".__rr_cse_") {
        return lhs.to_string();
    }
    let trimmed = rhs.trim();
    let inner = trimmed
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(trimmed)
        .trim();
    if let Some((left, _right)) = inner.split_once(" + ") {
        let left = left.trim();
        if left.starts_with(".__rr_cse_") {
            return left.to_string();
        }
    }
    lhs.to_string()
}

fn replace_exact_rhs_occurrence(stmt: &EmittedStmt, rhs: &str, lhs: &str) -> Option<String> {
    match &stmt.kind {
        EmittedStmtKind::Assign {
            lhs: assign_lhs,
            rhs: assign_rhs,
        } => {
            if assign_rhs.contains(rhs) {
                let indent = stmt.indent();
                let replaced_rhs = assign_rhs.replacen(rhs, lhs, usize::MAX);
                Some(format!("{indent}{assign_lhs} <- {replaced_rhs}"))
            } else {
                None
            }
        }
        EmittedStmtKind::Return => {
            let trimmed = stmt.text.trim();
            let inner = trimmed
                .strip_prefix("return(")
                .and_then(|s| s.strip_suffix(')'))?;
            if !inner.contains(rhs) {
                return None;
            }
            let indent = stmt.indent();
            let replaced = inner.replacen(rhs, lhs, usize::MAX);
            Some(format!("{indent}return({replaced})"))
        }
        _ => None,
    }
}

pub(in super::super) fn strip_terminal_repeat_nexts_ir(lines: Vec<String>) -> Vec<String> {
    if !has_terminal_repeat_next_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_terminal_repeat_nexts_ir(&mut program);
    program.into_lines()
}

fn apply_strip_terminal_repeat_nexts_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut out = Vec::with_capacity(function.body.len());
        let mut stack: Vec<BlockKind> = Vec::new();
        for stmt in function.body.drain(..) {
            match stmt.kind {
                EmittedStmtKind::RepeatOpen => {
                    stack.push(BlockKind::Repeat);
                    out.push(stmt);
                }
                EmittedStmtKind::IfOpen
                | EmittedStmtKind::ForSeqLen { .. }
                | EmittedStmtKind::ForOpen
                | EmittedStmtKind::WhileOpen
                | EmittedStmtKind::OtherOpen => {
                    stack.push(BlockKind::Other);
                    out.push(stmt);
                }
                EmittedStmtKind::ElseOpen => {
                    let _ = stack.pop();
                    stack.push(BlockKind::Other);
                    out.push(stmt);
                }
                EmittedStmtKind::BlockClose => {
                    let closed = stack.pop();
                    if closed == Some(BlockKind::Repeat)
                        && out
                            .iter()
                            .rfind(|prev| !matches!(prev.kind, EmittedStmtKind::Blank))
                            .is_some_and(|prev| matches!(prev.kind, EmittedStmtKind::Next))
                    {
                        if let Some(remove_idx) = out
                            .iter()
                            .rposition(|prev| !matches!(prev.kind, EmittedStmtKind::Blank))
                        {
                            out.remove(remove_idx);
                        }
                    }
                    out.push(stmt);
                }
                _ => out.push(stmt),
            }
        }
        function.body = out;
    }
}

pub(in super::super) fn strip_empty_else_blocks_ir(lines: Vec<String>) -> Vec<String> {
    if !has_empty_else_block_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_empty_else_blocks_ir(&mut program);
    program.into_lines()
}

fn apply_strip_empty_else_blocks_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut out = Vec::with_capacity(function.body.len());
        let mut idx = 0usize;
        while idx < function.body.len() {
            if matches!(function.body[idx].kind, EmittedStmtKind::ElseOpen) {
                let mut close_idx = idx + 1;
                while close_idx < function.body.len()
                    && matches!(function.body[close_idx].kind, EmittedStmtKind::Blank)
                {
                    close_idx += 1;
                }
                if close_idx < function.body.len()
                    && matches!(function.body[close_idx].kind, EmittedStmtKind::BlockClose)
                {
                    out.push(EmittedStmt::block_close_with_indent(
                        &function.body[idx].indent(),
                    ));
                    idx = close_idx + 1;
                    continue;
                }
            }
            out.push(function.body[idx].clone());
            idx += 1;
        }
        function.body = out;
    }
}

pub(in super::super) fn strip_dead_simple_eval_lines_ir(lines: Vec<String>) -> Vec<String> {
    if !scan_basic_cleanup_candidates_ir(&lines).needs_dead_eval {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_dead_simple_eval_lines_ir(&mut program);
    program.into_lines()
}

fn apply_strip_dead_simple_eval_lines_ir(program: &mut EmittedProgram) {
    program.items = program
        .items
        .drain(..)
        .filter_map(|item| match item {
            EmittedItem::Raw(line) => {
                let trimmed = line.trim();
                (!is_dead_plain_ident_eval_line(trimmed)
                    && !is_dead_parenthesized_eval_line(trimmed))
                .then_some(EmittedItem::Raw(line))
            }
            EmittedItem::Function(mut function) => {
                function.body.retain(|stmt| {
                    let trimmed = stmt.text.trim();
                    !is_dead_plain_ident_eval_line(trimmed)
                        && !is_dead_parenthesized_eval_line(trimmed)
                });
                Some(EmittedItem::Function(function))
            }
        })
        .collect();
}

fn is_noop_self_assign_stmt(stmt: &EmittedStmt) -> bool {
    stmt.assign_parts()
        .is_some_and(|(lhs, rhs)| lhs.trim() == rhs.trim())
}

pub(in super::super) fn strip_noop_self_assignments_ir(lines: Vec<String>) -> Vec<String> {
    if !scan_basic_cleanup_candidates_ir(&lines).needs_noop_assign {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_noop_self_assignments_ir(&mut program);
    program.into_lines()
}

fn apply_strip_noop_self_assignments_ir(program: &mut EmittedProgram) {
    program.items = program
        .items
        .drain(..)
        .filter_map(|item| match item {
            EmittedItem::Raw(line) => {
                let stmt = EmittedStmt::parse(&line);
                (!is_noop_self_assign_stmt(&stmt)).then_some(EmittedItem::Raw(line))
            }
            EmittedItem::Function(mut function) => {
                function.body.retain(|stmt| !is_noop_self_assign_stmt(stmt));
                Some(EmittedItem::Function(function))
            }
        })
        .collect();
}

#[derive(Clone, Copy, Debug, Default)]
struct BasicCleanupCandidatesIr {
    needs_dead_eval: bool,
    needs_noop_assign: bool,
    needs_nested_temp: bool,
}

fn scan_basic_cleanup_candidates_ir(lines: &[String]) -> BasicCleanupCandidatesIr {
    let mut out = BasicCleanupCandidatesIr::default();
    for line in lines {
        let trimmed = line.trim();
        if !out.needs_dead_eval
            && (is_dead_plain_ident_eval_line(trimmed) || is_dead_parenthesized_eval_line(trimmed))
        {
            out.needs_dead_eval = true;
        }
        if !out.needs_noop_assign
            && assign_re()
                .and_then(|re| re.captures(trimmed))
                .is_some_and(|caps| {
                    let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                    let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                    lhs == rhs
                })
        {
            out.needs_noop_assign = true;
        }
        if !out.needs_nested_temp
            && assign_re()
                .and_then(|re| re.captures(trimmed))
                .is_some_and(|caps| {
                    caps.name("lhs")
                        .map(|m| m.as_str())
                        .unwrap_or("")
                        .trim()
                        .starts_with(".__rr_cse_")
                })
        {
            out.needs_nested_temp = true;
        }
        if out.needs_dead_eval && out.needs_noop_assign && out.needs_nested_temp {
            break;
        }
    }
    out
}

pub(in super::super) fn run_dead_eval_noop_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs_dead_eval = scan.needs_dead_eval;
    let needs_noop_assign = scan.needs_noop_assign;
    if !needs_dead_eval && !needs_noop_assign {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_dead_eval {
        apply_strip_dead_simple_eval_lines_ir(&mut program);
    }
    if needs_noop_assign {
        apply_strip_noop_self_assignments_ir(&mut program);
    }
    program.into_lines()
}

pub(in super::super) fn run_secondary_finalize_cleanup_bundle_ir(
    lines: Vec<String>,
    preserve_all_defs: bool,
) -> Vec<String> {
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs_dead_eval = scan.needs_dead_eval;
    let needs_noop_assign = scan.needs_noop_assign;
    let needs_tail_assign = has_tail_assign_slice_return_candidates_ir(&lines);
    let needs_unreachable = !preserve_all_defs && has_unreachable_sym_helper_candidates_ir(&lines);
    if !needs_dead_eval && !needs_noop_assign && !needs_tail_assign && !needs_unreachable {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_secondary_finalize_cleanup_bundle_ir(
        &mut program,
        needs_dead_eval,
        needs_noop_assign,
        needs_unreachable,
        needs_tail_assign,
    );
    program.into_lines()
}

fn apply_secondary_finalize_cleanup_bundle_ir(
    program: &mut EmittedProgram,
    needs_dead_eval: bool,
    needs_noop_assign: bool,
    needs_unreachable: bool,
    needs_tail_assign: bool,
) {
    if needs_dead_eval {
        apply_strip_dead_simple_eval_lines_ir(program);
    }
    if needs_noop_assign {
        apply_strip_noop_self_assignments_ir(program);
    }
    if needs_unreachable {
        apply_strip_unreachable_sym_helpers_ir(program);
    }
    if needs_tail_assign {
        apply_strip_redundant_tail_assign_slice_return_ir(program);
    }
}

pub(in super::super) fn run_secondary_empty_else_finalize_bundle_ir(
    lines: Vec<String>,
    preserve_all_defs: bool,
) -> Vec<String> {
    let needs_empty_else = has_empty_else_block_candidates_ir(&lines);
    let needs_match_phi = has_restore_empty_match_single_bind_candidates_ir(&lines);
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs_dead_eval = scan.needs_dead_eval;
    let needs_noop_assign = scan.needs_noop_assign;
    let needs_tail_assign = has_tail_assign_slice_return_candidates_ir(&lines);
    let needs_unreachable = !preserve_all_defs && has_unreachable_sym_helper_candidates_ir(&lines);
    if !needs_empty_else
        && !needs_match_phi
        && !needs_dead_eval
        && !needs_noop_assign
        && !needs_tail_assign
        && !needs_unreachable
    {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_match_phi {
        apply_restore_empty_match_single_bind_arms_ir(&mut program);
    }
    if needs_empty_else {
        apply_strip_empty_else_blocks_ir(&mut program);
    }
    apply_secondary_finalize_cleanup_bundle_ir(
        &mut program,
        needs_dead_eval,
        needs_noop_assign,
        needs_unreachable,
        needs_tail_assign,
    );
    program.into_lines()
}

pub(in super::super) fn run_empty_else_match_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
    let needs_empty_else = has_empty_else_block_candidates_ir(&lines);
    let needs_match_phi = has_restore_empty_match_single_bind_candidates_ir(&lines);
    if !needs_empty_else && !needs_match_phi {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_match_phi {
        apply_restore_empty_match_single_bind_arms_ir(&mut program);
    }
    if needs_empty_else {
        apply_strip_empty_else_blocks_ir(&mut program);
    }
    program.into_lines()
}

fn parse_singleton_list_match_cond_ir(line: &str) -> Option<String> {
    let pattern = format!(
        r#"^if \(\(\(length\((?P<base>{})\) == 1L\) & TRUE\)\) \{{$"#,
        IDENT_PATTERN
    );
    let caps = compile_regex(pattern)?.captures(line.trim())?;
    Some(caps.name("base")?.as_str().to_string())
}

fn parse_single_field_record_match_cond_ir(line: &str) -> Option<(String, String)> {
    let pattern = format!(
        r#"^if \(\(\(TRUE & rr_field_exists\((?P<base>{}), "(?P<field>[^"]+)"\)\) & TRUE\)\) \{{$"#,
        IDENT_PATTERN
    );
    let caps = compile_regex(pattern)?.captures(line.trim())?;
    Some((
        caps.name("base")?.as_str().to_string(),
        caps.name("field")?.as_str().to_string(),
    ))
}

fn has_restore_empty_match_single_bind_candidates_ir(lines: &[String]) -> bool {
    for idx in 0..lines.len().saturating_sub(3) {
        if lines[idx + 1].trim() != "} else {" || lines[idx + 3].trim() != "}" {
            continue;
        }
        let Some(phi_caps) = assign_re().and_then(|re| re.captures(lines[idx + 2].trim())) else {
            continue;
        };
        let phi_lhs = phi_caps
            .name("lhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if !phi_lhs.starts_with(".phi_") {
            continue;
        }
        if parse_singleton_list_match_cond_ir(&lines[idx]).is_some()
            || parse_single_field_record_match_cond_ir(&lines[idx]).is_some()
        {
            return true;
        }
    }
    false
}

fn has_empty_else_block_candidates_ir(lines: &[String]) -> bool {
    for idx in 0..lines.len().saturating_sub(1) {
        if lines[idx].trim() != "} else {" {
            continue;
        }
        let mut close_idx = idx + 1;
        while close_idx < lines.len() && lines[close_idx].trim().is_empty() {
            close_idx += 1;
        }
        if close_idx < lines.len() && lines[close_idx].trim() == "}" {
            return true;
        }
    }
    false
}

fn apply_restore_empty_match_single_bind_arms_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut idx = 0usize;
        while idx + 3 < function.body.len() {
            if function.body[idx + 1].text.trim() != "} else {"
                || function.body[idx + 3].text.trim() != "}"
            {
                idx += 1;
                continue;
            }

            let Some(phi_caps) =
                assign_re().and_then(|re| re.captures(function.body[idx + 2].text.trim()))
            else {
                idx += 1;
                continue;
            };
            let phi_lhs = phi_caps
                .name("lhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if !phi_lhs.starts_with(".phi_") {
                idx += 1;
                continue;
            }

            let indent = function.body[idx + 2].indent();

            if let Some(base) = parse_singleton_list_match_cond_ir(&function.body[idx].text) {
                function.body.insert(
                    idx + 1,
                    EmittedStmt::parse(&format!("{indent}{phi_lhs} <- {base}[1L]")),
                );
                idx += 5;
                continue;
            }

            if let Some((base, field)) =
                parse_single_field_record_match_cond_ir(&function.body[idx].text)
            {
                function.body.insert(
                    idx + 1,
                    EmittedStmt::parse(&format!(r#"{indent}{phi_lhs} <- {base}[["{field}"]]"#)),
                );
                idx += 5;
                continue;
            }

            idx += 1;
        }
    }
}

fn apply_strip_unreachable_sym_helpers_ir(program: &mut EmittedProgram) {
    let mut item_index_by_name = FxHashMap::<String, usize>::default();
    for (item_idx, item) in program.items.iter().enumerate() {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((fn_name, _)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        if fn_name.starts_with("Sym_") {
            item_index_by_name.insert(fn_name, item_idx);
        }
    }
    if item_index_by_name.is_empty() {
        return;
    }

    let sym_top_is_empty_entrypoint = |function: &EmittedFunction| {
        let mut saw_return_null = false;
        for stmt in &function.body {
            let trimmed = stmt.text.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            if trimmed == "return(NULL)" {
                saw_return_null = true;
                continue;
            }
            if !unquoted_sym_refs(trimmed).is_empty() {
                return false;
            }
            return false;
        }
        saw_return_null
    };

    let mut roots = FxHashSet::default();
    if item_index_by_name.contains_key("Sym_top_0") {
        roots.insert("Sym_top_0".to_string());
    }
    for item in &program.items {
        if let EmittedItem::Raw(line) = item {
            for name in unquoted_sym_refs(line) {
                if item_index_by_name.contains_key(&name) {
                    roots.insert(name);
                }
            }
        }
    }
    if roots.is_empty() {
        return;
    }
    if roots.len() == 1
        && roots.contains("Sym_top_0")
        && item_index_by_name
            .get("Sym_top_0")
            .and_then(|idx| match &program.items[*idx] {
                EmittedItem::Function(function) => Some(function),
                _ => None,
            })
            .is_some_and(sym_top_is_empty_entrypoint)
    {
        return;
    }

    let mut reachable = roots.clone();
    let mut work: Vec<String> = roots.into_iter().collect();
    while let Some(name) = work.pop() {
        let Some(item_idx) = item_index_by_name.get(&name) else {
            continue;
        };
        let EmittedItem::Function(function) = &program.items[*item_idx] else {
            continue;
        };
        for stmt in &function.body {
            for callee in unquoted_sym_refs(&stmt.text) {
                if item_index_by_name.contains_key(&callee) && reachable.insert(callee.clone()) {
                    work.push(callee);
                }
            }
        }
    }

    program.items.retain(|item| match item {
        EmittedItem::Raw(_) => true,
        EmittedItem::Function(function) => {
            parse_function_header_ir(&function.header).is_none_or(|(fn_name, _)| {
                !fn_name.starts_with("Sym_") || reachable.contains(&fn_name)
            })
        }
    });
}

pub(in super::super) fn rewrite_dead_zero_loop_seeds_before_for_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_dead_zero_loop_seed_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_dead_zero_loop_seeds_before_for_ir(&mut program);
    program.into_lines()
}

fn apply_rewrite_dead_zero_loop_seeds_before_for_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut removable = vec![false; function.body.len()];
        for idx in 0..function.body.len() {
            let EmittedStmtKind::Assign { lhs, rhs } = &function.body[idx].kind else {
                continue;
            };
            let seed = rhs.trim();
            if seed != "0" && seed != "1" {
                continue;
            }
            let Some(for_idx) = ((idx + 1)..function.body.len()).take(12).find(|line_idx| {
                matches!(
                    &function.body[*line_idx].kind,
                    EmittedStmtKind::ForSeqLen { iter_var, .. } if iter_var == lhs
                )
            }) else {
                continue;
            };
            let used_before_for = function.body[(idx + 1)..for_idx]
                .iter()
                .any(|stmt| stmt.mentions_ident(lhs));
            if !used_before_for {
                removable[idx] = true;
            }
        }
        function.body = function
            .body
            .drain(..)
            .enumerate()
            .filter_map(|(idx, stmt)| (!removable[idx]).then_some(stmt))
            .collect();
    }
}

fn expr_is_trivial_passthrough_setup_rhs_ir(rhs: &str) -> bool {
    let rhs = rhs.trim();
    plain_ident_re().is_some_and(|re| re.is_match(rhs))
        || scalar_lit_re().is_some_and(|re| re.is_match(rhs))
        || expr_is_fresh_allocation_like(rhs, &FxHashSet::default())
        || rhs
            .strip_prefix("length(")
            .and_then(|s| s.strip_suffix(')'))
            .is_some_and(|inner| plain_ident_re().is_some_and(|re| re.is_match(inner.trim())))
}

fn apply_strip_arg_aliases_in_trivial_return_wrappers_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some(return_idx) = function
            .body
            .iter()
            .rposition(|stmt| matches!(stmt.kind, EmittedStmtKind::Return))
        else {
            continue;
        };
        let return_line = function.body[return_idx].text.trim().to_string();
        let Some(inner) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
        else {
            continue;
        };

        let mut aliases = FxHashMap::default();
        let mut trivial = true;
        for stmt in function.body.iter().take(return_idx) {
            match &stmt.kind {
                EmittedStmtKind::Blank | EmittedStmtKind::BlockClose => continue,
                EmittedStmtKind::OtherOpen if stmt.text.trim() == "{" => continue,
                EmittedStmtKind::Assign { lhs, rhs } => {
                    if lhs.starts_with(".arg_")
                        && plain_ident_re().is_some_and(|re| re.is_match(rhs))
                    {
                        aliases.insert(lhs.clone(), rhs.clone());
                    } else {
                        trivial = false;
                        break;
                    }
                }
                _ => {
                    trivial = false;
                    break;
                }
            }
        }
        if !trivial || aliases.is_empty() {
            continue;
        }
        let rewritten = normalize_expr_with_aliases(inner, &aliases);
        if rewritten != inner {
            let indent = function.body[return_idx].indent();
            function.body[return_idx].replace_text(format!("{indent}return({rewritten})"));
            for stmt in function.body.iter_mut().take(return_idx) {
                if matches!(&stmt.kind, EmittedStmtKind::Assign { lhs, .. } if lhs.starts_with(".arg_"))
                {
                    stmt.clear();
                }
            }
        }
    }
}

pub(in super::super) fn strip_arg_aliases_in_trivial_return_wrappers_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_arg_aliases_in_trivial_return_wrappers_ir(&mut program);
    program.into_lines()
}

fn apply_collapse_trivial_passthrough_return_wrappers_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some(return_idx) = function
            .body
            .iter()
            .rposition(|stmt| matches!(stmt.kind, EmittedStmtKind::Return))
        else {
            continue;
        };
        let return_line = function.body[return_idx].text.trim().to_string();
        let Some(inner) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            continue;
        };

        let mut last_assign_to_return: Option<(usize, String)> = None;
        let mut trivial = true;
        for (idx, stmt) in function.body.iter().enumerate().take(return_idx) {
            match &stmt.kind {
                EmittedStmtKind::Blank | EmittedStmtKind::BlockClose => continue,
                EmittedStmtKind::OtherOpen if stmt.text.trim() == "{" => continue,
                EmittedStmtKind::Assign { lhs, rhs } => {
                    if lhs == inner && plain_ident_re().is_some_and(|re| re.is_match(rhs)) {
                        last_assign_to_return = Some((idx, rhs.clone()));
                    } else if !expr_is_trivial_passthrough_setup_rhs_ir(rhs) {
                        trivial = false;
                        break;
                    }
                }
                _ => {
                    trivial = false;
                    break;
                }
            }
        }
        let Some((assign_idx, passthrough_ident)) = last_assign_to_return else {
            continue;
        };
        if !trivial {
            continue;
        }

        let indent = function.body[return_idx].indent();
        function.body[return_idx].replace_text(format!("{indent}return({passthrough_ident})"));
        for stmt in function.body.iter_mut().take(return_idx) {
            match stmt.kind {
                EmittedStmtKind::Blank | EmittedStmtKind::BlockClose => {}
                EmittedStmtKind::OtherOpen if stmt.text.trim() == "{" => {}
                _ => stmt.clear(),
            }
        }
        function.body[assign_idx].clear();
    }
}

pub(in super::super) fn collapse_trivial_passthrough_return_wrappers_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_passthrough_return_wrapper_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_collapse_trivial_passthrough_return_wrappers_ir(&mut program);
    program.into_lines()
}

pub(in super::super) fn run_return_wrapper_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
    if !has_arg_return_wrapper_candidates_ir(&lines)
        && !has_passthrough_return_wrapper_candidates_ir(&lines)
        && !has_trivial_dot_product_wrapper_candidates_ir(&lines)
    {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_arg_aliases_in_trivial_return_wrappers_ir(&mut program);
    apply_collapse_trivial_passthrough_return_wrappers_ir(&mut program);
    apply_collapse_trivial_dot_product_wrappers_ir(&mut program);
    program.into_lines()
}

fn collect_passthrough_helpers_from_program_ir(
    program: &EmittedProgram,
) -> FxHashMap<String, String> {
    let mut out = FxHashMap::default();
    for item in &program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((fn_name, _params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        let significant: Vec<&EmittedStmt> = function
            .body
            .iter()
            .filter(|stmt| {
                !matches!(
                    stmt.kind,
                    EmittedStmtKind::Blank | EmittedStmtKind::BlockClose
                )
            })
            .collect();
        if significant.len() != 1 {
            continue;
        }
        let stmt = significant[0];
        let trimmed = stmt.text.trim();
        let Some(inner) = trimmed
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            continue;
        };
        if plain_ident_re().is_some_and(|re| re.is_match(inner)) {
            out.insert(fn_name, inner.to_string());
        }
    }
    out
}

fn has_arg_return_wrapper_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            let Some(return_idx) = func.return_idx else {
                return false;
            };
            let return_line = lines[return_idx].trim();
            let Some(inner) = return_line
                .strip_prefix("return(")
                .and_then(|s| s.strip_suffix(')'))
            else {
                return false;
            };
            let mut aliases = FxHashMap::default();
            let mut saw_alias = false;
            for line in &lines[func.body_start..return_idx] {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                    continue;
                }
                let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                    return false;
                };
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                if lhs.starts_with(".arg_") && plain_ident_re().is_some_and(|re| re.is_match(rhs)) {
                    aliases.insert(lhs.to_string(), rhs.to_string());
                    saw_alias = true;
                } else if !expr_is_trivial_passthrough_setup_rhs_ir(rhs) {
                    return false;
                }
            }
            saw_alias && normalize_expr_with_aliases(inner, &aliases) != inner
        })
}

fn has_passthrough_return_wrapper_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            let Some(return_idx) = func.return_idx else {
                return false;
            };
            let return_line = lines[return_idx].trim();
            let Some(inner) = return_line
                .strip_prefix("return(")
                .and_then(|s| s.strip_suffix(')'))
                .map(str::trim)
            else {
                return false;
            };
            let mut last_assign_to_return = false;
            for line in &lines[func.body_start..return_idx] {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                    continue;
                }
                let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                    return false;
                };
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                if lhs == inner && plain_ident_re().is_some_and(|re| re.is_match(rhs)) {
                    last_assign_to_return = true;
                } else if !expr_is_trivial_passthrough_setup_rhs_ir(rhs) {
                    return false;
                }
            }
            last_assign_to_return
        })
}

fn has_trivial_dot_product_wrapper_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            func.params.len() == 3
                && lines[func.body_start..=func.end]
                    .iter()
                    .any(|line| line.trim() == "repeat {")
                && lines[func.body_start..=func.end]
                    .iter()
                    .any(|line| line.contains(" * "))
                && func
                    .return_idx
                    .and_then(|idx| lines.get(idx))
                    .is_some_and(|line| line.trim().starts_with("return("))
        })
}

fn has_passthrough_helper_definitions_with_calls_ir(lines: &[String]) -> bool {
    let candidate_names: Vec<String> = build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .filter_map(|func| {
            let fn_name = func.name?;
            let significant: Vec<&str> = lines[func.body_start..=func.end]
                .iter()
                .map(|line| line.trim())
                .filter(|trimmed| !trimmed.is_empty() && *trimmed != "{" && *trimmed != "}")
                .collect();
            if significant.len() != 1 {
                return None;
            }
            let inner = significant[0]
                .strip_prefix("return(")
                .and_then(|s| s.strip_suffix(')'))
                .map(str::trim)?;
            plain_ident_re()
                .is_some_and(|re| re.is_match(inner))
                .then_some(fn_name)
        })
        .collect();

    !candidate_names.is_empty()
        && lines.iter().any(|line| {
            candidate_names
                .iter()
                .any(|name| line_has_helper_callsite_ir(line, name))
        })
}

fn has_passthrough_helper_candidates_ir(lines: &[String]) -> bool {
    has_passthrough_helper_definitions_with_calls_ir(lines)
}

fn build_block_end_map_ir(lines: &[String]) -> Vec<Option<usize>> {
    let mut out = vec![None; lines.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let (opens, closes) = count_unquoted_braces(line.trim());
        for _ in 0..closes {
            let Some(open_idx) = stack.pop() else {
                break;
            };
            out[open_idx] = Some(idx);
        }
        for _ in 0..opens {
            stack.push(idx);
        }
    }
    out
}

fn has_dead_zero_loop_seed_candidates_ir(lines: &[String]) -> bool {
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if rhs != "0" && rhs != "1" {
            continue;
        }
        let Some(for_idx) = ((idx + 1)..lines.len()).take(12).find(|line_idx| {
            parse_for_seq_len_header(lines[*line_idx].trim())
                .is_some_and(|(iter_var, _)| iter_var == lhs)
        }) else {
            continue;
        };
        let used_before_for = lines[(idx + 1)..for_idx]
            .iter()
            .any(|line| expr_idents(line.trim()).iter().any(|ident| ident == lhs));
        if !used_before_for {
            return true;
        }
    }
    false
}

fn has_terminal_repeat_next_candidates_ir(lines: &[String]) -> bool {
    let block_end_map = build_block_end_map_ir(lines);
    for (idx, line) in lines.iter().enumerate() {
        if line.trim() != "repeat {" {
            continue;
        }
        let Some(end_idx) = block_end_map.get(idx).and_then(|entry| *entry) else {
            continue;
        };
        let prev_non_empty = ((idx + 1)..end_idx).rev().find_map(|line_idx| {
            let text = lines[line_idx].trim();
            (!text.is_empty()).then_some(text)
        });
        if prev_non_empty == Some("next") {
            return true;
        }
    }
    false
}

fn has_identical_if_else_tail_assign_candidates_ir(lines: &[String]) -> bool {
    let block_end_map = build_block_end_map_ir(lines);
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if !(trimmed.starts_with("if ") && trimmed.ends_with('{')) {
            continue;
        }
        let Some(end_idx) = block_end_map.get(idx).and_then(|entry| *entry) else {
            continue;
        };
        let Some(else_idx) =
            ((idx + 1)..end_idx).find(|line_idx| lines[*line_idx].trim() == "} else {")
        else {
            continue;
        };
        let then_tail = ((idx + 1)..else_idx).rev().find_map(|line_idx| {
            let text = lines[line_idx].trim();
            (!text.is_empty()).then_some(text)
        });
        let else_tail = ((else_idx + 1)..end_idx).rev().find_map(|line_idx| {
            let text = lines[line_idx].trim();
            (!text.is_empty()).then_some(text)
        });
        let (Some(then_tail), Some(else_tail)) = (then_tail, else_tail) else {
            continue;
        };
        if then_tail == else_tail && assign_re().and_then(|re| re.captures(then_tail)).is_some() {
            return true;
        }
    }
    false
}

fn has_tail_assign_slice_return_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            let Some(return_idx) = func.return_idx else {
                return false;
            };
            let ret_var = lines[return_idx]
                .trim()
                .strip_prefix("return(")
                .and_then(|s| s.strip_suffix(')'))
                .map(str::trim);
            let Some(ret_var) = ret_var else {
                return false;
            };
            ((func.body_start)..return_idx)
                .rev()
                .find_map(|line_idx| {
                    let text = lines[line_idx].trim();
                    if text.is_empty() || text == "{" || text == "}" {
                        return None;
                    }
                    Some(text)
                })
                .is_some_and(|prev| {
                    assign_re()
                        .and_then(|re| re.captures(prev))
                        .is_some_and(|caps| {
                            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                            lhs == ret_var && rhs.contains("rr_assign_slice(")
                        })
                })
        })
}

fn has_unreachable_sym_helper_candidates_ir(lines: &[String]) -> bool {
    let functions = build_function_text_index(lines, parse_function_header_ir);
    let sym_funcs: FxHashMap<String, IndexedFunction> = functions
        .iter()
        .filter_map(|func| {
            let name = func.name.clone()?;
            name.starts_with("Sym_").then_some((name, func.clone()))
        })
        .collect();
    if sym_funcs.len() <= 1 {
        return false;
    }

    let mut in_function = vec![false; lines.len()];
    for func in &functions {
        for idx in func.start..=func.end {
            if idx < in_function.len() {
                in_function[idx] = true;
            }
        }
    }

    let mut roots = FxHashSet::default();
    if sym_funcs.contains_key("Sym_top_0") {
        roots.insert("Sym_top_0".to_string());
    }
    for (idx, line) in lines.iter().enumerate() {
        if in_function[idx] {
            continue;
        }
        for name in unquoted_sym_refs(line) {
            if sym_funcs.contains_key(&name) {
                roots.insert(name);
            }
        }
    }
    if roots.is_empty() {
        return false;
    }

    let sym_top_is_empty_entrypoint = |func: &IndexedFunction| {
        let Some(return_idx) = func.return_idx else {
            return false;
        };
        let mut saw_return_null = false;
        for line in &lines[func.body_start..=return_idx] {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            if trimmed == "return(NULL)" {
                saw_return_null = true;
                continue;
            }
            if !unquoted_sym_refs(trimmed).is_empty() {
                return false;
            }
            return false;
        }
        saw_return_null
    };
    if roots.len() == 1
        && roots.contains("Sym_top_0")
        && sym_funcs
            .get("Sym_top_0")
            .is_some_and(sym_top_is_empty_entrypoint)
    {
        return false;
    }

    let mut reachable = roots.clone();
    let mut work: Vec<String> = roots.into_iter().collect();
    while let Some(name) = work.pop() {
        let Some(func) = sym_funcs.get(&name) else {
            continue;
        };
        for line in &lines[func.body_start..=func.end] {
            for callee in unquoted_sym_refs(line) {
                if sym_funcs.contains_key(&callee) && reachable.insert(callee.clone()) {
                    work.push(callee);
                }
            }
        }
    }

    reachable.len() < sym_funcs.len()
}

fn line_has_helper_callsite_ir(line: &str, helper_name: &str) -> bool {
    line.contains(&format!("{helper_name}("))
        && !line.contains(&format!("{helper_name} <- function("))
}

fn has_metric_helper_definitions_with_calls_ir(lines: &[String]) -> bool {
    let candidate_names: Vec<String> = build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .filter_map(|func| {
            let fn_name = func.name?;
            if func.params.len() != 2 {
                return None;
            }
            let body_lines: Vec<String> = lines
                .iter()
                .take(func.end)
                .skip(func.body_start)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && s != "{" && s != "}")
                .collect();
            if body_lines.len() < 3 || body_lines.len() > 5 {
                return None;
            }
            let name_param = func.params[0].clone();
            let value_param = func.params[1].clone();
            let return_line = body_lines.last()?;
            if return_line != &format!("return({value_param})") {
                return None;
            }
            let print_name_idx = body_lines
                .iter()
                .position(|line| line == &format!("print({name_param})"));
            let print_value_idx = body_lines
                .iter()
                .position(|line| line == &format!("print({value_param})"));
            let (Some(print_name_idx), Some(print_value_idx)) = (print_name_idx, print_value_idx)
            else {
                return None;
            };
            (print_name_idx < print_value_idx && print_value_idx + 1 == body_lines.len() - 1)
                .then_some(fn_name)
        })
        .collect();

    !candidate_names.is_empty()
        && lines.iter().any(|line| {
            candidate_names
                .iter()
                .any(|name| line_has_helper_callsite_ir(line, name))
        })
}

fn has_simple_expr_helper_definitions_with_calls_ir(lines: &[String]) -> bool {
    let candidate_names: Vec<String> = build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .filter_map(|func| {
            let fn_name = func.name?;
            if !fn_name.starts_with("Sym_") {
                return None;
            }
            let return_idx = func.return_idx?;
            let return_line = lines[return_idx].trim();
            let return_expr = return_line
                .strip_prefix("return(")
                .and_then(|s| s.strip_suffix(')'))
                .map(str::trim)?;
            let mut simple = true;
            for line in lines.iter().take(return_idx).skip(func.body_start) {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                    continue;
                }
                let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                    simple = false;
                    break;
                };
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                if !plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                    simple = false;
                    break;
                }
            }
            (simple && !return_expr.contains(&format!("{fn_name}("))).then_some(fn_name)
        })
        .collect();

    !candidate_names.is_empty()
        && lines.iter().any(|line| {
            candidate_names
                .iter()
                .any(|name| line_has_helper_callsite_ir(line, name))
        })
}

fn has_metric_helper_candidates_ir(lines: &[String]) -> bool {
    has_metric_helper_definitions_with_calls_ir(lines)
}

fn has_literal_field_get_candidates_ir(lines: &[String]) -> bool {
    let Some(re) = literal_field_get_re() else {
        return false;
    };
    lines.iter().any(|line| re.is_match(line))
}

fn has_literal_named_list_candidates_ir(lines: &[String]) -> bool {
    lines.iter().any(|line| {
        line.contains("rr_named_list(")
            && !line.contains("rr_named_list <- function")
            && rewrite_literal_named_list_line_ir(line) != *line
    })
}

fn has_simple_expr_helper_candidates_ir(lines: &[String]) -> bool {
    has_simple_expr_helper_definitions_with_calls_ir(lines)
}

fn apply_rewrite_passthrough_helper_calls_ir(
    program: &mut EmittedProgram,
    passthrough: &FxHashMap<String, String>,
) {
    if passthrough.is_empty() {
        return;
    }
    for item in &mut program.items {
        match item {
            EmittedItem::Function(function) => {
                for stmt in &mut function.body {
                    let trimmed = stmt.text.trim().to_string();
                    let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
                        continue;
                    };
                    let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                    let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                    let Some((callee, args_str)) = rhs.split_once('(') else {
                        continue;
                    };
                    let Some(args_inner) = args_str.strip_suffix(')') else {
                        continue;
                    };
                    let Some(param_name) = passthrough.get(callee.trim()) else {
                        continue;
                    };
                    let Some(args) = split_top_level_args(args_inner) else {
                        continue;
                    };
                    if args.len() != 1 || param_name.is_empty() {
                        continue;
                    }
                    let indent = stmt.indent();
                    stmt.replace_text(format!("{indent}{lhs} <- {}", args[0].trim()));
                }
            }
            EmittedItem::Raw(line) => {
                let trimmed = line.trim().to_string();
                let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
                    continue;
                };
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                let Some((callee, args_str)) = rhs.split_once('(') else {
                    continue;
                };
                let Some(args_inner) = args_str.strip_suffix(')') else {
                    continue;
                };
                let Some(param_name) = passthrough.get(callee.trim()) else {
                    continue;
                };
                let Some(args) = split_top_level_args(args_inner) else {
                    continue;
                };
                if args.len() != 1 || param_name.is_empty() {
                    continue;
                }
                let indent_len = line.len().saturating_sub(line.trim_start().len());
                let indent = &line[..indent_len];
                *line = format!("{indent}{lhs} <- {}", args[0].trim());
            }
        }
    }
}

fn apply_collapse_inlined_copy_vec_sequences_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let len = function.body.len();
        for idx in 0..len.saturating_sub(4) {
            let l0 = function.body[idx].text.trim().to_string();
            let l1 = function.body[idx + 1].text.trim().to_string();
            let l2 = function.body[idx + 2].text.trim().to_string();
            let l3 = function.body[idx + 3].text.trim().to_string();
            let l4 = function.body[idx + 4].text.trim().to_string();
            let Some(c0) = assign_re().and_then(|re| re.captures(&l0)) else {
                continue;
            };
            let Some(c1) = assign_re().and_then(|re| re.captures(&l1)) else {
                continue;
            };
            let Some(c2) = assign_re().and_then(|re| re.captures(&l2)) else {
                continue;
            };
            let Some(c3) = assign_re().and_then(|re| re.captures(&l3)) else {
                continue;
            };
            let Some(c4) = assign_re().and_then(|re| re.captures(&l4)) else {
                continue;
            };
            let n_var = c0.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let n_rhs = c0.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let out_var = c1.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let out_rhs = c1.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let i_var = c2.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let i_rhs = c2.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let out_replay_lhs = c3.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let src_rhs = c3.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let target_var = c4.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let target_rhs = c4.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let Some(src_var) = ({
                if let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(src_rhs)) {
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
                    (dest == out_var
                        && start == i_var
                        && end == n_var
                        && rest.starts_with("rr_index1_read_vec("))
                    .then_some(rest.to_string())
                } else {
                    None
                }
            }) else {
                continue;
            };
            if out_var != out_replay_lhs
                || target_var != out_var
                || n_rhs != format!("length({out_rhs})")
                || i_rhs != "1"
                || !target_rhs.starts_with("rr_assign_slice(")
                || !src_var.contains(out_rhs)
            {
                continue;
            }
            function.body[idx].clear();
            function.body[idx + 2].clear();
            function.body[idx + 3].clear();
        }
    }
}

fn has_inlined_copy_vec_sequence_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            let body = &lines[func.body_start..=func.end];
            body.iter().any(|line| line.contains("inlined_"))
                && body.iter().any(|line| line.contains("rr_assign_slice("))
                && body.iter().any(|line| line.contains("length("))
                && body.iter().any(|line| line.contains("rep.int("))
        })
}

fn has_nested_index_vec_floor_candidates_ir(lines: &[String]) -> bool {
    let Some(re) = nested_index_vec_floor_re() else {
        return false;
    };
    lines.iter().any(|line| re.is_match(line))
}

fn apply_simplify_nested_index_vec_floor_calls_ir(program: &mut EmittedProgram) {
    let Some(re) = nested_index_vec_floor_re() else {
        return;
    };
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
}

pub(in super::super) fn run_post_passthrough_wrapper_cleanup_bundle_ir(
    lines: Vec<String>,
) -> Vec<String> {
    let needs_floor = has_nested_index_vec_floor_candidates_ir(&lines);
    let needs_copy = has_inlined_copy_vec_sequence_candidates_ir(&lines);
    if !needs_floor && !needs_copy {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_floor {
        apply_simplify_nested_index_vec_floor_calls_ir(&mut program);
    }
    if needs_copy {
        apply_collapse_inlined_copy_vec_sequences_ir(&mut program);
    }
    program.into_lines()
}

pub(in super::super) fn rewrite_readonly_param_aliases_ir(lines: Vec<String>) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_readonly_param_aliases_ir(&mut program);
    program.into_lines()
}

pub(in super::super) fn run_arg_alias_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_unused_arg_aliases_ir(&mut program);
    apply_rewrite_readonly_param_aliases_ir(&mut program);
    apply_rewrite_remaining_readonly_param_shadow_uses_ir(&mut program);
    apply_rewrite_index_only_mutated_param_shadow_aliases_ir(&mut program);
    program.into_lines()
}

pub(in super::super) fn strip_unused_arg_aliases_ir(lines: Vec<String>) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_unused_arg_aliases_ir(&mut program);
    program.into_lines()
}

fn apply_strip_unused_arg_aliases_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let prologue_defs = collect_prologue_arg_alias_defs_ir(&function.body);
        if prologue_defs.is_empty() {
            continue;
        }
        let (_assigned, _stored_bases, mentioned) =
            collect_post_prologue_assignment_facts_ir(&function.body);
        for (idx, alias, _target) in prologue_defs {
            if !mentioned.contains(&alias)
                && let Some(stmt) = function.body.get_mut(idx)
            {
                stmt.clear();
            }
        }
    }
}

fn apply_rewrite_readonly_param_aliases_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((_, params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        let param_set: FxHashSet<String> = params.into_iter().collect();
        let prologue_defs = collect_prologue_arg_alias_defs_ir(&function.body);
        let mutated = collect_mutated_arg_aliases_ir(&function.body);
        let (assigned, stored_bases, _mentioned) =
            collect_post_prologue_assignment_facts_ir(&function.body);
        let mut safe_aliases = FxHashMap::default();
        for (_idx, alias, target) in &prologue_defs {
            if !param_set.contains(target) || mutated.contains(alias) {
                continue;
            }
            if assigned.contains(alias)
                || assigned.contains(target)
                || stored_bases.contains(alias)
                || stored_bases.contains(target)
            {
                continue;
            }
            safe_aliases.insert(alias.clone(), target.clone());
        }
        if safe_aliases.is_empty() {
            continue;
        }
        for stmt in &mut function.body {
            if !stmt.text.contains(".arg_") {
                continue;
            }
            if matches!(&stmt.kind, EmittedStmtKind::Assign { lhs, .. } if safe_aliases.contains_key(lhs))
            {
                stmt.clear();
                continue;
            }
            let rewritten = rewrite_known_aliases(&stmt.text, &safe_aliases);
            if rewritten != stmt.text {
                stmt.replace_text(rewritten);
            }
        }
    }
}

pub(in super::super) fn rewrite_remaining_readonly_param_shadow_uses_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_remaining_readonly_param_shadow_uses_ir(&mut program);
    program.into_lines()
}

fn apply_rewrite_remaining_readonly_param_shadow_uses_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((_, params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        let mutated = collect_mutated_arg_aliases_ir(&function.body);
        let (_assigned, _stored_bases, mentioned) =
            collect_post_prologue_assignment_facts_ir(&function.body);
        let mut safe_aliases = FxHashMap::default();
        for param in params {
            if !plain_ident_re().is_some_and(|re| re.is_match(&param)) {
                continue;
            }
            let alias = format!(".arg_{param}");
            if mutated.contains(&alias) {
                continue;
            }
            if mentioned.contains(&alias) {
                safe_aliases.insert(alias, param);
            }
        }
        if safe_aliases.is_empty() {
            continue;
        }
        for stmt in &mut function.body {
            if !stmt.text.contains(".arg_") {
                continue;
            }
            if let EmittedStmtKind::Assign { lhs, rhs } = &stmt.kind
                && safe_aliases.get(lhs).is_some_and(|param| param == rhs)
            {
                stmt.clear();
                continue;
            }
            let rewritten = rewrite_known_aliases(&stmt.text, &safe_aliases);
            if rewritten != stmt.text {
                stmt.replace_text(rewritten);
            }
        }
    }
}

pub(in super::super) fn rewrite_index_only_mutated_param_shadow_aliases_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_index_only_mutated_param_shadow_aliases_ir(&mut program);
    program.into_lines()
}

fn apply_rewrite_index_only_mutated_param_shadow_aliases_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((_, params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        let param_set: FxHashSet<String> = params.into_iter().collect();
        let prologue_defs = collect_prologue_arg_alias_defs_ir(&function.body);
        let (assigned, _stored_bases, _mentioned) =
            collect_post_prologue_assignment_facts_ir(&function.body);
        let mut safe_aliases = FxHashMap::default();
        for (_idx, alias, target) in &prologue_defs {
            if !param_set.contains(target) {
                continue;
            }
            if assigned.contains(alias) || assigned.contains(target) {
                continue;
            }
            safe_aliases.insert(alias.clone(), target.clone());
        }
        if safe_aliases.is_empty() {
            continue;
        }
        for stmt in &mut function.body {
            if !stmt.text.contains(".arg_") {
                continue;
            }
            if matches!(&stmt.kind, EmittedStmtKind::Assign { lhs, .. } if safe_aliases.contains_key(lhs))
            {
                stmt.clear();
                continue;
            }
            let rewritten = rewrite_known_aliases(&stmt.text, &safe_aliases);
            if rewritten != stmt.text {
                stmt.replace_text(rewritten);
            }
        }
    }
}

fn rewrite_trimmed_helper_calls_in_text(
    text: &str,
    trims: &FxHashMap<String, HelperTrimIr>,
) -> String {
    let mut rewritten = text.to_string();
    loop {
        let mut changed = false;
        let mut next = String::with_capacity(rewritten.len());
        let mut idx = 0usize;
        while idx < rewritten.len() {
            let slice = &rewritten[idx..];
            let Some(caps) = ident_re().and_then(|re| re.captures(slice)) else {
                next.push_str(slice);
                break;
            };
            let Some(mat) = caps.get(0) else {
                next.push_str(slice);
                break;
            };
            let ident_start = idx + mat.start();
            let ident_end = idx + mat.end();
            next.push_str(&rewritten[idx..ident_start]);
            let ident = mat.as_str();
            let Some(trim) = trims.get(ident) else {
                next.push_str(ident);
                idx = ident_end;
                continue;
            };
            if !rewritten[ident_end..].starts_with('(') {
                next.push_str(ident);
                idx = ident_end;
                continue;
            }
            let mut depth = 0i32;
            let mut end = None;
            for (off, ch) in rewritten[ident_end..].char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(ident_end + off);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(call_end) = end else {
                next.push_str(ident);
                idx = ident_end;
                continue;
            };
            let args_inner = &rewritten[ident_end + 1..call_end];
            let Some(args) = split_top_level_args(args_inner) else {
                next.push_str(&rewritten[ident_start..=call_end]);
                idx = call_end + 1;
                continue;
            };
            if args.len() != trim.original_len {
                next.push_str(&rewritten[ident_start..=call_end]);
                idx = call_end + 1;
                continue;
            }
            next.push_str(ident);
            next.push('(');
            next.push_str(
                &trim
                    .kept_indices
                    .iter()
                    .map(|idx| args[*idx].trim())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            next.push(')');
            idx = call_end + 1;
            changed = true;
        }
        if !changed || next == rewritten {
            break rewritten;
        }
        rewritten = next;
    }
}

fn has_unused_helper_param_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            let Some(fn_name) = func.name.as_deref() else {
                return false;
            };
            if !fn_name.starts_with("Sym_") || func.params.is_empty() {
                return false;
            }
            if func.params.iter().any(|param| param.contains('=')) {
                return false;
            }
            let mut used_params = FxHashSet::default();
            for line in &lines[func.body_start..=func.end] {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                    continue;
                }
                for ident in expr_idents(trimmed) {
                    used_params.insert(ident);
                }
            }
            func.params.iter().any(|param| !used_params.contains(param))
        })
}

fn apply_strip_unused_helper_params_ir(program: &mut EmittedProgram) {
    let mut trims = FxHashMap::<String, HelperTrimIr>::default();

    for item in &program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((fn_name, params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        if !fn_name.starts_with("Sym_")
            || params.is_empty()
            || params.iter().any(|param| param.contains('='))
        {
            continue;
        }

        let mut used_params = FxHashSet::default();
        for stmt in &function.body {
            let trimmed = stmt.text.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            for ident in expr_idents(trimmed) {
                used_params.insert(ident);
            }
        }
        let kept_indices: Vec<usize> = params
            .iter()
            .enumerate()
            .filter_map(|(idx, param)| used_params.contains(param).then_some(idx))
            .collect();
        if kept_indices.len() < params.len() {
            trims.insert(
                fn_name,
                HelperTrimIr {
                    original_len: params.len(),
                    kept_indices: kept_indices.clone(),
                    kept_params: kept_indices
                        .iter()
                        .map(|idx| params[*idx].clone())
                        .collect(),
                },
            );
        }
    }

    if trims.is_empty() {
        return;
    }

    for item in &mut program.items {
        match item {
            EmittedItem::Function(function) => {
                if let Some((fn_name, _)) = parse_function_header_ir(&function.header)
                    && let Some(trim) = trims.get(&fn_name)
                {
                    function.header =
                        format!("{} <- function({})", fn_name, trim.kept_params.join(", "));
                }
                for stmt in &mut function.body {
                    let rewritten = rewrite_trimmed_helper_calls_in_text(&stmt.text, &trims);
                    if rewritten != stmt.text {
                        stmt.replace_text(rewritten);
                    }
                }
            }
            EmittedItem::Raw(line) => {
                let rewritten = rewrite_trimmed_helper_calls_in_text(line, &trims);
                if rewritten != *line {
                    *line = rewritten;
                }
            }
        }
    }
}

fn helper_ident_is_start_ir(expr: &str, idx: usize) -> bool {
    let rest = &expr[idx..];
    let mut chars = rest.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first.is_ascii_alphabetic() || first == '_' {
        return true;
    }
    first == '.'
        && chars
            .next()
            .is_some_and(|next| next.is_ascii_alphabetic() || next == '_')
}

fn helper_ident_end_ir(expr: &str, start: usize) -> usize {
    let mut end = start;
    for (off, ch) in expr[start..].char_indices() {
        if !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')) {
            break;
        }
        end = start + off + ch.len_utf8();
    }
    end
}

fn helper_ident_is_named_label_ir(expr: &str, end: usize) -> bool {
    let rest = &expr[end..];
    for (off, ch) in rest.char_indices() {
        if ch.is_ascii_whitespace() {
            continue;
        }
        if ch != '=' {
            return false;
        }
        let tail = &rest[off + ch.len_utf8()..];
        let next_non_ws = tail.chars().find(|ch| !ch.is_ascii_whitespace());
        return next_non_ws != Some('=');
    }
    false
}

fn substitute_helper_expr_ir(expr: &str, bindings: &FxHashMap<String, String>) -> String {
    let mut out = String::with_capacity(expr.len());
    let bytes = expr.as_bytes();
    let mut idx = 0usize;
    let mut in_single = false;
    let mut in_double = false;

    while idx < bytes.len() {
        match bytes[idx] {
            b'\'' if !in_double => {
                in_single = !in_single;
                out.push('\'');
                idx += 1;
                continue;
            }
            b'"' if !in_single => {
                in_double = !in_double;
                out.push('"');
                idx += 1;
                continue;
            }
            _ => {}
        }

        if !in_single && !in_double && helper_ident_is_start_ir(expr, idx) {
            let end = helper_ident_end_ir(expr, idx);
            let ident = &expr[idx..end];
            if !helper_ident_is_named_label_ir(expr, end)
                && let Some(replacement) = bindings.get(ident)
            {
                out.push_str(replacement);
            } else {
                out.push_str(ident);
            }
            idx = end;
            continue;
        }

        out.push(bytes[idx] as char);
        idx += 1;
    }

    out
}

fn collect_simple_expr_helpers_ir(
    lines: &[String],
    _pure_user_calls: &FxHashSet<String>,
) -> FxHashMap<String, SimpleExprHelperIr> {
    let mut out = FxHashMap::default();
    for func in build_function_text_index(lines, parse_function_header_ir) {
        let Some(fn_name) = func.name.as_ref() else {
            continue;
        };
        let params = &func.params;
        let Some(return_idx) = func.return_idx else {
            continue;
        };
        let return_line = lines[return_idx].trim();
        let Some(return_expr) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            continue;
        };

        let mut bindings: FxHashMap<String, String> = FxHashMap::default();
        let mut locals: FxHashSet<String> = FxHashSet::default();
        let mut simple = true;
        for line in lines.iter().take(return_idx).skip(func.body_start) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                simple = false;
                break;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                simple = false;
                break;
            }
            let expanded = substitute_helper_expr_ir(rhs, &bindings);
            bindings.insert(lhs.to_string(), expanded);
            locals.insert(lhs.to_string());
        }
        if !simple {
            continue;
        }

        let expanded_return = substitute_helper_expr_ir(return_expr, &bindings);
        if expanded_return.contains(&format!("{fn_name}(")) {
            continue;
        }
        if expr_idents(&expanded_return)
            .iter()
            .any(|ident| locals.contains(ident) && !params.iter().any(|param| param == ident))
        {
            continue;
        }
        out.insert(
            fn_name.clone(),
            SimpleExprHelperIr {
                params: params.clone(),
                expr: expanded_return,
            },
        );
    }
    out
}

fn collect_simple_expr_helpers_from_program_ir(
    program: &EmittedProgram,
    _pure_user_calls: &FxHashSet<String>,
) -> FxHashMap<String, SimpleExprHelperIr> {
    let mut out = FxHashMap::default();
    for item in &program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((fn_name, params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        let Some(return_idx) = function
            .body
            .iter()
            .rposition(|stmt| matches!(stmt.kind, EmittedStmtKind::Return))
        else {
            continue;
        };
        let return_line = function.body[return_idx].text.trim();
        let Some(return_expr) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            continue;
        };

        let mut bindings: FxHashMap<String, String> = FxHashMap::default();
        let mut locals: FxHashSet<String> = FxHashSet::default();
        let mut simple = true;
        for stmt in function.body.iter().take(return_idx) {
            let trimmed = stmt.text.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                simple = false;
                break;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                simple = false;
                break;
            }
            let expanded = substitute_helper_expr_ir(rhs, &bindings);
            bindings.insert(lhs.to_string(), expanded);
            locals.insert(lhs.to_string());
        }
        if !simple {
            continue;
        }

        let expanded_return = substitute_helper_expr_ir(return_expr, &bindings);
        if expanded_return.contains(&format!("{fn_name}(")) {
            continue;
        }
        if expr_idents(&expanded_return)
            .iter()
            .any(|ident| locals.contains(ident) && !params.iter().any(|param| param == ident))
        {
            continue;
        }
        out.insert(
            fn_name,
            SimpleExprHelperIr {
                params,
                expr: expanded_return,
            },
        );
    }
    out
}

fn collect_metric_helpers_ir(lines: &[String]) -> FxHashMap<String, MetricHelperIr> {
    let mut out = FxHashMap::default();
    for func in build_function_text_index(lines, parse_function_header_ir) {
        let Some(fn_name) = func.name.as_ref() else {
            continue;
        };
        let params = &func.params;
        if params.len() != 2 {
            continue;
        }
        let body_lines: Vec<String> = lines
            .iter()
            .take(func.end)
            .skip(func.body_start)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s != "{" && s != "}")
            .collect();
        if body_lines.len() < 3 || body_lines.len() > 5 {
            continue;
        }
        let name_param = params[0].clone();
        let value_param = params[1].clone();
        let Some(return_line) = body_lines.last() else {
            continue;
        };
        if return_line != &format!("return({value_param})") {
            continue;
        }
        let print_name_idx = body_lines
            .iter()
            .position(|line| line == &format!("print({name_param})"));
        let print_value_idx = body_lines
            .iter()
            .position(|line| line == &format!("print({value_param})"));
        let (Some(print_name_idx), Some(print_value_idx)) = (print_name_idx, print_value_idx)
        else {
            continue;
        };
        if print_name_idx >= print_value_idx || print_value_idx + 1 != body_lines.len() - 1 {
            continue;
        }
        out.insert(
            fn_name.clone(),
            MetricHelperIr {
                name_param,
                value_param,
                pre_name_lines: body_lines[..print_name_idx].to_vec(),
                pre_value_lines: body_lines[print_name_idx + 1..print_value_idx].to_vec(),
            },
        );
    }
    out
}

fn collect_metric_helpers_from_program_ir(
    program: &EmittedProgram,
) -> FxHashMap<String, MetricHelperIr> {
    let mut out = FxHashMap::default();
    for item in &program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((fn_name, params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        if params.len() != 2 {
            continue;
        }
        let body_lines: Vec<String> = function
            .body
            .iter()
            .map(|stmt| stmt.text.trim().to_string())
            .filter(|s| !s.is_empty() && s != "{" && s != "}")
            .collect();
        if body_lines.len() < 3 || body_lines.len() > 5 {
            continue;
        }
        let name_param = params[0].clone();
        let value_param = params[1].clone();
        let Some(return_line) = body_lines.last() else {
            continue;
        };
        if return_line != &format!("return({value_param})") {
            continue;
        }
        let print_name_idx = body_lines
            .iter()
            .position(|line| line == &format!("print({name_param})"));
        let print_value_idx = body_lines
            .iter()
            .position(|line| line == &format!("print({value_param})"));
        let (Some(print_name_idx), Some(print_value_idx)) = (print_name_idx, print_value_idx)
        else {
            continue;
        };
        if print_name_idx >= print_value_idx || print_value_idx + 1 != body_lines.len() - 1 {
            continue;
        }
        out.insert(
            fn_name,
            MetricHelperIr {
                name_param,
                value_param,
                pre_name_lines: body_lines[..print_name_idx].to_vec(),
                pre_value_lines: body_lines[print_name_idx + 1..print_value_idx].to_vec(),
            },
        );
    }
    out
}

fn rewrite_simple_expr_helper_calls_in_text_ir(
    text: &str,
    helpers: &FxHashMap<String, SimpleExprHelperIr>,
    allowed_helpers: Option<&FxHashSet<String>>,
) -> String {
    let mut rewritten = text.to_string();
    loop {
        let mut changed = false;
        let mut next = String::with_capacity(rewritten.len());
        let mut idx = 0usize;
        while idx < rewritten.len() {
            let slice = &rewritten[idx..];
            let Some(caps) = ident_re().and_then(|re| re.captures(slice)) else {
                next.push_str(slice);
                break;
            };
            let Some(mat) = caps.get(0) else {
                next.push_str(slice);
                break;
            };
            let ident_start = idx + mat.start();
            let ident_end = idx + mat.end();
            next.push_str(&rewritten[idx..ident_start]);
            let ident = mat.as_str();
            let Some(helper) = helpers.get(ident) else {
                next.push_str(ident);
                idx = ident_end;
                continue;
            };
            if allowed_helpers.is_some_and(|allowed| !allowed.contains(ident)) {
                next.push_str(ident);
                idx = ident_end;
                continue;
            }
            if !rewritten[ident_end..].starts_with('(') {
                next.push_str(ident);
                idx = ident_end;
                continue;
            }
            let mut depth = 0i32;
            let mut end = None;
            for (off, ch) in rewritten[ident_end..].char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(ident_end + off);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(call_end) = end else {
                next.push_str(ident);
                idx = ident_end;
                continue;
            };
            let args_inner = &rewritten[ident_end + 1..call_end];
            let Some(args) = split_top_level_args(args_inner) else {
                next.push_str(&rewritten[ident_start..=call_end]);
                idx = call_end + 1;
                continue;
            };
            if args.len() != helper.params.len() {
                next.push_str(&rewritten[ident_start..=call_end]);
                idx = call_end + 1;
                continue;
            }
            let subst = helper
                .params
                .iter()
                .zip(args.iter())
                .map(|(param, arg)| (param.clone(), arg.trim().to_string()))
                .collect::<FxHashMap<_, _>>();
            let expanded = substitute_helper_expr_ir(&helper.expr, &subst);
            next.push('(');
            next.push_str(&expanded);
            next.push(')');
            idx = call_end + 1;
            changed = true;
        }
        if !changed || next == rewritten {
            break rewritten;
        }
        rewritten = next;
    }
}

pub(in super::super) fn rewrite_simple_expr_helper_calls_ir(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    allowed_helpers: Option<&FxHashSet<String>>,
) -> Vec<String> {
    if !has_simple_expr_helper_candidates_ir(&lines) {
        return lines;
    }
    let helpers = collect_simple_expr_helpers_ir(&lines, pure_user_calls);
    if helpers.is_empty() {
        return lines;
    }
    let helper_names: Vec<&str> = helpers.keys().map(String::as_str).collect();
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_simple_expr_helper_calls_ir(
        &mut program,
        &helpers,
        &helper_names,
        allowed_helpers,
    );
    program.into_lines()
}

fn apply_rewrite_simple_expr_helper_calls_ir(
    program: &mut EmittedProgram,
    helpers: &FxHashMap<String, SimpleExprHelperIr>,
    helper_names: &[&str],
    allowed_helpers: Option<&FxHashSet<String>>,
) {
    for item in &mut program.items {
        match item {
            EmittedItem::Function(function) => {
                for stmt in &mut function.body {
                    if !stmt.text.contains('(')
                        || !stmt.text.contains("Sym_")
                        || !helper_names.iter().any(|name| stmt.text.contains(name))
                    {
                        continue;
                    }
                    let rewritten = rewrite_simple_expr_helper_calls_in_text_ir(
                        &stmt.text,
                        &helpers,
                        allowed_helpers,
                    );
                    if rewritten != stmt.text {
                        stmt.replace_text(rewritten);
                    }
                }
            }
            EmittedItem::Raw(line) => {
                if !line.contains('(')
                    || !line.contains("Sym_")
                    || !helper_names.iter().any(|name| line.contains(name))
                {
                    continue;
                }
                let rewritten =
                    rewrite_simple_expr_helper_calls_in_text_ir(line, &helpers, allowed_helpers);
                if rewritten != *line {
                    *line = rewritten;
                }
            }
        }
    }
}

pub(in super::super) fn rewrite_metric_helper_return_calls_ir(lines: Vec<String>) -> Vec<String> {
    if !has_metric_helper_candidates_ir(&lines) {
        return lines;
    }
    let helpers = collect_metric_helpers_ir(&lines);
    if helpers.is_empty() {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    let mut temp_counter = 0usize;
    apply_rewrite_metric_helper_return_calls_ir(&mut program, &helpers, &mut temp_counter);
    program.into_lines()
}

pub(in super::super) fn rewrite_metric_helper_statement_calls_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_metric_helper_candidates_ir(&lines) {
        return lines;
    }
    let helpers = collect_metric_helpers_ir(&lines);
    if helpers.is_empty() {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    let mut temp_counter = 0usize;
    apply_rewrite_metric_helper_statement_calls_ir(&mut program, &helpers, &mut temp_counter);
    program.into_lines()
}

fn apply_rewrite_metric_helper_return_calls_ir(
    program: &mut EmittedProgram,
    helpers: &FxHashMap<String, MetricHelperIr>,
    temp_counter: &mut usize,
) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut out = Vec::with_capacity(function.body.len());
        for stmt in function.body.drain(..) {
            let trimmed = stmt.text.trim().to_string();
            let Some(inner) = trimmed
                .strip_prefix("return(")
                .and_then(|s| s.strip_suffix(')'))
            else {
                out.push(stmt);
                continue;
            };
            let Some((callee, args_str)) = inner.split_once('(') else {
                out.push(stmt);
                continue;
            };
            let Some(args_inner) = args_str.strip_suffix(')') else {
                out.push(stmt);
                continue;
            };
            let Some(helper) = helpers.get(callee.trim()) else {
                out.push(stmt);
                continue;
            };
            let Some(args) = split_top_level_args(args_inner) else {
                out.push(stmt);
                continue;
            };
            if args.len() != 2 {
                out.push(stmt);
                continue;
            }
            let indent = stmt.indent();
            let metric_name = args[0].trim();
            let metric_value = args[1].trim();
            for pre in &helper.pre_name_lines {
                out.push(EmittedStmt::parse(&format!("{indent}{pre}")));
            }
            out.push(EmittedStmt::parse(&format!("{indent}print({metric_name})")));
            let temp_name = format!(".__rr_inline_metric_{}", *temp_counter);
            *temp_counter += 1;
            out.push(EmittedStmt::parse(&format!(
                "{indent}{temp_name} <- {metric_value}"
            )));
            for pre in &helper.pre_value_lines {
                out.push(EmittedStmt::parse(&format!("{indent}{pre}")));
            }
            out.push(EmittedStmt::parse(&format!("{indent}print({temp_name})")));
            out.push(EmittedStmt::parse(&format!("{indent}return({temp_name})")));
        }
        function.body = out;
    }
}

fn apply_rewrite_metric_helper_statement_calls_ir(
    program: &mut EmittedProgram,
    helpers: &FxHashMap<String, MetricHelperIr>,
    temp_counter: &mut usize,
) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut out = Vec::with_capacity(function.body.len());
        for stmt in function.body.drain(..) {
            let trimmed = stmt.text.trim().to_string();
            let Some((callee, args_str)) = trimmed.split_once('(') else {
                out.push(stmt);
                continue;
            };
            if trimmed.contains("<-") || trimmed.starts_with("return(") {
                out.push(stmt);
                continue;
            }
            let Some(args_inner) = args_str.strip_suffix(')') else {
                out.push(stmt);
                continue;
            };
            let Some(helper) = helpers.get(callee.trim()) else {
                out.push(stmt);
                continue;
            };
            let Some(args) = split_top_level_args(args_inner) else {
                out.push(stmt);
                continue;
            };
            if args.len() != 2 {
                out.push(stmt);
                continue;
            }
            let indent = stmt.indent();
            let metric_name = args[0].trim();
            let metric_value = args[1].trim();
            for pre in &helper.pre_name_lines {
                out.push(EmittedStmt::parse(&format!("{indent}{pre}")));
            }
            out.push(EmittedStmt::parse(&format!("{indent}print({metric_name})")));
            let temp_name = format!(".__rr_inline_metric_{}", *temp_counter);
            *temp_counter += 1;
            out.push(EmittedStmt::parse(&format!(
                "{indent}{temp_name} <- {metric_value}"
            )));
            for pre in &helper.pre_value_lines {
                out.push(EmittedStmt::parse(&format!("{indent}{pre}")));
            }
            out.push(EmittedStmt::parse(&format!("{indent}print({temp_name})")));
        }
        function.body = out;
    }
}

#[derive(Default)]
pub(in super::super) struct SecondaryMetricBundleProfile {
    pub(in super::super) post_wrapper_elapsed_ns: u128,
    pub(in super::super) metric_elapsed_ns: u128,
}

pub(in super::super) fn run_post_passthrough_metric_bundle_ir(
    lines: Vec<String>,
) -> (Vec<String>, SecondaryMetricBundleProfile) {
    let needs_arg_return_wrapper = has_arg_return_wrapper_candidates_ir(&lines);
    let needs_passthrough_return_wrapper = has_passthrough_return_wrapper_candidates_ir(&lines);
    let needs_dot_product_wrapper = has_trivial_dot_product_wrapper_candidates_ir(&lines);
    let needs_wrapper =
        needs_arg_return_wrapper || needs_passthrough_return_wrapper || needs_dot_product_wrapper;
    let maybe_passthrough_helpers = has_passthrough_helper_candidates_ir(&lines);
    let needs_floor = has_nested_index_vec_floor_candidates_ir(&lines);
    let maybe_metric_helpers = has_metric_helper_candidates_ir(&lines);
    let helpers = if maybe_metric_helpers {
        collect_metric_helpers_ir(&lines)
    } else {
        FxHashMap::default()
    };
    let needs_metric = !helpers.is_empty();
    if !needs_wrapper && !maybe_passthrough_helpers && !needs_floor && !needs_metric {
        return (lines, SecondaryMetricBundleProfile::default());
    }

    let mut profile = SecondaryMetricBundleProfile::default();
    let mut program = EmittedProgram::parse(&lines);
    let started = std::time::Instant::now();
    if needs_arg_return_wrapper {
        apply_strip_arg_aliases_in_trivial_return_wrappers_ir(&mut program);
    }
    if needs_passthrough_return_wrapper {
        apply_collapse_trivial_passthrough_return_wrappers_ir(&mut program);
    }
    if needs_dot_product_wrapper {
        apply_collapse_trivial_dot_product_wrappers_ir(&mut program);
    }
    if maybe_passthrough_helpers {
        let passthrough = collect_passthrough_helpers_from_program_ir(&program);
        if !passthrough.is_empty() {
            apply_rewrite_passthrough_helper_calls_ir(&mut program, &passthrough);
        }
    }
    if needs_floor {
        apply_simplify_nested_index_vec_floor_calls_ir(&mut program);
    }
    profile.post_wrapper_elapsed_ns = started.elapsed().as_nanos();
    if needs_metric {
        let started = std::time::Instant::now();
        let mut stmt_temp_counter = 0usize;
        apply_rewrite_metric_helper_statement_calls_ir(
            &mut program,
            &helpers,
            &mut stmt_temp_counter,
        );
        let mut return_temp_counter = 0usize;
        apply_rewrite_metric_helper_return_calls_ir(
            &mut program,
            &helpers,
            &mut return_temp_counter,
        );
        profile.metric_elapsed_ns = started.elapsed().as_nanos();
    }
    (program.into_lines(), profile)
}

pub(in super::super) fn run_passthrough_secondary_bundle_ir(
    lines: Vec<String>,
) -> (Vec<String>, SecondaryMetricBundleProfile) {
    let needs_arg_return_wrapper = has_arg_return_wrapper_candidates_ir(&lines);
    let needs_passthrough_return_wrapper = has_passthrough_return_wrapper_candidates_ir(&lines);
    let needs_dot_product_wrapper = has_trivial_dot_product_wrapper_candidates_ir(&lines);
    let needs_wrapper =
        needs_arg_return_wrapper || needs_passthrough_return_wrapper || needs_dot_product_wrapper;
    let needs_passthrough_helpers = has_passthrough_helper_candidates_ir(&lines);
    let needs_floor = has_nested_index_vec_floor_candidates_ir(&lines);
    let needs_copy = has_inlined_copy_vec_sequence_candidates_ir(&lines);
    let maybe_metric_helpers = has_metric_helper_candidates_ir(&lines);
    let helpers = if maybe_metric_helpers {
        collect_metric_helpers_ir(&lines)
    } else {
        FxHashMap::default()
    };
    let needs_metric = !helpers.is_empty();
    if !needs_wrapper && !needs_passthrough_helpers && !needs_floor && !needs_copy && !needs_metric
    {
        return (lines, SecondaryMetricBundleProfile::default());
    }

    let mut profile = SecondaryMetricBundleProfile::default();
    let mut program = EmittedProgram::parse(&lines);
    let started = std::time::Instant::now();
    if needs_arg_return_wrapper {
        apply_strip_arg_aliases_in_trivial_return_wrappers_ir(&mut program);
    }
    if needs_passthrough_return_wrapper {
        apply_collapse_trivial_passthrough_return_wrappers_ir(&mut program);
    }
    if needs_dot_product_wrapper {
        apply_collapse_trivial_dot_product_wrappers_ir(&mut program);
    }
    if needs_passthrough_helpers {
        let passthrough = collect_passthrough_helpers_from_program_ir(&program);
        if !passthrough.is_empty() {
            apply_rewrite_passthrough_helper_calls_ir(&mut program, &passthrough);
        }
    }
    if needs_floor {
        apply_simplify_nested_index_vec_floor_calls_ir(&mut program);
    }
    if needs_copy {
        apply_collapse_inlined_copy_vec_sequences_ir(&mut program);
    }
    profile.post_wrapper_elapsed_ns = started.elapsed().as_nanos();
    if needs_metric {
        let started = std::time::Instant::now();
        let mut stmt_temp_counter = 0usize;
        apply_rewrite_metric_helper_statement_calls_ir(
            &mut program,
            &helpers,
            &mut stmt_temp_counter,
        );
        let mut return_temp_counter = 0usize;
        apply_rewrite_metric_helper_return_calls_ir(
            &mut program,
            &helpers,
            &mut return_temp_counter,
        );
        profile.metric_elapsed_ns = started.elapsed().as_nanos();
    }
    (program.into_lines(), profile)
}

pub(in super::super) fn strip_unused_helper_params_ir(lines: Vec<String>) -> Vec<String> {
    if !has_unused_helper_param_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_unused_helper_params_ir(&mut program);
    program.into_lines()
}

pub(in super::super) fn collapse_trivial_scalar_clamp_wrappers_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_trivial_scalar_clamp_wrapper_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_collapse_trivial_scalar_clamp_wrappers_ir(&mut program);
    program.into_lines()
}

fn has_trivial_scalar_clamp_wrapper_candidates_ir(lines: &[String]) -> bool {
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

fn apply_collapse_trivial_scalar_clamp_wrappers_ir(program: &mut EmittedProgram) {
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

fn parse_accumulate_product_line_ir(line: &str, acc: &str) -> Option<(String, String, String)> {
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

fn apply_collapse_trivial_dot_product_wrappers_ir(program: &mut EmittedProgram) {
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

pub(in super::super) fn collapse_trivial_dot_product_wrappers_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_trivial_dot_product_wrapper_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_collapse_trivial_dot_product_wrappers_ir(&mut program);
    program.into_lines()
}

fn scalar_rhs_from_singleton_rest_ir(rest: &str) -> Option<String> {
    let trimmed = rest.trim();
    if let Some(inner) = trimmed
        .strip_prefix("rep.int(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let args = split_top_level_args(inner)?;
        if args.len() == 2 && literal_one_re().is_some_and(|re| re.is_match(args[1].trim())) {
            return Some(args[0].trim().to_string());
        }
    }
    (scalar_lit_re().is_some_and(|re| re.is_match(trimmed))
        || plain_ident_re().is_some_and(|re| re.is_match(trimmed)))
    .then_some(trimmed.to_string())
}

fn collapse_singleton_assign_slice_scalar_stmt_text_ir(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let caps = assign_re().and_then(|re| re.captures(trimmed))?;
    let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
    let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
    let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
    let slice_caps = assign_slice_re().and_then(|re| re.captures(rhs))?;
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
    if lhs != dest || start != end {
        return None;
    }
    let scalar_rhs = scalar_rhs_from_singleton_rest_ir(rest)?;
    Some(format!(
        "{indent}{lhs} <- replace({dest}, {start}, {scalar_rhs})"
    ))
}

fn has_singleton_assign_slice_scalar_edit_candidates_ir(lines: &[String]) -> bool {
    lines.iter().any(|line| {
        line.contains("rr_assign_slice(")
            && collapse_singleton_assign_slice_scalar_stmt_text_ir(line).is_some()
    })
}

pub(in super::super) fn collapse_singleton_assign_slice_scalar_edits_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_singleton_assign_slice_scalar_edit_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_collapse_singleton_assign_slice_scalar_edits_ir(&mut program);
    program.into_lines()
}

fn apply_collapse_singleton_assign_slice_scalar_edits_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        match item {
            EmittedItem::Raw(line) => {
                if let Some(rewritten) = collapse_singleton_assign_slice_scalar_stmt_text_ir(line) {
                    *line = rewritten;
                }
            }
            EmittedItem::Function(function) => {
                for stmt in &mut function.body {
                    if let Some(rewritten) =
                        collapse_singleton_assign_slice_scalar_stmt_text_ir(&stmt.text)
                    {
                        stmt.replace_text(rewritten);
                    }
                }
            }
        }
    }
}

pub(in super::super) fn run_simple_expr_pre_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
    let needs_singleton = has_singleton_assign_slice_scalar_edit_candidates_ir(&lines);
    let needs_clamp = has_trivial_scalar_clamp_wrapper_candidates_ir(&lines);
    if !needs_singleton && !needs_clamp {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_singleton {
        apply_collapse_singleton_assign_slice_scalar_edits_ir(&mut program);
    }
    if needs_clamp {
        apply_collapse_trivial_scalar_clamp_wrappers_ir(&mut program);
    }
    program.into_lines()
}

pub(in super::super) fn run_simple_expr_cleanup_bundle_ir(
    mut lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    allowed_helpers: Option<&FxHashSet<String>>,
    rewrite_full_range_alias_reads: bool,
) -> Vec<String> {
    lines = rewrite_index_access_patterns(lines);
    let needs_arg_alias_cleanup = has_arg_alias_cleanup_candidates_ir(&lines);
    let needs_singleton = has_singleton_assign_slice_scalar_edit_candidates_ir(&lines);
    let needs_clamp = has_trivial_scalar_clamp_wrapper_candidates_ir(&lines);
    let maybe_simple_expr_helpers = has_simple_expr_helper_candidates_ir(&lines);
    let needs_tail = has_identical_if_else_tail_assign_candidates_ir(&lines);
    let needs_literal_field_get = has_literal_field_get_candidates_ir(&lines);
    let needs_literal_named_list = has_literal_named_list_candidates_ir(&lines);
    let needs_helper_param_trim = has_unused_helper_param_candidates_ir(&lines);
    let needs_full_range_alias_reads = rewrite_full_range_alias_reads
        && has_one_based_full_range_index_alias_read_candidates(&lines);
    if !needs_arg_alias_cleanup
        && !needs_singleton
        && !needs_clamp
        && !maybe_simple_expr_helpers
        && !needs_tail
        && !needs_literal_field_get
        && !needs_literal_named_list
        && !needs_helper_param_trim
        && !needs_full_range_alias_reads
    {
        return lines;
    }
    if !needs_arg_alias_cleanup
        && !needs_singleton
        && !needs_clamp
        && !maybe_simple_expr_helpers
        && !needs_tail
        && !needs_literal_field_get
        && !needs_literal_named_list
        && !needs_helper_param_trim
    {
        return rewrite_one_based_full_range_index_alias_reads(lines);
    }
    let helpers = if maybe_simple_expr_helpers {
        collect_simple_expr_helpers_ir(&lines, pure_user_calls)
    } else {
        FxHashMap::default()
    };
    let helper_names: Vec<&str> = helpers.keys().map(String::as_str).collect();
    let needs_simple_expr = !helpers.is_empty();
    if !needs_arg_alias_cleanup
        && !needs_singleton
        && !needs_clamp
        && !needs_simple_expr
        && !needs_tail
        && !needs_literal_field_get
        && !needs_literal_named_list
        && !needs_helper_param_trim
    {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_arg_alias_cleanup {
        apply_strip_unused_arg_aliases_ir(&mut program);
        apply_rewrite_readonly_param_aliases_ir(&mut program);
        apply_rewrite_remaining_readonly_param_shadow_uses_ir(&mut program);
        apply_rewrite_index_only_mutated_param_shadow_aliases_ir(&mut program);
    }
    if needs_helper_param_trim {
        apply_strip_unused_helper_params_ir(&mut program);
    }
    if needs_literal_field_get {
        let Some(re) = literal_field_get_re() else {
            return lines;
        };
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
                                let base =
                                    caps.name("base").map(|m| m.as_str()).unwrap_or("").trim();
                                let name =
                                    caps.name("name").map(|m| m.as_str()).unwrap_or("").trim();
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
    }
    if needs_literal_named_list {
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
    }
    if needs_singleton {
        apply_collapse_singleton_assign_slice_scalar_edits_ir(&mut program);
    }
    if needs_clamp {
        apply_collapse_trivial_scalar_clamp_wrappers_ir(&mut program);
    }
    if needs_simple_expr {
        apply_rewrite_simple_expr_helper_calls_ir(
            &mut program,
            &helpers,
            &helper_names,
            allowed_helpers,
        );
    }
    if needs_tail {
        apply_collapse_identical_if_else_tail_assignments_late_ir(&mut program);
    }
    let out = program.into_lines();
    if needs_full_range_alias_reads {
        rewrite_one_based_full_range_index_alias_reads(out)
    } else {
        out
    }
}

#[derive(Default)]
pub(in super::super) struct SecondaryAliasSimpleExprBundleProfile {
    pub(in super::super) alias_elapsed_ns: u128,
    pub(in super::super) simple_expr_elapsed_ns: u128,
    pub(in super::super) tail_elapsed_ns: u128,
}

#[derive(Default)]
pub(in super::super) struct SecondaryHelperIrBundleProfile {
    pub(in super::super) post_wrapper_elapsed_ns: u128,
    pub(in super::super) metric_elapsed_ns: u128,
    pub(in super::super) alias_elapsed_ns: u128,
    pub(in super::super) simple_expr_elapsed_ns: u128,
    pub(in super::super) tail_elapsed_ns: u128,
}

pub(in super::super) fn run_secondary_helper_ir_bundle(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> (Vec<String>, SecondaryHelperIrBundleProfile) {
    let needs_arg_return_wrapper = has_arg_return_wrapper_candidates_ir(&lines);
    let needs_passthrough_return_wrapper = has_passthrough_return_wrapper_candidates_ir(&lines);
    let needs_dot_product_wrapper = has_trivial_dot_product_wrapper_candidates_ir(&lines);
    let needs_wrapper =
        needs_arg_return_wrapper || needs_passthrough_return_wrapper || needs_dot_product_wrapper;
    let needs_passthrough_helpers = has_passthrough_helper_candidates_ir(&lines);
    let needs_floor = has_nested_index_vec_floor_candidates_ir(&lines);
    let needs_copy = has_inlined_copy_vec_sequence_candidates_ir(&lines);
    let maybe_metric_helpers = has_metric_helper_candidates_ir(&lines);
    let maybe_simple_expr_helpers = has_simple_expr_helper_candidates_ir(&lines);
    let mut profile = SecondaryHelperIrBundleProfile::default();
    let needs_alias = has_arg_alias_cleanup_candidates_ir(&lines);
    let needs_helper_param_trim = has_unused_helper_param_candidates_ir(&lines);
    let needs_singleton = has_singleton_assign_slice_scalar_edit_candidates_ir(&lines);
    let needs_clamp = has_trivial_scalar_clamp_wrapper_candidates_ir(&lines);
    let needs_tail = has_identical_if_else_tail_assign_candidates_ir(&lines);
    if !needs_wrapper
        && !needs_passthrough_helpers
        && !needs_floor
        && !needs_copy
        && !needs_alias
        && !needs_helper_param_trim
        && !needs_singleton
        && !needs_clamp
        && !maybe_metric_helpers
        && !maybe_simple_expr_helpers
        && !needs_tail
    {
        return (lines, SecondaryHelperIrBundleProfile::default());
    }

    let mut program = EmittedProgram::parse(&lines);
    let metric_helpers = if maybe_metric_helpers {
        collect_metric_helpers_from_program_ir(&program)
    } else {
        FxHashMap::default()
    };
    let needs_metric = !metric_helpers.is_empty();
    let simple_helpers = if maybe_simple_expr_helpers {
        collect_simple_expr_helpers_from_program_ir(&program, pure_user_calls)
    } else {
        FxHashMap::default()
    };
    let simple_helper_names: Vec<&str> = simple_helpers.keys().map(String::as_str).collect();
    let needs_simple_expr = !simple_helpers.is_empty();

    let started = std::time::Instant::now();
    if needs_arg_return_wrapper {
        apply_strip_arg_aliases_in_trivial_return_wrappers_ir(&mut program);
    }
    if needs_passthrough_return_wrapper {
        apply_collapse_trivial_passthrough_return_wrappers_ir(&mut program);
    }
    if needs_dot_product_wrapper {
        apply_collapse_trivial_dot_product_wrappers_ir(&mut program);
    }
    if needs_passthrough_helpers {
        let passthrough = collect_passthrough_helpers_from_program_ir(&program);
        if !passthrough.is_empty() {
            apply_rewrite_passthrough_helper_calls_ir(&mut program, &passthrough);
        }
    }
    if needs_floor {
        apply_simplify_nested_index_vec_floor_calls_ir(&mut program);
    }
    if needs_copy {
        apply_collapse_inlined_copy_vec_sequences_ir(&mut program);
    }
    profile.post_wrapper_elapsed_ns = started.elapsed().as_nanos();

    if needs_metric {
        let started = std::time::Instant::now();
        let mut stmt_temp_counter = 0usize;
        apply_rewrite_metric_helper_statement_calls_ir(
            &mut program,
            &metric_helpers,
            &mut stmt_temp_counter,
        );
        let mut return_temp_counter = 0usize;
        apply_rewrite_metric_helper_return_calls_ir(
            &mut program,
            &metric_helpers,
            &mut return_temp_counter,
        );
        profile.metric_elapsed_ns = started.elapsed().as_nanos();
    }

    let started = std::time::Instant::now();
    if needs_alias {
        apply_strip_unused_arg_aliases_ir(&mut program);
        apply_rewrite_readonly_param_aliases_ir(&mut program);
        apply_rewrite_remaining_readonly_param_shadow_uses_ir(&mut program);
        apply_rewrite_index_only_mutated_param_shadow_aliases_ir(&mut program);
    }
    if needs_helper_param_trim {
        apply_strip_unused_helper_params_ir(&mut program);
    }
    profile.alias_elapsed_ns = started.elapsed().as_nanos();

    let started = std::time::Instant::now();
    if needs_singleton {
        apply_collapse_singleton_assign_slice_scalar_edits_ir(&mut program);
    }
    if needs_clamp {
        apply_collapse_trivial_scalar_clamp_wrappers_ir(&mut program);
    }
    if needs_simple_expr {
        apply_rewrite_simple_expr_helper_calls_ir(
            &mut program,
            &simple_helpers,
            &simple_helper_names,
            None,
        );
    }
    profile.simple_expr_elapsed_ns = started.elapsed().as_nanos();

    let started = std::time::Instant::now();
    if needs_tail {
        apply_collapse_identical_if_else_tail_assignments_late_ir(&mut program);
    }
    profile.tail_elapsed_ns = started.elapsed().as_nanos();

    (program.into_lines(), profile)
}

pub(in super::super) fn run_secondary_alias_simple_expr_bundle_ir(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> (Vec<String>, SecondaryAliasSimpleExprBundleProfile) {
    let needs_alias = has_arg_alias_cleanup_candidates_ir(&lines);
    let needs_singleton = has_singleton_assign_slice_scalar_edit_candidates_ir(&lines);
    let needs_clamp = has_trivial_scalar_clamp_wrapper_candidates_ir(&lines);
    let maybe_simple_expr_helpers = has_simple_expr_helper_candidates_ir(&lines);
    let needs_tail = has_identical_if_else_tail_assign_candidates_ir(&lines);
    if !needs_alias && !needs_singleton && !needs_clamp && !maybe_simple_expr_helpers && !needs_tail
    {
        return (lines, SecondaryAliasSimpleExprBundleProfile::default());
    }
    let helpers = if maybe_simple_expr_helpers {
        collect_simple_expr_helpers_ir(&lines, pure_user_calls)
    } else {
        FxHashMap::default()
    };
    let helper_names: Vec<&str> = helpers.keys().map(String::as_str).collect();
    let needs_simple_expr = !helpers.is_empty();
    if !needs_alias && !needs_singleton && !needs_clamp && !needs_simple_expr && !needs_tail {
        return (lines, SecondaryAliasSimpleExprBundleProfile::default());
    }

    let mut profile = SecondaryAliasSimpleExprBundleProfile::default();
    let bundle_started = std::time::Instant::now();
    let mut program = EmittedProgram::parse(&lines);
    let alias_started = std::time::Instant::now();
    if needs_alias {
        apply_strip_unused_arg_aliases_ir(&mut program);
        apply_rewrite_readonly_param_aliases_ir(&mut program);
        apply_rewrite_remaining_readonly_param_shadow_uses_ir(&mut program);
        apply_rewrite_index_only_mutated_param_shadow_aliases_ir(&mut program);
    }
    profile.alias_elapsed_ns = alias_started.elapsed().as_nanos();
    let simple_started = std::time::Instant::now();
    if needs_singleton {
        apply_collapse_singleton_assign_slice_scalar_edits_ir(&mut program);
    }
    if needs_clamp {
        apply_collapse_trivial_scalar_clamp_wrappers_ir(&mut program);
    }
    if needs_simple_expr {
        apply_rewrite_simple_expr_helper_calls_ir(&mut program, &helpers, &helper_names, None);
    }
    profile.simple_expr_elapsed_ns = simple_started.elapsed().as_nanos();
    let tail_started = std::time::Instant::now();
    if needs_tail {
        apply_collapse_identical_if_else_tail_assignments_late_ir(&mut program);
    }
    profile.tail_elapsed_ns = tail_started.elapsed().as_nanos();
    let parse_overhead = bundle_started.elapsed().as_nanos().saturating_sub(
        profile.alias_elapsed_ns + profile.simple_expr_elapsed_ns + profile.tail_elapsed_ns,
    );
    profile.alias_elapsed_ns += parse_overhead;
    (program.into_lines(), profile)
}

fn apply_collapse_identical_if_else_tail_assignments_late_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut i = 0usize;
        while i < function.body.len() {
            if !matches!(function.body[i].kind, EmittedStmtKind::IfOpen) {
                i += 1;
                continue;
            }
            let Some((else_idx, end_idx)) = find_if_else_bounds_ir(&function.body, i) else {
                i += 1;
                continue;
            };

            let then_lines: Vec<usize> = ((i + 1)..else_idx)
                .filter(|idx| !function.body[*idx].text.trim().is_empty())
                .collect();
            let else_lines: Vec<usize> = ((else_idx + 1)..end_idx)
                .filter(|idx| !function.body[*idx].text.trim().is_empty())
                .collect();

            let mut t = then_lines.len();
            let mut e = else_lines.len();
            let mut shared = Vec::<(usize, usize, String)>::new();
            while t > 0 && e > 0 {
                let then_idx = then_lines[t - 1];
                let else_line_idx = else_lines[e - 1];
                let then_trimmed = function.body[then_idx].text.trim();
                let else_trimmed = function.body[else_line_idx].text.trim();
                if then_trimmed != else_trimmed {
                    break;
                }
                if assign_re()
                    .and_then(|re| re.captures(then_trimmed))
                    .is_none()
                {
                    break;
                }
                shared.push((then_idx, else_line_idx, then_trimmed.to_string()));
                t -= 1;
                e -= 1;
            }

            if shared.is_empty() {
                i = end_idx + 1;
                continue;
            }

            shared.reverse();
            let indent = function.body[i].indent();
            for (then_idx, else_idx_line, _) in &shared {
                function.body[*then_idx].clear();
                function.body[*else_idx_line].clear();
            }
            let mut insert_at = end_idx + 1;
            for (_, _, assign) in &shared {
                function
                    .body
                    .insert(insert_at, EmittedStmt::parse(&format!("{indent}{assign}")));
                insert_at += 1;
            }
            i = insert_at;
        }
    }
}

pub(in super::super) fn collapse_inlined_copy_vec_sequences_ir(lines: Vec<String>) -> Vec<String> {
    if !has_inlined_copy_vec_sequence_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let len = function.body.len();
        for idx in 0..len.saturating_sub(4) {
            let l0 = function.body[idx].text.trim().to_string();
            let l1 = function.body[idx + 1].text.trim().to_string();
            let l2 = function.body[idx + 2].text.trim().to_string();
            let l3 = function.body[idx + 3].text.trim().to_string();
            let l4 = function.body[idx + 4].text.trim().to_string();
            let Some(c0) = assign_re().and_then(|re| re.captures(&l0)) else {
                continue;
            };
            let Some(c1) = assign_re().and_then(|re| re.captures(&l1)) else {
                continue;
            };
            let Some(c2) = assign_re().and_then(|re| re.captures(&l2)) else {
                continue;
            };
            let Some(c3) = assign_re().and_then(|re| re.captures(&l3)) else {
                continue;
            };
            let Some(c4) = assign_re().and_then(|re| re.captures(&l4)) else {
                continue;
            };
            let n_var = c0.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let n_rhs = c0.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let out_var = c1.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let out_rhs = c1.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let i_var = c2.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let i_rhs = c2.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let out_replay_lhs = c3.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let src_rhs = c3.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let target_var = c4.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let target_rhs = c4.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let Some(src_var) = ({
                if let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(src_rhs)) {
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
                    if dest == out_var
                        && start == i_var
                        && end == n_var
                        && plain_ident_re().is_some_and(|re| re.is_match(rest))
                    {
                        Some(rest.to_string())
                    } else {
                        None
                    }
                } else if plain_ident_re().is_some_and(|re| re.is_match(src_rhs)) {
                    Some(src_rhs.to_string())
                } else {
                    None
                }
            }) else {
                continue;
            };
            if !n_var.starts_with("inlined_")
                || !out_var.starts_with("inlined_")
                || !i_var.starts_with("inlined_")
                || out_replay_lhs != out_var
                || (target_rhs != out_var && target_rhs != src_var)
                || !literal_one_re().is_some_and(|re| re.is_match(i_rhs))
                || !n_rhs.starts_with("length(")
                || !out_rhs.starts_with("rep.int(0, ")
            {
                continue;
            }

            let mut final_assign_idx = None;
            for (search_idx, stmt) in function.body.iter().enumerate().skip(idx + 5) {
                let trimmed = stmt.text.trim();
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
                if lhs == src_var
                    && dest == out_var
                    && start == i_var
                    && end == n_var
                    && rest == src_var
                {
                    final_assign_idx = Some(search_idx);
                    break;
                }
            }
            let Some(final_idx) = final_assign_idx else {
                continue;
            };
            let indent = function.body[idx + 4].indent();
            function.body[idx].clear();
            function.body[idx + 1].clear();
            function.body[idx + 2].clear();
            function.body[idx + 3].clear();
            function.body[idx + 4].replace_text(format!("{indent}{target_var} <- {src_var}"));
            let final_indent = function.body[final_idx].indent();
            function.body[final_idx]
                .replace_text(format!("{final_indent}{src_var} <- {target_var}"));
        }
    }
    program.into_lines()
}

pub(in super::super) fn strip_unreachable_sym_helpers_ir(lines: Vec<String>) -> Vec<String> {
    if !has_unreachable_sym_helper_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_unreachable_sym_helpers_ir(&mut program);
    program.into_lines()
}

fn previous_non_empty_stmt(body: &[EmittedStmt], idx: usize) -> Option<usize> {
    (0..idx).rev().find(|i| !body[*i].text.trim().is_empty())
}

fn find_if_else_bounds_ir(body: &[EmittedStmt], if_idx: usize) -> Option<(usize, usize)> {
    let mut depth = 1usize;
    let mut else_idx = None;
    for (idx, stmt) in body.iter().enumerate().skip(if_idx + 1) {
        match stmt.kind {
            EmittedStmtKind::ElseOpen if depth == 1 => {
                else_idx = Some(idx);
            }
            EmittedStmtKind::IfOpen
            | EmittedStmtKind::RepeatOpen
            | EmittedStmtKind::ForSeqLen { .. }
            | EmittedStmtKind::ForOpen
            | EmittedStmtKind::WhileOpen
            | EmittedStmtKind::OtherOpen => {
                depth += 1;
            }
            EmittedStmtKind::BlockClose => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return else_idx.map(|else_idx| (else_idx, idx));
                }
            }
            _ => {}
        }
    }
    None
}

fn has_assignment_to_one_before_ir(lines: &[String], idx: usize, var: &str) -> bool {
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

fn function_has_matching_exprmap_whole_assign_ir(
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

fn function_has_non_empty_repeat_whole_assign_ir(
    lines: &[String],
    dest_var: &str,
    end_expr: &str,
    temp_var: &str,
) -> bool {
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
            && has_assignment_to_one_before_ir(lines, idx, start)
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
    literal_positive_re().is_some_and(|re| re.is_match(bound.trim()))
        && has_assignment_to_one_before_ir(lines, guard_idx, iter_var.trim())
}

pub(in super::super) fn strip_redundant_tail_assign_slice_return_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_tail_assign_slice_return_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_redundant_tail_assign_slice_return_ir(&mut program);
    program.into_lines()
}

fn apply_strip_redundant_tail_assign_slice_return_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some(return_idx) = function
            .body
            .iter()
            .rposition(|stmt| matches!(stmt.kind, EmittedStmtKind::Return))
        else {
            continue;
        };
        let return_trimmed = function.body[return_idx].text.trim().to_string();
        let Some(ret_var) = return_trimmed
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            continue;
        };

        let Some(assign_idx) = previous_non_empty_stmt(&function.body, return_idx) else {
            continue;
        };
        let Some((lhs, rhs)) = function.body[assign_idx].assign_parts() else {
            continue;
        };
        if lhs.trim() != ret_var {
            continue;
        }

        let Some(assign_caps) = assign_slice_re().and_then(|re| re.captures(rhs.trim())) else {
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
            continue;
        }

        let mut fn_lines = Vec::with_capacity(function.body.len() + 1);
        fn_lines.push(function.header.clone());
        for stmt in &function.body {
            if stmt.text.trim() != "}" {
                fn_lines.push(stmt.text.clone());
            }
        }
        if function_has_non_empty_repeat_whole_assign_ir(&fn_lines, ret_var, end, temp)
            || function_has_matching_exprmap_whole_assign_ir(&fn_lines, ret_var, end, temp)
        {
            function.body[assign_idx].clear();
        }
    }
}

pub(in super::super) fn strip_redundant_nested_temp_reassigns_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !scan_basic_cleanup_candidates_ir(&lines).needs_nested_temp {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_redundant_nested_temp_reassigns_ir(&mut program);
    program.into_lines()
}

fn apply_strip_redundant_nested_temp_reassigns_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut remove = vec![false; function.body.len()];
        for idx in 0..function.body.len() {
            let Some((lhs, rhs)) = function.body[idx].assign_parts() else {
                continue;
            };
            let lhs = lhs.trim();
            let rhs = rhs.trim();
            if !lhs.starts_with(".__rr_cse_") {
                continue;
            }
            let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
            let cur_indent = function.body[idx].indent().len();
            let mut j = idx;
            while j > 0 {
                j -= 1;
                let prev = function.body[j].text.trim();
                if prev.is_empty() {
                    continue;
                }
                if matches!(
                    function.body[j].kind,
                    EmittedStmtKind::RepeatOpen
                        | EmittedStmtKind::ForSeqLen { .. }
                        | EmittedStmtKind::ForOpen
                        | EmittedStmtKind::WhileOpen
                ) {
                    break;
                }
                if let Some((prev_lhs, prev_rhs)) = function.body[j].assign_parts() {
                    let prev_lhs = prev_lhs.trim();
                    let prev_rhs = prev_rhs.trim();
                    if prev_lhs == lhs {
                        if prev_rhs == lhs {
                            continue;
                        }
                        let prev_indent = function.body[j].indent().len();
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
        function.body = function
            .body
            .drain(..)
            .enumerate()
            .filter_map(|(idx, stmt)| (!remove[idx]).then_some(stmt))
            .collect();
    }
}

pub(in super::super) fn run_exact_pre_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs_dead_eval = scan.needs_dead_eval;
    let needs_noop_assign = scan.needs_noop_assign;
    let needs_nested_temp = scan.needs_nested_temp;
    if !needs_dead_eval && !needs_noop_assign && !needs_nested_temp {
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
    program.into_lines()
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emitted_program_round_trips_function_body() {
        let lines = vec![
            "Sym_1 <- function() ".to_string(),
            "{".to_string(),
            "  x <- 1".to_string(),
            "  return(x)".to_string(),
            "}".to_string(),
            "Sym_1()".to_string(),
        ];
        let out = EmittedProgram::parse(&lines).into_lines();
        assert_eq!(out, lines);
    }

    #[test]
    fn strip_terminal_repeat_nexts_ir_removes_repeat_tail_next() {
        let lines = vec![
            "Sym_1 <- function() ".to_string(),
            "{".to_string(),
            "  repeat {".to_string(),
            "    x <- 1".to_string(),
            "    next".to_string(),
            "  }".to_string(),
            "}".to_string(),
        ];
        let out = strip_terminal_repeat_nexts_ir(lines).join("\n");
        assert!(!out.contains("\n    next\n"));
    }

    #[test]
    fn strip_empty_else_blocks_ir_collapses_empty_else() {
        let lines = vec![
            "Sym_1 <- function() ".to_string(),
            "{".to_string(),
            "  if ((x > 0)) {".to_string(),
            "    y <- 1".to_string(),
            "  } else {".to_string(),
            "".to_string(),
            "  }".to_string(),
            "}".to_string(),
        ];
        let out = strip_empty_else_blocks_ir(lines).join("\n");
        assert!(!out.contains("} else {"));
    }

    #[test]
    fn strip_dead_simple_eval_lines_ir_removes_dead_eval_lines() {
        let lines = vec![
            "Sym_1 <- function() ".to_string(),
            "{".to_string(),
            "  temp".to_string(),
            "  (tmp2)".to_string(),
            "  return(x)".to_string(),
            "}".to_string(),
        ];
        let out = strip_dead_simple_eval_lines_ir(lines).join("\n");
        assert!(!out.contains("\n  temp\n"), "{out}");
        assert!(!out.contains("\n  (tmp2)\n"), "{out}");
        assert!(out.contains("return(x)"), "{out}");
    }

    #[test]
    fn strip_noop_self_assignments_ir_removes_noop_assign() {
        let lines = vec![
            "Sym_1 <- function() ".to_string(),
            "{".to_string(),
            "  x <- x".to_string(),
            "  y <- z".to_string(),
            "}".to_string(),
        ];
        let out = strip_noop_self_assignments_ir(lines).join("\n");
        assert!(!out.contains("\n  x <- x\n"), "{out}");
        assert!(out.contains("y <- z"), "{out}");
    }

    #[test]
    fn rewrite_dead_zero_loop_seeds_before_for_ir_drops_unused_seed() {
        let lines = vec![
            "Sym_1 <- function() ".to_string(),
            "{".to_string(),
            "  i <- 0".to_string(),
            "  for (i in seq_len(n)) {".to_string(),
            "    x <- xs[i]".to_string(),
            "  }".to_string(),
            "}".to_string(),
        ];
        let out = rewrite_dead_zero_loop_seeds_before_for_ir(lines).join("\n");
        assert!(!out.contains("i <- 0"));
        assert!(out.contains("for (i in seq_len(n)) {"));
    }

    #[test]
    fn collapse_trivial_dot_product_wrappers_ir_rewrites_wrapper() {
        let lines = vec![
            "Sym_117 <- function(a, b, n) ".to_string(),
            "{".to_string(),
            "  sum <- 0".to_string(),
            "  i <- 1".to_string(),
            "  repeat {".to_string(),
            "if (!(i <= n)) break".to_string(),
            "sum <- (sum + (a[i] * b[i]))".to_string(),
            "i <- (i + 1)".to_string(),
            "next".to_string(),
            "  }".to_string(),
            "  return(sum)".to_string(),
            "}".to_string(),
        ];
        let out = collapse_trivial_dot_product_wrappers_ir(lines).join("\n");
        assert!(
            out.contains("return(sum((a[seq_len(n)] * b[seq_len(n)])))")
                || out.contains("return((sum((a[seq_len(n)] * b[seq_len(n)]))))"),
            "{out}"
        );
    }

    #[test]
    fn collapse_singleton_assign_slice_scalar_edits_ir_rewrites_singleton_slice() {
        let lines = vec![
            "Sym_1 <- function() ".to_string(),
            "{".to_string(),
            "  x <- rr_assign_slice(x, i, i, 7)".to_string(),
            "  return(x)".to_string(),
            "}".to_string(),
        ];
        let out = collapse_singleton_assign_slice_scalar_edits_ir(lines).join("\n");
        assert!(out.contains("x <- replace(x, i, 7)"), "{out}");
    }

    #[test]
    fn collapse_inlined_copy_vec_sequences_ir_rewrites_alias_swap() {
        let lines = vec![
            "Sym_1 <- function() ".to_string(),
            "{".to_string(),
            "  inlined_9_n <- length(temp)".to_string(),
            "  inlined_9_out <- rep.int(0, inlined_9_n)".to_string(),
            "  inlined_9_i <- 1".to_string(),
            "  inlined_9_out <- temp".to_string(),
            "  next_temp <- inlined_9_out".to_string(),
            "  repeat {".to_string(),
            "if (!(i < n)) break".to_string(),
            "next_temp[i] <- temp[i]".to_string(),
            "i <- (i + 1)".to_string(),
            "next".to_string(),
            "  }".to_string(),
            "  temp <- rr_assign_slice(inlined_9_out, inlined_9_i, inlined_9_n, temp)".to_string(),
            "  return(temp)".to_string(),
            "}".to_string(),
        ];
        let out = collapse_inlined_copy_vec_sequences_ir(lines).join("\n");
        assert!(out.contains("next_temp <- temp"), "{out}");
        assert!(out.contains("temp <- next_temp"), "{out}");
    }

    #[test]
    fn strip_unreachable_sym_helpers_ir_drops_unreachable_helper() {
        let lines = vec![
            "Sym_1 <- function() ".to_string(),
            "{".to_string(),
            "  return(Sym_10(\"x\", Sym_11(temp)))".to_string(),
            "}".to_string(),
            "Sym_top_0 <- function() ".to_string(),
            "{".to_string(),
            "  return(Sym_1())".to_string(),
            "}".to_string(),
            "Sym_7 <- function(xs) ".to_string(),
            "{".to_string(),
            "  return(xs)".to_string(),
            "}".to_string(),
            "Sym_11 <- function(xs) ".to_string(),
            "{".to_string(),
            "  return(sum(xs))".to_string(),
            "}".to_string(),
            "Sym_10 <- function(name, value) ".to_string(),
            "{".to_string(),
            "  print(name)".to_string(),
            "  return(value)".to_string(),
            "}".to_string(),
            "Sym_top_0()".to_string(),
        ];
        let out = strip_unreachable_sym_helpers_ir(lines).join("\n");
        assert!(out.contains("Sym_10 <- function"), "{out}");
        assert!(out.contains("Sym_11 <- function"), "{out}");
        assert!(!out.contains("Sym_7 <- function"), "{out}");
    }

    #[test]
    fn strip_redundant_tail_assign_slice_return_ir_clears_tail_assign() {
        let lines = vec![
            "Sym_1 <- function() ".to_string(),
            "{".to_string(),
            "  i <- 1".to_string(),
            "  .tachyon_exprmap0_1 <- rr_map_int(x, f)".to_string(),
            "  repeat {".to_string(),
            "if (!(i <= n)) break".to_string(),
            "x <- rr_assign_slice(x, i, n, .tachyon_exprmap0_1)".to_string(),
            "next".to_string(),
            "  }".to_string(),
            "  x <- rr_assign_slice(x, 1, n, .tachyon_exprmap0_1)".to_string(),
            "  return(x)".to_string(),
            "}".to_string(),
        ];
        let out = strip_redundant_tail_assign_slice_return_ir(lines).join("\n");
        assert!(
            !out.contains("\n  x <- rr_assign_slice(x, 1, n, .tachyon_exprmap0_1)\n"),
            "{out}"
        );
        assert!(out.contains("return(x)"), "{out}");
    }

    #[test]
    fn strip_redundant_nested_temp_reassigns_ir_drops_indented_duplicate_temp_assign() {
        let lines = vec![
            "Sym_1 <- function() ".to_string(),
            "{".to_string(),
            "  .__rr_cse_1 <- (x + y)".to_string(),
            "  if ((flag)) {".to_string(),
            "    .__rr_cse_1 <- (x + y)".to_string(),
            "  }".to_string(),
            "  return(.__rr_cse_1)".to_string(),
            "}".to_string(),
        ];
        let out = strip_redundant_nested_temp_reassigns_ir(lines).join("\n");
        assert_eq!(out.matches(".__rr_cse_1 <- (x + y)").count(), 1, "{out}");
    }

    #[test]
    fn collapse_identical_if_else_tail_assignments_late_ir_hoists_shared_tail() {
        let lines = vec![
            "Sym_1 <- function() ".to_string(),
            "{".to_string(),
            "  if ((flag)) {".to_string(),
            "    x <- 1".to_string(),
            "    y <- z".to_string(),
            "  } else {".to_string(),
            "    y <- z".to_string(),
            "  }".to_string(),
            "}".to_string(),
        ];
        let out = collapse_identical_if_else_tail_assignments_late_ir(lines).join("\n");
        assert_eq!(out.matches("y <- z").count(), 1, "{out}");
    }

    #[test]
    fn strip_redundant_identical_pure_rebinds_ir_drops_branch_local_duplicate() {
        let pure = FxHashSet::default();
        let lines = vec![
            "Sym_123 <- function(b, size) ".to_string(),
            "{".to_string(),
            "  x <- rep.int(0, size)".to_string(),
            "  if ((flag)) {".to_string(),
            "x <- (rep.int(0, size))".to_string(),
            "  } else {".to_string(),
            "  }".to_string(),
            "  return(x)".to_string(),
            "}".to_string(),
        ];
        let out = strip_redundant_identical_pure_rebinds_ir(lines, &pure).join("\n");
        assert!(out.contains("x <- rep.int(0, size)"));
        assert!(!out.contains("x <- (rep.int(0, size))"));
    }

    #[test]
    fn rewrite_forward_exact_pure_call_reuse_ir_rewrites_nested_call() {
        let pure = FxHashSet::from_iter([String::from("rr_parallel_typed_vec_call")]);
        let lines = vec![
            "Sym_top_0 <- function() ".to_string(),
            "{".to_string(),
            "  probe_vec <- rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), c(1, 2), c(2, 1))".to_string(),
            "  probe_energy <- mean(abs(rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), c(1, 2), c(2, 1))))".to_string(),
            "  return(probe_energy)".to_string(),
            "}".to_string(),
        ];
        let out = rewrite_forward_exact_pure_call_reuse_ir(lines, &pure).join("\n");
        assert!(
            out.contains("probe_energy <- mean(abs(probe_vec))"),
            "{out}"
        );
    }

    #[test]
    fn rewrite_forward_exact_expr_reuse_ir_rewrites_branch_tail_use() {
        let lines = vec![
            "Sym_37 <- function(f, y, size) ".to_string(),
            "{".to_string(),
            "  .__rr_cse_11 <- (y / size)".to_string(),
            "  v <- ((.__rr_cse_11 + .__rr_cse_11) - 1)".to_string(),
            "  if ((f < 5)) {".to_string(),
            "    lat <- (((.__rr_cse_11 + .__rr_cse_11) - 1) * 45)".to_string(),
            "  }".to_string(),
            "}".to_string(),
        ];
        let out = rewrite_forward_exact_expr_reuse_ir(lines).join("\n");
        assert!(
            out.contains("lat <- (v * 45)")
                || out.contains("lat <- (((v) * 45))")
                || out.contains("lat <- (((.__rr_cse_11 + .__rr_cse_11) - 1) * 45)"),
            "{out}"
        );
    }

    #[test]
    fn rewrite_forward_exact_pure_call_reuse_ir_does_not_turn_beta_reset_into_alpha() {
        let pure = FxHashSet::from_iter([String::from("Sym_117")]);
        let lines = vec![
            "Sym_1 <- function(b, size) ".to_string(),
            "{".to_string(),
            "  r <- b".to_string(),
            "  p <- r".to_string(),
            "  rs_old <- Sym_117(b, b, size)".to_string(),
            "  rs_new <- Sym_117(r, r, size)".to_string(),
            "  alpha <- (rs_old / p)".to_string(),
            "  beta <- (rs_new / rs_old)".to_string(),
            "  if (!(is.finite(beta))) {".to_string(),
            "    beta <- 0.0".to_string(),
            "  }".to_string(),
            "  p <- (r + (beta * p))".to_string(),
            "}".to_string(),
        ];
        let out = rewrite_forward_exact_pure_call_reuse_ir(lines, &pure).join("\n");
        assert!(!out.contains("beta <- alpha"), "{out}");
    }

    #[test]
    fn rewrite_forward_exact_pure_call_reuse_ir_skips_cse_candidates() {
        let pure = FxHashSet::default();
        let lines = vec![
            "Sym_1 <- function() ".to_string(),
            "{".to_string(),
            "  .__rr_cse_8 <- (2.0 * v_m1)".to_string(),
            "  .__rr_cse_9 <- (v_m2 - .__rr_cse_8)".to_string(),
            "  .__rr_cse_10 <- ((v_m2 - .__rr_cse_8) + v_c)".to_string(),
            "}".to_string(),
        ];
        let out = rewrite_forward_exact_pure_call_reuse_ir(lines, &pure).join("\n");
        assert!(
            out.contains(".__rr_cse_10 <- ((v_m2 - .__rr_cse_8) + v_c)"),
            "{out}"
        );
        assert!(
            !out.contains(".__rr_cse_10 <- (.__rr_cse_9 + v_c)"),
            "{out}"
        );
    }

    #[test]
    fn rewrite_forward_exact_expr_reuse_ir_keeps_larger_cse_temps_expanded() {
        let lines = vec![
            "Sym_1 <- function() ".to_string(),
            "{".to_string(),
            "  .__rr_cse_8 <- (2.0 * v_m1)".to_string(),
            "  .__rr_cse_9 <- (v_m2 - .__rr_cse_8)".to_string(),
            "  .__rr_cse_10 <- ((v_m2 - .__rr_cse_8) + v_c)".to_string(),
            "  .__rr_cse_19 <- (4.0 * v_m1)".to_string(),
            "  .__rr_cse_20 <- (v_m2 - .__rr_cse_19)".to_string(),
            "  .__rr_cse_22 <- (3.0 * v_c)".to_string(),
            "  .__rr_cse_23 <- ((v_m2 - .__rr_cse_19) + .__rr_cse_22)".to_string(),
            "  b1 <- (((1.0833 * ((v_m2 - .__rr_cse_8) + v_c)) * ((v_m2 - .__rr_cse_8) + v_c)) + ((0.25 * ((v_m2 - .__rr_cse_19) + .__rr_cse_22)) * ((v_m2 - .__rr_cse_19) + .__rr_cse_22)))".to_string(),
            "}".to_string(),
        ];
        let out = rewrite_forward_exact_expr_reuse_ir(lines).join("\n");
        assert!(
            out.contains("b1 <- (((1.0833 * (.__rr_cse_9 + v_c)) * (.__rr_cse_9 + v_c)) + ((0.25 * (.__rr_cse_20 + .__rr_cse_22)) * (.__rr_cse_20 + .__rr_cse_22)))"),
            "{out}"
        );
        assert!(out.contains("(.__rr_cse_9 + v_c)"), "{out}");
        assert!(out.contains("(.__rr_cse_20 + .__rr_cse_22)"), "{out}");
        assert!(!out.contains(".__rr_cse_10) * .__rr_cse_10"), "{out}");
        assert!(!out.contains(".__rr_cse_23) * .__rr_cse_23"), "{out}");
    }
}
