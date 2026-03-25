use crate::mir::*;
use std::collections::HashSet;

/// Checks if a statement has side effects.
pub fn stmt_is_pure(stmt: &Instr, fn_ir: &FnIR) -> bool {
    match stmt {
        Instr::Assign { src, .. } => rvalue_is_pure(*src, fn_ir),
        Instr::Eval { val, .. } => rvalue_is_pure(*val, fn_ir),
        Instr::StoreIndex1D { .. } => false, // Memory write is a side effect
        Instr::StoreIndex2D { .. } => false, // Memory write is a side effect
        Instr::StoreIndex3D { .. } => false, // Memory write is a side effect
    }
}

/// Checks if an Rvalue (ValueKind) is pure.
pub fn rvalue_is_pure(vid: ValueId, fn_ir: &FnIR) -> bool {
    let mut visiting = HashSet::new();
    rvalue_is_pure_inner(vid, fn_ir, &mut visiting)
}

fn rvalue_is_pure_inner(vid: ValueId, fn_ir: &FnIR, visiting: &mut HashSet<ValueId>) -> bool {
    // Cycles through Phi/self-referential values are not considered provably pure.
    if !visiting.insert(vid) {
        return false;
    }

    let val = &fn_ir.values[vid];
    let pure = match &val.kind {
        ValueKind::Const(_) => true,
        ValueKind::Binary { lhs, rhs, .. } => {
            rvalue_is_pure_inner(*lhs, fn_ir, visiting)
                && rvalue_is_pure_inner(*rhs, fn_ir, visiting)
        }
        ValueKind::Unary { rhs, .. } => rvalue_is_pure_inner(*rhs, fn_ir, visiting),
        ValueKind::Phi { args } => args
            .iter()
            .all(|(v, _)| rvalue_is_pure_inner(*v, fn_ir, visiting)),
        ValueKind::Call { callee, args, .. } => {
            if call_is_pure(callee) {
                args.iter()
                    .all(|a| rvalue_is_pure_inner(*a, fn_ir, visiting))
            } else {
                false
            }
        }
        ValueKind::Len { base } => rvalue_is_pure_inner(*base, fn_ir, visiting),
        ValueKind::Indices { base } => rvalue_is_pure_inner(*base, fn_ir, visiting),
        ValueKind::Range { start, end } => {
            rvalue_is_pure_inner(*start, fn_ir, visiting)
                && rvalue_is_pure_inner(*end, fn_ir, visiting)
        }
        ValueKind::Index1D { base, idx, .. } => {
            rvalue_is_pure_inner(*base, fn_ir, visiting)
                && rvalue_is_pure_inner(*idx, fn_ir, visiting)
        }
        ValueKind::Index2D { base, r, c } => {
            rvalue_is_pure_inner(*base, fn_ir, visiting)
                && rvalue_is_pure_inner(*r, fn_ir, visiting)
                && rvalue_is_pure_inner(*c, fn_ir, visiting)
        }
        ValueKind::Index3D { base, i, j, k } => {
            rvalue_is_pure_inner(*base, fn_ir, visiting)
                && rvalue_is_pure_inner(*i, fn_ir, visiting)
                && rvalue_is_pure_inner(*j, fn_ir, visiting)
                && rvalue_is_pure_inner(*k, fn_ir, visiting)
        }
        _ => false, // Conservative default
    };

    visiting.remove(&vid);
    pure
}

/// Checks if a function call (by name) is pure based on a whitelist.
pub fn call_is_pure(callee: &str) -> bool {
    match callee {
        // Built-in pure functions
        "length"
        | "seq_len"
        | "seq_along"
        | "abs"
        | "sqrt"
        | "sin"
        | "cos"
        | "tan"
        | "asin"
        | "acos"
        | "atan"
        | "atan2"
        | "sinh"
        | "cosh"
        | "tanh"
        | "log"
        | "log10"
        | "log2"
        | "exp"
        | "sign"
        | "floor"
        | "ceiling"
        | "trunc"
        | "dim"
        | "dimnames"
        | "nrow"
        | "ncol"
        | "t"
        | "diag"
        | "round"
        | "gamma"
        | "lgamma"
        | "c"
        | "numeric"
        | "rep.int"
        | "vector"
        | "matrix"
        | "sum"
        | "mean"
        | "var"
        | "sd"
        | "min"
        | "max"
        | "prod"
        | "pmax"
        | "pmin"
        | "colSums"
        | "rowSums"
        | "%*%"
        | "crossprod"
        | "tcrossprod"
        | "rbind"
        | "cbind"
        | "is.na"
        | "is.finite"
        | "rr_field_get"
        | "rr_field_exists"
        | "rr_list_rest"
        | "rr_named_list"
        | "rr_row_sum_range"
        | "rr_col_sum_range"
        | "rr_dim1_sum_range"
        | "rr_dim2_sum_range"
        | "rr_dim3_sum_range"
        | "rr_dim1_reduce_range"
        | "rr_dim2_reduce_range"
        | "rr_dim3_reduce_range"
        | "rr_dim1_read_values"
        | "rr_dim2_read_values"
        | "rr_dim3_read_values"
        | "rr_array3_gather_values"
        | "rr_index_vec_floor"
        | "rr_index1_read_vec"
        | "rr_index1_read_vec_floor"
        | "rr_gather"
        | "rr_wrap_index_vec"
        | "rr_wrap_index_vec_i"
        | "rr_idx_cube_vec_i" => true,
        _ => false,
    }
}

/// Checks if an entire basic block is effect-free (excluding terminator).
pub fn block_is_pure(bid: BlockId, fn_ir: &FnIR) -> bool {
    let block = &fn_ir.blocks[bid];
    block.instrs.iter().all(|i| stmt_is_pure(i, fn_ir))
}

/// Checks if a loop is pure (no side effects in any of its body blocks).
pub fn loop_is_pure(fn_ir: &FnIR, body: &std::collections::HashSet<BlockId>) -> bool {
    for &bid in body {
        if !block_is_pure(bid, fn_ir) {
            return false;
        }
        // Also check if terminator has side effects (usually not, but If/Goto are pure)
        let block = &fn_ir.blocks[bid];
        match &block.term {
            Terminator::Return(Some(v)) => {
                if !rvalue_is_pure(*v, fn_ir) {
                    return false;
                }
            }
            Terminator::If { cond, .. } => {
                if !rvalue_is_pure(*cond, fn_ir) {
                    return false;
                }
            }
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::flow::Facts;
    use crate::utils::Span;

    #[test]
    fn rr_bool_is_not_treated_as_pure_call() {
        assert!(!call_is_pure("rr_bool"));
    }

    #[test]
    fn phi_cycle_does_not_recurse_forever() {
        let mut fn_ir = FnIR::new("phi_cycle".to_string(), vec![]);
        let b0 = fn_ir.add_block();
        fn_ir.entry = b0;
        fn_ir.body_head = b0;

        let phi = fn_ir.add_value(
            ValueKind::Phi { args: Vec::new() },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[phi].kind = ValueKind::Phi {
            args: vec![(phi, b0)],
        };
        fn_ir.values[phi].phi_block = Some(b0);
        fn_ir.blocks[b0].term = Terminator::Return(Some(phi));

        assert!(!rvalue_is_pure(phi, &fn_ir));
    }
}
