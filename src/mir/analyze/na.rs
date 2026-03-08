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
                ValueKind::Index1D { .. } | ValueKind::Index2D { .. } => {
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
        "length" | "seq_len" | "seq_along" => return NaState::Never,
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
        "c" | "sum" | "mean" | "var" | "sd" | "min" | "max" | "prod" | "rr_row_sum_range"
        | "rr_col_sum_range" | "colSums" | "rowSums" | "%*%" | "crossprod" | "tcrossprod" => {
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
