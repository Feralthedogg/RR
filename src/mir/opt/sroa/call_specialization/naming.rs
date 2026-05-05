use super::*;
pub(crate) fn unique_sroa_specialized_name(
    caller_name: &str,
    callee_name: &str,
    call: ValueId,
    reserved_names: &FxHashSet<String>,
) -> String {
    let base = format!(
        "{}__rr_sroa_{}__call{}",
        sanitize_symbol_segment(callee_name),
        sanitize_symbol_segment(caller_name),
        call
    );
    if !reserved_names.contains(&base) {
        return base;
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{base}_{suffix}");
        if !reserved_names.contains(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

pub(crate) fn unique_sroa_return_specialized_name(
    callee_name: &str,
    field: &str,
    reserved_names: &FxHashSet<String>,
) -> String {
    let base = format!(
        "{}__rr_sroa_ret_{}",
        sanitize_symbol_segment(callee_name),
        sanitize_symbol_segment(field)
    );
    if !reserved_names.contains(&base) {
        return base;
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{base}_{suffix}");
        if !reserved_names.contains(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

pub(crate) fn unique_sroa_return_temp_var(fn_ir: &FnIR, alias_var: &str, field: &str) -> String {
    let used = used_var_names(fn_ir);
    let base = format!(
        "{}__rr_sroa_ret_{}",
        sanitize_symbol_segment(alias_var),
        sanitize_symbol_segment(field)
    );
    if !used.contains(&base) {
        return base;
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{base}_{suffix}");
        if !used.contains(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

pub(crate) fn used_var_names(fn_ir: &FnIR) -> FxHashSet<String> {
    let mut used: FxHashSet<String> = fn_ir.params.iter().cloned().collect();
    for value in &fn_ir.values {
        if let Some(origin) = value.origin_var.as_ref() {
            used.insert(origin.clone());
        }
        if let ValueKind::Load { var } = &value.kind {
            used.insert(var.clone());
        }
    }
    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            if let Instr::Assign { dst, .. } = instr {
                used.insert(dst.clone());
            }
        }
    }
    used
}

pub(crate) fn unique_field_param_name(
    callee: &FnIR,
    param_index: usize,
    field_index: usize,
    field: &str,
    used_params: &mut FxHashSet<String>,
) -> String {
    let base = callee
        .params
        .get(param_index)
        .map(|param| sanitize_symbol_segment(param))
        .unwrap_or_else(|| format!("arg{param_index}"));
    let field = sanitize_symbol_segment(field);
    let seed = format!("{base}__rr_sroa_{field_index}_{field}");
    if used_params.insert(seed.clone()) {
        return seed;
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{seed}_{suffix}");
        if used_params.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}

pub(crate) fn sanitize_symbol_segment(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("sym");
    }
    if out
        .as_bytes()
        .first()
        .is_some_and(|first| first.is_ascii_digit())
    {
        out.insert(0, '_');
    }
    out
}

pub(crate) fn param_default_at(fn_ir: &FnIR, index: usize) -> Option<String> {
    fn_ir
        .param_default_r_exprs
        .get(index)
        .cloned()
        .unwrap_or(None)
}

pub(crate) fn param_span_at(fn_ir: &FnIR, index: usize) -> Span {
    fn_ir.param_spans.get(index).copied().unwrap_or_default()
}

pub(crate) fn param_ty_hint_at(fn_ir: &FnIR, index: usize) -> TypeState {
    fn_ir
        .param_ty_hints
        .get(index)
        .cloned()
        .unwrap_or_else(TypeState::unknown)
}

pub(crate) fn param_term_hint_at(fn_ir: &FnIR, index: usize) -> TypeTerm {
    fn_ir
        .param_term_hints
        .get(index)
        .cloned()
        .unwrap_or(TypeTerm::Any)
}

pub(crate) fn param_hint_span_at(fn_ir: &FnIR, index: usize) -> Option<Span> {
    fn_ir.param_hint_spans.get(index).copied().unwrap_or(None)
}
