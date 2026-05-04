use super::*;

#[derive(Debug, Clone)]
pub(crate) struct EmittedProgram {
    pub(crate) items: Vec<EmittedItem>,
}

#[derive(Debug, Clone)]
pub(crate) enum EmittedItem {
    Raw(String),
    Function(EmittedFunction),
}

#[derive(Debug, Clone)]
pub(crate) struct EmittedFunction {
    pub(crate) header: String,
    pub(crate) body: Vec<EmittedStmt>,
}

#[derive(Debug, Clone)]
pub(crate) struct EmittedStmt {
    pub(crate) text: String,
    pub(crate) kind: EmittedStmtKind,
}

#[derive(Debug, Clone)]
pub(crate) enum EmittedStmtKind {
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
pub(crate) enum BlockKind {
    Repeat,
    Other,
}

impl EmittedStmt {
    pub(crate) fn parse(line: &str) -> Self {
        Self {
            text: line.to_string(),
            kind: parse_emitted_stmt_kind(line.trim()),
        }
    }

    pub(crate) fn block_close_with_indent(indent: &str) -> Self {
        Self {
            text: format!("{indent}}}"),
            kind: EmittedStmtKind::BlockClose,
        }
    }

    pub(crate) fn render(&self) -> String {
        self.text.clone()
    }

    pub(crate) fn indent(&self) -> String {
        self.text
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect()
    }

    pub(crate) fn mentions_ident(&self, ident: &str) -> bool {
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

    pub(crate) fn assign_parts(&self) -> Option<(&str, &str)> {
        match &self.kind {
            EmittedStmtKind::Assign { lhs, rhs } => Some((lhs.as_str(), rhs.as_str())),
            _ => None,
        }
    }

    pub(crate) fn replace_text(&mut self, new_text: String) {
        *self = EmittedStmt::parse(&new_text);
    }

    pub(crate) fn clear(&mut self) {
        *self = EmittedStmt::parse("");
    }
}

pub(crate) fn parse_emitted_assign_kind(trimmed: &str) -> Option<EmittedStmtKind> {
    let caps = assign_re().and_then(|re| re.captures(trimmed))?;
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
    Some(EmittedStmtKind::Assign { lhs, rhs })
}

pub(crate) fn parse_emitted_stmt_kind(trimmed: &str) -> EmittedStmtKind {
    if trimmed.is_empty() {
        EmittedStmtKind::Blank
    } else if let Some(kind) = parse_emitted_assign_kind(trimmed) {
        kind
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
    }
}

impl EmittedProgram {
    pub(crate) fn parse(lines: &[String]) -> Self {
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

    pub(crate) fn into_lines(self) -> Vec<String> {
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

pub(crate) fn parse_for_seq_len_header(trimmed: &str) -> Option<(String, String)> {
    let inner = trimmed.strip_prefix("for (")?.strip_suffix(") {")?;
    let (iter_var, rest) = inner.split_once(" in seq_len(")?;
    let end_expr = rest.strip_suffix(')')?;
    Some((iter_var.trim().to_string(), end_expr.trim().to_string()))
}

#[derive(Debug, Clone)]
pub(crate) struct HelperTrimIr {
    pub(crate) original_len: usize,
    pub(crate) kept_indices: Vec<usize>,
    pub(crate) kept_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct SimpleExprHelperIr {
    pub(crate) params: Vec<String>,
    pub(crate) expr: String,
}

#[derive(Debug, Clone)]
pub(crate) struct MetricHelperIr {
    pub(crate) name_param: String,
    pub(crate) value_param: String,
    pub(crate) pre_name_lines: Vec<String>,
    pub(crate) pre_value_lines: Vec<String>,
}

pub(crate) fn parse_function_header_ir(line: &str) -> Option<(String, Vec<String>)> {
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

pub(crate) fn collect_prologue_arg_alias_defs_ir(
    body: &[EmittedStmt],
) -> Vec<(usize, String, String)> {
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

pub(crate) fn has_arg_alias_cleanup_candidates_ir(lines: &[String]) -> bool {
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

pub(crate) fn collect_mutated_arg_aliases_ir(body: &[EmittedStmt]) -> FxHashSet<String> {
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

pub(crate) fn collect_post_prologue_assignment_facts_ir(
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

pub(crate) fn is_cse_add_chain_rhs(rhs: &str) -> bool {
    let trimmed = rhs.trim();
    let inner = trimmed
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(trimmed)
        .trim();
    inner.starts_with(".__rr_cse_") && inner.contains(" + ")
}

pub(crate) fn prefer_smaller_cse_symbol(lhs: &str, rhs: &str) -> String {
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

pub(crate) fn replace_exact_rhs_occurrence(
    stmt: &EmittedStmt,
    rhs: &str,
    lhs: &str,
) -> Option<String> {
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
