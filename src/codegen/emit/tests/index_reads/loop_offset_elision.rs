use super::*;
#[test]
pub(crate) fn index1d_expr_elides_when_loop_offset_is_proven_safe() {
    let mut backend = RBackend::new();
    backend
        .loop_analysis
        .active_scalar_loop_indices
        .push(ActiveScalarLoopIndex {
            var: "i".to_string(),
            start_min: 2,
            cmp: ScalarLoopCmp::Lt,
        });
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "a".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("a".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "i".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("i".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Const(Lit::Int(1)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Binary {
                op: BinOp::Sub,
                lhs: 1,
                rhs: 2,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Index1D {
                base: 0,
                idx: 3,
                is_safe: false,
                is_na_safe: false,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Double, false),
            value_term: TypeTerm::Double,
            escape: EscapeStatus::Unknown,
        },
    ];

    let rendered = backend.resolve_val(4, &values, &[], false);
    assert_eq!(rendered, "a[(i - 1L)]");
}
