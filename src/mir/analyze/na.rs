use crate::mir::*;
use crate::syntax::ast::Lit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NaState {
    Never,
    Maybe,
    Always,
}

impl NaState {
    // Merge states coming from different control-flow paths (phi/join point).
    pub fn merge_flow(a: NaState, b: NaState) -> NaState {
        use NaState::*;
        match (a, b) {
            (Never, Never) => Never,
            (Always, Always) => Always,
            _ => Maybe,
        }
    }

    // NA propagation for expression operators (binary/call arg aggregation).
    pub fn propagate(a: NaState, b: NaState) -> NaState {
        use NaState::*;
        match (a, b) {
            (Always, _) | (_, Always) => Always,
            (Maybe, _) | (_, Maybe) => Maybe,
            (Never, Never) => Never,
        }
    }
}

pub fn compute_na_states(fn_ir: &FnIR) -> Vec<NaState> {
    let mut states = vec![NaState::Maybe; fn_ir.values.len()];

    // Seed obvious cases
    for (id, val) in fn_ir.values.iter().enumerate() {
        states[id] = match &val.kind {
            ValueKind::Const(Lit::Na) => NaState::Always,
            ValueKind::Const(_) => NaState::Never,
            ValueKind::Len { .. } | ValueKind::Indices { .. } => NaState::Never,
            _ => NaState::Maybe,
        };
    }

    let mut changed = true;
    while changed {
        changed = false;
        for (id, val) in fn_ir.values.iter().enumerate() {
            let new_state = match &val.kind {
                ValueKind::Const(Lit::Na) => NaState::Always,
                ValueKind::Const(_) => NaState::Never,
                ValueKind::Len { .. } | ValueKind::Indices { .. } => NaState::Never,
                ValueKind::Range { start, end } => NaState::propagate(states[*start], states[*end]),
                ValueKind::Binary { lhs, rhs, .. } => {
                    NaState::propagate(states[*lhs], states[*rhs])
                }
                ValueKind::Unary { rhs, .. } => states[*rhs],
                ValueKind::RecordLit { fields } => {
                    let mut acc = NaState::Never;
                    for (_, value) in fields {
                        acc = NaState::propagate(acc, states[*value]);
                    }
                    acc
                }
                ValueKind::FieldGet { base, .. } => states[*base],
                ValueKind::FieldSet { base, value, .. } => {
                    NaState::propagate(states[*base], states[*value])
                }
                ValueKind::Phi { args } => {
                    let mut it = args.iter();
                    let mut acc = if let Some((v, _)) = it.next() {
                        states[*v]
                    } else {
                        NaState::Maybe
                    };
                    for (v, _) in it {
                        acc = NaState::merge_flow(acc, states[*v]);
                    }
                    acc
                }
                ValueKind::Index1D { .. }
                | ValueKind::Index2D { .. }
                | ValueKind::Index3D { .. } => {
                    // Indexing can always produce NA depending on contents.
                    NaState::Maybe
                }
                ValueKind::Call { callee, args, .. } => call_na_behavior(callee, args, &states),
                ValueKind::Intrinsic { args, .. } => {
                    let mut acc = NaState::Never;
                    for a in args {
                        acc = NaState::propagate(acc, states[*a]);
                    }
                    acc
                }
                ValueKind::Param { .. } | ValueKind::Load { .. } | ValueKind::RSymbol { .. } => {
                    NaState::Maybe
                }
            };

            if new_state != states[id] {
                states[id] = new_state;
                changed = true;
            }
        }
    }

    states
}

fn call_na_behavior(callee: &str, args: &[ValueId], states: &[NaState]) -> NaState {
    // Functions that never return NA regardless of args.
    match callee {
        "length" | "seq_len" | "seq_along" | "dim" | "dimnames" | "nrow" | "ncol" | "t" => {
            return NaState::Never;
        }
        _ => {}
    }

    // Functions that propagate NA from their arguments.
    match callee {
        "abs" | "sqrt" | "sin" | "cos" | "tan" | "log" | "exp" | "floor" | "ceiling" | "round"
        | "trunc" => {
            let mut acc = NaState::Never;
            for a in args {
                acc = NaState::propagate(acc, states[*a]);
            }
            return acc;
        }
        "c"
        | "sum"
        | "mean"
        | "var"
        | "sd"
        | "min"
        | "max"
        | "prod"
        | "rr_reduce_range"
        | "rr_can_reduce_range"
        | "rr_tile_map_range"
        | "rr_tile_reduce_range"
        | "rr_row_sum_range"
        | "rr_col_sum_range"
        | "rr_col_reduce_range"
        | "rr_can_col_reduce_range"
        | "rr_tile_col_binop_assign"
        | "rr_tile_col_reduce_range"
        | "rr_matrix_reduce_rect"
        | "rr_can_matrix_reduce_rect"
        | "rr_tile_matrix_binop_assign"
        | "rr_tile_matrix_reduce_rect"
        | "rr_dim1_sum_range"
        | "rr_dim2_sum_range"
        | "rr_dim3_sum_range"
        | "rr_dim1_reduce_range"
        | "rr_can_dim1_reduce_range"
        | "rr_can_array3_reduce_cube"
        | "rr_array3_reduce_cube"
        | "rr_array3_binop_cube_assign"
        | "rr_tile_array3_reduce_cube"
        | "rr_tile_array3_binop_cube_assign"
        | "rr_tile_dim1_binop_assign"
        | "rr_tile_dim1_reduce_range"
        | "rr_dim2_reduce_range"
        | "rr_dim3_reduce_range"
        | "rr_dim1_read_values"
        | "rr_dim2_read_values"
        | "rr_dim3_read_values"
        | "rr_array3_gather_values"
        | "rr_can_same_len"
        | "rr_can_same_or_scalar"
        | "rr_same_matrix_shape_or_scalar"
        | "rr_same_array3_shape_or_scalar"
        | "rr_can_same_matrix_shape_or_scalar"
        | "rr_can_same_array3_shape_or_scalar"
        | "rr_matrix_binop_assign"
        | "colSums"
        | "rowSums"
        | "%*%"
        | "crossprod"
        | "tcrossprod" => {
            let mut acc = NaState::Never;
            for a in args {
                acc = NaState::propagate(acc, states[*a]);
            }
            return acc;
        }
        "diag" | "rbind" | "cbind" => {
            let mut acc = NaState::Never;
            for a in args {
                acc = NaState::propagate(acc, states[*a]);
            }
            return acc;
        }
        _ => {}
    }

    // Unknown calls are conservatively Maybe.
    NaState::Maybe
}
