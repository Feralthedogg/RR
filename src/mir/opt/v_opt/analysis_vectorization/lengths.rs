use super::*;
pub(crate) fn is_loop_compatible_base(lp: &LoopInfo, fn_ir: &FnIR, base: ValueId) -> bool {
    if loop_matches_vec(lp, fn_ir, base) {
        return true;
    }
    let Some(loop_key) = loop_length_key(lp, fn_ir) else {
        return false;
    };
    match vector_length_key(fn_ir, base) {
        Some(k) => canonical_value(fn_ir, k) == canonical_value(fn_ir, loop_key),
        None => false,
    }
}

pub(crate) fn loop_covers_whole_destination(
    lp: &LoopInfo,
    fn_ir: &FnIR,
    base: ValueId,
    start: ValueId,
) -> bool {
    is_const_one(fn_ir, start) && loop_matches_full_base(lp, fn_ir, base)
}

pub(crate) fn loop_length_key(lp: &LoopInfo, fn_ir: &FnIR) -> Option<ValueId> {
    if lp.limit_adjust != 0 {
        return None;
    }
    let limit = lp.limit?;
    match &fn_ir.values[limit].kind {
        ValueKind::Len { base } => vector_length_key(fn_ir, *base),
        _ => Some(resolve_load_alias_value(fn_ir, limit)),
    }
}

pub(crate) fn affine_iv_offset(fn_ir: &FnIR, idx: ValueId, iv_phi: ValueId) -> Option<i64> {
    if is_iv_equivalent(fn_ir, idx, iv_phi) {
        return Some(0);
    }
    match &fn_ir.values[idx].kind {
        ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } => {
            if is_iv_equivalent(fn_ir, *lhs, iv_phi)
                && let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*rhs].kind
            {
                return Some(k);
            }
            if is_iv_equivalent(fn_ir, *rhs, iv_phi)
                && let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*lhs].kind
            {
                return Some(k);
            }
            None
        }
        ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } => {
            if is_iv_equivalent(fn_ir, *lhs, iv_phi)
                && let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*rhs].kind
            {
                return Some(-k);
            }
            None
        }
        _ => None,
    }
}

pub(crate) fn loop_matches_full_base(lp: &LoopInfo, fn_ir: &FnIR, base: ValueId) -> bool {
    if lp.limit_adjust != 0 {
        return false;
    }

    let base = canonical_value(fn_ir, base);
    if lp.is_seq_along.map(|b| canonical_value(fn_ir, b)) == Some(base) {
        return true;
    }
    if let Some(loop_base) = lp.is_seq_along
        && let (Some(a), Some(b)) = (
            resolve_base_var(fn_ir, base),
            resolve_base_var(fn_ir, loop_base),
        )
        && a == b
    {
        return true;
    }

    if let Some(limit) = lp
        .is_seq_len
        .map(|limit| resolve_load_alias_value(fn_ir, limit))
    {
        if let ValueKind::Len { base: len_base } = fn_ir.values[limit].kind {
            if canonical_value(fn_ir, len_base) == base {
                return true;
            }
            if let (Some(a), Some(b)) = (
                resolve_base_var(fn_ir, base),
                resolve_base_var(fn_ir, len_base),
            ) && a == b
            {
                return true;
            }
        }

        if base_length_key(fn_ir, base).is_some_and(|base_key| {
            canonical_value(fn_ir, base_key) == canonical_value(fn_ir, limit)
        }) {
            return true;
        }
    }

    if let (Some(base_key), Some(loop_key)) =
        (base_length_key(fn_ir, base), loop_length_key(lp, fn_ir))
    {
        return canonical_value(fn_ir, base_key) == canonical_value(fn_ir, loop_key);
    }

    false
}

pub(crate) fn loop_matches_vec(lp: &LoopInfo, fn_ir: &FnIR, base: ValueId) -> bool {
    let base = canonical_value(fn_ir, base);
    if lp.is_seq_along.map(|b| canonical_value(fn_ir, b)) == Some(base) {
        return true;
    }
    if let Some(loop_base) = lp.is_seq_along
        && let (Some(a), Some(b)) = (
            resolve_base_var(fn_ir, base),
            resolve_base_var(fn_ir, loop_base),
        )
        && a == b
    {
        return true;
    }
    if let Some(limit) = lp.is_seq_len
        && let ValueKind::Len { base: len_base } = fn_ir.values[limit].kind
    {
        if canonical_value(fn_ir, len_base) == base {
            return true;
        }
        if let (Some(a), Some(b)) = (
            resolve_base_var(fn_ir, base),
            resolve_base_var(fn_ir, len_base),
        ) && a == b
        {
            return true;
        }
    }
    false
}

pub(crate) fn vector_length_key(fn_ir: &FnIR, root: ValueId) -> Option<ValueId> {
    fn unify_length_keys(
        fn_ir: &FnIR,
        keys: impl IntoIterator<Item = Option<ValueId>>,
    ) -> Option<ValueId> {
        let mut out: Option<ValueId> = None;
        for key in keys {
            let key = key.map(|k| resolve_load_alias_value(fn_ir, k))?;
            match out {
                None => out = Some(key),
                Some(prev) if resolve_load_alias_value(fn_ir, prev) == key => {}
                Some(_) => return None,
            }
        }
        out
    }

    fn rec_var(
        fn_ir: &FnIR,
        var: &str,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<ValueId> {
        if !seen_vars.insert(var.to_string()) {
            return None;
        }
        if let Some(param_index) = fn_ir.params.iter().position(|p| p == var) {
            let param_vid = fn_ir
                .values
                .iter()
                .position(|value| matches!(value.kind, ValueKind::Param { index } if index == param_index))
                .map(|vid| vid as ValueId);
            if let Some(param_vid) = param_vid {
                return Some(resolve_load_alias_value(fn_ir, param_vid));
            }
        }

        let mut saw_assign = false;
        let mut keys = Vec::new();
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                saw_assign = true;
                keys.push(rec(fn_ir, *src, seen_vals, seen_vars));
            }
        }
        if !saw_assign {
            return None;
        }
        unify_length_keys(fn_ir, keys)
    }

    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<ValueId> {
        let root = canonical_value(fn_ir, root);
        if !seen_vals.insert(root) {
            return None;
        }
        if matches!(fn_ir.values[root].kind, ValueKind::Phi { .. })
            && let Some(var) = fn_ir.values[root].origin_var.as_deref()
            && let Some(key) = rec_var(fn_ir, var, seen_vals, seen_vars)
        {
            seen_vals.remove(&root);
            return Some(key);
        }
        let out = match &fn_ir.values[root].kind {
            ValueKind::Load { var } => rec_var(fn_ir, var, seen_vals, seen_vars),
            ValueKind::Phi { args } => {
                let keys = args
                    .iter()
                    .filter_map(|(arg, _)| {
                        let arg = canonical_value(fn_ir, *arg);
                        if arg == root {
                            None
                        } else {
                            rec(fn_ir, arg, seen_vals, seen_vars)
                        }
                    })
                    .collect::<Vec<_>>();
                if keys.is_empty() {
                    None
                } else {
                    unify_length_keys(fn_ir, keys.into_iter().map(Some))
                }
            }
            ValueKind::Call { args, .. } => {
                let resolved = resolve_call_info(fn_ir, root);
                let callee = resolved
                    .as_ref()
                    .map(|call| call.callee.as_str())
                    .unwrap_or("rr_call_closure");
                let call_args = resolved
                    .as_ref()
                    .map(|call| call.args.as_slice())
                    .unwrap_or(args.as_slice());
                if callee == "seq_len" && call_args.len() == 1 {
                    Some(resolve_load_alias_value(fn_ir, call_args[0]))
                } else if matches!(callee, "rep.int" | "numeric") && !call_args.is_empty() {
                    let len_arg = if callee == "rep.int" {
                        call_args
                            .get(1)
                            .copied()
                            .or_else(|| call_args.first().copied())
                    } else {
                        call_args.first().copied()
                    }?;
                    Some(resolve_load_alias_value(fn_ir, len_arg))
                } else if is_builtin_vector_safe_call(callee, call_args.len())
                    || matches!(callee, "ifelse" | "rr_ifelse_strict")
                {
                    let mut vec_keys = Vec::new();
                    let mut saw_vectorish = false;
                    for arg in call_args {
                        let key = rec(fn_ir, *arg, seen_vals, seen_vars);
                        if key.is_some() {
                            saw_vectorish = true;
                            vec_keys.push(key);
                        } else if !is_scalar_value(fn_ir, *arg) {
                            return None;
                        }
                    }
                    if !saw_vectorish {
                        None
                    } else {
                        unify_length_keys(fn_ir, vec_keys)
                    }
                } else {
                    None
                }
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                let lk = rec(fn_ir, *lhs, seen_vals, seen_vars);
                let rk = rec(fn_ir, *rhs, seen_vals, seen_vars);
                match (lk, rk) {
                    (Some(a), Some(b))
                        if canonical_value(fn_ir, a) == canonical_value(fn_ir, b) =>
                    {
                        Some(canonical_value(fn_ir, a))
                    }
                    (Some(k), None) if is_scalar_value(fn_ir, *rhs) => Some(k),
                    (None, Some(k)) if is_scalar_value(fn_ir, *lhs) => Some(k),
                    _ => None,
                }
            }
            _ => None,
        };
        seen_vals.remove(&root);
        out
    }
    rec(
        fn_ir,
        root,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}

pub(crate) fn variable_length_key(fn_ir: &FnIR, var: &str) -> Option<ValueId> {
    let mut keys = Vec::new();
    for bb in &fn_ir.blocks {
        for ins in &bb.instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            keys.push(vector_length_key(fn_ir, *src));
        }
    }
    if keys.is_empty() {
        return None;
    }
    let mut out: Option<ValueId> = None;
    for key in keys {
        let key = key.map(|k| resolve_load_alias_value(fn_ir, k))?;
        match out {
            None => out = Some(key),
            Some(prev) if resolve_load_alias_value(fn_ir, prev) == key => {}
            Some(_) => return None,
        }
    }
    out
}

pub(crate) fn base_length_key(fn_ir: &FnIR, base: ValueId) -> Option<ValueId> {
    if let Some(var) = resolve_base_var(fn_ir, base)
        && let Some(key) = variable_length_key(fn_ir, &var)
    {
        return Some(key);
    }
    vector_length_key(fn_ir, base)
}

pub(crate) fn same_length_proven(fn_ir: &FnIR, a: ValueId, b: ValueId) -> bool {
    let a = canonical_value(fn_ir, a);
    let b = canonical_value(fn_ir, b);
    if a == b {
        return true;
    }
    if fn_ir.values[a].value_ty.len_sym.is_some()
        && fn_ir.values[a].value_ty.len_sym == fn_ir.values[b].value_ty.len_sym
    {
        return true;
    }
    match (vector_length_key(fn_ir, a), vector_length_key(fn_ir, b)) {
        (Some(ka), Some(kb)) => canonical_value(fn_ir, ka) == canonical_value(fn_ir, kb),
        _ => false,
    }
}
