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

    // Install instructions and jump to the BODY HEAD (skipping the prologue in entry)
    fn_ir.blocks[bid].instrs = new_instrs;
    let body_head = fn_ir.body_head;
    fn_ir.blocks[bid].term = Terminator::Goto(body_head);

    true
}
