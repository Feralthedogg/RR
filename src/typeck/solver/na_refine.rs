use super::*;
use crate::mir::analyze::na::NaState;
use crate::mir::analyze::range::{RangeFacts, SymbolicBound, analyze_ranges, ensure_value_range};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PathNaFacts {
    pub(crate) values: FxHashMap<ValueId, NaState>,
    pub(crate) vars: FxHashMap<String, NaState>,
    pub(crate) elem_non_na_values: FxHashSet<ValueId>,
    pub(crate) elem_non_na_vars: FxHashSet<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct PathNaAnalysis {
    pub(crate) entry: Vec<Option<PathNaFacts>>,
    pub(crate) exit: Vec<Option<PathNaFacts>>,
    pub(crate) ranges: Vec<RangeFacts>,
    pub(crate) owners: Vec<Option<usize>>,
}

pub(crate) struct NaEvalEnv<'a> {
    pub(crate) fn_ir: &'a FnIR,
    pub(crate) facts: &'a PathNaFacts,
    pub(crate) range_facts: Option<&'a RangeFacts>,
}

pub(crate) struct NaEvalScratch<'a> {
    pub(crate) memo: &'a mut FxHashMap<ValueId, NaState>,
    pub(crate) visiting: &'a mut FxHashSet<ValueId>,
}

pub(crate) fn analyze_path_sensitive_na(fn_ir: &FnIR) -> PathNaAnalysis {
    let mut entry = vec![None; fn_ir.blocks.len()];
    let mut exit = vec![None; fn_ir.blocks.len()];
    let ranges = analyze_ranges(fn_ir);
    let owners = collect_unique_value_owners(fn_ir);
    let mut work = VecDeque::new();

    if fn_ir.entry < fn_ir.blocks.len() {
        entry[fn_ir.entry] = Some(PathNaFacts::default());
        work.push_back(fn_ir.entry);
    }

    while let Some(bid) = work.pop_front() {
        let Some(mut facts) = entry.get(bid).and_then(Option::clone) else {
            continue;
        };

        for instr in &fn_ir.blocks[bid].instrs {
            transfer_path_na_instr(&mut facts, instr, fn_ir);
        }
        exit[bid] = Some(facts.clone());

        match &fn_ir.blocks[bid].term {
            Terminator::Goto(target) => {
                if merge_path_na_facts(&mut entry[*target], &facts) {
                    work.push_back(*target);
                }
            }
            Terminator::If {
                cond,
                then_bb,
                else_bb,
            } => {
                let mut then_facts = facts.clone();
                refine_path_na_condition(&mut then_facts, *cond, true, fn_ir);
                if merge_path_na_facts(&mut entry[*then_bb], &then_facts) {
                    work.push_back(*then_bb);
                }

                let mut else_facts = facts;
                refine_path_na_condition(&mut else_facts, *cond, false, fn_ir);
                if merge_path_na_facts(&mut entry[*else_bb], &else_facts) {
                    work.push_back(*else_bb);
                }
            }
            Terminator::Return(_) | Terminator::Unreachable => {}
        }
    }

    PathNaAnalysis {
        entry,
        exit,
        ranges,
        owners,
    }
}

pub(crate) fn merge_path_na_facts(dst: &mut Option<PathNaFacts>, src: &PathNaFacts) -> bool {
    match dst {
        None => {
            *dst = Some(src.clone());
            true
        }
        Some(existing) => {
            let old = existing.clone();
            existing
                .values
                .retain(|value, state| src.values.get(value) == Some(state));
            existing
                .vars
                .retain(|var, state| src.vars.get(var) == Some(state));
            existing
                .elem_non_na_values
                .retain(|value| src.elem_non_na_values.contains(value));
            existing
                .elem_non_na_vars
                .retain(|var| src.elem_non_na_vars.contains(var));
            *existing != old
        }
    }
}

pub(crate) fn transfer_path_na_instr(facts: &mut PathNaFacts, instr: &Instr, fn_ir: &FnIR) {
    match instr {
        Instr::Assign { dst, src, .. } => {
            let state = effective_na_state(fn_ir, *src, facts, None);
            set_var_na_fact(facts, dst, state);
            if elements_non_na_fact(fn_ir, *src, facts) {
                facts.elem_non_na_vars.insert(dst.clone());
            } else {
                facts.elem_non_na_vars.remove(dst);
            }
        }
        Instr::StoreIndex1D { base, .. }
        | Instr::StoreIndex2D { base, .. }
        | Instr::StoreIndex3D { base, .. } => {
            if let Some(var) = value_var_name(fn_ir, *base) {
                set_var_na_fact(facts, &var, NaState::Maybe);
                facts.elem_non_na_vars.remove(&var);
            }
            facts.elem_non_na_values.remove(base);
        }
        Instr::UnsafeRBlock { .. } => {
            facts.vars.clear();
            facts.values.clear();
            facts.elem_non_na_values.clear();
            facts.elem_non_na_vars.clear();
        }
        Instr::Eval { .. } => {}
    }
}

pub(crate) fn refine_path_na_condition(
    facts: &mut PathNaFacts,
    cond: ValueId,
    is_then: bool,
    fn_ir: &FnIR,
) {
    match &fn_ir.values[cond].kind {
        ValueKind::Unary {
            op: crate::syntax::ast::UnaryOp::Not,
            rhs,
        } => refine_path_na_condition(facts, *rhs, !is_then, fn_ir),
        ValueKind::Binary { op, lhs, rhs } => match (op, is_then) {
            (crate::syntax::ast::BinOp::And, true) => {
                refine_path_na_condition(facts, *lhs, true, fn_ir);
                refine_path_na_condition(facts, *rhs, true, fn_ir);
            }
            (crate::syntax::ast::BinOp::Or, false) => {
                refine_path_na_condition(facts, *lhs, false, fn_ir);
                refine_path_na_condition(facts, *rhs, false, fn_ir);
            }
            _ => {}
        },
        ValueKind::Call { callee, args, .. } => {
            let builtin = callee.strip_prefix("base::").unwrap_or(callee);
            if refine_vector_wide_na_condition(facts, builtin, args, is_then, fn_ir) {
                return;
            }
            let Some(arg) = args.first().copied() else {
                return;
            };
            if !can_refine_whole_value_na(fn_ir, arg) {
                return;
            }
            match (builtin, is_then) {
                ("is.na", true) => set_value_na_fact(fn_ir, facts, arg, NaState::Always),
                ("is.na", false) | ("is.finite", true) => {
                    set_value_na_fact(fn_ir, facts, arg, NaState::Never)
                }
                _ => {}
            }
        }
        _ => {}
    }
}

pub(crate) fn refine_vector_wide_na_condition(
    facts: &mut PathNaFacts,
    builtin: &str,
    args: &[ValueId],
    is_then: bool,
    fn_ir: &FnIR,
) -> bool {
    let Some(first) = args.first().copied() else {
        return false;
    };

    let proven = match (builtin, is_then) {
        ("any", false) => extract_is_na_arg(fn_ir, first),
        ("anyNA", false) => Some(first),
        ("all", true) => {
            extract_not_is_na_arg(fn_ir, first).or_else(|| extract_is_finite_arg(fn_ir, first))
        }
        _ => None,
    };

    let Some(value) = proven else {
        return false;
    };
    set_elements_non_na_fact(fn_ir, facts, value);
    true
}

pub(crate) fn extract_is_na_arg(fn_ir: &FnIR, vid: ValueId) -> Option<ValueId> {
    let ValueKind::Call { callee, args, .. } = &fn_ir.values.get(vid)?.kind else {
        return None;
    };
    let builtin = callee.strip_prefix("base::").unwrap_or(callee);
    (builtin == "is.na")
        .then(|| args.first().copied())
        .flatten()
}

pub(crate) fn extract_not_is_na_arg(fn_ir: &FnIR, vid: ValueId) -> Option<ValueId> {
    let ValueKind::Unary {
        op: crate::syntax::ast::UnaryOp::Not,
        rhs,
    } = &fn_ir.values.get(vid)?.kind
    else {
        return None;
    };
    extract_is_na_arg(fn_ir, *rhs)
}

pub(crate) fn extract_is_finite_arg(fn_ir: &FnIR, vid: ValueId) -> Option<ValueId> {
    let ValueKind::Call { callee, args, .. } = &fn_ir.values.get(vid)?.kind else {
        return None;
    };
    let builtin = callee.strip_prefix("base::").unwrap_or(callee);
    (builtin == "is.finite")
        .then(|| args.first().copied())
        .flatten()
}

pub(crate) fn can_refine_whole_value_na(fn_ir: &FnIR, vid: ValueId) -> bool {
    fn_ir
        .values
        .get(vid)
        .is_some_and(|value| value.value_ty.shape == ShapeTy::Scalar)
}

pub(crate) fn set_value_na_fact(
    fn_ir: &FnIR,
    facts: &mut PathNaFacts,
    vid: ValueId,
    state: NaState,
) {
    match state {
        NaState::Never | NaState::Always => {
            facts.values.insert(vid, state);
            if let Some(var) = value_var_name(fn_ir, vid) {
                facts.vars.insert(var, state);
            }
        }
        NaState::Maybe => {
            facts.values.remove(&vid);
            if let Some(var) = value_var_name(fn_ir, vid) {
                facts.vars.remove(&var);
            }
        }
    }
}

pub(crate) fn set_elements_non_na_fact(fn_ir: &FnIR, facts: &mut PathNaFacts, vid: ValueId) {
    if can_refine_whole_value_na(fn_ir, vid) {
        set_value_na_fact(fn_ir, facts, vid, NaState::Never);
        return;
    }
    if can_refine_elements_na(fn_ir, vid) {
        facts.elem_non_na_values.insert(vid);
        if let Some(var) = value_var_name(fn_ir, vid) {
            facts.elem_non_na_vars.insert(var);
        }
    }
}

pub(crate) fn can_refine_elements_na(fn_ir: &FnIR, vid: ValueId) -> bool {
    fn_ir.values.get(vid).is_some_and(|value| {
        matches!(value.value_ty.shape, ShapeTy::Vector | ShapeTy::Matrix)
            || matches!(
                &value.value_term,
                TypeTerm::Vector(_)
                    | TypeTerm::VectorLen(_, _)
                    | TypeTerm::Matrix(_)
                    | TypeTerm::MatrixDim(_, _, _)
            )
    })
}

pub(crate) fn set_var_na_fact(facts: &mut PathNaFacts, var: &str, state: NaState) {
    match state {
        NaState::Never | NaState::Always => {
            facts.vars.insert(var.to_string(), state);
        }
        NaState::Maybe => {
            facts.vars.remove(var);
        }
    }
}

pub(crate) fn value_var_name(fn_ir: &FnIR, vid: ValueId) -> Option<String> {
    match &fn_ir.values.get(vid)?.kind {
        ValueKind::Load { var } => Some(var.clone()),
        ValueKind::Param { index } => fn_ir.params.get(*index).cloned(),
        _ => fn_ir.values[vid].origin_var.clone(),
    }
}

pub(crate) fn effective_na_state(
    fn_ir: &FnIR,
    vid: ValueId,
    facts: &PathNaFacts,
    range_facts: Option<&RangeFacts>,
) -> NaState {
    let mut memo = FxHashMap::default();
    let mut visiting = FxHashSet::default();
    let env = NaEvalEnv {
        fn_ir,
        facts,
        range_facts,
    };
    let mut scratch = NaEvalScratch {
        memo: &mut memo,
        visiting: &mut visiting,
    };
    effective_na_state_inner(&env, vid, &mut scratch)
}

pub(crate) fn effective_na_state_inner(
    env: &NaEvalEnv<'_>,
    vid: ValueId,
    scratch: &mut NaEvalScratch<'_>,
) -> NaState {
    let fn_ir = env.fn_ir;
    if let Some(state) = scratch.memo.get(&vid) {
        return *state;
    }
    if let Some(state) = env.facts.values.get(&vid) {
        scratch.memo.insert(vid, *state);
        return *state;
    }
    if let Some(var) = value_var_name(fn_ir, vid)
        && let Some(state) = env.facts.vars.get(&var)
    {
        scratch.memo.insert(vid, *state);
        return *state;
    }
    if elements_non_na_fact(fn_ir, vid, env.facts) {
        scratch.memo.insert(vid, NaState::Never);
        return NaState::Never;
    }
    if !scratch.visiting.insert(vid) {
        return global_na_state(fn_ir.values[vid].value_ty);
    }

    let state = match &fn_ir.values[vid].kind {
        ValueKind::Const(crate::syntax::ast::Lit::Na) => NaState::Always,
        ValueKind::Const(_) | ValueKind::Len { .. } | ValueKind::Indices { .. } => NaState::Never,
        ValueKind::Param { .. } | ValueKind::Load { .. } | ValueKind::RSymbol { .. } => {
            global_na_state(fn_ir.values[vid].value_ty)
        }
        ValueKind::Range { start, end } => NaState::propagate(
            effective_na_state_inner(env, *start, scratch),
            effective_na_state_inner(env, *end, scratch),
        ),
        ValueKind::Unary { rhs, .. } => effective_na_state_inner(env, *rhs, scratch),
        ValueKind::Binary { lhs, rhs, .. } => NaState::propagate(
            effective_na_state_inner(env, *lhs, scratch),
            effective_na_state_inner(env, *rhs, scratch),
        ),
        ValueKind::Phi { args } => {
            let mut it = args.iter();
            let mut acc = if let Some((value, _)) = it.next() {
                effective_na_state_inner(env, *value, scratch)
            } else {
                NaState::Maybe
            };
            for (value, _) in it {
                acc = NaState::merge_flow(acc, effective_na_state_inner(env, *value, scratch));
            }
            acc
        }
        ValueKind::RecordLit { fields } => fields.iter().fold(NaState::Never, |acc, (_, value)| {
            NaState::propagate(acc, effective_na_state_inner(env, *value, scratch))
        }),
        ValueKind::FieldGet { .. } | ValueKind::FieldSet { .. } => {
            global_na_state(fn_ir.values[vid].value_ty)
        }
        ValueKind::Call {
            callee,
            args,
            names,
        } => effective_call_na_state(
            env,
            callee,
            args,
            names,
            scratch,
            global_na_state(fn_ir.values[vid].value_ty),
        ),
        ValueKind::Intrinsic { args, .. } => args.iter().fold(NaState::Never, |acc, arg| {
            NaState::propagate(acc, effective_na_state_inner(env, *arg, scratch))
        }),
        ValueKind::Index1D { base, idx, .. } => {
            if index_read_is_non_na(env, vid, *base, *idx, scratch) {
                NaState::Never
            } else {
                NaState::Maybe
            }
        }
        ValueKind::Index2D { .. } | ValueKind::Index3D { .. } => NaState::Maybe,
    };

    scratch.visiting.remove(&vid);
    scratch.memo.insert(vid, state);
    state
}

pub(crate) fn effective_call_na_state(
    env: &NaEvalEnv<'_>,
    callee: &str,
    args: &[ValueId],
    names: &[Option<String>],
    scratch: &mut NaEvalScratch<'_>,
    fallback: NaState,
) -> NaState {
    let fn_ir = env.fn_ir;
    let builtin = callee.strip_prefix("base::").unwrap_or(callee);
    match builtin {
        "length" | "seq_len" | "seq_along" | "dim" | "dimnames" | "nrow" | "ncol" | "which"
        | "which.min" | "which.max" | "is.na" | "is.finite" | "isTRUE" | "isFALSE" | "anyNA"
        | "numeric" | "double" | "integer" | "logical" | "character" => {
            return NaState::Never;
        }
        "sum" | "prod" | "min" | "max"
            if named_bool_arg(fn_ir, args, names, "na.rm") == Some(true) =>
        {
            return NaState::Never;
        }
        _ => {}
    }

    if matches!(
        builtin,
        "abs"
            | "sqrt"
            | "sin"
            | "cos"
            | "tan"
            | "log"
            | "log10"
            | "log2"
            | "exp"
            | "floor"
            | "ceiling"
            | "round"
            | "trunc"
            | "any"
            | "all"
            | "c"
            | "rep"
            | "rep.int"
            | "sum"
            | "prod"
            | "min"
            | "max"
            | "mean"
    ) {
        args.iter()
            .enumerate()
            .fold(NaState::Never, |acc, (idx, arg)| {
                if names.get(idx).and_then(Option::as_deref) == Some("na.rm") {
                    acc
                } else {
                    NaState::propagate(acc, effective_na_state_inner(env, *arg, scratch))
                }
            })
    } else {
        fallback
    }
}

pub(crate) fn global_na_state(ty: TypeState) -> NaState {
    if ty.na == NaTy::Never {
        NaState::Never
    } else {
        NaState::Maybe
    }
}

pub(crate) fn index_read_is_non_na(
    env: &NaEvalEnv<'_>,
    vid: ValueId,
    base: ValueId,
    idx: ValueId,
    scratch: &mut NaEvalScratch<'_>,
) -> bool {
    let fn_ir = env.fn_ir;
    let base_non_na = fn_ir.values[base].value_ty.na == NaTy::Never
        || elements_non_na_fact(fn_ir, base, env.facts);
    let idx_non_na = effective_na_state_inner(env, idx, scratch) == NaState::Never;
    base_non_na && idx_non_na && index_read_is_in_bounds(fn_ir, vid, base, idx, env.range_facts)
}

pub(crate) fn elements_non_na_fact(fn_ir: &FnIR, vid: ValueId, facts: &PathNaFacts) -> bool {
    if fn_ir.values[vid].value_ty.na == NaTy::Never {
        return true;
    }
    if facts.elem_non_na_values.contains(&vid) {
        return true;
    }
    value_var_name(fn_ir, vid).is_some_and(|var| facts.elem_non_na_vars.contains(&var))
}

pub(crate) fn index_read_is_in_bounds(
    fn_ir: &FnIR,
    vid: ValueId,
    base: ValueId,
    idx: ValueId,
    range_facts: Option<&RangeFacts>,
) -> bool {
    if matches!(
        fn_ir.values[vid].kind,
        ValueKind::Index1D { is_safe: true, .. }
    ) {
        return true;
    }
    if const_index_in_known_vector_len(fn_ir, base, idx) {
        return true;
    }
    let Some(range_facts) = range_facts else {
        return false;
    };
    let mut range_facts = range_facts.clone();
    let interval = ensure_value_range(idx, &fn_ir.values, &mut range_facts);
    interval.proves_in_bounds(base) || interval_in_known_vector_len(fn_ir, base, &interval)
}

pub(crate) fn const_index_in_known_vector_len(fn_ir: &FnIR, base: ValueId, idx: ValueId) -> bool {
    let Some(len) = known_vector_len(&fn_ir.values[base].value_term) else {
        return false;
    };
    let ValueKind::Const(crate::syntax::ast::Lit::Int(index)) = fn_ir.values[idx].kind else {
        return false;
    };
    index >= 1 && index <= len
}

pub(crate) fn interval_in_known_vector_len(
    fn_ir: &FnIR,
    base: ValueId,
    interval: &crate::mir::analyze::range::RangeInterval,
) -> bool {
    let Some(len) = known_vector_len(&fn_ir.values[base].value_term) else {
        return false;
    };
    matches!(interval.lo, SymbolicBound::Const(lo) if lo >= 1)
        && matches!(interval.hi, SymbolicBound::Const(hi) if hi <= len)
}

pub(crate) fn known_vector_len(term: &TypeTerm) -> Option<i64> {
    match term {
        TypeTerm::VectorLen(_, Some(len)) => Some(*len),
        _ => None,
    }
}

pub(crate) fn apply_path_sensitive_na_refinements(
    fn_ir: &mut FnIR,
    analysis: &PathNaAnalysis,
) -> bool {
    let mut changed = false;

    for vid in 0..fn_ir.values.len() {
        let Some(owner) = analysis.owners.get(vid).copied().flatten() else {
            continue;
        };
        let Some(facts) = analysis.entry.get(owner).and_then(Option::as_ref) else {
            continue;
        };
        let range_facts = analysis.ranges.get(owner);
        let state = effective_na_state(fn_ir, vid, facts, range_facts);
        if state == NaState::Never
            && !matches!(
                fn_ir.values[vid].kind,
                ValueKind::Param { .. } | ValueKind::Load { .. }
            )
            && fn_ir.values[vid].value_ty.na != NaTy::Never
        {
            fn_ir.values[vid].value_ty.na = NaTy::Never;
            changed = true;
        }

        let ValueKind::Index1D {
            base, idx, is_safe, ..
        } = fn_ir.values[vid].kind
        else {
            continue;
        };
        let mut memo = FxHashMap::default();
        let mut visiting = FxHashSet::default();
        let env = NaEvalEnv {
            fn_ir,
            facts,
            range_facts,
        };
        let mut scratch = NaEvalScratch {
            memo: &mut memo,
            visiting: &mut visiting,
        };
        let idx_non_na = effective_na_state_inner(&env, idx, &mut scratch) == NaState::Never;
        if idx_non_na
            && let ValueKind::Index1D {
                ref mut is_na_safe, ..
            } = fn_ir.values[vid].kind
            && !*is_na_safe
        {
            *is_na_safe = true;
            changed = true;
        }
        if fn_ir.values[base].value_ty.na == NaTy::Never
            && idx_non_na
            && (is_safe || index_read_is_in_bounds(fn_ir, vid, base, idx, range_facts))
            && fn_ir.values[vid].value_ty.na != NaTy::Never
        {
            fn_ir.values[vid].value_ty.na = NaTy::Never;
            changed = true;
        }
    }

    changed |= apply_path_sensitive_store_index_safety(fn_ir, analysis);
    changed
}

pub(crate) fn apply_path_sensitive_store_index_safety(
    fn_ir: &mut FnIR,
    analysis: &PathNaAnalysis,
) -> bool {
    let mut changed = false;
    for bid in 0..fn_ir.blocks.len() {
        let Some(mut facts) = analysis.entry.get(bid).and_then(Option::clone) else {
            continue;
        };
        let instr_len = fn_ir.blocks[bid].instrs.len();
        for idx in 0..instr_len {
            let instr = fn_ir.blocks[bid].instrs[idx].clone();
            if let Instr::StoreIndex1D { idx: index, .. } = instr {
                let index_non_na =
                    effective_na_state(fn_ir, index, &facts, analysis.ranges.get(bid))
                        == NaState::Never;
                if index_non_na
                    && let Instr::StoreIndex1D {
                        ref mut is_na_safe, ..
                    } = fn_ir.blocks[bid].instrs[idx]
                    && !*is_na_safe
                {
                    *is_na_safe = true;
                    changed = true;
                }
            }
            transfer_path_na_instr(&mut facts, &instr, fn_ir);
        }
    }
    changed
}

pub(crate) fn path_refined_return_type(fn_ir: &FnIR, analysis: &PathNaAnalysis) -> TypeState {
    let mut ret_ty = TypeState::unknown();
    let reachable = compute_reachable(fn_ir);
    for (bid, bb) in fn_ir.blocks.iter().enumerate() {
        if !reachable.get(bid).copied().unwrap_or(false) {
            continue;
        }
        if let Terminator::Return(Some(value)) = bb.term {
            let mut value_ty = fn_ir.values[value].value_ty;
            if let Some(facts) = analysis.exit.get(bid).and_then(Option::as_ref) {
                let state = effective_na_state(fn_ir, value, facts, analysis.ranges.get(bid));
                if state == NaState::Never {
                    value_ty.na = NaTy::Never;
                }
            }
            ret_ty = ret_ty.join(value_ty);
        }
    }

    if ret_ty == TypeState::unknown()
        && let Some(h) = fn_ir.ret_ty_hint
    {
        ret_ty = h;
    }

    ret_ty
}

pub(crate) fn collect_unique_value_owners(fn_ir: &FnIR) -> Vec<Option<usize>> {
    let mut owners = vec![FxHashSet::default(); fn_ir.values.len()];
    for (bid, block) in fn_ir.blocks.iter().enumerate() {
        for instr in &block.instrs {
            for root in instr_roots(instr) {
                collect_value_owner(fn_ir, root, bid, &mut owners, &mut FxHashSet::default());
            }
        }
        for root in terminator_roots(&block.term) {
            collect_value_owner(fn_ir, root, bid, &mut owners, &mut FxHashSet::default());
        }
    }

    owners
        .into_iter()
        .map(|owner_set| {
            if owner_set.len() == 1 {
                owner_set.iter().next().copied()
            } else {
                None
            }
        })
        .collect()
}

pub(crate) fn collect_value_owner(
    fn_ir: &FnIR,
    vid: ValueId,
    bid: usize,
    owners: &mut [FxHashSet<usize>],
    seen: &mut FxHashSet<ValueId>,
) {
    if vid >= fn_ir.values.len() || !seen.insert(vid) {
        return;
    }
    owners[vid].insert(bid);
    for dep in crate::mir::value_dependencies(&fn_ir.values[vid].kind) {
        collect_value_owner(fn_ir, dep, bid, owners, seen);
    }
}

pub(crate) fn instr_roots(instr: &Instr) -> Vec<ValueId> {
    match instr {
        Instr::Assign { src, .. } => vec![*src],
        Instr::Eval { val, .. } => vec![*val],
        Instr::StoreIndex1D { base, idx, val, .. } => vec![*base, *idx, *val],
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => vec![*base, *r, *c, *val],
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => vec![*base, *i, *j, *k, *val],
        Instr::UnsafeRBlock { .. } => Vec::new(),
    }
}

pub(crate) fn terminator_roots(term: &Terminator) -> Vec<ValueId> {
    match term {
        Terminator::If { cond, .. } => vec![*cond],
        Terminator::Return(Some(value)) => vec![*value],
        Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => Vec::new(),
    }
}
