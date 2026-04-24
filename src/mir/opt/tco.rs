use crate::mir::*;

pub fn optimize(fn_ir: &mut FnIR) -> bool {
    let mut changed = false;

    // TCO: If we have a tail call to the current function, replace it with a loop.
    // 1. Find Return(Call(self, ...))
    for bid in 0..fn_ir.blocks.len() {
        let is_tail_call = match &fn_ir.blocks[bid].term {
            Terminator::Return(Some(val_id)) => {
                let val = &fn_ir.values[*val_id];
                if let ValueKind::Call { callee, .. } = &val.kind {
                    callee == &fn_ir.name
                } else {
                    false
                }
            }
            _ => false,
        };

        if is_tail_call {
            // Rewrite tail call into loop jump when eligible.
            if perform_tco(fn_ir, bid) {
                changed = true;
            }
        }
    }

    changed
}

fn param_runtime_var(fn_ir: &FnIR, index: usize) -> Option<String> {
    for v in &fn_ir.values {
        if let ValueKind::Param { index: i } = v.kind
            && i == index
        {
            if let Some(name) = &v.origin_var {
                return Some(name.clone());
            }
            break;
        }
    }
    fn_ir.params.get(index).cloned()
}

fn tail_arg_is_direct_param_or_const(fn_ir: &FnIR, arg_id: ValueId) -> bool {
    match &fn_ir.values[arg_id].kind {
        ValueKind::Const(_) | ValueKind::Param { .. } => true,
        ValueKind::Load { var } => fn_ir.params.iter().any(|param| param == var),
        _ => false,
    }
}

fn ensure_separate_body_head(fn_ir: &mut FnIR) -> BlockId {
    if fn_ir.body_head != fn_ir.entry {
        return fn_ir.body_head;
    }

    let entry = fn_ir.entry;
    let new_body_head = fn_ir.add_block();
    let old_instrs = std::mem::take(&mut fn_ir.blocks[entry].instrs);
    let old_term = std::mem::replace(
        &mut fn_ir.blocks[entry].term,
        Terminator::Goto(new_body_head),
    );
    fn_ir.blocks[new_body_head].instrs = old_instrs;
    fn_ir.blocks[new_body_head].term = old_term;
    fn_ir.body_head = new_body_head;
    new_body_head
}

fn perform_tco(fn_ir: &mut FnIR, bid: BlockId) -> bool {
    let ret_val_id = if let Terminator::Return(Some(v)) = &fn_ir.blocks[bid].term {
        *v
    } else {
        return false;
    };

    let (args, span) = if let ValueKind::Call { args, .. } = &fn_ir.values[ret_val_id].kind {
        (args.clone(), fn_ir.values[ret_val_id].span)
    } else {
        return false;
    };

    if args.len() != fn_ir.params.len() {
        return false;
    }

    if !args
        .iter()
        .copied()
        .all(|arg_id| tail_arg_is_direct_param_or_const(fn_ir, arg_id))
    {
        return false;
    }

    // Prepare moves for Parallel Copy
    let mut moves = Vec::new();
    for (i, arg_id) in args.iter().enumerate() {
        let Some(dst_var) = param_runtime_var(fn_ir, i) else {
            return false;
        };
        moves.push(crate::mir::opt::parallel_copy::Move {
            dst: dst_var,
            src: *arg_id,
        });
    }

    // Build new instruction list using safe parallel assignments without
    // borrowing the block while mutating fn_ir.
    let mut new_instrs = Vec::new();
    crate::mir::opt::parallel_copy::emit_parallel_copy(fn_ir, &mut new_instrs, moves, span);

    let body_head = ensure_separate_body_head(fn_ir);
    let rewrite_bid = if bid == fn_ir.entry { body_head } else { bid };

    // Install instructions and jump to the BODY HEAD (skipping the prologue in entry)
    fn_ir.blocks[rewrite_bid].instrs = new_instrs;
    fn_ir.blocks[rewrite_bid].term = Terminator::Goto(body_head);

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::Span;

    #[test]
    fn complex_tail_call_args_do_not_trigger_tco() {
        let mut fn_ir = FnIR::new(
            "recur".to_string(),
            vec!["n".to_string(), "acc".to_string()],
        );
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let n = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::dummy(),
            Facts::empty(),
            Some("n".to_string()),
        );
        let acc = fn_ir.add_value(
            ValueKind::Param { index: 1 },
            Span::dummy(),
            Facts::empty(),
            Some("acc".to_string()),
        );
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let dec_n = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Sub,
                lhs: n,
                rhs: one,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let call = fn_ir.add_value(
            ValueKind::Call {
                callee: "recur".to_string(),
                args: vec![dec_n, acc],
                names: vec![],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(call));

        assert!(!optimize(&mut fn_ir));
        assert!(matches!(
            fn_ir.blocks[entry].term,
            Terminator::Return(Some(_))
        ));
    }

    #[test]
    fn temp_load_tail_call_args_do_not_trigger_tco() {
        let mut fn_ir = FnIR::new(
            "recur".to_string(),
            vec!["n".to_string(), "acc".to_string()],
        );
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let temp_n = fn_ir.add_value(
            ValueKind::Load {
                var: "tmp_n".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            Some("tmp_n".to_string()),
        );
        let acc = fn_ir.add_value(
            ValueKind::Param { index: 1 },
            Span::dummy(),
            Facts::empty(),
            Some("acc".to_string()),
        );
        let call = fn_ir.add_value(
            ValueKind::Call {
                callee: "recur".to_string(),
                args: vec![temp_n, acc],
                names: vec![],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(call));

        assert!(!optimize(&mut fn_ir));
        assert!(matches!(
            fn_ir.blocks[entry].term,
            Terminator::Return(Some(_))
        ));
    }
}
