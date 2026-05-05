use super::super::common::*;
use super::*;

#[test]
pub(crate) fn helper_heavy_whole_auto_builtin_call_map_stays_runtime_guarded() {
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
            origin_var: Some("out".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(6)),
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
            kind: ValueKind::Const(Lit::Str("abs".to_string())),
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
                var: "src".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("src".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 8,
            kind: ValueKind::Load {
                var: "idx".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("idx".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 9,
            kind: ValueKind::Call {
                callee: "rr_gather".to_string(),
                args: vec![7, 8],
                names: vec![None, None],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 10,
            kind: ValueKind::Call {
                callee: "rr_call_map_whole_auto".to_string(),
                args: vec![0, 3, 4, 5, 9],
                names: vec![None; 5],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("out".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.bind_value_to_var(0, "out");
    backend.bind_var_to_value("out", 0);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "out".to_string(),
                src: 10,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("helper-heavy whole-auto builtin call-map emission should succeed");

    assert!(backend.output.contains("out <- rr_call_map_whole_auto("));
    assert!(!backend.output.contains("out <- abs("));
}

#[test]
pub(crate) fn whole_dest_end_known_var_matches_len_alias_expr() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Param { index: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("xs".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Len { base: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("n".to_string()),
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
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![2, 1],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("out".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Len { base: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "n".to_string(),
                src: 1,
                span: Span::dummy(),
            },
            &values,
            &["xs".to_string()],
        )
        .expect("len alias assignment should emit");
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "out".to_string(),
                src: 3,
                span: Span::dummy(),
            },
            &values,
            &["xs".to_string()],
        )
        .expect("fresh allocation assignment should emit");

    assert!(backend.whole_dest_end_matches_known_var("out", 4, &values, &["xs".to_string()]));
}

#[test]
pub(crate) fn whole_range_sym17_allocator_like_assign_resolves_direct_rhs() {
    let mut backend = backend_with_sym17_fresh();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Param { index: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("size".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
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
            id: 2,
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
            id: 3,
            kind: ValueKind::Call {
                callee: "Sym_17".to_string(),
                args: vec![0, 1, 2],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("y".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
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
            id: 5,
            kind: ValueKind::Param { index: 1 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("src".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 6,
            kind: ValueKind::Call {
                callee: "rr_assign_slice".to_string(),
                args: vec![3, 4, 0, 5],
                names: vec![None; 4],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("y".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "y".to_string(),
                src: 3,
                span: Span::dummy(),
            },
            &values,
            &["size".to_string(), "src".to_string()],
        )
        .expect("fresh allocator assignment should emit");

    let rendered = backend
        .try_resolve_whole_range_self_assign_rhs(
            "y",
            6,
            &values,
            &["size".to_string(), "src".to_string()],
        )
        .expect("whole-range Sym_17 replay should resolve directly");
    assert_eq!(rendered, "src");
}

#[test]
pub(crate) fn singleton_size_boundary_assign_collapses_rep_int_wrapper_to_scalar_rhs() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Param { index: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("size".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(6)),
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
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![1, 0],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("nf".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Const(Lit::Int(5)),
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
            id: 5,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![3, 4],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 6,
            kind: ValueKind::Call {
                callee: "rr_assign_slice".to_string(),
                args: vec![2, 0, 0, 5],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("nf".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "nf".to_string(),
                src: 2,
                span: Span::dummy(),
            },
            &values,
            &["size".to_string()],
        )
        .expect("base alloc should emit");

    let rendered = backend.resolve_rr_idx_cube_vec_arg_expr(6, &values, &["size".to_string()]);
    assert_eq!(rendered, "replace(nf, size, 5L)");
}

#[test]
pub(crate) fn singleton_size_boundary_assign_rep_int_scalar_reuses_live_plain_symbol_alias() {
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
            kind: ValueKind::Param { index: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("size".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![2, 3],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("nf".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
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
            id: 6,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![2, 5],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 7,
            kind: ValueKind::Call {
                callee: "rr_assign_slice".to_string(),
                args: vec![4, 3, 3, 6],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("nf".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sum_ab");
    backend.bind_value_to_var(2, "sum_ab");
    backend.bind_var_to_value("sum_ab", 2);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "nf".to_string(),
                src: 4,
                span: Span::dummy(),
            },
            &values,
            &["size".to_string()],
        )
        .expect("base alloc should emit");

    let rendered = backend.resolve_rr_idx_cube_vec_arg_expr(7, &values, &["size".to_string()]);
    assert_eq!(rendered, "replace(nf, size, sum_ab)");
}

#[test]
pub(crate) fn singleton_size_boundary_assign_direct_scalar_len_reuses_live_plain_symbol_alias() {
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
        Value {
            id: 4,
            kind: ValueKind::Param { index: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("size".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![3, 4],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("nf".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 6,
            kind: ValueKind::Call {
                callee: "rr_assign_slice".to_string(),
                args: vec![5, 4, 4, 3],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("nf".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sum_ab");
    backend.bind_value_to_var(2, "sum_ab");
    backend.bind_var_to_value("sum_ab", 2);
    backend.note_var_write("n");
    backend.bind_value_to_var(3, "n");
    backend.bind_var_to_value("n", 3);

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

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "nf".to_string(),
                src: 5,
                span: Span::dummy(),
            },
            &values,
            &["size".to_string()],
        )
        .expect("base alloc should emit");

    let rendered = backend.resolve_rr_idx_cube_vec_arg_expr(6, &values, &["size".to_string()]);
    assert_eq!(rendered, "replace(nf, size, n)");
}

#[test]
pub(crate) fn singleton_size_boundary_assign_direct_scalar_binary_reuses_live_plain_symbol_alias() {
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
            value_term: TypeTerm::Double,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Param { index: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("size".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![2, 3],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("nf".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
            kind: ValueKind::Call {
                callee: "rr_assign_slice".to_string(),
                args: vec![4, 3, 3, 2],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("nf".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sum_ab");
    backend.bind_value_to_var(2, "sum_ab");
    backend.bind_var_to_value("sum_ab", 2);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "nf".to_string(),
                src: 4,
                span: Span::dummy(),
            },
            &values,
            &["size".to_string()],
        )
        .expect("base alloc should emit");

    let rendered = backend.resolve_rr_idx_cube_vec_arg_expr(5, &values, &["size".to_string()]);
    assert_eq!(rendered, "rr_assign_slice(nf, size, size, sum_ab)");
}

#[test]
pub(crate) fn singleton_size_boundary_assign_direct_scalar_pure_call_reuses_live_plain_symbol_alias()
 {
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
            kind: ValueKind::Call {
                callee: "abs".to_string(),
                args: vec![0],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("abs_a".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Double,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Param { index: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("size".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![1, 2],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("nf".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Call {
                callee: "rr_assign_slice".to_string(),
                args: vec![3, 2, 2, 1],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("nf".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("abs_a");
    backend.bind_value_to_var(1, "abs_a");
    backend.bind_var_to_value("abs_a", 1);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "nf".to_string(),
                src: 3,
                span: Span::dummy(),
            },
            &values,
            &["size".to_string()],
        )
        .expect("base alloc should emit");

    let rendered = backend.resolve_rr_idx_cube_vec_arg_expr(4, &values, &["size".to_string()]);
    assert_eq!(rendered, "rr_assign_slice(nf, size, size, abs_a)");
}
