use super::*;

pub(super) fn try_resolve_whole_range_self_assign_rhs(
    this: &RBackend,
    dst: &str,
    src: usize,
    values: &[Value],
    params: &[String],
) -> Option<String> {
    let ValueKind::Call { callee, args, .. } = &values.get(src)?.kind else {
        return None;
    };
    if callee != "rr_assign_slice" || args.len() < 4 {
        return None;
    }
    let base_var = this.resolve_named_mutable_base_var(args[0], values, params)?;
    if base_var != dst {
        return None;
    }
    if !this.value_is_known_one(args[1], values) {
        return None;
    }
    if !this.value_is_full_dest_end(args[0], args[2], values, params, &mut FxHashSet::default())
        && !this.whole_dest_end_matches_known_var(dst, args[2], values, params)
    {
        return None;
    }
    Some(this.normalize_whole_range_vector_expr(
        this.resolve_bound_temp_expr(args[3], values, params, &mut FxHashSet::default()),
        args[1],
        args[2],
        values,
        params,
    ))
}

pub(super) fn try_render_constant_safe_partial_self_assign(
    this: &RBackend,
    dst: &str,
    src: usize,
    values: &[Value],
    params: &[String],
) -> Option<String> {
    let ValueKind::Call { callee, args, .. } = &values.get(src)?.kind else {
        return None;
    };
    if callee != "rr_assign_slice" || args.len() < 4 {
        return None;
    }
    let base_var = this.resolve_named_mutable_base_var(args[0], values, params)?;
    if base_var != dst {
        if std::env::var_os("RR_DEBUG_PARTIAL_SLICE").is_some() {
            eprintln!(
                "RR_DEBUG_PARTIAL_SLICE skip=base_mismatch fn={} dst={} base_var={} src={}",
                this.current_fn_name, dst, base_var, src
            );
        }
        return None;
    }
    let Some(start) = this.const_index_int_value(args[1], values) else {
        if std::env::var_os("RR_DEBUG_PARTIAL_SLICE").is_some() {
            eprintln!(
                "RR_DEBUG_PARTIAL_SLICE skip=start_nonconst fn={} dst={} start_expr={}",
                this.current_fn_name,
                dst,
                this.resolve_val(args[1], values, params, false)
            );
        }
        return None;
    };
    let Some(end) = this.const_index_int_value(args[2], values) else {
        if std::env::var_os("RR_DEBUG_PARTIAL_SLICE").is_some() {
            eprintln!(
                "RR_DEBUG_PARTIAL_SLICE skip=end_nonconst fn={} dst={} end_expr={}",
                this.current_fn_name,
                dst,
                this.resolve_val(args[2], values, params, false)
            );
        }
        return None;
    };
    if start < 1 || end < start {
        if std::env::var_os("RR_DEBUG_PARTIAL_SLICE").is_some() {
            eprintln!(
                "RR_DEBUG_PARTIAL_SLICE skip=range_invalid fn={} dst={} start={} end={}",
                this.current_fn_name, dst, start, end
            );
        }
        return None;
    }
    let Some(known_end) = this.known_full_end_bound_for_var(dst, values) else {
        if std::env::var_os("RR_DEBUG_PARTIAL_SLICE").is_some() {
            eprintln!(
                "RR_DEBUG_PARTIAL_SLICE skip=unknown_known_end fn={} dst={}",
                this.current_fn_name, dst
            );
        }
        return None;
    };
    if end > known_end {
        if std::env::var_os("RR_DEBUG_PARTIAL_SLICE").is_some() {
            eprintln!(
                "RR_DEBUG_PARTIAL_SLICE skip=end_oob fn={} dst={} end={} known_end={}",
                this.current_fn_name, dst, end, known_end
            );
        }
        return None;
    }
    if !this.rep_int_matches_slice_len(args[3], start, end, values) {
        if std::env::var_os("RR_DEBUG_PARTIAL_SLICE").is_some() {
            eprintln!(
                "RR_DEBUG_PARTIAL_SLICE skip=len_mismatch fn={} dst={} start={} end={} rhs={}",
                this.current_fn_name,
                dst,
                start,
                end,
                this.resolve_bound_temp_expr(args[3], values, params, &mut FxHashSet::default())
            );
        }
        return None;
    }
    let rhs = this.resolve_bound_temp_expr(args[3], values, params, &mut FxHashSet::default());
    if std::env::var_os("RR_DEBUG_PARTIAL_SLICE").is_some() {
        eprintln!(
            "RR_DEBUG_PARTIAL_SLICE hit fn={} stmt={} [{}:{}] rhs={}",
            this.current_fn_name, dst, start, end, rhs
        );
    }
    Some(format!("{dst}[{start}:{end}] <- {rhs}"))
}

pub(super) fn try_render_safe_idx_cube_row_slice_assign(
    this: &RBackend,
    dst: &str,
    src: usize,
    values: &[Value],
    params: &[String],
) -> Option<String> {
    let ValueKind::Call { callee, args, .. } = &values.get(src)?.kind else {
        return None;
    };
    if *callee != "rr_assign_slice" || args.len() < 4 {
        return None;
    }
    let base_var = this.resolve_named_mutable_base_var(args[0], values, params)?;
    if base_var != dst {
        return None;
    }
    let row_size_expr = this.idx_cube_row_size_expr(args[1], args[2], values, params)?;
    if !this.value_matches_known_length_expr(args[3], row_size_expr.as_str(), values, params) {
        return None;
    }
    let start_expr = this.resolve_preferred_live_expr_alias(args[1], values, params);
    let end_expr = this.resolve_preferred_live_expr_alias(args[2], values, params);
    let rhs = this.resolve_bound_temp_expr(args[3], values, params, &mut FxHashSet::default());
    Some(format!("{dst}[{start_expr}:{end_expr}] <- {rhs}"))
}

pub(super) fn try_resolve_whole_range_call_map_rhs(
    this: &RBackend,
    dst: &str,
    src: usize,
    values: &[Value],
    params: &[String],
) -> Option<String> {
    let ValueKind::Call { callee, args, .. } = &values.get(src)?.kind else {
        return None;
    };
    if *callee != "rr_call_map_slice_auto" || args.len() < 7 {
        return None;
    }
    let dest_var = this.resolve_named_mutable_base_var(args[0], values, params)?;
    if dest_var != dst || !this.value_is_known_one(args[1], values) {
        return None;
    }
    if !this.value_is_full_dest_end(args[0], args[2], values, params, &mut FxHashSet::default())
        && !this.whole_dest_end_matches_known_var(dst, args[2], values, params)
    {
        return None;
    }
    let callee_name = this.const_string_value(args[3], values)?;
    let vector_slots = this.resolve_val(args[5], values, params, false);
    let helper_cost = this.resolve_val(args[4], values, params, false);
    let rendered_args: Vec<String> = args[6..]
        .iter()
        .map(|arg| {
            this.normalize_whole_range_vector_expr(
                this.resolve_bound_temp_expr(*arg, values, params, &mut FxHashSet::default()),
                args[1],
                args[2],
                values,
                params,
            )
        })
        .collect();
    if this.direct_call_map_slots_supported(
        callee_name.as_str(),
        rendered_args.len(),
        args[5],
        values,
    ) && let Some(expr) =
        this.direct_whole_range_call_map_expr(callee_name.as_str(), &rendered_args)
    {
        return Some(expr);
    }
    Some(this.render_call_map_whole_auto_expr(
        dst,
        callee_name.as_str(),
        helper_cost.as_str(),
        vector_slots.as_str(),
        &rendered_args,
    ))
}

pub(super) fn try_resolve_whole_auto_call_map_rhs(
    this: &RBackend,
    dst: &str,
    src: usize,
    values: &[Value],
    params: &[String],
) -> Option<String> {
    let ValueKind::Call { callee, args, .. } = &values.get(src)?.kind else {
        return None;
    };
    if *callee != "rr_call_map_whole_auto" || args.len() < 5 {
        return None;
    }
    let dest_var = this.resolve_named_mutable_base_var(args[0], values, params)?;
    if dest_var != dst {
        return None;
    }
    let callee_name = this.const_string_value(args[1], values)?;
    let helper_cost = this.resolve_val(args[2], values, params, false);
    let vector_slots = this.resolve_val(args[3], values, params, false);
    let rendered_args: Vec<String> = args[4..]
        .iter()
        .map(|arg| this.resolve_bound_temp_expr(*arg, values, params, &mut FxHashSet::default()))
        .collect();
    if this.direct_call_map_slots_supported(
        callee_name.as_str(),
        rendered_args.len(),
        args[3],
        values,
    ) && args[4..].iter().all(|arg| {
        !this.value_requires_runtime_auto_profit_guard(*arg, values, &mut FxHashSet::default())
    }) && let Some(expr) =
        this.direct_whole_range_call_map_expr(callee_name.as_str(), &rendered_args)
    {
        return Some(expr);
    }
    Some(this.render_call_map_whole_auto_expr(
        dst,
        callee_name.as_str(),
        helper_cost.as_str(),
        vector_slots.as_str(),
        &rendered_args,
    ))
}

pub(super) fn try_resolve_mutated_whole_range_copy_alias(
    this: &RBackend,
    src: usize,
    values: &[Value],
    params: &[String],
) -> Option<String> {
    let ValueKind::Call { callee, args, .. } = &values.get(src)?.kind else {
        return None;
    };
    if *callee != "rr_assign_slice" || args.len() < 4 {
        return None;
    }
    let base_var = this.resolve_named_mutable_base_var(args[0], values, params)?;
    if !this.value_is_known_one(args[1], values) {
        return None;
    }
    if !this.value_is_full_dest_end(args[0], args[2], values, params, &mut FxHashSet::default())
        && !this.whole_dest_end_matches_known_var(base_var.as_str(), args[2], values, params)
    {
        return None;
    }
    let rhs = this.normalize_whole_range_vector_expr(
        this.resolve_bound_temp_expr(args[3], values, params, &mut FxHashSet::default()),
        args[1],
        args[2],
        values,
        params,
    );
    if !RBackend::is_plain_symbol_expr(rhs.as_str()) || rhs == base_var {
        return None;
    }
    this.resolve_mutated_descendant_var(src)
        .filter(|var| var != &base_var && var != &rhs)
}

pub(super) fn resolve_bound_temp_expr(
    this: &RBackend,
    val_id: usize,
    values: &[Value],
    params: &[String],
    seen: &mut FxHashSet<usize>,
) -> String {
    if !seen.insert(val_id) {
        return this.resolve_val(val_id, values, params, false);
    }
    if let Some(ValueKind::Load { var }) = values.get(val_id).map(|v| &v.kind)
        && let Some(alias) = this.resolve_readonly_arg_alias_name(var, values)
    {
        return alias;
    }
    if let Some(ValueKind::Load { var }) = values.get(val_id).map(|v| &v.kind)
        && var.starts_with('.')
        && let Some(bound) = this.resolve_bound_value_id(var)
        && bound != val_id
    {
        return resolve_bound_temp_expr(this, bound, values, params, seen);
    }
    if let Some(ValueKind::Load { var }) = values.get(val_id).map(|v| &v.kind)
        && let Some(stripped) = var.strip_prefix(".arg_")
        && this.current_var_version(var) <= 1
        && !stripped.is_empty()
    {
        return stripped.to_string();
    }
    if let Some(bound) = this.resolve_bound_value(val_id)
        && !bound.starts_with('.')
    {
        return bound;
    }
    this.rewrite_live_readonly_arg_aliases(this.resolve_val(val_id, values, params, true), values)
}

pub(super) fn resolve_named_mutable_base_var(
    this: &RBackend,
    val_id: usize,
    values: &[Value],
    params: &[String],
) -> Option<String> {
    if let Some(var) = RBackend::named_mutable_base_expr(
        val_id,
        values,
        &this.value_tracker.value_bindings,
        &this.value_tracker.var_versions,
    ) {
        return Some(var);
    }
    let rendered = this.resolve_mutable_base(val_id, values, params);
    is_plain_symbol_expr(rendered.as_str()).then_some(rendered)
}

pub(super) fn resolve_mutated_descendant_var(this: &RBackend, val_id: usize) -> Option<String> {
    let mut candidate: Option<String> = None;
    for (var, (bound_val_id, version)) in &this.value_tracker.var_value_bindings {
        if *bound_val_id != val_id {
            continue;
        }
        if this.current_var_version(var) <= *version {
            continue;
        }
        if candidate.is_some() {
            return None;
        }
        candidate = Some(var.clone());
    }
    candidate
}

pub(super) fn is_plain_symbol_expr(expr: &str) -> bool {
    !expr.is_empty()
        && expr
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.'))
}

pub(super) fn direct_call_map_slots_supported(
    this: &RBackend,
    callee_name: &str,
    arg_count: usize,
    vector_slots_val: usize,
    values: &[Value],
) -> bool {
    let Some(slots) = const_int_vector_values(this, vector_slots_val, values) else {
        return false;
    };
    match (callee_name, arg_count) {
        ("abs" | "log" | "sqrt", 1) => slots == [1],
        ("pmax" | "pmin", 2) => {
            !slots.is_empty()
                && slots.len() <= 2
                && slots.iter().all(|slot| matches!(*slot, 1 | 2))
                && slots.windows(2).all(|w| w[0] < w[1])
        }
        _ => false,
    }
}

pub(super) fn const_int_vector_values(
    this: &RBackend,
    val_id: usize,
    values: &[Value],
) -> Option<Vec<i64>> {
    match values.get(val_id).map(|v| &v.kind) {
        Some(ValueKind::Call { callee, args, .. }) if callee == "c" => args
            .iter()
            .map(|arg| const_int_value(this, *arg, values))
            .collect(),
        Some(ValueKind::Load { var }) => this
            .resolve_bound_value_id(var)
            .and_then(|bound| const_int_vector_values(this, bound, values)),
        _ => None,
    }
}

pub(super) fn const_int_value(this: &RBackend, val_id: usize, values: &[Value]) -> Option<i64> {
    const_int_value_impl(this, val_id, values, &mut FxHashSet::default())
}

pub(super) fn const_int_value_impl(
    this: &RBackend,
    val_id: usize,
    values: &[Value],
    seen: &mut FxHashSet<usize>,
) -> Option<i64> {
    if !seen.insert(val_id) {
        return None;
    }
    match values.get(val_id).map(|v| &v.kind) {
        Some(ValueKind::Const(Lit::Int(v))) => Some(*v),
        Some(ValueKind::Const(Lit::Float(v)))
            if v.is_finite()
                && (*v - v.trunc()).abs() < f64::EPSILON
                && *v >= i64::MIN as f64
                && *v <= i64::MAX as f64 =>
        {
            Some(*v as i64)
        }
        Some(ValueKind::Load { var }) => this
            .resolve_bound_value_id(var)
            .and_then(|bound| const_int_value_impl(this, bound, values, seen)),
        Some(ValueKind::Unary {
            op: UnaryOp::Neg,
            rhs,
        }) => const_int_value_impl(this, *rhs, values, seen).map(|v| -v),
        Some(ValueKind::Binary { op, lhs, rhs }) => {
            let lhs = const_int_value_impl(this, *lhs, values, seen)?;
            let rhs = const_int_value_impl(this, *rhs, values, seen)?;
            match op {
                BinOp::Add => Some(lhs.saturating_add(rhs)),
                BinOp::Sub => Some(lhs.saturating_sub(rhs)),
                BinOp::Mul => Some(lhs.saturating_mul(rhs)),
                BinOp::Div if rhs != 0 && lhs % rhs == 0 => Some(lhs / rhs),
                BinOp::Mod if rhs != 0 => Some(lhs % rhs),
                _ => None,
            }
        }
        _ => None,
    }
}

pub(super) fn const_index_int_value(
    this: &RBackend,
    val_id: usize,
    values: &[Value],
) -> Option<i64> {
    const_int_value(this, val_id, values)
}

pub(super) fn value_requires_runtime_auto_profit_guard(
    this: &RBackend,
    val_id: usize,
    values: &[Value],
    seen: &mut FxHashSet<usize>,
) -> bool {
    if !seen.insert(val_id) {
        return false;
    }
    match values.get(val_id).map(|v| &v.kind) {
        Some(ValueKind::Const(_))
        | Some(ValueKind::Param { .. })
        | Some(ValueKind::RSymbol { .. }) => false,
        Some(ValueKind::Phi { args }) => args
            .iter()
            .any(|(arg, _)| value_requires_runtime_auto_profit_guard(this, *arg, values, seen)),
        Some(ValueKind::Len { base })
        | Some(ValueKind::Indices { base })
        | Some(ValueKind::Unary { rhs: base, .. }) => {
            value_requires_runtime_auto_profit_guard(this, *base, values, seen)
        }
        Some(ValueKind::Range { start, end }) => {
            value_requires_runtime_auto_profit_guard(this, *start, values, seen)
                || value_requires_runtime_auto_profit_guard(this, *end, values, seen)
        }
        Some(ValueKind::Binary { lhs, rhs, .. }) => {
            value_requires_runtime_auto_profit_guard(this, *lhs, values, seen)
                || value_requires_runtime_auto_profit_guard(this, *rhs, values, seen)
        }
        Some(ValueKind::Call { callee, args, .. }) => {
            callee.starts_with("rr_")
                || args
                    .iter()
                    .any(|arg| value_requires_runtime_auto_profit_guard(this, *arg, values, seen))
        }
        Some(ValueKind::RecordLit { fields }) => fields
            .iter()
            .any(|(_, value)| value_requires_runtime_auto_profit_guard(this, *value, values, seen)),
        Some(ValueKind::FieldGet { base, .. }) => {
            value_requires_runtime_auto_profit_guard(this, *base, values, seen)
        }
        Some(ValueKind::FieldSet { base, value, .. }) => {
            value_requires_runtime_auto_profit_guard(this, *base, values, seen)
                || value_requires_runtime_auto_profit_guard(this, *value, values, seen)
        }
        Some(ValueKind::Intrinsic { args, .. }) => args
            .iter()
            .any(|arg| value_requires_runtime_auto_profit_guard(this, *arg, values, seen)),
        Some(ValueKind::Index1D { .. })
        | Some(ValueKind::Index2D { .. })
        | Some(ValueKind::Index3D { .. }) => true,
        Some(ValueKind::Load { var }) => this.resolve_bound_value_id(var).is_some_and(|bound| {
            value_requires_runtime_auto_profit_guard(this, bound, values, seen)
        }),
        None => false,
    }
}

pub(super) fn direct_whole_range_call_map_expr(
    this: &RBackend,
    callee_name: &str,
    rendered_args: &[String],
) -> Option<String> {
    let rendered_args: Vec<String> = rendered_args
        .iter()
        .map(|arg| wrap_backend_builtin_expr(this, arg))
        .collect();
    match (
        callee_name,
        rendered_args.as_slice(),
        this.analysis.direct_builtin_vector_math,
    ) {
        ("pmax", [lhs, rhs], true) => Some(format!("pmax({lhs}, {rhs})")),
        ("pmin", [lhs, rhs], true) => Some(format!("pmin({lhs}, {rhs})")),
        ("abs", [arg], true) => Some(format!("abs({arg})")),
        ("log", [arg], true) => Some(format!("log({arg})")),
        ("sqrt", [arg], true) => Some(format!("sqrt({arg})")),
        ("pmax", [lhs, rhs], false) => Some(format!("rr_intrinsic_vec_pmax_f64({lhs}, {rhs})")),
        ("pmin", [lhs, rhs], false) => Some(format!("rr_intrinsic_vec_pmin_f64({lhs}, {rhs})")),
        ("abs", [arg], false) => Some(format!("rr_intrinsic_vec_abs_f64({arg})")),
        ("log", [arg], false) => Some(format!("rr_intrinsic_vec_log_f64({arg})")),
        ("sqrt", [arg], false) => Some(format!("rr_intrinsic_vec_sqrt_f64({arg})")),
        _ => None,
    }
}

pub(super) fn render_call_map_whole_auto_expr(
    _this: &RBackend,
    dest: &str,
    callee_name: &str,
    helper_cost: &str,
    vector_slots: &str,
    rendered_args: &[String],
) -> String {
    let mut args = Vec::with_capacity(4 + rendered_args.len());
    args.push(dest.to_string());
    args.push(format!("\"{}\"", callee_name));
    args.push(helper_cost.to_string());
    args.push(vector_slots.to_string());
    args.extend(rendered_args.iter().cloned());
    format!("rr_call_map_whole_auto({})", args.join(", "))
}

pub(super) fn const_string_value(
    this: &RBackend,
    val_id: usize,
    values: &[Value],
) -> Option<String> {
    match values.get(val_id).map(|v| &v.kind) {
        Some(ValueKind::Const(Lit::Str(s))) => Some(s.clone()),
        Some(ValueKind::Load { var }) => this
            .resolve_bound_value_id(var)
            .and_then(|bound| const_string_value(this, bound, values)),
        _ => None,
    }
}

pub(super) fn normalize_whole_range_vector_expr(
    this: &RBackend,
    expr: String,
    start: usize,
    end: usize,
    values: &[Value],
    params: &[String],
) -> String {
    let mut normalized =
        this.rewrite_known_full_range_index_reads(&expr, start, end, values, params);
    if normalized.contains("rr_ifelse_strict(") && !normalized.contains("rr_index1_read_vec(") {
        normalized = normalized.replace("rr_ifelse_strict(", "ifelse(");
    }
    normalized = rewrite_known_one_based_full_range_alias_reads(this, &normalized, values, params);
    normalized
}

pub(super) fn wrap_backend_builtin_expr(this: &RBackend, expr: &str) -> String {
    if this.analysis.direct_builtin_vector_math {
        return expr.trim().to_string();
    }
    let trimmed = expr.trim();
    if let Some(inner) = trimmed
        .strip_prefix("abs(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return format!("rr_intrinsic_vec_abs_f64({inner})");
    }
    if let Some(inner) = trimmed
        .strip_prefix("log(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return format!("rr_intrinsic_vec_log_f64({inner})");
    }
    if let Some(inner) = trimmed
        .strip_prefix("sqrt(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return format!("rr_intrinsic_vec_sqrt_f64({inner})");
    }
    trimmed.to_string()
}

pub(super) fn rewrite_known_one_based_full_range_alias_reads(
    this: &RBackend,
    expr: &str,
    values: &[Value],
    params: &[String],
) -> String {
    let pattern = format!(
        r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*(?P<idx>[^\)]*)\)",
        IDENT_PATTERN
    );
    let Some(re) = compile_regex(pattern) else {
        return expr.to_string();
    };
    re.replace_all(expr, |caps: &Captures<'_>| {
        let base = caps.name("base").map(|m| m.as_str()).unwrap_or("").trim();
        let idx_expr = caps.name("idx").map(|m| m.as_str()).unwrap_or("").trim();
        let Some(end_expr) = this.known_full_end_expr_for_var(base) else {
            return caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string();
        };
        if expr_is_one_based_full_range_for_end(idx_expr, end_expr) {
            return base.to_string();
        }
        let Some(alias_name) = extract_one_based_alias_name(idx_expr) else {
            return caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string();
        };
        let is_full = this
            .resolve_temp_bound_value_id(alias_name.as_str())
            .is_some_and(|bound| {
                this.value_is_one_based_full_range_alias(
                    bound,
                    end_expr,
                    values,
                    params,
                    &mut FxHashSet::default(),
                )
            });
        if is_full {
            base.to_string()
        } else {
            caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
        }
    })
    .to_string()
}

pub(super) fn expr_is_one_based_full_range_for_end(idx_expr: &str, end_expr: &str) -> bool {
    let idx = idx_expr
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();
    let end = end_expr
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();
    ["1L", "1", "1.0", "1.0L"].iter().any(|start| {
        idx == format!("{start}:{end}") || idx == format!("rr_index_vec_floor({start}:{end})")
    })
}

pub(super) fn extract_one_based_alias_name(idx_expr: &str) -> Option<String> {
    let trimmed = idx_expr.trim();
    if let Some(re) = compile_regex(format!(r"^{}$", IDENT_PATTERN))
        && re.is_match(trimmed)
    {
        return Some(trimmed.to_string());
    }
    if let Some(inner) = trimmed
        .strip_prefix("rr_index_vec_floor(")
        .and_then(|s| s.strip_suffix(')'))
        && let Some(re) = compile_regex(format!(r"^{}$", IDENT_PATTERN))
        && re.is_match(inner.trim())
    {
        return Some(inner.trim().to_string());
    }
    None
}

pub(super) fn whole_dest_end_matches_known_var(
    this: &RBackend,
    var: &str,
    end: usize,
    values: &[Value],
    params: &[String],
) -> bool {
    let end_rendered = this.resolve_val(end, values, params, false);
    let end_canonical = this.resolve_known_full_end_expr(end, values, params);
    this.known_full_end_expr_for_var(var)
        .is_some_and(|known| known == end_rendered || end_canonical.as_deref() == Some(known))
}

pub(super) fn known_full_end_bound_for_var(
    this: &RBackend,
    var: &str,
    values: &[Value],
) -> Option<i64> {
    this.resolve_bound_value_id(var)
        .and_then(|bound| this.known_full_end_bound_for_value(bound, values))
}

pub(super) fn idx_cube_row_size_expr(
    this: &RBackend,
    start: usize,
    end: usize,
    values: &[Value],
    params: &[String],
) -> Option<String> {
    if let Some(ValueKind::Load { var }) = values.get(start).map(|v| &v.kind)
        && let Some(bound) = this.resolve_bound_value_id(var)
        && bound != start
    {
        return idx_cube_row_size_expr(this, bound, end, values, params);
    }
    if let Some(ValueKind::Load { var }) = values.get(end).map(|v| &v.kind)
        && let Some(bound) = this.resolve_bound_value_id(var)
        && bound != end
    {
        return idx_cube_row_size_expr(this, start, bound, values, params);
    }
    let ValueKind::Call {
        callee: start_callee,
        args: start_args,
        names: start_names,
    } = &values.get(start)?.kind
    else {
        return None;
    };
    let ValueKind::Call {
        callee: end_callee,
        args: end_args,
        names: end_names,
    } = &values.get(end)?.kind
    else {
        return None;
    };
    if start_callee != "rr_idx_cube_vec_i"
        || end_callee != "rr_idx_cube_vec_i"
        || start_args.len() != 4
        || end_args.len() != 4
        || start_names.iter().any(Option::is_some)
        || end_names.iter().any(Option::is_some)
    {
        return None;
    }
    let start_face = this.resolve_val(start_args[0], values, params, false);
    let end_face = this.resolve_val(end_args[0], values, params, false);
    let start_x = this.resolve_val(start_args[1], values, params, false);
    let end_x = this.resolve_val(end_args[1], values, params, false);
    let start_size = this.resolve_val(start_args[3], values, params, false);
    let end_size = this.resolve_val(end_args[3], values, params, false);
    if start_face != end_face || start_x != end_x || start_size != end_size {
        return None;
    }
    if !this.value_is_known_one(start_args[2], values) {
        return None;
    }
    let end_y = this.resolve_val(end_args[2], values, params, false);
    if end_y != start_size {
        return None;
    }
    Some(start_size)
}

pub(super) fn value_matches_known_length_expr(
    this: &RBackend,
    val_id: usize,
    target_end_expr: &str,
    values: &[Value],
    params: &[String],
) -> bool {
    if this
        .resolve_known_full_end_expr(val_id, values, params)
        .as_deref()
        == Some(target_end_expr)
    {
        return true;
    }
    match values.get(val_id).map(|v| &v.kind) {
        Some(ValueKind::Load { var }) => {
            this.resolve_bound_value_id(var).is_some_and(|bound| {
                value_matches_known_length_expr(this, bound, target_end_expr, values, params)
            }) || this.resolve_val(val_id, values, params, false) == target_end_expr
        }
        Some(ValueKind::Param { index }) => this.resolve_param(*index, params) == target_end_expr,
        Some(ValueKind::Call { args, .. }) | Some(ValueKind::Intrinsic { args, .. }) => {
            args.iter().any(|arg| {
                value_matches_known_length_expr(this, *arg, target_end_expr, values, params)
            }) || (args.iter().any(|arg| {
                this.value_can_be_allocator_scalar_arg(*arg, values)
                    && this.resolve_val(*arg, values, params, false) == target_end_expr
            }) && args
                .iter()
                .any(|arg| !this.value_can_be_allocator_scalar_arg(*arg, values)))
        }
        _ => false,
    }
}

pub(super) fn rep_int_matches_slice_len(
    this: &RBackend,
    val_id: usize,
    start: i64,
    end: i64,
    values: &[Value],
) -> bool {
    let expected = end - start + 1;
    match values.get(val_id).map(|v| &v.kind) {
        Some(ValueKind::Call { callee, args, .. }) if callee == "rep.int" && args.len() >= 2 => {
            const_index_int_value(this, args[1], values) == Some(expected)
        }
        Some(ValueKind::Load { var }) => this
            .resolve_bound_value_id(var)
            .is_some_and(|bound| rep_int_matches_slice_len(this, bound, start, end, values)),
        _ => false,
    }
}

pub(super) fn value_is_full_dest_end(
    this: &RBackend,
    base: usize,
    end: usize,
    values: &[Value],
    params: &[String],
    seen: &mut FxHashSet<usize>,
) -> bool {
    if !seen.insert(base) {
        return false;
    }
    let end_rendered = this.resolve_val(end, values, params, false);
    let end_canonical = this.resolve_known_full_end_expr(end, values, params);
    let ok = match values.get(base).map(|v| &v.kind) {
        Some(ValueKind::Call { callee, args, .. })
            if this.call_is_known_fresh_allocation(callee)
                && this
                    .fresh_allocation_len_arg_index(callee.as_str(), args, values)
                    .is_some() =>
        {
            let len_idx = this
                .fresh_allocation_len_arg_index(callee.as_str(), args, values)
                .unwrap_or(0);
            let len_rendered = this.resolve_val(args[len_idx], values, params, false);
            len_rendered == end_rendered
                || this
                    .resolve_known_full_end_expr(args[len_idx], values, params)
                    .zip(end_canonical.as_ref())
                    .is_some_and(|(lhs, rhs)| lhs == *rhs)
        }
        Some(ValueKind::Call { callee, args, .. })
            if callee == "rr_assign_slice" && !args.is_empty() =>
        {
            value_is_full_dest_end(this, args[0], end, values, params, seen)
        }
        Some(ValueKind::Load { var }) => this
            .resolve_bound_value_id(var)
            .is_some_and(|bound| value_is_full_dest_end(this, bound, end, values, params, seen)),
        Some(ValueKind::Len { base: len_base }) => {
            this.resolve_val(*len_base, values, params, false) == end_rendered
                || this
                    .resolve_known_full_end_expr(*len_base, values, params)
                    .zip(end_canonical.as_ref())
                    .is_some_and(|(lhs, rhs)| lhs == *rhs)
        }
        _ => false,
    };
    seen.remove(&base);
    ok
}
