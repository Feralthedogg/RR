use crate::mir::analyze::{alias, effects, na};
use crate::mir::opt::loop_analysis::LoopAnalyzer;
use crate::mir::*;
use std::collections::{HashMap, HashSet, VecDeque};

pub fn optimize(fn_ir: &mut FnIR) -> bool {
    if !is_safe_gvn_candidate(fn_ir) {
        return false;
    }

    let mut changed = false;
    let mut value_table: HashMap<ValueKind, ValueId> = HashMap::new();
    let mut replacements: HashMap<ValueId, ValueId> = HashMap::new();
    let reachable = compute_reachable(fn_ir);
    let def_blocks = compute_def_blocks(fn_ir, &reachable);
    let doms = compute_dominators(fn_ir, &reachable);
    let na_states = na::compute_na_states(fn_ir);
    let cse_ctx = build_cse_context(fn_ir);
    let has_loops = !LoopAnalyzer::new(fn_ir).find_loops().is_empty();

    // Identify redundant values and group by normalized kind.
    for v in &fn_ir.values {
        if !is_cse_eligible(fn_ir, &v.kind, &cse_ctx) {
            continue;
        }

        // Canonicalization: ensure binary ops are sorted if commutative
        let kind = canonicalize_kind(v.kind.clone(), &replacements);

        if let Some(&existing_id) = value_table.get(&kind) {
            if na_states[existing_id] != na_states[v.id] {
                // Don't CSE if NA behavior differs.
                value_table.insert(kind, v.id);
                continue;
            }
            if dominates_value(existing_id, v.id, &def_blocks, &doms, has_loops) {
                replacements.insert(v.id, existing_id);
                changed = true;
            } else {
                // Keep a new representative if dominance isn't guaranteed.
                value_table.insert(kind, v.id);
            }
        } else {
            value_table.insert(kind, v.id);
        }
    }

    // 2. Perform replacements
    if changed {
        apply_replacements(fn_ir, &replacements);
    }

    changed
}

fn is_safe_gvn_candidate(fn_ir: &FnIR) -> bool {
    !fn_ir.values.iter().any(|value| match &value.kind {
        ValueKind::Call { callee, .. } => is_unsafe_runtime_helper(callee),
        _ => false,
    })
}

fn is_unsafe_runtime_helper(callee: &str) -> bool {
    matches!(
        callee,
        "rr_recur_add_const"
            | "rr_assign_slice"
            | "rr_shift_assign"
            | "rr_assign_index_vec"
            | "rr_assign_index_vec_masked"
    )
}

#[derive(Debug, Default)]
struct CseContext {
    mutated_aliases: HashSet<alias::AliasClass>,
    has_unknown_mutation: bool,
    has_impure_call: bool,
}

fn build_cse_context(fn_ir: &FnIR) -> CseContext {
    let mut ctx = CseContext::default();

    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            if let Instr::StoreIndex1D { base, .. }
            | Instr::StoreIndex2D { base, .. }
            | Instr::StoreIndex3D { base, .. } = instr
            {
                let cls = alias::alias_class_for_base(fn_ir, *base);
                if matches!(cls, alias::AliasClass::Unknown) {
                    ctx.has_unknown_mutation = true;
                } else {
                    ctx.mutated_aliases.insert(cls);
                }
            }
        }
    }

    for val in &fn_ir.values {
        if let ValueKind::Call { callee, .. } = &val.kind
            && !effects::call_is_pure(callee)
        {
            ctx.has_impure_call = true;
            break;
        }
    }

    ctx
}

fn is_cse_eligible(fn_ir: &FnIR, kind: &ValueKind, ctx: &CseContext) -> bool {
    match kind {
        ValueKind::Call { callee, args, .. } => {
            if !effects::call_is_pure(callee) {
                return false;
            }
            if call_returns_fresh_value(callee) {
                return false;
            }
            if is_unsafe_runtime_helper(callee) {
                return false;
            }
            if ctx.has_impure_call || ctx.has_unknown_mutation {
                return false;
            }
            !args
                .iter()
                .any(|a| value_reads_mutated_alias(fn_ir, *a, ctx, &mut HashSet::new()))
        }
        ValueKind::Phi { args } if args.is_empty() => false, // Incomplete phi
        ValueKind::Load { .. } => false,                     // Loads can change across assignments
        ValueKind::Index2D { base, .. } => {
            if ctx.has_impure_call || ctx.has_unknown_mutation {
                return false;
            }
            let cls = alias::alias_class_for_base(fn_ir, *base);
            if matches!(cls, alias::AliasClass::Unknown) {
                return false;
            }
            !ctx.mutated_aliases.contains(&cls)
        }
        ValueKind::Index3D { base, .. } => {
            if ctx.has_impure_call || ctx.has_unknown_mutation {
                return false;
            }
            let cls = alias::alias_class_for_base(fn_ir, *base);
            if matches!(cls, alias::AliasClass::Unknown) {
                return false;
            }
            !ctx.mutated_aliases.contains(&cls)
        }
        ValueKind::Index1D {
            base,
            is_safe,
            is_na_safe,
            ..
        } => {
            if !*is_safe || !*is_na_safe {
                return false;
            }
            if ctx.has_impure_call || ctx.has_unknown_mutation {
                return false;
            }
            let cls = alias::alias_class_for_base(fn_ir, *base);
            if matches!(cls, alias::AliasClass::Unknown) {
                return false;
            }
            !ctx.mutated_aliases.contains(&cls)
        }
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            if ctx.has_impure_call || ctx.has_unknown_mutation {
                return false;
            }
            let cls = alias::alias_class_for_base(fn_ir, *base);
            if matches!(cls, alias::AliasClass::Unknown) {
                return false;
            }
            !ctx.mutated_aliases.contains(&cls)
        }
        _ => true,
    }
}

fn call_returns_fresh_value(callee: &str) -> bool {
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

fn value_reads_mutated_alias(
    fn_ir: &FnIR,
    vid: ValueId,
    ctx: &CseContext,
    seen: &mut HashSet<ValueId>,
) -> bool {
    if !seen.insert(vid) {
        return false;
    }
    match &fn_ir.values[vid].kind {
        ValueKind::Index1D { base, idx, .. } => {
            let cls = alias::alias_class_for_base(fn_ir, *base);
            if matches!(cls, alias::AliasClass::Unknown) || ctx.mutated_aliases.contains(&cls) {
                return true;
            }
            value_reads_mutated_alias(fn_ir, *base, ctx, seen)
                || value_reads_mutated_alias(fn_ir, *idx, ctx, seen)
        }
        ValueKind::Index2D { base, r, c } => {
            let cls = alias::alias_class_for_base(fn_ir, *base);
            if matches!(cls, alias::AliasClass::Unknown) || ctx.mutated_aliases.contains(&cls) {
                return true;
            }
            value_reads_mutated_alias(fn_ir, *base, ctx, seen)
                || value_reads_mutated_alias(fn_ir, *r, ctx, seen)
                || value_reads_mutated_alias(fn_ir, *c, ctx, seen)
        }
        ValueKind::Index3D { base, i, j, k } => {
            let cls = alias::alias_class_for_base(fn_ir, *base);
            if matches!(cls, alias::AliasClass::Unknown) || ctx.mutated_aliases.contains(&cls) {
                return true;
            }
            value_reads_mutated_alias(fn_ir, *base, ctx, seen)
                || value_reads_mutated_alias(fn_ir, *i, ctx, seen)
                || value_reads_mutated_alias(fn_ir, *j, ctx, seen)
                || value_reads_mutated_alias(fn_ir, *k, ctx, seen)
        }
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            let cls = alias::alias_class_for_base(fn_ir, *base);
            if matches!(cls, alias::AliasClass::Unknown) || ctx.mutated_aliases.contains(&cls) {
                return true;
            }
            value_reads_mutated_alias(fn_ir, *base, ctx, seen)
        }
        ValueKind::Binary { lhs, rhs, .. } => {
            value_reads_mutated_alias(fn_ir, *lhs, ctx, seen)
                || value_reads_mutated_alias(fn_ir, *rhs, ctx, seen)
        }
        ValueKind::Unary { rhs, .. } => value_reads_mutated_alias(fn_ir, *rhs, ctx, seen),
        ValueKind::Call { args, .. } => args
            .iter()
            .any(|a| value_reads_mutated_alias(fn_ir, *a, ctx, seen)),
        ValueKind::Load { .. } | ValueKind::Param { .. } => {
            let cls = alias::alias_class_for_base(fn_ir, vid);
            matches!(cls, alias::AliasClass::Unknown) || ctx.mutated_aliases.contains(&cls)
        }
        ValueKind::Phi { args } => args
            .iter()
            .any(|(a, _)| value_reads_mutated_alias(fn_ir, *a, ctx, seen)),
        ValueKind::Range { start, end } => {
            value_reads_mutated_alias(fn_ir, *start, ctx, seen)
                || value_reads_mutated_alias(fn_ir, *end, ctx, seen)
        }
        _ => false,
    }
}

// Proof correspondence:
// `proof/lean/RRProofs/GvnSubset.lean` and the Coq `GvnSubset.v` companion
// model the reduced expression-level slice used here: commutative `add`
// canonicalization, intrinsic wrappers, and `fieldset -> field` reads all
// normalize through the same structural walk before CSE lookup.
fn canonicalize_kind(mut kind: ValueKind, replacements: &HashMap<ValueId, ValueId>) -> ValueKind {
    // 1. Map inputs to their canonical IDs if already replaced
    match &mut kind {
        ValueKind::Binary { op, lhs, rhs } => {
            if let Some(&n) = replacements.get(lhs) {
                *lhs = n;
            }
            if let Some(&n) = replacements.get(rhs) {
                *rhs = n;
            }
            if is_commutative_binop(*op) && *lhs > *rhs {
                std::mem::swap(lhs, rhs);
            }
        }
        ValueKind::Unary { rhs, .. } => {
            if let Some(&n) = replacements.get(rhs) {
                *rhs = n;
            }
        }
        ValueKind::Phi { args } => {
            for (a, _) in args {
                if let Some(&n) = replacements.get(a) {
                    *a = n;
                }
            }
        }
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
            for a in args {
                if let Some(&n) = replacements.get(a) {
                    *a = n;
                }
            }
        }
        ValueKind::RecordLit { fields } => {
            for (_, value) in fields {
                if let Some(&n) = replacements.get(value) {
                    *value = n;
                }
            }
        }
        ValueKind::FieldGet { base, .. } => {
            if let Some(&n) = replacements.get(base) {
                *base = n;
            }
        }
        ValueKind::FieldSet { base, value, .. } => {
            if let Some(&n) = replacements.get(base) {
                *base = n;
            }
            if let Some(&n) = replacements.get(value) {
                *value = n;
            }
        }
        ValueKind::Index1D { base, idx, .. } => {
            if let Some(&n) = replacements.get(base) {
                *base = n;
            }
            if let Some(&n) = replacements.get(idx) {
                *idx = n;
            }
        }
        ValueKind::Index2D { base, r, c } => {
            if let Some(&n) = replacements.get(base) {
                *base = n;
            }
            if let Some(&n) = replacements.get(r) {
                *r = n;
            }
            if let Some(&n) = replacements.get(c) {
                *c = n;
            }
        }
        ValueKind::Index3D { base, i, j, k } => {
            if let Some(&n) = replacements.get(base) {
                *base = n;
            }
            if let Some(&n) = replacements.get(i) {
                *i = n;
            }
            if let Some(&n) = replacements.get(j) {
                *j = n;
            }
            if let Some(&n) = replacements.get(k) {
                *k = n;
            }
        }
        ValueKind::Len { base } => {
            if let Some(&n) = replacements.get(base) {
                *base = n;
            }
        }
        ValueKind::Indices { base } => {
            if let Some(&n) = replacements.get(base) {
                *base = n;
            }
        }
        ValueKind::Range { start, end } => {
            if let Some(&n) = replacements.get(start) {
                *start = n;
            }
            if let Some(&n) = replacements.get(end) {
                *end = n;
            }
        }
        _ => {}
    }
    kind
}

fn is_commutative_binop(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Add | BinOp::Mul | BinOp::Eq | BinOp::Ne | BinOp::And | BinOp::Or
    )
}

fn replace_value_id(value: &mut ValueId, replacements: &HashMap<ValueId, ValueId>) {
    if let Some(&new_value) = replacements.get(value) {
        *value = new_value;
    }
}

fn replace_value_ids<'a>(
    values: impl IntoIterator<Item = &'a mut ValueId>,
    replacements: &HashMap<ValueId, ValueId>,
) {
    for value in values {
        replace_value_id(value, replacements);
    }
}

fn apply_replacements_to_instr(instr: &mut Instr, replacements: &HashMap<ValueId, ValueId>) {
    match instr {
        Instr::Assign { src, .. } => replace_value_id(src, replacements),
        Instr::Eval { val, .. } => replace_value_id(val, replacements),
        Instr::StoreIndex1D { base, idx, val, .. } => {
            replace_value_ids([base, idx, val], replacements);
        }
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => {
            replace_value_ids([base, r, c, val], replacements);
        }
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => {
            replace_value_ids([base, i, j, k, val], replacements);
        }
        Instr::UnsafeRBlock { .. } => {}
    }
}

fn apply_replacements_to_terminator(
    term: &mut Terminator,
    replacements: &HashMap<ValueId, ValueId>,
) {
    match term {
        Terminator::If { cond, .. } => replace_value_id(cond, replacements),
        Terminator::Return(Some(value)) => replace_value_id(value, replacements),
        _ => {}
    }
}

fn apply_replacements_to_value_kind(
    kind: &mut ValueKind,
    replacements: &HashMap<ValueId, ValueId>,
) {
    match kind {
        ValueKind::Binary { lhs, rhs, .. } => replace_value_ids([lhs, rhs], replacements),
        ValueKind::Unary { rhs, .. } => replace_value_id(rhs, replacements),
        ValueKind::Phi { args } => {
            for (arg, _) in args {
                replace_value_id(arg, replacements);
            }
        }
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
            replace_value_ids(args.iter_mut(), replacements);
        }
        ValueKind::RecordLit { fields } => {
            for (_, value) in fields {
                replace_value_id(value, replacements);
            }
        }
        ValueKind::FieldGet { base, .. } => replace_value_id(base, replacements),
        ValueKind::FieldSet { base, value, .. } => {
            replace_value_ids([base, value], replacements);
        }
        ValueKind::Index1D { base, idx, .. } => replace_value_ids([base, idx], replacements),
        ValueKind::Index2D { base, r, c } => replace_value_ids([base, r, c], replacements),
        ValueKind::Index3D { base, i, j, k } => replace_value_ids([base, i, j, k], replacements),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            replace_value_id(base, replacements);
        }
        ValueKind::Range { start, end } => replace_value_ids([start, end], replacements),
        _ => {}
    }
}

fn apply_replacements(fn_ir: &mut FnIR, replacements: &HashMap<ValueId, ValueId>) {
    for b in &mut fn_ir.blocks {
        for instr in &mut b.instrs {
            apply_replacements_to_instr(instr, replacements);
        }
        apply_replacements_to_terminator(&mut b.term, replacements);
    }
    // Also update nested ValueKinds
    for i in 0..fn_ir.values.len() {
        let mut kind = fn_ir.values[i].kind.clone();
        apply_replacements_to_value_kind(&mut kind, replacements);
        fn_ir.values[i].kind = kind;
    }
}

fn compute_reachable(fn_ir: &FnIR) -> HashSet<BlockId> {
    let mut reachable = HashSet::new();
    let mut queue = vec![fn_ir.entry];
    reachable.insert(fn_ir.entry);

    let mut head = 0;
    while head < queue.len() {
        let bid = queue[head];
        head += 1;
        if let Some(blk) = fn_ir.blocks.get(bid) {
            match &blk.term {
                Terminator::Goto(t) if reachable.insert(*t) => {
                    queue.push(*t);
                }
                Terminator::If {
                    then_bb, else_bb, ..
                } => {
                    if reachable.insert(*then_bb) {
                        queue.push(*then_bb);
                    }
                    if reachable.insert(*else_bb) {
                        queue.push(*else_bb);
                    }
                }
                _ => {}
            }
        }
    }
    reachable
}

fn build_pred_map(fn_ir: &FnIR) -> HashMap<BlockId, Vec<BlockId>> {
    let mut map = HashMap::new();
    for (src, blk) in fn_ir.blocks.iter().enumerate() {
        let targets = match &blk.term {
            Terminator::Goto(t) => vec![*t],
            Terminator::If {
                then_bb, else_bb, ..
            } => vec![*then_bb, *else_bb],
            _ => vec![],
        };
        for t in targets {
            map.entry(t).or_insert_with(Vec::new).push(src);
        }
    }
    map
}

fn compute_dominators(
    fn_ir: &FnIR,
    reachable: &HashSet<BlockId>,
) -> HashMap<BlockId, HashSet<BlockId>> {
    let preds = build_pred_map(fn_ir);
    let all_blocks: HashSet<BlockId> = reachable.iter().cloned().collect();
    let mut doms: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();

    doms.insert(fn_ir.entry, std::iter::once(fn_ir.entry).collect());
    for &b in &all_blocks {
        if b != fn_ir.entry {
            doms.insert(b, all_blocks.clone());
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for &bb in &all_blocks {
            if bb == fn_ir.entry {
                continue;
            }
            let pred_list = preds.get(&bb).cloned().unwrap_or_default();
            if pred_list.is_empty() {
                continue;
            }

            let mut new_dom: Option<HashSet<BlockId>> = None;
            for p in pred_list {
                if !reachable.contains(&p) {
                    continue;
                }
                if let Some(p_dom) = doms.get(&p) {
                    match new_dom {
                        None => new_dom = Some(p_dom.clone()),
                        Some(ref mut set) => set.retain(|x| p_dom.contains(x)),
                    }
                }
            }

            if let Some(mut set) = new_dom {
                set.insert(bb);
                if doms.get(&bb).is_some_and(|curr| set != *curr) {
                    doms.insert(bb, set);
                    changed = true;
                }
            }
        }
    }

    doms
}

fn compute_def_blocks(fn_ir: &FnIR, reachable: &HashSet<BlockId>) -> Vec<Option<BlockId>> {
    let mut def_blocks: Vec<Option<BlockId>> = vec![None; fn_ir.values.len()];
    let mut worklist: VecDeque<(ValueId, BlockId)> = VecDeque::new();
    let mut visited: HashSet<ValueId> = HashSet::new();

    for (vid, val) in fn_ir.values.iter().enumerate() {
        if let Some(bb) = val.phi_block {
            def_blocks[vid] = Some(bb);
        }
    }

    for bid in 0..fn_ir.blocks.len() {
        if !reachable.contains(&bid) {
            continue;
        }
        let blk = &fn_ir.blocks[bid];
        for instr in &blk.instrs {
            match instr {
                Instr::Assign { src, .. } => worklist.push_back((*src, bid)),
                Instr::Eval { val, .. } => worklist.push_back((*val, bid)),
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    worklist.push_back((*base, bid));
                    worklist.push_back((*idx, bid));
                    worklist.push_back((*val, bid));
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    worklist.push_back((*base, bid));
                    worklist.push_back((*r, bid));
                    worklist.push_back((*c, bid));
                    worklist.push_back((*val, bid));
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    worklist.push_back((*base, bid));
                    worklist.push_back((*i, bid));
                    worklist.push_back((*j, bid));
                    worklist.push_back((*k, bid));
                    worklist.push_back((*val, bid));
                }
                Instr::UnsafeRBlock { .. } => {}
            }
        }
        match &blk.term {
            Terminator::If { cond, .. } => worklist.push_back((*cond, bid)),
            Terminator::Return(Some(v)) => worklist.push_back((*v, bid)),
            _ => {}
        }
    }

    while let Some((vid, bid)) = worklist.pop_front() {
        if !visited.insert(vid) {
            continue;
        }
        if def_blocks[vid].is_none() {
            def_blocks[vid] = Some(bid);
        }

        let val = &fn_ir.values[vid];
        match &val.kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                worklist.push_back((*lhs, bid));
                worklist.push_back((*rhs, bid));
            }
            ValueKind::Unary { rhs, .. } => {
                worklist.push_back((*rhs, bid));
            }
            ValueKind::Call { args, .. } => {
                for a in args {
                    worklist.push_back((*a, bid));
                }
            }
            ValueKind::Intrinsic { args, .. } => {
                for a in args {
                    worklist.push_back((*a, bid));
                }
            }
            ValueKind::Phi { args } => {
                for (a, _) in args {
                    worklist.push_back((*a, bid));
                }
            }
            ValueKind::RecordLit { fields } => {
                for (_, value) in fields {
                    worklist.push_back((*value, bid));
                }
            }
            ValueKind::FieldGet { base, .. } => {
                worklist.push_back((*base, bid));
            }
            ValueKind::FieldSet { base, value, .. } => {
                worklist.push_back((*base, bid));
                worklist.push_back((*value, bid));
            }
            ValueKind::Index1D { base, idx, .. } => {
                worklist.push_back((*base, bid));
                worklist.push_back((*idx, bid));
            }
            ValueKind::Index2D { base, r, c } => {
                worklist.push_back((*base, bid));
                worklist.push_back((*r, bid));
                worklist.push_back((*c, bid));
            }
            ValueKind::Index3D { base, i, j, k } => {
                worklist.push_back((*base, bid));
                worklist.push_back((*i, bid));
                worklist.push_back((*j, bid));
                worklist.push_back((*k, bid));
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                worklist.push_back((*base, bid));
            }
            ValueKind::Range { start, end } => {
                worklist.push_back((*start, bid));
                worklist.push_back((*end, bid));
            }
            _ => {}
        }
    }

    def_blocks
}

fn dominates_value(
    existing: ValueId,
    current: ValueId,
    def_blocks: &[Option<BlockId>],
    doms: &HashMap<BlockId, HashSet<BlockId>>,
    same_block_only: bool,
) -> bool {
    match (
        def_blocks.get(existing).and_then(|x| *x),
        def_blocks.get(current).and_then(|x| *x),
    ) {
        (Some(def_existing), Some(def_current)) => {
            if def_existing == def_current {
                // Values in the same block are only safe to reuse when the
                // representative was created earlier in SSA/value order.
                existing < current
            } else {
                !same_block_only
                    && doms
                        .get(&def_current)
                        .is_some_and(|set| set.contains(&def_existing))
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::flow::Facts;
    use crate::syntax::ast::BinOp;
    use crate::utils::Span;

    #[path = "core_cases.rs"]
    mod core_cases;
}
