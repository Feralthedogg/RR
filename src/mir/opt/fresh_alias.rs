use crate::mir::analyze::effects;
use crate::mir::*;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Default)]
struct AnalysisCtx {
    pure_memo: FxHashMap<String, bool>,
    fresh_memo: FxHashMap<String, bool>,
    visiting_pure: FxHashSet<String>,
    visiting_fresh: FxHashSet<String>,
}

pub fn optimize_program(all_fns: &mut FxHashMap<String, FnIR>) -> bool {
    let fresh_user_calls = collect_fresh_returning_user_functions(all_fns);
    let mut changed = false;
    let mut names: Vec<_> = all_fns.keys().cloned().collect();
    names.sort();
    for name in names {
        let Some(fn_ir) = all_fns.get_mut(&name) else {
            continue;
        };
        changed |= optimize_function(fn_ir, &fresh_user_calls);
    }
    changed
}

pub(crate) fn collect_fresh_returning_user_functions_for_parallel(
    all_fns: &FxHashMap<String, FnIR>,
) -> FxHashSet<String> {
    collect_fresh_returning_user_functions(all_fns)
}

pub(crate) fn optimize_function_with_fresh_user_calls(
    fn_ir: &mut FnIR,
    fresh_user_calls: &FxHashSet<String>,
) -> bool {
    optimize_function(fn_ir, fresh_user_calls)
}

fn optimize_function(fn_ir: &mut FnIR, fresh_user_calls: &FxHashSet<String>) -> bool {
    let in_states = compute_in_states(fn_ir, fresh_user_calls);
    let mut changed = false;
    let mut seen_fresh_recipe_assigns: FxHashSet<ValueId> = FxHashSet::default();
    for bid in 0..fn_ir.blocks.len() {
        let mut fresh_vars = in_states.get(bid).cloned().unwrap_or_default();
        let instr_len = fn_ir.blocks[bid].instrs.len();
        for idx in 0..instr_len {
            let instr = fn_ir.blocks[bid].instrs[idx].clone();
            match instr {
                Instr::Assign { dst, src, span } => {
                    if std::env::var_os("RR_DEBUG_FRESH_ALIAS").is_some()
                        && fn_ir.name == "Sym_303"
                        && (dst == "u_stage" || dst == "u_new" || dst == "qr")
                    {
                        eprintln!(
                            "fresh_alias fn={} bid={} dst={} src={} kind={:?} in_state={:?}",
                            fn_ir.name, bid, dst, src, fn_ir.values[src].kind, fresh_vars
                        );
                    }
                    if let Some((recipe, from_alias)) =
                        fresh_source_value_with_fallback(fn_ir, src, &fresh_vars, fresh_user_calls)
                    {
                        if std::env::var_os("RR_DEBUG_FRESH_ALIAS").is_some()
                            && fn_ir.name == "Sym_303"
                            && (dst == "u_stage" || dst == "u_new" || dst == "qr")
                        {
                            eprintln!(
                                "  recipe={} from_alias={} seen_before={} kind={:?}",
                                recipe,
                                from_alias,
                                seen_fresh_recipe_assigns.contains(&recipe),
                                fn_ir.values[recipe].kind
                            );
                        }
                        let recipe_already_bound_elsewhere = fresh_vars
                            .iter()
                            .any(|(var, vid)| var != &dst && *vid == recipe);
                        let recipe_seen_before = matches!(
                            &fn_ir.values[recipe].kind,
                            ValueKind::Call { callee, .. } if is_fresh_call(callee, fresh_user_calls)
                        ) && seen_fresh_recipe_assigns.contains(&recipe);
                        let should_clone =
                            from_alias || recipe_already_bound_elsewhere || recipe_seen_before;
                        let new_src = if should_clone {
                            clone_value_metadata(fn_ir, recipe)
                        } else {
                            recipe
                        };
                        if should_clone {
                            if let Instr::Assign { src, .. } = &mut fn_ir.blocks[bid].instrs[idx] {
                                *src = new_src;
                            }
                            changed = true;
                        }
                        if matches!(
                            &fn_ir.values[recipe].kind,
                            ValueKind::Call { callee, .. } if is_fresh_call(callee, fresh_user_calls)
                        ) {
                            seen_fresh_recipe_assigns.insert(recipe);
                        }
                        fresh_vars.insert(dst, new_src);
                    } else {
                        let _ = span;
                        fresh_vars.remove(&dst);
                    }
                }
                Instr::StoreIndex1D { base, .. }
                | Instr::StoreIndex2D { base, .. }
                | Instr::StoreIndex3D { base, .. } => {
                    if let Some(var) = resolve_base_var(fn_ir, base) {
                        fresh_vars.remove(&var);
                    }
                }
                Instr::Eval { .. } => {}
            }
        }
    }
    changed
}

fn compute_in_states(
    fn_ir: &FnIR,
    fresh_user_calls: &FxHashSet<String>,
) -> Vec<FxHashMap<String, ValueId>> {
    let preds = build_pred_map(fn_ir);
    let mut in_states = vec![FxHashMap::default(); fn_ir.blocks.len()];
    let mut out_states = vec![FxHashMap::default(); fn_ir.blocks.len()];
    let mut changed = true;
    while changed {
        changed = false;
        for bid in 0..fn_ir.blocks.len() {
            let incoming = if bid == fn_ir.entry {
                FxHashMap::default()
            } else {
                intersect_pred_states(&preds.get(&bid).cloned().unwrap_or_default(), &out_states)
            };
            if incoming != in_states[bid] {
                in_states[bid] = incoming.clone();
                changed = true;
            }
            let outgoing = transfer_block_state(fn_ir, bid, incoming, fresh_user_calls);
            if outgoing != out_states[bid] {
                out_states[bid] = outgoing;
                changed = true;
            }
        }
    }
    in_states
}

fn build_pred_map(fn_ir: &FnIR) -> FxHashMap<BlockId, Vec<BlockId>> {
    let mut preds: FxHashMap<BlockId, Vec<BlockId>> = FxHashMap::default();
    for bid in 0..fn_ir.blocks.len() {
        match fn_ir.blocks[bid].term {
            Terminator::Goto(target) => preds.entry(target).or_default().push(bid),
            Terminator::If {
                then_bb, else_bb, ..
            } => {
                preds.entry(then_bb).or_default().push(bid);
                preds.entry(else_bb).or_default().push(bid);
            }
            Terminator::Return(_) | Terminator::Unreachable => {}
        }
    }
    preds
}

fn intersect_pred_states(
    preds: &[BlockId],
    out_states: &[FxHashMap<String, ValueId>],
) -> FxHashMap<String, ValueId> {
    let Some((&first, rest)) = preds.split_first() else {
        return FxHashMap::default();
    };
    let mut out = out_states[first].clone();
    for pred in rest {
        out.retain(|var, recipe| out_states[*pred].get(var).copied() == Some(*recipe));
    }
    out
}

fn transfer_block_state(
    fn_ir: &FnIR,
    bid: BlockId,
    mut state: FxHashMap<String, ValueId>,
    fresh_user_calls: &FxHashSet<String>,
) -> FxHashMap<String, ValueId> {
    for instr in &fn_ir.blocks[bid].instrs {
        match instr {
            Instr::Assign { dst, src, .. } => {
                if let Some((recipe, _from_alias)) =
                    fresh_source_value(fn_ir, *src, &state, fresh_user_calls)
                {
                    state.insert(dst.clone(), recipe);
                } else {
                    state.remove(dst);
                }
            }
            Instr::StoreIndex1D { base, .. }
            | Instr::StoreIndex2D { base, .. }
            | Instr::StoreIndex3D { base, .. } => {
                if let Some(var) = resolve_base_var(fn_ir, *base) {
                    state.remove(&var);
                }
            }
            Instr::Eval { .. } => {}
        }
    }
    state
}

fn fresh_source_value(
    fn_ir: &FnIR,
    src: ValueId,
    fresh_vars: &FxHashMap<String, ValueId>,
    fresh_user_calls: &FxHashSet<String>,
) -> Option<(ValueId, bool)> {
    match &fn_ir.values.get(src)?.kind {
        ValueKind::Call { callee, .. } if is_fresh_call(callee, fresh_user_calls) => {
            Some((src, false))
        }
        ValueKind::Load { var } => fresh_vars.get(var).copied().map(|vid| (vid, true)),
        _ => None,
    }
}

fn fresh_source_value_with_fallback(
    fn_ir: &FnIR,
    src: ValueId,
    fresh_vars: &FxHashMap<String, ValueId>,
    fresh_user_calls: &FxHashSet<String>,
) -> Option<(ValueId, bool)> {
    if let Some(found) = fresh_source_value(fn_ir, src, fresh_vars, fresh_user_calls) {
        return Some(found);
    }
    match &fn_ir.values.get(src)?.kind {
        ValueKind::Load { var } => {
            global_fresh_var_recipe(fn_ir, var, fresh_user_calls).map(|recipe| (recipe, true))
        }
        _ => None,
    }
}

fn global_fresh_var_recipe(
    fn_ir: &FnIR,
    var: &str,
    fresh_user_calls: &FxHashSet<String>,
) -> Option<ValueId> {
    fn rec(
        fn_ir: &FnIR,
        var: &str,
        fresh_user_calls: &FxHashSet<String>,
        seen: &mut FxHashSet<String>,
    ) -> Option<ValueId> {
        if !seen.insert(var.to_string()) {
            return None;
        }

        let mut assign_src: Option<ValueId> = None;
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                match instr {
                    Instr::Assign { dst, src, .. } if dst == var => match assign_src {
                        None => assign_src = Some(*src),
                        Some(prev) if prev == *src => {}
                        Some(_) => return None,
                    },
                    Instr::StoreIndex1D { base, .. }
                    | Instr::StoreIndex2D { base, .. }
                    | Instr::StoreIndex3D { base, .. } => {
                        if resolve_base_var(fn_ir, *base).as_deref() == Some(var) {
                            return None;
                        }
                    }
                    _ => {}
                }
            }
        }

        let src = assign_src?;
        match &fn_ir.values[src].kind {
            ValueKind::Call { callee, .. } if is_fresh_call(callee, fresh_user_calls) => Some(src),
            ValueKind::Load { var: inner } => rec(fn_ir, inner, fresh_user_calls, seen),
            _ => None,
        }
    }

    rec(fn_ir, var, fresh_user_calls, &mut FxHashSet::default())
}

fn resolve_base_var(fn_ir: &FnIR, base: ValueId) -> Option<String> {
    match &fn_ir.values.get(base)?.kind {
        ValueKind::Load { var } => Some(var.clone()),
        _ => fn_ir.values.get(base)?.origin_var.clone(),
    }
}

fn clone_value_metadata(fn_ir: &mut FnIR, vid: ValueId) -> ValueId {
    let original = fn_ir.values[vid].clone();
    let new_id = fn_ir.add_value(original.kind.clone(), original.span, original.facts, None);
    fn_ir.values[new_id].value_ty = original.value_ty;
    fn_ir.values[new_id].value_term = original.value_term;
    fn_ir.values[new_id].escape = original.escape;
    new_id
}

fn is_fresh_call(callee: &str, fresh_user_calls: &FxHashSet<String>) -> bool {
    matches!(
        callee,
        "rep.int"
            | "numeric"
            | "integer"
            | "logical"
            | "character"
            | "vector"
            | "matrix"
            | "c"
            | "seq_len"
            | "seq_along"
            | "rr_named_list"
    ) || fresh_user_calls.contains(callee)
}

fn collect_fresh_returning_user_functions(all_fns: &FxHashMap<String, FnIR>) -> FxHashSet<String> {
    fn helper_is_functionally_pure(callee: &str) -> bool {
        matches!(
            callee,
            "rr_assign_slice"
                | "rr_ifelse_strict"
                | "rr_index1_read"
                | "rr_index1_read_strict"
                | "rr_index1_read_vec"
                | "rr_index1_read_vec_floor"
                | "rr_index_vec_floor"
                | "rr_gather"
                | "rr_wrap_index_vec"
                | "rr_wrap_index_vec_i"
                | "rr_idx_cube_vec_i"
                | "rr_named_list"
                | "rr_field_get"
                | "rr_field_exists"
                | "rr_list_pattern_matchable"
        )
    }

    fn helper_is_fresh_result(callee: &str) -> bool {
        matches!(
            callee,
            "rep.int"
                | "numeric"
                | "integer"
                | "logical"
                | "character"
                | "vector"
                | "matrix"
                | "c"
                | "seq_len"
                | "seq_along"
                | "rr_named_list"
        )
    }

    fn value_is_functionally_pure(
        all_fns: &FxHashMap<String, FnIR>,
        fn_ir: &FnIR,
        vid: ValueId,
        ctx: &mut AnalysisCtx,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        if !seen.insert(vid) {
            return false;
        }
        let pure = match &fn_ir.values[vid].kind {
            ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::RSymbol { .. } => true,
            ValueKind::Phi { args } => args
                .iter()
                .all(|(src, _)| value_is_functionally_pure(all_fns, fn_ir, *src, ctx, seen)),
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                value_is_functionally_pure(all_fns, fn_ir, *base, ctx, seen)
            }
            ValueKind::Range { start, end } => {
                value_is_functionally_pure(all_fns, fn_ir, *start, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *end, ctx, seen)
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                value_is_functionally_pure(all_fns, fn_ir, *lhs, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *rhs, ctx, seen)
            }
            ValueKind::Unary { rhs, .. } => {
                value_is_functionally_pure(all_fns, fn_ir, *rhs, ctx, seen)
            }
            ValueKind::RecordLit { fields } => fields
                .iter()
                .all(|(_, value)| value_is_functionally_pure(all_fns, fn_ir, *value, ctx, seen)),
            ValueKind::FieldGet { base, .. } => {
                value_is_functionally_pure(all_fns, fn_ir, *base, ctx, seen)
            }
            ValueKind::FieldSet { base, value, .. } => {
                value_is_functionally_pure(all_fns, fn_ir, *base, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *value, ctx, seen)
            }
            ValueKind::Index1D { base, idx, .. } => {
                value_is_functionally_pure(all_fns, fn_ir, *base, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *idx, ctx, seen)
            }
            ValueKind::Index2D { base, r, c } => {
                value_is_functionally_pure(all_fns, fn_ir, *base, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *r, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *c, ctx, seen)
            }
            ValueKind::Index3D { base, i, j, k } => {
                value_is_functionally_pure(all_fns, fn_ir, *base, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *i, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *j, ctx, seen)
                    && value_is_functionally_pure(all_fns, fn_ir, *k, ctx, seen)
            }
            ValueKind::Intrinsic { args, .. } => args
                .iter()
                .all(|arg| value_is_functionally_pure(all_fns, fn_ir, *arg, ctx, seen)),
            ValueKind::Call { callee, args, .. } => {
                let user_pure = all_fns.get(callee).is_some_and(|callee_ir| {
                    function_is_referentially_pure(all_fns, callee, callee_ir, ctx)
                });
                (effects::call_is_pure(callee) || helper_is_functionally_pure(callee) || user_pure)
                    && args
                        .iter()
                        .all(|arg| value_is_functionally_pure(all_fns, fn_ir, *arg, ctx, seen))
            }
        };
        seen.remove(&vid);
        pure
    }

    fn value_is_fresh_result(
        all_fns: &FxHashMap<String, FnIR>,
        fn_ir: &FnIR,
        vid: ValueId,
        ctx: &mut AnalysisCtx,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        fn local_fresh_var_recipe(
            all_fns: &FxHashMap<String, FnIR>,
            fn_ir: &FnIR,
            var: &str,
            ctx: &mut AnalysisCtx,
            seen_vars: &mut FxHashSet<String>,
        ) -> Option<ValueId> {
            if !seen_vars.insert(var.to_string()) {
                return None;
            }
            let mut assign_src: Option<ValueId> = None;
            for block in &fn_ir.blocks {
                for instr in &block.instrs {
                    match instr {
                        Instr::Assign { dst, src, .. } if dst == var => match assign_src {
                            None => assign_src = Some(*src),
                            Some(prev) if prev == *src => {}
                            Some(_) => return None,
                        },
                        Instr::StoreIndex1D { base, .. }
                        | Instr::StoreIndex2D { base, .. }
                        | Instr::StoreIndex3D { base, .. } => {
                            if resolve_base_var(fn_ir, *base).as_deref() == Some(var) {
                                return None;
                            }
                        }
                        _ => {}
                    }
                }
            }
            let src = assign_src?;
            match &fn_ir.values[src].kind {
                ValueKind::Call { callee, .. } => {
                    let user_fresh = all_fns.get(callee).is_some_and(|callee_ir| {
                        function_is_fresh_returning(all_fns, callee, callee_ir, ctx)
                    });
                    if helper_is_fresh_result(callee) || user_fresh {
                        Some(src)
                    } else {
                        None
                    }
                }
                ValueKind::Load { var: inner } => {
                    local_fresh_var_recipe(all_fns, fn_ir, inner, ctx, seen_vars)
                }
                _ => None,
            }
        }

        if !seen.insert(vid) {
            return false;
        }
        let fresh = match &fn_ir.values[vid].kind {
            ValueKind::Const(_) => true,
            ValueKind::Call { callee, args, .. } => {
                let user_fresh = all_fns.get(callee).is_some_and(|callee_ir| {
                    function_is_fresh_returning(all_fns, callee, callee_ir, ctx)
                });
                (helper_is_fresh_result(callee) || user_fresh)
                    && args.iter().all(|arg| {
                        value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *arg,
                            ctx,
                            &mut FxHashSet::default(),
                        )
                    })
            }
            ValueKind::Load { var } => {
                local_fresh_var_recipe(all_fns, fn_ir, var, ctx, &mut FxHashSet::default())
                    .is_some()
            }
            _ => false,
        };
        seen.remove(&vid);
        fresh
    }

    fn function_is_referentially_pure(
        all_fns: &FxHashMap<String, FnIR>,
        name: &str,
        fn_ir: &FnIR,
        ctx: &mut AnalysisCtx,
    ) -> bool {
        if let Some(cached) = ctx.pure_memo.get(name) {
            return *cached;
        }
        if !ctx.visiting_pure.insert(name.to_string()) {
            return false;
        }
        if fn_ir.requires_conservative_optimization() {
            ctx.pure_memo.insert(name.to_string(), false);
            ctx.visiting_pure.remove(name);
            return false;
        }
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                match instr {
                    Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => {
                        if !value_is_functionally_pure(
                            all_fns,
                            fn_ir,
                            *src,
                            ctx,
                            &mut FxHashSet::default(),
                        ) {
                            ctx.pure_memo.insert(name.to_string(), false);
                            ctx.visiting_pure.remove(name);
                            return false;
                        }
                    }
                    Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. }
                    | Instr::StoreIndex3D { .. } => {
                        ctx.pure_memo.insert(name.to_string(), false);
                        ctx.visiting_pure.remove(name);
                        return false;
                    }
                }
            }
            match &block.term {
                Terminator::If { cond, .. } => {
                    if !value_is_functionally_pure(
                        all_fns,
                        fn_ir,
                        *cond,
                        ctx,
                        &mut FxHashSet::default(),
                    ) {
                        ctx.pure_memo.insert(name.to_string(), false);
                        ctx.visiting_pure.remove(name);
                        return false;
                    }
                }
                Terminator::Return(Some(v)) => {
                    if !value_is_functionally_pure(
                        all_fns,
                        fn_ir,
                        *v,
                        ctx,
                        &mut FxHashSet::default(),
                    ) {
                        ctx.pure_memo.insert(name.to_string(), false);
                        ctx.visiting_pure.remove(name);
                        return false;
                    }
                }
                Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
            }
        }
        ctx.pure_memo.insert(name.to_string(), true);
        ctx.visiting_pure.remove(name);
        true
    }

    fn function_is_fresh_returning(
        all_fns: &FxHashMap<String, FnIR>,
        name: &str,
        fn_ir: &FnIR,
        ctx: &mut AnalysisCtx,
    ) -> bool {
        if let Some(cached) = ctx.fresh_memo.get(name) {
            return *cached;
        }
        if !ctx.visiting_fresh.insert(name.to_string()) {
            return false;
        }
        if !function_is_referentially_pure(all_fns, name, fn_ir, ctx) {
            ctx.fresh_memo.insert(name.to_string(), false);
            ctx.visiting_fresh.remove(name);
            return false;
        }
        let mut saw_return = false;
        for block in &fn_ir.blocks {
            if let Terminator::Return(Some(v)) = &block.term {
                saw_return = true;
                if !value_is_fresh_result(all_fns, fn_ir, *v, ctx, &mut FxHashSet::default()) {
                    ctx.fresh_memo.insert(name.to_string(), false);
                    ctx.visiting_fresh.remove(name);
                    return false;
                }
            }
        }
        ctx.fresh_memo.insert(name.to_string(), saw_return);
        ctx.visiting_fresh.remove(name);
        saw_return
    }

    let mut out = FxHashSet::default();
    let mut ctx = AnalysisCtx::default();
    let mut names: Vec<_> = all_fns.keys().cloned().collect();
    names.sort();
    for name in names {
        let Some(fn_ir) = all_fns.get(&name) else {
            continue;
        };
        if function_is_fresh_returning(all_fns, &name, fn_ir, &mut ctx) {
            out.insert(name);
        }
    }
    out
}
