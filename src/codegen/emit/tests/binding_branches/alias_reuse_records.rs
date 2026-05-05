use super::*;
#[test]
pub(crate) fn field_get_reuses_live_plain_symbol_alias_for_equivalent_record_lit_base() {
    let mut backend = RBackend::new();
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
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::RecordLit {
                fields: vec![("width".to_string(), 0)],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("cfg".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::NamedList(vec![("width".to_string(), TypeTerm::Any)]),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::RecordLit {
                fields: vec![("width".to_string(), 0)],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::NamedList(vec![("width".to_string(), TypeTerm::Any)]),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::FieldGet {
                base: 2,
                field: "width".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("width_copy".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "cfg".to_string(),
                src: 1,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("cfg assign should emit");
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "width_copy".to_string(),
                src: 3,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("field get should emit");

    assert!(
        backend.output.contains("width_copy <- cfg[[\"width\"]]"),
        "{}",
        backend.output
    );
    assert!(
        !backend
            .output
            .contains("width_copy <- list(width = a)[[\"width\"]]"),
        "{}",
        backend.output
    );
}

#[test]
pub(crate) fn nested_binary_expr_reuses_live_plain_symbol_alias() {
    let mut backend = RBackend::new();
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
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "b".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("b".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("sum_ab".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Load {
                var: "c".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("c".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 2,
                rhs: 3,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("total".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sum_ab");
    backend.bind_value_to_var(2, "sum_ab");
    backend.bind_var_to_value("sum_ab", 2);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "total".to_string(),
                src: 4,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("nested binary assign should emit");

    assert!(
        backend.output.contains("total <- (sum_ab + c)"),
        "{}",
        backend.output
    );
    assert!(
        !backend.output.contains("total <- ((a + b) + c)"),
        "{}",
        backend.output
    );
}

#[test]
pub(crate) fn len_expr_reuses_live_plain_symbol_alias() {
    let mut backend = RBackend::new();
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
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "b".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("b".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("sum_ab".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Len { base: 2 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("n".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Int,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sum_ab");
    backend.bind_value_to_var(2, "sum_ab");
    backend.bind_var_to_value("sum_ab", 2);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "n".to_string(),
                src: 3,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("len assign should emit");

    assert!(
        backend.output.contains("n <- length(sum_ab)"),
        "{}",
        backend.output
    );
    assert!(
        !backend.output.contains("n <- length((a + b))"),
        "{}",
        backend.output
    );
}

#[test]
pub(crate) fn return_reuses_live_plain_symbol_alias() {
    let mut backend = RBackend::new();
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
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "b".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("b".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("sum_ab".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sum_ab");
    backend.bind_value_to_var(2, "sum_ab");
    backend.bind_var_to_value("sum_ab", 2);

    backend
        .emit_term(&Terminator::Return(Some(2)), &values, &[])
        .expect("return should emit");

    assert!(
        backend.output.contains("return(sum_ab)"),
        "{}",
        backend.output
    );
    assert!(
        !backend.output.contains("return((a + b))"),
        "{}",
        backend.output
    );
}

#[test]
pub(crate) fn return_equivalent_binary_expr_reuses_live_plain_symbol_alias() {
    let mut backend = RBackend::new();
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
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "b".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("b".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("sum_ab".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sum_ab");
    backend.bind_value_to_var(2, "sum_ab");
    backend.bind_var_to_value("sum_ab", 2);

    backend
        .emit_term(&Terminator::Return(Some(3)), &values, &[])
        .expect("return should emit");

    assert!(
        backend.output.contains("return(sum_ab)"),
        "{}",
        backend.output
    );
    assert!(
        !backend.output.contains("return((a + b))"),
        "{}",
        backend.output
    );
}

#[test]
pub(crate) fn unary_not_is_finite_reuses_live_plain_symbol_alias() {
    let mut backend = RBackend::new();
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
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "b".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("b".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("sum_ab".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Call {
                callee: "is.finite".to_string(),
                args: vec![2],
                names: vec![None],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Logical,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sum_ab");
    backend.bind_value_to_var(2, "sum_ab");
    backend.bind_var_to_value("sum_ab", 2);

    let rendered = backend.resolve_unary_expr(UnaryOp::Not, 3, &values, &[]);
    assert_eq!(rendered, "!(is.finite(sum_ab))");
}

#[test]
pub(crate) fn matrix_intrinsic_reuses_live_plain_symbol_alias() {
    let mut backend = RBackend::new();
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
            value_ty: TypeState::matrix(PrimTy::Double, false),
            value_term: TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "b".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("b".to_string()),
            phi_block: None,
            value_ty: TypeState::matrix(PrimTy::Double, false),
            value_term: TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("sum_m".to_string()),
            phi_block: None,
            value_ty: TypeState::matrix(PrimTy::Double, false),
            value_term: TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Load {
                var: "c".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("c".to_string()),
            phi_block: None,
            value_ty: TypeState::matrix(PrimTy::Double, false),
            value_term: TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sum_m");
    backend.bind_value_to_var(2, "sum_m");
    backend.bind_var_to_value("sum_m", 2);

    let rendered = backend.resolve_intrinsic_expr(IntrinsicOp::VecAddF64, &[2, 3], &values, &[]);
    assert_eq!(rendered, "(sum_m + c)");
}

#[test]
pub(crate) fn record_lit_reuses_live_plain_symbol_alias() {
    let mut backend = RBackend::new();
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
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "b".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("b".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("sum_ab".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::RecordLit {
                fields: vec![("width".to_string(), 2)],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("cfg".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::NamedList(vec![("width".to_string(), TypeTerm::Any)]),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sum_ab");
    backend.bind_value_to_var(2, "sum_ab");
    backend.bind_var_to_value("sum_ab", 2);

    let rendered = backend.resolve_val(3, &values, &[], false);
    assert_eq!(rendered, "list(width = sum_ab)");
}

#[test]
pub(crate) fn record_lit_reuses_live_plain_symbol_alias_for_equivalent_scalar_expr() {
    let mut backend = RBackend::new();
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
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "b".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("b".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("sum_ab".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::RecordLit {
                fields: vec![("width".to_string(), 3)],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("cfg".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::NamedList(vec![("width".to_string(), TypeTerm::Any)]),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sum_ab");
    backend.bind_value_to_var(2, "sum_ab");
    backend.bind_var_to_value("sum_ab", 2);

    let rendered = backend.resolve_val(4, &values, &[], false);
    assert_eq!(rendered, "list(width = sum_ab)");
}

#[test]
pub(crate) fn field_set_reuses_live_plain_symbol_alias_for_rhs() {
    let mut backend = RBackend::new();
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
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "b".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("b".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("sum_ab".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Load {
                var: "cfg".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("cfg".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::NamedList(vec![("width".to_string(), TypeTerm::Any)]),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
            kind: ValueKind::FieldSet {
                base: 4,
                field: "width".to_string(),
                value: 3,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("cfg".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::NamedList(vec![("width".to_string(), TypeTerm::Any)]),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sum_ab");
    backend.bind_value_to_var(2, "sum_ab");
    backend.bind_var_to_value("sum_ab", 2);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "cfg".to_string(),
                src: 5,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("field set emission should succeed");

    assert!(
        backend.output.contains("cfg[[\"width\"]] <- sum_ab"),
        "{}",
        backend.output
    );
    assert!(
        !backend.output.contains("cfg[[\"width\"]] <- (a + b)"),
        "{}",
        backend.output
    );
}

#[test]
pub(crate) fn call_args_reuse_live_plain_symbol_alias_for_identical_scalar_exprs() {
    let mut backend = RBackend::new();
    let int_scalar_ty = TypeState::scalar(PrimTy::Int, false);
    let int_scalar_facts = Facts::new(Facts::INT_SCALAR, crate::mir::flow::Interval::BOTTOM);
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
            value_ty: int_scalar_ty,
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "b".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("b".to_string()),
            phi_block: None,
            value_ty: int_scalar_ty,
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: int_scalar_facts,
            origin_var: Some("sum_ab".to_string()),
            phi_block: None,
            value_ty: int_scalar_ty,
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: int_scalar_facts,
            origin_var: None,
            phi_block: None,
            value_ty: int_scalar_ty,
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Call {
                callee: "sqrt".to_string(),
                args: vec![3],
                names: vec![None],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sum_ab");
    backend.bind_value_to_var(2, "sum_ab");
    backend.bind_var_to_value("sum_ab", 2);

    let rendered = backend.resolve_val(4, &values, &[], false);
    assert_eq!(rendered, "sqrt(sum_ab)");
}
