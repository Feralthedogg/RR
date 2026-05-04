//! Source-loading, MIR synthesis, and emitted-R assembly helpers for the
//! compiler pipeline.
//!
//! The functions in this module prepare stable per-function jobs, lower HIR to
//! MIR, and concatenate emitted function fragments into the final artifact.
use super::*;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CachedModuleArtifact {
    pub(crate) schema: String,
    #[serde(default)]
    pub(crate) schema_version: u32,
    pub(crate) compiler_version: String,
    pub(crate) canonical_path: String,
    pub(crate) source_len: u64,
    pub(crate) source_mtime_ns: u128,
    pub(crate) public_symbols: Vec<String>,
    pub(crate) public_function_arities: Vec<(String, usize)>,
    pub(crate) emit_roots: Vec<String>,
    pub(crate) module_fingerprint: u64,
    pub(crate) symbols: Vec<(u32, String)>,
    #[serde(default)]
    pub(crate) source_metadata: Option<crate::syntax::ast::Program>,
    pub(crate) module: crate::hir::def::HirModule,
}

pub(crate) struct ModuleLoadJob {
    pub(crate) path: PathBuf,
    pub(crate) content: Option<String>,
    pub(crate) ast: Option<crate::syntax::ast::Program>,
    pub(crate) mod_id: u32,
    pub(crate) is_entry: bool,
    pub(crate) imports_preloaded: bool,
}

/// Owned lowering job for a user-declared function.
pub(crate) struct MirLowerJob {
    pub(crate) fn_name: String,
    pub(crate) is_public: bool,
    pub(crate) params: Vec<String>,
    pub(crate) var_names: FxHashMap<crate::hir::def::LocalId, String>,
    pub(crate) hir_fn: crate::hir::def::HirFn,
}

/// Synthetic lowering job used for module top-level entry shims.
pub(crate) struct TopLevelMirLowerJob {
    pub(crate) fn_name: String,
    pub(crate) hir_fn: crate::hir::def::HirFn,
}

/// One parallel emission result before the final artifact is concatenated.
pub(crate) struct EmittedFnFragment {
    pub(crate) code: String,
    pub(crate) map: Vec<MapEntry>,
    pub(crate) cache_hit: bool,
    pub(crate) optimized_code: Option<String>,
    pub(crate) optimized_map: Option<Vec<MapEntry>>,
    pub(crate) optimized_cache_hit: bool,
}

/// Cheap work estimate used by the scheduler for HIR-lowering jobs.
pub(crate) fn hir_fn_work_size(f: &crate::hir::def::HirFn) -> usize {
    let stmt_count = f.body.stmts.len();
    let local_count = f.local_names.len();
    (stmt_count * 8)
        .saturating_add(local_count)
        .saturating_add(f.params.len())
}

pub(crate) fn preserve_source_names_in_output(program: &ProgramIR, output: String) -> String {
    let output = preserve_readonly_arg_alias_names(output);
    let map = source_function_name_map(program);
    if map.is_empty() {
        output
    } else {
        replace_r_identifiers(&output, &map)
    }
}

pub(crate) fn source_function_name_map(program: &ProgramIR) -> FxHashMap<String, String> {
    let mut counts = FxHashMap::<String, usize>::default();
    for unit in &program.fns {
        if let Some(user_name) = unit.ir.as_ref().and_then(|ir| ir.user_name.as_ref()) {
            *counts.entry(user_name.clone()).or_default() += 1;
        }
    }

    let mut map = FxHashMap::default();
    for unit in &program.fns {
        let Some(fn_ir) = unit.ir.as_ref() else {
            continue;
        };
        let Some(user_name) = fn_ir.user_name.as_ref() else {
            continue;
        };
        if unit.name == *user_name
            || unit.is_top_level
            || !unit.name.starts_with("Sym_")
            || user_name_is_reserved_for_r_output(user_name)
            || counts.get(user_name).copied().unwrap_or(0) != 1
        {
            continue;
        }
        map.insert(unit.name.clone(), user_name.clone());
    }
    map
}

pub(crate) fn user_name_is_reserved_for_r_output(name: &str) -> bool {
    !is_plain_r_identifier(name)
        || is_r_reserved_word(name)
        || crate::mir::def::builtin_kind_for_name(name).is_some()
        || is_runtime_reserved_output_symbol(name)
}

pub(crate) fn is_runtime_reserved_output_symbol(name: &str) -> bool {
    name.starts_with(".phi_")
        || name.starts_with(".tachyon_")
        || name.starts_with("Sym_")
        || name.starts_with("__lambda_")
        || name.starts_with("rr_")
}

pub(crate) fn preserve_readonly_arg_alias_names(output: String) -> String {
    let mut lines: Vec<String> = output.lines().map(str::to_string).collect();
    if lines.is_empty() {
        return output;
    }

    let had_trailing_newline = output.ends_with('\n');
    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function(") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(params) = parse_r_function_header_params(&lines[fn_start]) else {
            fn_start += 1;
            continue;
        };
        let fn_end = find_r_function_segment_end(&lines, fn_start);
        let replacements = readonly_arg_alias_replacements(&lines[fn_start..fn_end], &params);
        if !replacements.is_empty() {
            for line in &mut lines[fn_start..fn_end] {
                let rewritten = replace_r_identifiers(line, &replacements);
                if is_noop_self_assignment(rewritten.trim()) {
                    line.clear();
                } else {
                    *line = rewritten;
                }
            }
        }
        fn_start = fn_end;
    }

    let mut rewritten = lines.join("\n");
    if had_trailing_newline {
        rewritten.push('\n');
    }
    rewritten
}

pub(crate) fn parse_r_function_header_params(line: &str) -> Option<Vec<String>> {
    let start = line.find("<- function(")? + "<- function(".len();
    let tail = &line[start..];
    let end = tail.find(')')?;
    let params = &tail[..end];
    let mut out = Vec::new();
    for raw in params.split(',') {
        let name = raw.split('=').next().unwrap_or_default().trim().to_string();
        if is_plain_r_identifier(&name) {
            out.push(name);
        }
    }
    Some(out)
}

pub(crate) fn find_r_function_segment_end(lines: &[String], fn_start: usize) -> usize {
    let mut idx = fn_start + 1;
    while idx < lines.len() {
        if lines[idx].contains("<- function(") {
            break;
        }
        idx += 1;
    }
    idx
}

pub(crate) fn readonly_arg_alias_replacements(
    lines: &[String],
    params: &[String],
) -> FxHashMap<String, String> {
    let mut replacements = FxHashMap::default();
    for param in params {
        let alias = format!(".arg_{param}");
        if !lines
            .iter()
            .any(|line| r_code_mentions_identifier(line, &alias))
        {
            continue;
        }
        if lines
            .iter()
            .map(|line| line.trim())
            .any(|line| arg_alias_rewrite_is_unsafe(line, &alias, param))
        {
            continue;
        }
        replacements.insert(alias, param.clone());
    }
    replacements
}

pub(crate) fn arg_alias_rewrite_is_unsafe(line: &str, alias: &str, param: &str) -> bool {
    if is_simple_arg_alias_init(line, alias, param) {
        return false;
    }
    line_writes_r_lvalue(line, alias) || line_writes_r_lvalue(line, param)
}

pub(crate) fn is_simple_arg_alias_init(line: &str, alias: &str, param: &str) -> bool {
    line == format!("{alias} <- {param}")
}

pub(crate) fn is_noop_self_assignment(line: &str) -> bool {
    let Some((lhs, rhs)) = line.split_once("<-") else {
        return false;
    };
    let lhs = lhs.trim();
    let rhs = rhs.trim();
    !lhs.is_empty() && lhs == rhs && is_plain_r_identifier(lhs)
}

pub(crate) fn line_writes_r_lvalue(line: &str, var: &str) -> bool {
    if !line.starts_with(var) {
        return false;
    }
    let rest = &line[var.len()..];
    rest.starts_with(" <-") || rest.starts_with("[") || rest.starts_with("$")
}

pub(crate) fn r_code_mentions_identifier(line: &str, ident: &str) -> bool {
    let bytes = line.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() {
        let ch = bytes[idx] as char;
        if ch == '"' || ch == '\'' || ch == '`' {
            idx = skip_quoted_r_token(line, idx, ch);
            continue;
        }
        if ch == '#' {
            break;
        }
        if is_r_identifier_start_byte(bytes[idx]) {
            let start = idx;
            idx += 1;
            while idx < bytes.len() && is_r_identifier_continue_byte(bytes[idx]) {
                idx += 1;
            }
            if &line[start..idx] == ident {
                return true;
            }
            continue;
        }
        idx += 1;
    }
    false
}

pub(crate) fn is_plain_r_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '.') {
        return false;
    }
    if first == '.'
        && let Some(second) = name.chars().nth(1)
        && second.is_ascii_digit()
    {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.')
}

pub(crate) fn is_r_reserved_word(name: &str) -> bool {
    matches!(
        name,
        "if" | "else"
            | "repeat"
            | "while"
            | "function"
            | "for"
            | "in"
            | "next"
            | "break"
            | "TRUE"
            | "FALSE"
            | "NULL"
            | "Inf"
            | "NaN"
            | "NA"
            | "NA_integer_"
            | "NA_real_"
            | "NA_complex_"
            | "NA_character_"
    )
}

pub(crate) fn replace_r_identifiers(
    input: &str,
    replacements: &FxHashMap<String, String>,
) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() {
        let ch = bytes[idx] as char;
        if ch == '"' || ch == '\'' || ch == '`' {
            let next = copy_quoted_r_token(input, idx, &mut out, ch);
            idx = next;
            continue;
        }
        if ch == '#' {
            let next = copy_r_comment(input, idx, &mut out);
            idx = next;
            continue;
        }
        if is_r_identifier_start_byte(bytes[idx]) {
            let start = idx;
            idx += 1;
            while idx < bytes.len() && is_r_identifier_continue_byte(bytes[idx]) {
                idx += 1;
            }
            let token = &input[start..idx];
            if let Some(replacement) = replacements.get(token) {
                out.push_str(replacement);
            } else {
                out.push_str(token);
            }
            continue;
        }
        out.push(ch);
        idx += 1;
    }
    out
}

pub(crate) fn copy_quoted_r_token(
    input: &str,
    start: usize,
    out: &mut String,
    quote: char,
) -> usize {
    let bytes = input.as_bytes();
    let mut idx = start;
    while idx < bytes.len() {
        let ch = bytes[idx] as char;
        out.push(ch);
        idx += 1;
        if ch == '\\' && idx < bytes.len() {
            out.push(bytes[idx] as char);
            idx += 1;
            continue;
        }
        if ch == quote {
            break;
        }
    }
    idx
}

pub(crate) fn skip_quoted_r_token(input: &str, start: usize, quote: char) -> usize {
    let bytes = input.as_bytes();
    let mut idx = start;
    while idx < bytes.len() {
        let ch = bytes[idx] as char;
        idx += 1;
        if ch == '\\' && idx < bytes.len() {
            idx += 1;
            continue;
        }
        if ch == quote {
            break;
        }
    }
    idx
}

pub(crate) fn copy_r_comment(input: &str, start: usize, out: &mut String) -> usize {
    let bytes = input.as_bytes();
    let mut idx = start;
    while idx < bytes.len() {
        let ch = bytes[idx] as char;
        out.push(ch);
        idx += 1;
        if ch == '\n' {
            break;
        }
    }
    idx
}

pub(crate) fn is_r_identifier_start_byte(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'.'
}

pub(crate) fn is_r_identifier_continue_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'.'
}

/// Collect user-defined callees that are reachable from a single MIR function.
pub(crate) fn called_user_fns(
    fn_ir: &crate::mir::def::FnIR,
    program: &ProgramIR,
) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    for value in &fn_ir.values {
        match &value.kind {
            crate::mir::def::ValueKind::Call { callee, .. } => {
                let canonical = callee.strip_suffix("_fresh").unwrap_or(callee);
                if program.contains_name(canonical) {
                    out.insert(canonical.to_string());
                }
            }
            crate::mir::def::ValueKind::Load { var } if program.contains_name(var) => {
                out.insert(var.clone());
            }
            crate::mir::def::ValueKind::RSymbol { name } if program.contains_name(name) => {
                out.insert(name.clone());
            }
            _ => {}
        }
    }
    out
}

pub(crate) fn collect_seq_len_param_end_slots_by_fn(
    program: &ProgramIR,
) -> FxHashMap<String, FxHashMap<usize, usize>> {
    fn unique_assign_source(
        fn_ir: &crate::mir::def::FnIR,
        var: &str,
    ) -> Option<crate::mir::def::ValueId> {
        let mut src: Option<crate::mir::def::ValueId> = None;
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                let crate::mir::def::Instr::Assign {
                    dst, src: value, ..
                } = instr
                else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                match src {
                    None => src = Some(*value),
                    Some(prev) if prev == *value => {}
                    Some(_) => return None,
                }
            }
        }
        src
    }

    fn resolve_load_alias_value(
        fn_ir: &crate::mir::def::FnIR,
        vid: crate::mir::def::ValueId,
    ) -> crate::mir::def::ValueId {
        let mut cur = vid;
        let mut seen = FxHashSet::default();
        while seen.insert(cur) {
            let crate::mir::def::ValueKind::Load { var } = &fn_ir.values[cur].kind else {
                break;
            };
            let Some(src) = unique_assign_source(fn_ir, var) else {
                break;
            };
            cur = src;
        }
        cur
    }

    fn seq_len_base_arg_slot(
        fn_ir: &crate::mir::def::FnIR,
        call_args: &[crate::mir::def::ValueId],
        arg: crate::mir::def::ValueId,
    ) -> Option<usize> {
        let resolved = resolve_load_alias_value(fn_ir, arg);
        let crate::mir::def::ValueKind::Call {
            callee,
            args,
            names,
        } = &fn_ir.values[resolved].kind
        else {
            return None;
        };
        if callee != "seq_len" || args.len() != 1 || names.iter().any(|name| name.is_some()) {
            return None;
        }
        let base = resolve_load_alias_value(fn_ir, args[0]);
        let mut matches = call_args
            .iter()
            .enumerate()
            .filter_map(|(slot, candidate)| {
                (resolve_load_alias_value(fn_ir, *candidate) == base).then_some(slot)
            });
        let slot = matches.next()?;
        matches.next().is_none().then_some(slot)
    }

    let mut summaries: FxHashMap<String, FxHashMap<usize, usize>> = FxHashMap::default();

    for slot in program.all_slots() {
        let Some(fn_ir) = program.get_slot(slot) else {
            continue;
        };
        for value in &fn_ir.values {
            let crate::mir::def::ValueKind::Call {
                callee,
                args,
                names,
            } = &value.kind
            else {
                continue;
            };
            if names.iter().any(|name| name.is_some()) {
                continue;
            }
            let canonical = callee.strip_suffix("_fresh").unwrap_or(callee.as_str());
            let Some(callee_ir) = program.get(canonical) else {
                continue;
            };
            if args.len() != callee_ir.params.len() {
                continue;
            }
            let local: FxHashMap<usize, usize> = args
                .iter()
                .enumerate()
                .filter_map(|(slot, arg)| {
                    seq_len_base_arg_slot(fn_ir, args, *arg).map(|end_slot| (slot, end_slot))
                })
                .collect();
            let entry = summaries
                .entry(canonical.to_string())
                .or_insert(local.clone());
            if entry != &local {
                entry.retain(|slot, end_slot| local.get(slot) == Some(end_slot));
            }
        }
    }

    summaries.retain(|_, slots| !slots.is_empty());
    summaries
}

pub(crate) fn reachable_emit_order_slots(program: &ProgramIR) -> Vec<FnSlot> {
    if program.emit_roots.is_empty() {
        return program.emit_order.clone();
    }

    let mut reachable = FxHashSet::default();
    let mut worklist: Vec<String> = program
        .emit_root_names()
        .iter()
        .filter(|name| program.contains_name(name.as_str()))
        .cloned()
        .collect();

    while let Some(name) = worklist.pop() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        let Some(fn_ir) = program.get(&name) else {
            continue;
        };
        for callee in called_user_fns(fn_ir, program) {
            if !reachable.contains(&callee) {
                worklist.push(callee);
            }
        }
    }

    if reachable.iter().all(|name| name.starts_with("Sym_top_")) {
        return program.emit_order.clone();
    }

    program
        .emit_order
        .iter()
        .copied()
        .filter(|slot| {
            program
                .fns
                .get(*slot)
                .is_some_and(|unit| reachable.contains(unit.name.as_str()))
        })
        .collect()
}

#[path = "source_emit/cached_emit.rs"]
pub(crate) mod cached_emit;
#[path = "source_emit/mir_synthesis.rs"]
pub(crate) mod mir_synthesis;
#[path = "source_emit/module_artifacts.rs"]
pub(crate) mod module_artifacts;
#[path = "source_emit/raw_emit.rs"]
pub(crate) mod raw_emit;
#[path = "source_emit/source_analysis.rs"]
pub(crate) mod source_analysis;

pub(crate) use cached_emit::{EmitFunctionsRequest, emit_r_functions_cached};
pub(crate) use mir_synthesis::run_mir_synthesis;
pub(crate) use source_analysis::run_source_analysis_and_canonicalization;
