use crate::mir::analyze::{alias, effects, na};
use crate::mir::*;
use std::collections::{HashMap, HashSet, VecDeque};

pub fn optimize(fn_ir: &mut FnIR) -> bool {
    let mut changed = false;
    let mut value_table: HashMap<ValueKind, ValueId> = HashMap::new();
    let mut replacements: HashMap<ValueId, ValueId> = HashMap::new();
    let reachable = compute_reachable(fn_ir);
    let def_blocks = compute_def_blocks(fn_ir, &reachable);
    let doms = compute_dominators(fn_ir, &reachable);
    let na_states = na::compute_na_states(fn_ir);
    let cse_ctx = build_cse_context(fn_ir);

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
            if dominates_value(existing_id, v.id, &def_blocks, &doms) {
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
            if let Instr::StoreIndex1D { base, .. } | Instr::StoreIndex2D { base, .. } = instr {
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
            if ctx.has_impure_call || ctx.has_unknown_mutation {
                return false;
            }
            if !args
                .iter()
                .all(|a| matches!(fn_ir.values[*a].kind, ValueKind::Const(_)))
            {
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

fn canonicalize_kind(mut kind: ValueKind, replacements: &HashMap<ValueId, ValueId>) -> ValueKind {
    // 1. Map inputs to their canonical IDs if already replaced
    match &mut kind {
        ValueKind::Binary { lhs, rhs, .. } => {
            if let Some(&n) = replacements.get(lhs) {
                *lhs = n;
            }
            if let Some(&n) = replacements.get(rhs) {
                *rhs = n;
            }
            // Sort if commutative
            // Add/Mul/Eq/Ne/...
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

fn apply_replacements(fn_ir: &mut FnIR, replacements: &HashMap<ValueId, ValueId>) {
    for b in &mut fn_ir.blocks {
        for instr in &mut b.instrs {
            match instr {
                Instr::Assign { src, .. } => {
                    if let Some(&n) = replacements.get(src) {
                        *src = n;
                    }
                }
                Instr::Eval { val, .. } => {
                    if let Some(&n) = replacements.get(val) {
                        *val = n;
                    }
                }
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    if let Some(&n) = replacements.get(base) {
                        *base = n;
                    }
                    if let Some(&n) = replacements.get(idx) {
                        *idx = n;
                    }
                    if let Some(&n) = replacements.get(val) {
                        *val = n;
                    }
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    if let Some(&n) = replacements.get(base) {
                        *base = n;
                    }
                    if let Some(&n) = replacements.get(r) {
                        *r = n;
                    }
                    if let Some(&n) = replacements.get(c) {
                        *c = n;
                    }
                    if let Some(&n) = replacements.get(val) {
                        *val = n;
                    }
                }
            }
        }
        match &mut b.term {
            Terminator::If { cond, .. } => {
                if let Some(&n) = replacements.get(cond) {
                    *cond = n;
                }
            }
            Terminator::Return(Some(v)) => {
                if let Some(&n) = replacements.get(v) {
                    *v = n;
                }
            }
            _ => {}
        }
    }
    // Also update nested ValueKinds
    for i in 0..fn_ir.values.len() {
        let mut kind = fn_ir.values[i].kind.clone();
        match &mut kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                if let Some(&n) = replacements.get(lhs) {
                    *lhs = n;
                }
                if let Some(&n) = replacements.get(rhs) {
                    *rhs = n;
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
            ValueKind::Call { args, .. } => {
                for a in args {
                    if let Some(&n) = replacements.get(a) {
                        *a = n;
                    }
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
            _ => {}
        }
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
                Terminator::Goto(t) => {
                    if reachable.insert(*t) {
                        queue.push(*t);
                    }
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
                if set != *doms.get(&bb).unwrap() {
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
            ValueKind::Phi { args } => {
                for (a, _) in args {
                    worklist.push_back((*a, bid));
                }
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
                doms.get(&def_current)
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

    fn one_block_fn(name: &str) -> FnIR {
        let mut f = FnIR::new(name.to_string(), vec![]);
        let b0 = f.add_block();
        f.entry = b0;
        f.body_head = b0;
        f
    }

    #[test]
    fn gvn_cse_pure_calls() {
        let mut fn_ir = one_block_fn("gvn_call");
        let c1 = fn_ir.add_value(
            ValueKind::Const(Lit::Int(4)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let call1 = fn_ir.add_value(
            ValueKind::Call {
                callee: "abs".to_string(),
                args: vec![c1],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let call2 = fn_ir.add_value(
            ValueKind::Call {
                callee: "abs".to_string(),
                args: vec![c1],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let sum = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: call1,
                rhs: call2,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(sum));

        let changed = optimize(&mut fn_ir);
        assert!(changed, "expected pure-call CSE to fire");
        match fn_ir.values[sum].kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                assert_eq!(lhs, rhs, "expected duplicated pure call to be CSE'd")
            }
            _ => panic!("sum value shape changed unexpectedly"),
        }
    }

    #[test]
    fn gvn_cse_index2d_reads_when_unmutated() {
        let mut fn_ir = one_block_fn("gvn_index2d");
        let base = fn_ir.add_value(
            ValueKind::Load {
                var: "m".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            Some("m".to_string()),
        );
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let idx1 = fn_ir.add_value(
            ValueKind::Index2D {
                base,
                r: one,
                c: one,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let idx2 = fn_ir.add_value(
            ValueKind::Index2D {
                base,
                r: one,
                c: one,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let sum = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: idx1,
                rhs: idx2,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(sum));

        let changed = optimize(&mut fn_ir);
        assert!(changed, "expected Index2D CSE to fire");
        match fn_ir.values[sum].kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                assert_eq!(lhs, rhs, "expected duplicated Index2D read to be CSE'd")
            }
            _ => panic!("sum value shape changed unexpectedly"),
        }
    }
}
