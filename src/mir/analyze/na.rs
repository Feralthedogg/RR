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
                ValueKind::FieldGet { base, field } => {
                    field_value_na_state(*base, field, &fn_ir.values, &states)
                }
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

fn field_value_na_state(
    base: ValueId,
    field: &str,
    values: &[Value],
    states: &[NaState],
) -> NaState {
    let Some(values_for_field) = collect_record_field_values(values, base, field) else {
        return states[base];
    };
    let mut it = values_for_field.into_iter();
    let Some(first) = it.next() else {
        return states[base];
    };
    let mut acc = states[first];
    for value in it {
        acc = NaState::merge_flow(acc, states[value]);
    }
    acc
}

fn call_na_behavior(callee: &str, args: &[ValueId], states: &[NaState]) -> NaState {
    if call_never_returns_na(callee) {
        return NaState::Never;
    }
    if call_propagates_arg_na(callee) {
        return propagate_arg_na(args, states);
    }
    unknown_call_na_state()
}

fn call_never_returns_na(callee: &str) -> bool {
    matches!(
        callee,
        "length" | "seq_len" | "seq_along" | "dim" | "dimnames" | "nrow" | "ncol" | "t"
    )
}

fn call_propagates_arg_na(callee: &str) -> bool {
    call_is_na_preserving_math(callee)
        || call_is_na_preserving_reduction(callee)
        || matches!(callee, "diag" | "rbind" | "cbind")
}

fn call_is_na_preserving_math(callee: &str) -> bool {
    matches!(
        callee,
        "abs"
            | "sqrt"
            | "sin"
            | "cos"
            | "tan"
            | "log"
            | "exp"
            | "floor"
            | "ceiling"
            | "round"
            | "trunc"
    )
}

fn call_is_na_preserving_reduction(callee: &str) -> bool {
    matches!(
        callee,
        "c" | "sum"
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
            | "tcrossprod"
    )
}

fn propagate_arg_na(args: &[ValueId], states: &[NaState]) -> NaState {
    let mut acc = NaState::Never;
    for arg in args {
        acc = NaState::propagate(acc, states[*arg]);
    }
    acc
}

fn unknown_call_na_state() -> NaState {
    NaState::Maybe
}

#[cfg(test)]
mod tests {
    use super::{NaState, compute_na_states};
    use crate::mir::{Facts, FnIR, Lit, ValueKind};
    use crate::utils::Span;

    #[test]
    fn field_get_does_not_inherit_na_from_other_record_fields() {
        let mut f = FnIR::new("na_field_get_record".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let ok = f.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let na = f.add_value(
            ValueKind::Const(Lit::Na),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let record = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), ok), ("y".to_string(), na)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let field = f.add_value(
            ValueKind::FieldGet {
                base: record,
                field: "x".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        let states = compute_na_states(&f);
        assert_eq!(states[field], NaState::Never);
    }

    #[test]
    fn field_get_tracks_fieldset_override_precisely() {
        let mut f = FnIR::new("na_field_get_fieldset".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let na = f.add_value(
            ValueKind::Const(Lit::Na),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let record = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), na)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let updated = f.add_value(
            ValueKind::FieldSet {
                base: record,
                field: "x".to_string(),
                value: one,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let field = f.add_value(
            ValueKind::FieldGet {
                base: updated,
                field: "x".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        let states = compute_na_states(&f);
        assert_eq!(states[field], NaState::Never);
    }

    #[test]
    fn nested_field_get_does_not_inherit_na_from_sibling_fields() {
        let mut f = FnIR::new("na_nested_field_get".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let ok = f.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let na = f.add_value(
            ValueKind::Const(Lit::Na),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let inner = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), ok), ("y".to_string(), na)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let outer = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("inner".to_string(), inner)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let inner_field = f.add_value(
            ValueKind::FieldGet {
                base: outer,
                field: "inner".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let field = f.add_value(
            ValueKind::FieldGet {
                base: inner_field,
                field: "x".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        let states = compute_na_states(&f);
        assert_eq!(states[field], NaState::Never);
    }

    #[test]
    fn field_get_reads_precise_state_through_phi_merged_records() {
        let mut f = FnIR::new("na_phi_field_get".to_string(), Vec::new());
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let ok = f.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let na = f.add_value(
            ValueKind::Const(Lit::Na),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let rec_a = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), ok), ("y".to_string(), na)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let rec_b = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), ok), ("y".to_string(), na)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(rec_a, left), (rec_b, right)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        f.values[phi].phi_block = Some(merge);
        let field = f.add_value(
            ValueKind::FieldGet {
                base: phi,
                field: "x".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        let states = compute_na_states(&f);
        assert_eq!(states[field], NaState::Never);
    }

    #[test]
    fn field_get_merges_state_through_phi_merged_records() {
        let mut f = FnIR::new("na_phi_field_join".to_string(), Vec::new());
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let ok = f.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let na = f.add_value(
            ValueKind::Const(Lit::Na),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let rec_a = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), ok)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let rec_b = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), na)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(rec_a, left), (rec_b, right)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        f.values[phi].phi_block = Some(merge);
        let field = f.add_value(
            ValueKind::FieldGet {
                base: phi,
                field: "x".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        let states = compute_na_states(&f);
        assert_eq!(states[field], NaState::Maybe);
    }
}
