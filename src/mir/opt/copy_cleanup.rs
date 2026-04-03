use crate::mir::*;
use rustc_hash::FxHashMap;

pub fn optimize(fn_ir: &mut FnIR) -> bool {
    let preds = build_pred_map(fn_ir);
    let in_states = compute_in_states(fn_ir, &preds);
    let mut changed = false;

    for (bid, in_state) in in_states.iter().cloned().enumerate() {
        let old_instrs = std::mem::take(&mut fn_ir.blocks[bid].instrs);
        let mut new_instrs = Vec::with_capacity(old_instrs.len());
        let mut aliases = in_state;

        for mut instr in old_instrs {
            rewrite_instr_values(fn_ir, &mut instr, &aliases, &mut changed);

            match &instr {
                Instr::Assign { dst, src, .. } => {
                    invalidate_aliases(dst, &mut aliases);
                    if let ValueKind::Load { var } = &fn_ir.values[*src].kind {
                        let canonical = resolve_alias(var, &aliases);
                        if canonical == *dst {
                            changed = true;
                            continue;
                        }
                        aliases.insert(dst.clone(), canonical);
                    }
                }
                Instr::StoreIndex1D { base, .. }
                | Instr::StoreIndex2D { base, .. }
                | Instr::StoreIndex3D { base, .. } => {
                    if let Some(var) = written_base_var(fn_ir, *base) {
                        invalidate_aliases(&var, &mut aliases);
                    }
                }
                Instr::Eval { .. } => {}
            }

            new_instrs.push(instr);
        }

        let mut term = fn_ir.blocks[bid].term.clone();
        rewrite_term_values(fn_ir, &mut term, &aliases, &mut changed);
        fn_ir.blocks[bid].instrs = new_instrs;
        fn_ir.blocks[bid].term = term;
    }

    changed
}

fn build_pred_map(fn_ir: &FnIR) -> FxHashMap<BlockId, Vec<BlockId>> {
    let mut preds: FxHashMap<BlockId, Vec<BlockId>> = FxHashMap::default();
    for (bid, block) in fn_ir.blocks.iter().enumerate() {
        match block.term {
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

fn intersect_alias_maps(
    left: &FxHashMap<String, String>,
    right: &FxHashMap<String, String>,
) -> FxHashMap<String, String> {
    let mut out = FxHashMap::default();
    for (key, value) in left {
        if right.get(key) == Some(value) {
            out.insert(key.clone(), value.clone());
        }
    }
    out
}

fn transfer_alias_state(
    fn_ir: &FnIR,
    bid: BlockId,
    start: &FxHashMap<String, String>,
) -> FxHashMap<String, String> {
    let mut aliases = start.clone();
    for instr in &fn_ir.blocks[bid].instrs {
        match instr {
            Instr::Assign { dst, src, .. } => {
                invalidate_aliases(dst, &mut aliases);
                if let ValueKind::Load { var } = &fn_ir.values[*src].kind {
                    let canonical = resolve_alias(var, &aliases);
                    if canonical != *dst {
                        aliases.insert(dst.clone(), canonical);
                    }
                }
            }
            Instr::StoreIndex1D { base, .. }
            | Instr::StoreIndex2D { base, .. }
            | Instr::StoreIndex3D { base, .. } => {
                if let Some(var) = written_base_var(fn_ir, *base) {
                    invalidate_aliases(&var, &mut aliases);
                }
            }
            Instr::Eval { .. } => {}
        }
    }
    aliases
}

fn compute_in_states(
    fn_ir: &FnIR,
    preds: &FxHashMap<BlockId, Vec<BlockId>>,
) -> Vec<FxHashMap<String, String>> {
    let mut in_states = vec![FxHashMap::default(); fn_ir.blocks.len()];
    let mut out_states = vec![FxHashMap::default(); fn_ir.blocks.len()];
    let mut iterations = 0usize;

    loop {
        iterations += 1;
        assert!(
            iterations <= fn_ir.blocks.len().saturating_mul(16).max(16),
            "copy_cleanup compute_in_states did not converge for {} after {} iterations",
            fn_ir.name,
            iterations
        );
        let mut changed = false;
        for bid in 0..fn_ir.blocks.len() {
            let pred_list = preds.get(&bid).cloned().unwrap_or_default();
            let new_in = if let Some((first, rest)) = pred_list.split_first() {
                let mut acc = out_states[*first].clone();
                for pred in rest {
                    acc = intersect_alias_maps(&acc, &out_states[*pred]);
                }
                acc
            } else {
                FxHashMap::default()
            };
            if new_in != in_states[bid] {
                in_states[bid] = new_in.clone();
                changed = true;
            }
            let new_out = transfer_alias_state(fn_ir, bid, &new_in);
            if new_out != out_states[bid] {
                out_states[bid] = new_out;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    in_states
}

fn resolve_alias(var: &str, aliases: &FxHashMap<String, String>) -> String {
    let mut cur = var;
    let mut steps = 0usize;
    while let Some(next) = aliases.get(cur) {
        if next == cur || steps > aliases.len() {
            break;
        }
        cur = next;
        steps += 1;
    }
    cur.to_string()
}

fn alias_chain_contains(aliases: &FxHashMap<String, String>, start: &str, needle: &str) -> bool {
    let mut cur = start;
    let mut steps = 0usize;
    while let Some(next) = aliases.get(cur) {
        if next == needle {
            return true;
        }
        if next == cur || steps > aliases.len() {
            break;
        }
        cur = next;
        steps += 1;
    }
    false
}

fn invalidate_aliases(var: &str, aliases: &mut FxHashMap<String, String>) {
    aliases.remove(var);
    let doomed: Vec<String> = aliases
        .keys()
        .filter(|name| alias_chain_contains(aliases, name, var))
        .cloned()
        .collect();
    for name in doomed {
        aliases.remove(&name);
    }
}

fn written_base_var(fn_ir: &FnIR, base: ValueId) -> Option<String> {
    if let Some(var) = fn_ir.values[base].origin_var.as_ref() {
        return Some(var.clone());
    }
    match &fn_ir.values[base].kind {
        ValueKind::Load { var } => Some(var.clone()),
        _ => None,
    }
}

fn clone_value_with_kind(fn_ir: &mut FnIR, old_vid: ValueId, kind: ValueKind) -> ValueId {
    let old = fn_ir.values[old_vid].clone();
    let new_id = fn_ir.add_value(kind, old.span, old.facts, old.origin_var.clone());
    fn_ir.values[new_id].value_ty = old.value_ty;
    fn_ir.values[new_id].value_term = old.value_term;
    fn_ir.values[new_id].phi_block = old.phi_block;
    fn_ir.values[new_id].escape = old.escape;
    new_id
}

fn rewrite_value_aliases(
    fn_ir: &mut FnIR,
    vid: ValueId,
    aliases: &FxHashMap<String, String>,
    changed: &mut bool,
) -> ValueId {
    let val = fn_ir.values[vid].clone();
    match val.kind {
        ValueKind::Load { var } => {
            let canonical = resolve_alias(&var, aliases);
            if canonical == var {
                vid
            } else {
                *changed = true;
                clone_value_with_kind(fn_ir, vid, ValueKind::Load { var: canonical })
            }
        }
        ValueKind::Binary { op, lhs, rhs } => {
            let new_lhs = rewrite_value_aliases(fn_ir, lhs, aliases, changed);
            let new_rhs = rewrite_value_aliases(fn_ir, rhs, aliases, changed);
            if new_lhs == lhs && new_rhs == rhs {
                vid
            } else {
                *changed = true;
                clone_value_with_kind(
                    fn_ir,
                    vid,
                    ValueKind::Binary {
                        op,
                        lhs: new_lhs,
                        rhs: new_rhs,
                    },
                )
            }
        }
        ValueKind::Unary { op, rhs } => {
            let new_rhs = rewrite_value_aliases(fn_ir, rhs, aliases, changed);
            if new_rhs == rhs {
                vid
            } else {
                *changed = true;
                clone_value_with_kind(fn_ir, vid, ValueKind::Unary { op, rhs: new_rhs })
            }
        }
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            let new_args: Vec<_> = args
                .iter()
                .map(|arg| rewrite_value_aliases(fn_ir, *arg, aliases, changed))
                .collect();
            if new_args == args {
                vid
            } else {
                *changed = true;
                clone_value_with_kind(
                    fn_ir,
                    vid,
                    ValueKind::Call {
                        callee,
                        args: new_args,
                        names,
                    },
                )
            }
        }
        ValueKind::Intrinsic { op, args } => {
            let new_args: Vec<_> = args
                .iter()
                .map(|arg| rewrite_value_aliases(fn_ir, *arg, aliases, changed))
                .collect();
            if new_args == args {
                vid
            } else {
                *changed = true;
                clone_value_with_kind(fn_ir, vid, ValueKind::Intrinsic { op, args: new_args })
            }
        }
        ValueKind::RecordLit { fields } => {
            let new_fields: Vec<_> = fields
                .iter()
                .map(|(name, value)| {
                    (
                        name.clone(),
                        rewrite_value_aliases(fn_ir, *value, aliases, changed),
                    )
                })
                .collect();
            if new_fields == fields {
                vid
            } else {
                *changed = true;
                clone_value_with_kind(fn_ir, vid, ValueKind::RecordLit { fields: new_fields })
            }
        }
        ValueKind::FieldGet { base, field } => {
            let new_base = rewrite_value_aliases(fn_ir, base, aliases, changed);
            if new_base == base {
                vid
            } else {
                *changed = true;
                clone_value_with_kind(
                    fn_ir,
                    vid,
                    ValueKind::FieldGet {
                        base: new_base,
                        field,
                    },
                )
            }
        }
        ValueKind::FieldSet { base, field, value } => {
            let new_base = rewrite_value_aliases(fn_ir, base, aliases, changed);
            let new_value = rewrite_value_aliases(fn_ir, value, aliases, changed);
            if new_base == base && new_value == value {
                vid
            } else {
                *changed = true;
                clone_value_with_kind(
                    fn_ir,
                    vid,
                    ValueKind::FieldSet {
                        base: new_base,
                        field,
                        value: new_value,
                    },
                )
            }
        }
        ValueKind::Index1D {
            base,
            idx,
            is_safe,
            is_na_safe,
        } => {
            let new_base = rewrite_value_aliases(fn_ir, base, aliases, changed);
            let new_idx = rewrite_value_aliases(fn_ir, idx, aliases, changed);
            if new_base == base && new_idx == idx {
                vid
            } else {
                *changed = true;
                clone_value_with_kind(
                    fn_ir,
                    vid,
                    ValueKind::Index1D {
                        base: new_base,
                        idx: new_idx,
                        is_safe,
                        is_na_safe,
                    },
                )
            }
        }
        ValueKind::Index2D { base, r, c } => {
            let new_base = rewrite_value_aliases(fn_ir, base, aliases, changed);
            let new_r = rewrite_value_aliases(fn_ir, r, aliases, changed);
            let new_c = rewrite_value_aliases(fn_ir, c, aliases, changed);
            if new_base == base && new_r == r && new_c == c {
                vid
            } else {
                *changed = true;
                clone_value_with_kind(
                    fn_ir,
                    vid,
                    ValueKind::Index2D {
                        base: new_base,
                        r: new_r,
                        c: new_c,
                    },
                )
            }
        }
        ValueKind::Index3D { base, i, j, k } => {
            let new_base = rewrite_value_aliases(fn_ir, base, aliases, changed);
            let new_i = rewrite_value_aliases(fn_ir, i, aliases, changed);
            let new_j = rewrite_value_aliases(fn_ir, j, aliases, changed);
            let new_k = rewrite_value_aliases(fn_ir, k, aliases, changed);
            if new_base == base && new_i == i && new_j == j && new_k == k {
                vid
            } else {
                *changed = true;
                clone_value_with_kind(
                    fn_ir,
                    vid,
                    ValueKind::Index3D {
                        base: new_base,
                        i: new_i,
                        j: new_j,
                        k: new_k,
                    },
                )
            }
        }
        ValueKind::Len { base } => {
            let new_base = rewrite_value_aliases(fn_ir, base, aliases, changed);
            if new_base == base {
                vid
            } else {
                *changed = true;
                clone_value_with_kind(fn_ir, vid, ValueKind::Len { base: new_base })
            }
        }
        ValueKind::Indices { base } => {
            let new_base = rewrite_value_aliases(fn_ir, base, aliases, changed);
            if new_base == base {
                vid
            } else {
                *changed = true;
                clone_value_with_kind(fn_ir, vid, ValueKind::Indices { base: new_base })
            }
        }
        ValueKind::Range { start, end } => {
            let new_start = rewrite_value_aliases(fn_ir, start, aliases, changed);
            let new_end = rewrite_value_aliases(fn_ir, end, aliases, changed);
            if new_start == start && new_end == end {
                vid
            } else {
                *changed = true;
                clone_value_with_kind(
                    fn_ir,
                    vid,
                    ValueKind::Range {
                        start: new_start,
                        end: new_end,
                    },
                )
            }
        }
        ValueKind::Const(_)
        | ValueKind::Phi { .. }
        | ValueKind::Param { .. }
        | ValueKind::RSymbol { .. } => vid,
    }
}

fn rewrite_instr_values(
    fn_ir: &mut FnIR,
    instr: &mut Instr,
    aliases: &FxHashMap<String, String>,
    changed: &mut bool,
) {
    match instr {
        Instr::Assign { src, .. } => {
            *src = rewrite_value_aliases(fn_ir, *src, aliases, changed);
        }
        Instr::Eval { val, .. } => {
            *val = rewrite_value_aliases(fn_ir, *val, aliases, changed);
        }
        Instr::StoreIndex1D { base, idx, val, .. } => {
            *base = rewrite_value_aliases(fn_ir, *base, aliases, changed);
            *idx = rewrite_value_aliases(fn_ir, *idx, aliases, changed);
            *val = rewrite_value_aliases(fn_ir, *val, aliases, changed);
        }
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => {
            *base = rewrite_value_aliases(fn_ir, *base, aliases, changed);
            *r = rewrite_value_aliases(fn_ir, *r, aliases, changed);
            *c = rewrite_value_aliases(fn_ir, *c, aliases, changed);
            *val = rewrite_value_aliases(fn_ir, *val, aliases, changed);
        }
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => {
            *base = rewrite_value_aliases(fn_ir, *base, aliases, changed);
            *i = rewrite_value_aliases(fn_ir, *i, aliases, changed);
            *j = rewrite_value_aliases(fn_ir, *j, aliases, changed);
            *k = rewrite_value_aliases(fn_ir, *k, aliases, changed);
            *val = rewrite_value_aliases(fn_ir, *val, aliases, changed);
        }
    }
}

fn rewrite_term_values(
    fn_ir: &mut FnIR,
    term: &mut Terminator,
    aliases: &FxHashMap<String, String>,
    changed: &mut bool,
) {
    match term {
        Terminator::If { cond, .. } => {
            *cond = rewrite_value_aliases(fn_ir, *cond, aliases, changed);
        }
        Terminator::Return(Some(val)) => {
            *val = rewrite_value_aliases(fn_ir, *val, aliases, changed);
        }
        Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::Span;

    #[test]
    fn block_local_alias_chain_is_collapsed() {
        let mut fn_ir = FnIR::new("copy_cleanup_chain".to_string(), vec![]);
        let b0 = fn_ir.add_block();
        fn_ir.entry = b0;
        fn_ir.body_head = b0;

        let load_src = fn_ir.add_value(
            ValueKind::Load {
                var: "src".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            Some("src".to_string()),
        );
        let load_tmp = fn_ir.add_value(
            ValueKind::Load {
                var: "tmp".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            Some("tmp".to_string()),
        );

        fn_ir.blocks[b0].instrs.push(Instr::Assign {
            dst: "tmp".to_string(),
            src: load_src,
            span: Span::dummy(),
        });
        fn_ir.blocks[b0].instrs.push(Instr::Assign {
            dst: "dst".to_string(),
            src: load_tmp,
            span: Span::dummy(),
        });
        fn_ir.blocks[b0].term = Terminator::Return(None);

        let changed = optimize(&mut fn_ir);
        assert!(changed);
        let Instr::Assign { src, .. } = &fn_ir.blocks[b0].instrs[1] else {
            panic!("expected second instruction to stay an assignment");
        };
        match &fn_ir.values[*src].kind {
            ValueKind::Load { var } => assert_eq!(var, "src"),
            other => panic!("expected rewritten load(src), got {:?}", other),
        }
    }

    #[test]
    fn self_copy_after_alias_canonicalization_is_removed() {
        let mut fn_ir = FnIR::new("copy_cleanup_self".to_string(), vec![]);
        let b0 = fn_ir.add_block();
        fn_ir.entry = b0;
        fn_ir.body_head = b0;

        let load_x = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let load_y = fn_ir.add_value(
            ValueKind::Load {
                var: "y".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            Some("y".to_string()),
        );

        fn_ir.blocks[b0].instrs.push(Instr::Assign {
            dst: "y".to_string(),
            src: load_x,
            span: Span::dummy(),
        });
        fn_ir.blocks[b0].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: load_y,
            span: Span::dummy(),
        });
        fn_ir.blocks[b0].term = Terminator::Return(None);

        let changed = optimize(&mut fn_ir);
        assert!(changed);
        assert_eq!(fn_ir.blocks[b0].instrs.len(), 1);
        let Instr::Assign { dst, .. } = &fn_ir.blocks[b0].instrs[0] else {
            panic!("expected first instruction to remain assignment");
        };
        assert_eq!(dst, "y");
    }

    #[test]
    fn alias_is_carried_across_single_predecessor_goto_block() {
        let mut fn_ir = FnIR::new("copy_cleanup_goto".to_string(), vec![]);
        let b0 = fn_ir.add_block();
        let b1 = fn_ir.add_block();
        fn_ir.entry = b0;
        fn_ir.body_head = b0;

        let load_src = fn_ir.add_value(
            ValueKind::Load {
                var: "src".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            Some("src".to_string()),
        );
        let load_tmp = fn_ir.add_value(
            ValueKind::Load {
                var: "tmp".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            Some("tmp".to_string()),
        );

        fn_ir.blocks[b0].instrs.push(Instr::Assign {
            dst: "tmp".to_string(),
            src: load_src,
            span: Span::dummy(),
        });
        fn_ir.blocks[b0].term = Terminator::Goto(b1);
        fn_ir.blocks[b1].instrs.push(Instr::Assign {
            dst: "dst".to_string(),
            src: load_tmp,
            span: Span::dummy(),
        });
        fn_ir.blocks[b1].term = Terminator::Return(None);

        let changed = optimize(&mut fn_ir);
        assert!(changed);
        let Instr::Assign { src, .. } = &fn_ir.blocks[b1].instrs[0] else {
            panic!("expected assignment in successor block");
        };
        match &fn_ir.values[*src].kind {
            ValueKind::Load { var } => assert_eq!(var, "src"),
            other => panic!("expected rewritten load(src), got {:?}", other),
        }
    }

    #[test]
    fn alias_is_carried_through_join_only_when_all_preds_agree() {
        let mut fn_ir = FnIR::new("copy_cleanup_join".to_string(), vec![]);
        let entry = fn_ir.add_block();
        let left = fn_ir.add_block();
        let right = fn_ir.add_block();
        let join = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let load_src = fn_ir.add_value(
            ValueKind::Load {
                var: "src".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            Some("src".to_string()),
        );
        let load_tmp = fn_ir.add_value(
            ValueKind::Load {
                var: "tmp".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            Some("tmp".to_string()),
        );

        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        fn_ir.blocks[left].instrs.push(Instr::Assign {
            dst: "tmp".to_string(),
            src: load_src,
            span: Span::dummy(),
        });
        fn_ir.blocks[left].term = Terminator::Goto(join);
        fn_ir.blocks[right].instrs.push(Instr::Assign {
            dst: "tmp".to_string(),
            src: load_src,
            span: Span::dummy(),
        });
        fn_ir.blocks[right].term = Terminator::Goto(join);
        fn_ir.blocks[join].instrs.push(Instr::Assign {
            dst: "dst".to_string(),
            src: load_tmp,
            span: Span::dummy(),
        });
        fn_ir.blocks[join].term = Terminator::Return(None);

        let changed = optimize(&mut fn_ir);
        assert!(changed);
        let Instr::Assign { src, .. } = &fn_ir.blocks[join].instrs[0] else {
            panic!("expected assignment in join block");
        };
        match &fn_ir.values[*src].kind {
            ValueKind::Load { var } => assert_eq!(var, "src"),
            other => panic!("expected rewritten load(src), got {:?}", other),
        }
    }
}
