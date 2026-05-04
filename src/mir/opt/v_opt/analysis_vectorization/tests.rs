use super::super::super::api::{rank_vector_plans, vector_plan_label};
use super::super::super::planning::ExprMapEntry;
use super::{
    BlockStore3DMatch, ResolvedCallInfo, VectorPlan, classify_store_3d_in_block, is_iv_equivalent,
    resolve_call_info,
};
use crate::mir::{BuiltinKind, FnIR, Instr, ValueKind};
use crate::utils::Span;

#[test]
fn rank_prefers_specific_vector_plan_over_generic_map() {
    let mut plans = vec![
        VectorPlan::Map {
            dest: 1,
            src: 2,
            op: crate::syntax::ast::BinOp::Add,
            other: 3,
            shadow_vars: Vec::new(),
        },
        VectorPlan::CondMap {
            dest: 1,
            cond: 4,
            then_val: 5,
            else_val: 6,
            iv_phi: 7,
            start: 8,
            end: 9,
            whole_dest: true,
            shadow_vars: Vec::new(),
        },
    ];
    rank_vector_plans(&mut plans);
    assert_eq!(vector_plan_label(&plans[0]), "cond_map");
}

#[test]
fn rank_prefers_multi_output_expr_map_over_single_expr_map() {
    let mut plans = vec![
        VectorPlan::ExprMap {
            dest: 1,
            expr: 2,
            iv_phi: 3,
            start: 4,
            end: 5,
            whole_dest: true,
            shadow_vars: Vec::new(),
        },
        VectorPlan::MultiExprMap {
            entries: vec![
                ExprMapEntry {
                    dest: 1,
                    expr: 2,
                    whole_dest: true,
                    shadow_vars: Vec::new(),
                },
                ExprMapEntry {
                    dest: 6,
                    expr: 7,
                    whole_dest: true,
                    shadow_vars: Vec::new(),
                },
            ],
            iv_phi: 3,
            start: 4,
            end: 5,
        },
    ];
    rank_vector_plans(&mut plans);
    assert_eq!(vector_plan_label(&plans[0]), "multi_expr_map");
}

#[test]
fn resolve_call_info_canonicalizes_simple_builtin_alias() {
    let mut fn_ir = FnIR::new("alias".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let floor_load = fn_ir.add_value(
        ValueKind::Load {
            var: "floor".to_string(),
        },
        Span::default(),
        crate::mir::Facts::empty(),
        Some("floor".to_string()),
    );
    fn_ir.blocks[entry].instrs.push(crate::mir::Instr::Assign {
        dst: "floor_fn".to_string(),
        src: floor_load,
        span: Span::default(),
    });

    let alias_load = fn_ir.add_value(
        ValueKind::Load {
            var: "floor_fn".to_string(),
        },
        Span::default(),
        crate::mir::Facts::empty(),
        Some("floor_fn".to_string()),
    );
    let arg = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        crate::mir::Facts::empty(),
        None,
    );
    let call = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_call_closure".to_string(),
            args: vec![alias_load, arg],
            names: vec![None, None],
        },
        Span::default(),
        crate::mir::Facts::empty(),
        None,
    );

    let resolved = resolve_call_info(&fn_ir, call).expect("expected call alias to resolve");
    assert_eq!(
        resolved,
        ResolvedCallInfo {
            callee: "floor".to_string(),
            builtin_kind: Some(BuiltinKind::Floor),
            args: vec![arg]
        }
    );
}

#[test]
fn iv_equivalence_budget_bails_out_on_deep_floor_chain() {
    let mut fn_ir = FnIR::new("deep_iv".to_string(), vec!["x".to_string()]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let iv_phi = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        crate::mir::Facts::empty(),
        Some("x".to_string()),
    );

    let mut current = iv_phi;
    for _ in 0..300 {
        current = fn_ir.add_value(
            ValueKind::Call {
                callee: "floor".to_string(),
                args: vec![current],
                names: vec![None],
            },
            Span::default(),
            crate::mir::Facts::empty(),
            None,
        );
    }

    assert!(
        !is_iv_equivalent(&fn_ir, current, iv_phi),
        "expected deep recursive proof to bail out conservatively"
    );
}

#[test]
fn classify_store_3d_rejects_block_with_eval_side_effect() {
    let mut fn_ir = FnIR::new("store3d_eval".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let zero = fn_ir.add_value(
        ValueKind::Const(crate::mir::Lit::Int(0)),
        Span::default(),
        crate::mir::Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(crate::mir::Lit::Int(1)),
        Span::default(),
        crate::mir::Facts::empty(),
        None,
    );
    let base = fn_ir.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        crate::mir::Facts::empty(),
        Some("x".to_string()),
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        crate::mir::Facts::empty(),
        None,
    );

    fn_ir.blocks[entry].instrs.push(Instr::Eval {
        val: impure,
        span: Span::default(),
    });
    fn_ir.blocks[entry].instrs.push(Instr::StoreIndex3D {
        base,
        i: one,
        j: one,
        k: one,
        val: zero,
        span: Span::default(),
    });

    assert!(matches!(
        classify_store_3d_in_block(&fn_ir, entry),
        BlockStore3DMatch::Invalid
    ));
}
