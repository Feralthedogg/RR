use super::common::*;

#[test]
fn constant_safe_partial_self_assign_renders_direct_slice_write() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![1, 2],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("b".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(0)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Const(Lit::Int(192)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("n".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Const(Lit::Int(88)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("i".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Const(Lit::Int(104)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
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
            id: 6,
            kind: ValueKind::Binary {
                op: BinOp::Sub,
                lhs: 4,
                rhs: 3,
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
            id: 7,
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
            id: 8,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 6,
                rhs: 7,
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
            id: 9,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![5, 8],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 10,
            kind: ValueKind::Load {
                var: "b".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("b".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 11,
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
            id: 12,
            kind: ValueKind::Call {
                callee: "rr_assign_slice".to_string(),
                args: vec![10, 11, 4, 9],
                names: vec![None; 4],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("b".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.bind_value_to_var(0, "b");
    backend.bind_var_to_value("b", 0);
    backend.bind_value_to_var(3, "i");
    backend.bind_var_to_value("i", 3);

    let rendered = backend
        .try_render_constant_safe_partial_self_assign("b", 12, &values, &[])
        .expect("constant partial fill should render as direct slice write");
    assert_eq!(rendered, "b[88:104] <- rep.int(1L, ((104L - i) + 1L))");
}

#[test]
fn constant_safe_partial_self_assign_recovers_alias_base_var() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![1, 2],
                names: vec![None; 2],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("b".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(0)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Const(Lit::Int(192)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("n".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Const(Lit::Int(88)),
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
            kind: ValueKind::Const(Lit::Int(104)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
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
            id: 6,
            kind: ValueKind::Binary {
                op: BinOp::Sub,
                lhs: 4,
                rhs: 5,
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
            id: 7,
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
            id: 8,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 6,
                rhs: 7,
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
            id: 9,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![7, 8],
                names: vec![None; 2],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Int, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 10,
            kind: ValueKind::Load {
                var: ".tmp_b".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 11,
            kind: ValueKind::Call {
                callee: "rr_assign_slice".to_string(),
                args: vec![10, 5, 4, 9],
                names: vec![None; 4],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("b".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.bind_value_to_var(0, "b");
    backend.bind_var_to_value("b", 0);
    backend.bind_value_to_var(10, "b");
    backend.bind_value_to_var(3, "i");
    backend.bind_var_to_value("i", 3);

    let rendered = backend
        .try_render_constant_safe_partial_self_assign("b", 11, &values, &[])
        .expect("alias base should still render as direct slice write");
    assert_eq!(rendered, "b[88:104] <- rep.int(1L, ((104L - i) + 1L))");
}

#[test]
fn whole_auto_builtin_call_map_is_emitted_as_direct_vector_call() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![2, 1],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("score".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(8)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Const(Lit::Int(0)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Const(Lit::Str("pmax".to_string())),
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
            kind: ValueKind::Const(Lit::Int(25)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
            kind: ValueKind::Call {
                callee: "c".to_string(),
                args: vec![6],
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
        Value {
            id: 6,
            kind: ValueKind::Const(Lit::Int(1)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 7,
            kind: ValueKind::Load {
                var: "x".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("x".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 8,
            kind: ValueKind::Const(Lit::Float(0.05)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 9,
            kind: ValueKind::Call {
                callee: "rr_call_map_whole_auto".to_string(),
                args: vec![0, 3, 4, 5, 7, 8],
                names: vec![None; 6],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("score".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.bind_value_to_var(0, "score");
    backend.bind_var_to_value("score", 0);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "score".to_string(),
                src: 9,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("whole-auto builtin call-map emission should succeed");

    assert!(backend.output.contains("score <- pmax(x, 0.05)"));
    assert!(!backend.output.contains("rr_call_map_whole_auto("));
}

#[test]
fn whole_auto_pmax_zip_call_map_is_emitted_as_direct_vector_call() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![2, 1],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("score".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(8)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Const(Lit::Int(0)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Const(Lit::Str("pmax".to_string())),
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
            kind: ValueKind::Const(Lit::Int(44)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
            kind: ValueKind::Call {
                callee: "c".to_string(),
                args: vec![6, 7],
                names: vec![None, None],
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
            id: 6,
            kind: ValueKind::Const(Lit::Int(1)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 7,
            kind: ValueKind::Const(Lit::Int(2)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 8,
            kind: ValueKind::Load {
                var: "x".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("x".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 9,
            kind: ValueKind::Load {
                var: "z".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("z".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 10,
            kind: ValueKind::Call {
                callee: "rr_call_map_whole_auto".to_string(),
                args: vec![0, 3, 4, 5, 8, 9],
                names: vec![None; 6],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("score".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.bind_value_to_var(0, "score");
    backend.bind_var_to_value("score", 0);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "score".to_string(),
                src: 10,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("whole-auto pmax zip call-map emission should succeed");

    assert!(backend.output.contains("score <- pmax(x, z)"));
    assert!(!backend.output.contains("rr_call_map_whole_auto("));
}

#[test]
fn whole_range_pmax_zip_call_map_slice_is_emitted_as_direct_vector_call() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![2, 1],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("score".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(8)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Const(Lit::Int(0)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Const(Lit::Int(1)),
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
            kind: ValueKind::Const(Lit::Str("pmax".to_string())),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
            kind: ValueKind::Const(Lit::Int(44)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 6,
            kind: ValueKind::Call {
                callee: "c".to_string(),
                args: vec![3, 7],
                names: vec![None, None],
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
            id: 7,
            kind: ValueKind::Const(Lit::Int(2)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 8,
            kind: ValueKind::Load {
                var: "x".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("x".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 9,
            kind: ValueKind::Load {
                var: "z".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("z".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 10,
            kind: ValueKind::Call {
                callee: "rr_call_map_slice_auto".to_string(),
                args: vec![0, 3, 1, 4, 5, 6, 8, 9],
                names: vec![None; 8],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("score".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.bind_value_to_var(0, "score");
    backend.bind_var_to_value("score", 0);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "score".to_string(),
                src: 10,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("whole-range pmax zip call-map slice emission should succeed");

    assert!(backend.output.contains("score <- pmax(x, z)"));
    assert!(!backend.output.contains("rr_call_map_slice_auto("));
}
