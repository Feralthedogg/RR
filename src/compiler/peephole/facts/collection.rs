use super::{
    ArgAliasDef, ExactReuseCandidate, ExactReuseCandidateSets, FunctionFacts, FunctionLineFacts,
    assign_re, build_function_text_index, collect_mutated_arg_aliases_in_lines,
    count_unquoted_braces, expr_idents, expr_is_exact_reusable_scalar, indexed_store_base_re,
    is_control_flow_boundary, is_literal_field_read_expr, is_loop_open_boundary,
    parse_function_header, plain_ident_re,
};
use super::{FxHashMap, FxHashSet, IndexedFunction};
use regex::{Captures, Regex};

pub(crate) fn collect_function_facts(lines: &[String]) -> Vec<FunctionFacts> {
    let Some(assign_re) = assign_re() else {
        return Vec::new();
    };
    let plain_ident_re = plain_ident_re();
    build_function_text_index(lines, parse_function_header)
        .into_iter()
        .map(|function| {
            FunctionFactsBuilder::new(lines, function, assign_re, plain_ident_re).collect()
        })
        .collect()
}

pub(crate) struct FunctionFactsBuilder<'a> {
    pub(crate) lines: &'a [String],
    pub(crate) function: IndexedFunction,
    pub(crate) assign_re: &'a Regex,
    pub(crate) plain_ident_re: Option<&'a Regex>,
    pub(crate) line_facts: Vec<FunctionLineFacts>,
    pub(crate) defs: FxHashMap<String, Vec<usize>>,
    pub(crate) uses: FxHashMap<String, Vec<usize>>,
    pub(crate) helper_call_lines: Vec<usize>,
    pub(crate) param_set: FxHashSet<String>,
    pub(crate) mutated_arg_aliases: FxHashSet<String>,
    pub(crate) prologue_arg_alias_defs: Vec<ArgAliasDef>,
    pub(crate) non_prologue_assigned_idents: FxHashSet<String>,
    pub(crate) stored_bases: FxHashSet<String>,
    pub(crate) mentioned_arg_aliases: FxHashSet<String>,
    pub(crate) exact_reuse_candidates: ExactReuseCandidateSets,
    pub(crate) in_prologue_arg_aliases: bool,
    pub(crate) block_stack: Vec<bool>,
}

impl<'a> FunctionFactsBuilder<'a> {
    pub(crate) fn new(
        lines: &'a [String],
        function: IndexedFunction,
        assign_re: &'a Regex,
        plain_ident_re: Option<&'a Regex>,
    ) -> Self {
        let line_capacity = function.end.saturating_sub(function.start) + 1;
        let param_set = function.params.iter().cloned().collect();
        let mutated_arg_aliases =
            collect_mutated_arg_aliases_in_lines(lines, function.start, function.end);
        Self {
            lines,
            function,
            assign_re,
            plain_ident_re,
            line_facts: Vec::with_capacity(line_capacity),
            defs: FxHashMap::default(),
            uses: FxHashMap::default(),
            helper_call_lines: Vec::new(),
            param_set,
            mutated_arg_aliases,
            prologue_arg_alias_defs: Vec::new(),
            non_prologue_assigned_idents: FxHashSet::default(),
            stored_bases: FxHashSet::default(),
            mentioned_arg_aliases: FxHashSet::default(),
            exact_reuse_candidates: ExactReuseCandidateSets::default(),
            in_prologue_arg_aliases: true,
            block_stack: Vec::new(),
        }
    }

    pub(crate) fn collect(mut self) -> FunctionFacts {
        for line_idx in self.function.start..=self.function.end {
            let facts = self.collect_line(line_idx);
            self.line_facts.push(facts);
        }
        populate_inline_region_ends(self.lines, &self.function, &mut self.line_facts);
        FunctionFacts {
            function: self.function,
            line_facts: self.line_facts,
            defs: self.defs,
            uses: self.uses,
            helper_call_lines: self.helper_call_lines,
            param_set: self.param_set,
            mutated_arg_aliases: self.mutated_arg_aliases,
            prologue_arg_alias_defs: self.prologue_arg_alias_defs,
            non_prologue_assigned_idents: self.non_prologue_assigned_idents,
            stored_bases: self.stored_bases,
            mentioned_arg_aliases: self.mentioned_arg_aliases,
            exact_reuse_candidates: self.exact_reuse_candidates,
        }
    }

    pub(crate) fn collect_line(&mut self, line_idx: usize) -> FunctionLineFacts {
        let line = &self.lines[line_idx];
        let trimmed = line.trim();
        let indent = line.len() - line.trim_start().len();
        let mut facts = self.initial_line_facts(line_idx, indent, trimmed);
        self.record_helper_call_line(line_idx, line);

        if let Some(caps) = self.assign_re.captures(trimmed) {
            self.collect_assignment_line(line_idx, indent, trimmed, &caps, &mut facts);
        } else {
            self.collect_non_assignment_line(line_idx, trimmed, &mut facts);
        }

        self.record_indexed_store_base(trimmed);
        self.update_block_stack(trimmed);
        facts
    }

    pub(crate) fn initial_line_facts(
        &self,
        line_idx: usize,
        indent: usize,
        trimmed: &str,
    ) -> FunctionLineFacts {
        FunctionLineFacts {
            line_idx,
            indent,
            region_end: next_straight_line_region_end(self.lines, &self.function, line_idx, indent),
            inline_region_end: self.function.end + 1,
            next_non_empty_line: None,
            in_loop_body: self.block_stack.iter().any(|is_loop| *is_loop),
            is_assign: false,
            is_control_boundary: is_control_flow_boundary(trimmed),
            lhs: None,
            rhs: None,
            idents: Vec::new(),
        }
    }

    pub(crate) fn record_helper_call_line(&mut self, line_idx: usize, line: &str) {
        if line_idx > self.function.start
            && !line.contains("<- function")
            && line.contains("Sym_")
            && line.contains('(')
        {
            self.helper_call_lines.push(line_idx);
        }
    }

    pub(crate) fn collect_assignment_line(
        &mut self,
        line_idx: usize,
        indent: usize,
        trimmed: &str,
        caps: &Captures<'_>,
        facts: &mut FunctionLineFacts,
    ) {
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let idents = expr_idents(rhs);

        facts.is_assign = true;
        facts.lhs = Some(lhs.to_string());
        facts.rhs = Some(rhs.to_string());
        facts.idents = idents.clone();

        self.defs.entry(lhs.to_string()).or_default().push(line_idx);
        self.record_uses(line_idx, &idents);
        self.record_assignment_scope(line_idx, trimmed, lhs, rhs);
        self.record_exact_reuse_candidates(line_idx, indent, facts.region_end, lhs, rhs, idents);
    }

    pub(crate) fn collect_non_assignment_line(
        &mut self,
        line_idx: usize,
        trimmed: &str,
        facts: &mut FunctionLineFacts,
    ) {
        let idents = expr_idents(trimmed);
        self.record_uses(line_idx, &idents);
        facts.idents = idents;
        if line_idx >= self.function.body_start && !trimmed.is_empty() && trimmed != "{" {
            self.in_prologue_arg_aliases = false;
        }
    }

    pub(crate) fn record_uses(&mut self, line_idx: usize, idents: &[String]) {
        for ident in idents {
            self.uses.entry(ident.clone()).or_default().push(line_idx);
            if ident.starts_with(".arg_") {
                self.mentioned_arg_aliases.insert(ident.clone());
            }
        }
    }

    pub(crate) fn record_assignment_scope(
        &mut self,
        line_idx: usize,
        trimmed: &str,
        lhs: &str,
        rhs: &str,
    ) {
        let is_prologue_alias_def = line_idx >= self.function.body_start
            && self.in_prologue_arg_aliases
            && lhs.starts_with(".arg_")
            && self.plain_ident_re.is_some_and(|re| re.is_match(rhs));
        if is_prologue_alias_def {
            self.prologue_arg_alias_defs.push(ArgAliasDef {
                line_idx,
                alias: lhs.to_string(),
                target: rhs.to_string(),
            });
        } else if line_idx >= self.function.body_start && !trimmed.is_empty() && trimmed != "{" {
            self.in_prologue_arg_aliases = false;
            self.non_prologue_assigned_idents.insert(lhs.to_string());
        }
    }

    pub(crate) fn record_exact_reuse_candidates(
        &mut self,
        line_idx: usize,
        indent: usize,
        region_end: usize,
        lhs: &str,
        rhs: &str,
        idents: Vec<String>,
    ) {
        if self.assignment_can_define_reuse_candidate(line_idx, lhs) {
            let candidate = ExactReuseCandidate {
                line_idx,
                indent,
                region_end,
                lhs: lhs.to_string(),
                rhs: rhs.to_string(),
                idents,
            };
            if rhs.contains('(') {
                self.exact_reuse_candidates
                    .call_assign_candidates
                    .push(candidate.clone());
                self.exact_reuse_candidates
                    .pure_rebind_candidates
                    .push(candidate);
            } else if is_literal_field_read_expr(rhs) {
                self.exact_reuse_candidates
                    .pure_rebind_candidates
                    .push(candidate);
            }
        }

        if line_idx >= self.function.body_start
            && line_idx < self.function.end
            && self.plain_ident_re.is_some_and(|re| re.is_match(lhs))
            && expr_is_exact_reusable_scalar(rhs)
        {
            self.exact_reuse_candidates
                .exact_expr_candidates
                .push(ExactReuseCandidate {
                    line_idx,
                    indent,
                    region_end,
                    lhs: lhs.to_string(),
                    rhs: rhs.to_string(),
                    idents: expr_idents(rhs),
                });
        }
    }

    pub(crate) fn assignment_can_define_reuse_candidate(&self, line_idx: usize, lhs: &str) -> bool {
        line_idx >= self.function.body_start
            && line_idx < self.function.end
            && self.plain_ident_re.is_some_and(|re| re.is_match(lhs))
            && !lhs.starts_with(".arg_")
            && !lhs.starts_with(".__rr_cse_")
    }

    pub(crate) fn record_indexed_store_base(&mut self, trimmed: &str) {
        if let Some(caps) = indexed_store_base_re().and_then(|re| re.captures(trimmed)) {
            let base = caps.name("base").map(|m| m.as_str()).unwrap_or("").trim();
            self.stored_bases.insert(base.to_string());
        }
    }

    pub(crate) fn update_block_stack(&mut self, trimmed: &str) {
        if trimmed == "} else {" {
            let _ = self.block_stack.pop();
            self.block_stack.push(false);
            return;
        }

        let (opens, closes) = count_unquoted_braces(trimmed);
        for _ in 0..closes {
            let _ = self.block_stack.pop();
        }
        let loop_open = is_loop_open_boundary(trimmed);
        for open_idx in 0..opens {
            self.block_stack.push(loop_open && open_idx == 0);
        }
    }
}

pub(crate) fn populate_inline_region_ends(
    lines: &[String],
    function: &IndexedFunction,
    line_facts: &mut [FunctionLineFacts],
) {
    let mut next_boundary = function.end + 1;
    let mut next_non_empty_line = None;
    for fact in line_facts.iter_mut().rev() {
        fact.inline_region_end = next_boundary;
        fact.next_non_empty_line = next_non_empty_line;
        let trimmed = lines[fact.line_idx].trim();
        if !trimmed.is_empty() {
            next_non_empty_line = Some(fact.line_idx);
        }
        if !trimmed.is_empty()
            && (lines[fact.line_idx].contains("<- function")
                || fact.is_control_boundary
                || trimmed == "}")
        {
            next_boundary = fact.line_idx;
        }
    }
}

pub(crate) fn next_straight_line_region_end(
    lines: &[String],
    function: &IndexedFunction,
    start_idx: usize,
    indent: usize,
) -> usize {
    for (line_idx, line) in lines
        .iter()
        .enumerate()
        .take(function.end.saturating_add(1))
        .skip(start_idx + 1)
    {
        let trimmed = line.trim();
        let next_indent = line.len() - line.trim_start().len();
        if line.contains("<- function")
            || trimmed == "repeat {"
            || trimmed.starts_with("while")
            || trimmed.starts_with("for")
            || (!trimmed.is_empty() && next_indent < indent)
        {
            return line_idx;
        }
    }
    function.end + 1
}
