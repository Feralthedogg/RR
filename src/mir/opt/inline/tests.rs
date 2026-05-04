use super::driver::InlineSite;
use super::*;
use crate::mir::flow::Facts;
use crate::syntax::ast::Lit;
use crate::typeck::{PrimTy, TypeState, TypeTerm};
use crate::utils::Span;

pub(crate) fn tiny_fn(name: &str) -> FnIR {
    let mut f = FnIR::new(name.to_string(), vec![]);
    let entry = f.add_block();
    f.entry = entry;
    f.body_head = entry;
    let c = f.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    f.blocks[entry].term = Terminator::Return(Some(c));
    f
}

#[test]
pub(crate) fn inline_growth_cap_is_saturating() {
    let cap = InlineGrowthBudget::growth_cap(100, 25, 0);
    assert!(cap >= 125);
    let huge = InlineGrowthBudget::growth_cap(usize::MAX - 16, 1000, 0);
    assert!(huge >= usize::MAX - 16);
}

#[test]
pub(crate) fn inline_growth_budget_blocks_when_no_growth_allowed() {
    let mut all = FxHashMap::default();
    all.insert("caller".to_string(), tiny_fn("caller"));
    let policy = InlinePolicy {
        max_blocks: 24,
        max_instrs: 160,
        max_cost: 220,
        max_callsite_cost: 240,
        max_kernel_cost: 170,
        max_caller_instrs: 480,
        max_total_instrs: 900,
        max_unit_growth_pct: 0,
        max_fn_growth_pct: 0,
        min_growth_abs: 0,
        allow_loops: false,
    };
    let budget = InlineGrowthBudget::new(&all, &policy);
    let caller = all.get("caller").unwrap();
    let caller_ir = MirInliner::fn_ir_size(caller);
    let caller_limit = budget.caller_limit("caller");
    assert!(!budget.can_inline(caller_ir, caller_limit, 1));
}

pub(crate) fn expression_helper(name: &str, chain_len: usize) -> FnIR {
    let mut f = FnIR::new(name.to_string(), vec!["x".to_string()]);
    let entry = f.add_block();
    f.entry = entry;
    f.body_head = entry;
    let mut current = f.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    for idx in 0..chain_len {
        let c = f.add_value(
            ValueKind::Const(Lit::Int(idx as i64)),
            Span::default(),
            Facts::empty(),
            None,
        );
        current = f.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: current,
                rhs: c,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
    }
    f.blocks[entry].term = Terminator::Return(Some(current));
    f
}

pub(crate) fn caller_returning_helper_call(callee: &str) -> FnIR {
    let mut caller = FnIR::new("caller".to_string(), vec!["arg".to_string()]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let arg = caller.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("arg".to_string()),
    );
    let call = caller.add_value(
        ValueKind::Call {
            callee: callee.to_string(),
            args: vec![arg],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(call));
    caller
}

#[test]
pub(crate) fn o3_selective_inlining_keeps_large_expression_kernel_as_call() {
    let mut all = FxHashMap::default();
    all.insert(
        "large_expr".to_string(),
        expression_helper("large_expr", 96),
    );
    all.insert(
        "caller".to_string(),
        caller_returning_helper_call("large_expr"),
    );

    let changed = MirInliner::new_aggressive().optimize(&mut all);
    assert!(
        !changed,
        "large leaf expression helpers should stay as kernel calls under O3"
    );
    let caller = all.get("caller").expect("caller should remain");
    let ret = match caller.blocks[caller.entry].term {
        Terminator::Return(Some(v)) => v,
        _ => panic!("expected return"),
    };
    assert!(
        matches!(caller.values[ret].kind, ValueKind::Call { .. }),
        "large expression kernel should not inline into caller"
    );
}

#[test]
pub(crate) fn o3_selective_inlining_still_inlines_tiny_expression_kernel() {
    let mut all = FxHashMap::default();
    all.insert("tiny_expr".to_string(), expression_helper("tiny_expr", 2));
    all.insert(
        "caller".to_string(),
        caller_returning_helper_call("tiny_expr"),
    );

    let changed = MirInliner::new_aggressive().optimize(&mut all);
    assert!(changed, "tiny leaf expression helpers should still inline");
    let caller = all.get("caller").expect("caller should remain");
    let ret = match caller.blocks[caller.entry].term {
        Terminator::Return(Some(v)) => v,
        _ => panic!("expected return"),
    };
    assert!(
        !matches!(caller.values[ret].kind, ValueKind::Call { .. }),
        "tiny expression kernel should inline away"
    );
}

#[test]
pub(crate) fn inline_value_calls_rejects_store_index3d_side_effect_helpers() {
    let mut callee = FnIR::new("helper3d".to_string(), vec!["arr".to_string()]);
    let centry = callee.add_block();
    callee.entry = centry;
    callee.body_head = centry;
    let arr = callee.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("arr".to_string()),
    );
    let one = callee.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let zero = callee.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    callee.blocks[centry].instrs.push(Instr::StoreIndex3D {
        base: arr,
        i: one,
        j: one,
        k: one,
        val: zero,
        span: Span::default(),
    });
    callee.blocks[centry].term = Terminator::Return(Some(zero));

    let mut caller = FnIR::new("caller".to_string(), vec!["arr".to_string()]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let carg = caller.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("arr".to_string()),
    );
    let call = caller.add_value(
        ValueKind::Call {
            callee: "helper3d".to_string(),
            args: vec![carg],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(call));

    let mut all = FxHashMap::default();
    all.insert("helper3d".to_string(), callee);
    all.insert("caller".to_string(), caller);

    let changed = MirInliner::new().optimize(&mut all);
    assert!(
        !changed,
        "StoreIndex3D helpers must not inline as pure expressions"
    );
    let caller = all.get("caller").expect("caller should remain present");
    assert!(matches!(
        caller.blocks[entry].term,
        Terminator::Return(Some(v)) if matches!(caller.values[v].kind, ValueKind::Call { .. })
    ));
}

#[test]
pub(crate) fn inline_rejects_helpers_with_nested_calls() {
    let mut callee = FnIR::new("call_wrapper".to_string(), vec!["x".to_string()]);
    let centry = callee.add_block();
    callee.entry = centry;
    callee.body_head = centry;
    let x = callee.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let nested = callee.add_value(
        ValueKind::Call {
            callee: "print".to_string(),
            args: vec![x],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    callee.blocks[centry].instrs.push(Instr::Eval {
        val: nested,
        span: Span::default(),
    });
    callee.blocks[centry].term = Terminator::Return(Some(x));

    let mut caller = FnIR::new("caller".to_string(), vec!["arg".to_string()]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let arg = caller.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("arg".to_string()),
    );
    let call = caller.add_value(
        ValueKind::Call {
            callee: "call_wrapper".to_string(),
            args: vec![arg],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].instrs.push(Instr::Assign {
        dst: "out".to_string(),
        src: call,
        span: Span::default(),
    });
    caller.blocks[entry].term = Terminator::Return(Some(call));

    let mut all = FxHashMap::default();
    all.insert("call_wrapper".to_string(), callee);
    all.insert("caller".to_string(), caller);

    let changed = MirInliner::new().optimize(&mut all);
    assert!(
        !changed,
        "helpers with nested calls should not be chosen for full-program inlining"
    );
    let caller = all.get("caller").expect("caller should remain present");
    let Instr::Assign { src, .. } = &caller.blocks[entry].instrs[0] else {
        panic!("expected original call assignment to remain");
    };
    assert!(
        matches!(caller.values[*src].kind, ValueKind::Call { .. }),
        "nested-call helper must remain as a call"
    );
}

#[test]
pub(crate) fn perform_inline_remaps_record_field_value_ids() {
    let mut callee = FnIR::new("field_helper".to_string(), vec!["x".to_string()]);
    let centry = callee.add_block();
    callee.entry = centry;
    callee.body_head = centry;
    let param_x = callee.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let rec = callee.add_value(
        ValueKind::RecordLit {
            fields: vec![("v".to_string(), param_x)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let ret = callee.add_value(
        ValueKind::FieldGet {
            base: rec,
            field: "v".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    callee.values[ret].value_ty = TypeState::scalar(PrimTy::Double, true);
    callee.values[ret].value_term = TypeTerm::Double;
    callee.values[ret].escape = EscapeStatus::Local;
    callee.blocks[centry].instrs.push(Instr::Assign {
        dst: "out".to_string(),
        src: ret,
        span: Span::default(),
    });
    callee.blocks[centry].instrs.push(Instr::Eval {
        val: ret,
        span: Span::default(),
    });
    callee.blocks[centry].term = Terminator::Return(Some(ret));

    let mut caller = FnIR::new("caller".to_string(), vec!["arg".to_string()]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let c1 = caller.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let arg = caller.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("arg".to_string()),
    );
    let call = caller.add_value(
        ValueKind::Call {
            callee: "field_helper".to_string(),
            args: vec![arg],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let sum = caller.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: c1,
            rhs: call,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].instrs.push(Instr::Assign {
        dst: "y".to_string(),
        src: call,
        span: Span::default(),
    });
    caller.blocks[entry].term = Terminator::Return(Some(sum));

    let inliner = MirInliner::new();
    inliner.perform_inline(
        &mut caller,
        &callee,
        InlineSite {
            call_block: entry,
            instr_idx: 0,
            call_args: &[arg],
            call_val_target: call,
            call_dst: Some("y".to_string()),
            call_span: Span::default(),
        },
    );

    let ret_id = caller
        .blocks
        .iter()
        .find_map(|blk| match blk.term {
            Terminator::Return(Some(v)) => Some(v),
            _ => None,
        })
        .expect("expected return after inline");
    let ValueKind::Binary { rhs, .. } = caller.values[ret_id].kind else {
        panic!("expected return sum to stay binary");
    };
    let ValueKind::FieldGet { base, .. } = caller.values[rhs].kind else {
        panic!("expected inlined field get");
    };
    assert_eq!(
        caller.values[rhs].value_ty,
        TypeState::scalar(PrimTy::Double, true),
        "full-function inline should preserve cloned value TypeState"
    );
    assert_eq!(
        caller.values[rhs].value_term,
        TypeTerm::Double,
        "full-function inline should preserve cloned value TypeTerm"
    );
    assert_eq!(
        caller.values[rhs].escape,
        EscapeStatus::Local,
        "full-function inline should preserve cloned escape metadata"
    );
    let ValueKind::RecordLit { ref fields } = caller.values[base].kind else {
        panic!("expected inlined record literal");
    };
    assert_eq!(
        fields[0].1, arg,
        "record field should remap to caller arg, not stale callee id"
    );
}

#[test]
pub(crate) fn inline_value_calls_supports_record_field_helpers() {
    let mut callee = FnIR::new("field_expr_helper".to_string(), vec!["x".to_string()]);
    let centry = callee.add_block();
    callee.entry = centry;
    callee.body_head = centry;
    let param_x = callee.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let rec = callee.add_value(
        ValueKind::RecordLit {
            fields: vec![("v".to_string(), param_x)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let ret = callee.add_value(
        ValueKind::FieldGet {
            base: rec,
            field: "v".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    callee.blocks[centry].term = Terminator::Return(Some(ret));

    let mut caller = FnIR::new("caller".to_string(), vec!["arg".to_string()]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let arg = caller.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("arg".to_string()),
    );
    let one = caller.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let call = caller.add_value(
        ValueKind::Call {
            callee: "field_expr_helper".to_string(),
            args: vec![arg],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let sum = caller.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: call,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(sum));

    let inliner = MirInliner::new();
    let ret = inliner
        .can_inline_expr(&callee)
        .expect("pure field helper should be expression-inlineable");
    let replacement = inliner
        .inline_call_value(&mut caller, call, &callee, ret, &[arg])
        .expect("field helper should clone into caller value graph");
    inliner.replace_uses(&mut caller, call, replacement);

    let ret_id = match caller.blocks[entry].term {
        Terminator::Return(Some(v)) => v,
        _ => panic!("expected return"),
    };
    let ValueKind::Binary { lhs, .. } = caller.values[ret_id].kind else {
        panic!("expected return sum");
    };
    let ValueKind::FieldGet { base, .. } = caller.values[lhs].kind else {
        panic!("expected inlined field get");
    };
    let ValueKind::RecordLit { ref fields } = caller.values[base].kind else {
        panic!("expected inlined record literal");
    };
    assert_eq!(
        fields[0].1, arg,
        "inlined field helper should remap record payload to caller arg"
    );
}

#[test]
pub(crate) fn inline_value_calls_supports_intrinsic_helpers() {
    let mut callee = FnIR::new("intrinsic_expr_helper".to_string(), vec!["x".to_string()]);
    let centry = callee.add_block();
    callee.entry = centry;
    callee.body_head = centry;
    let param_x = callee.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let ret = callee.add_value(
        ValueKind::Intrinsic {
            op: IntrinsicOp::VecAbsF64,
            args: vec![param_x],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    callee.values[ret].value_ty = TypeState::vector(PrimTy::Double, true);
    callee.values[ret].value_term = TypeTerm::Vector(Box::new(TypeTerm::Double));
    callee.values[ret].escape = EscapeStatus::Local;
    callee.blocks[centry].term = Terminator::Return(Some(ret));

    let mut caller = FnIR::new("caller".to_string(), vec!["arg".to_string()]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let arg = caller.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("arg".to_string()),
    );
    let one = caller.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let call = caller.add_value(
        ValueKind::Call {
            callee: "intrinsic_expr_helper".to_string(),
            args: vec![arg],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let sum = caller.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: call,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(sum));

    let inliner = MirInliner::new();
    let ret = inliner
        .can_inline_expr(&callee)
        .expect("pure intrinsic helper should be expression-inlineable");
    let replacement = inliner
        .inline_call_value(&mut caller, call, &callee, ret, &[arg])
        .expect("intrinsic helper should clone into caller value graph");
    inliner.replace_uses(&mut caller, call, replacement);

    let ret_id = match caller.blocks[entry].term {
        Terminator::Return(Some(v)) => v,
        _ => panic!("expected return"),
    };
    let ValueKind::Binary { lhs, .. } = caller.values[ret_id].kind else {
        panic!("expected return sum");
    };
    let ValueKind::Intrinsic { ref args, .. } = caller.values[lhs].kind else {
        panic!("expected inlined intrinsic");
    };
    assert_eq!(
        caller.values[lhs].value_ty,
        TypeState::vector(PrimTy::Double, true),
        "expression inline should preserve cloned value TypeState"
    );
    assert_eq!(
        caller.values[lhs].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double)),
        "expression inline should preserve cloned value TypeTerm"
    );
    assert_eq!(
        caller.values[lhs].escape,
        EscapeStatus::Local,
        "expression inline should preserve cloned escape metadata"
    );
    assert_eq!(
        args.as_slice(),
        &[arg],
        "inlined intrinsic helper should remap args to caller"
    );
}

#[test]
pub(crate) fn inline_value_calls_supports_fieldset_helpers() {
    let mut callee = FnIR::new("fieldset_expr_helper".to_string(), vec!["x".to_string()]);
    let centry = callee.add_block();
    callee.entry = centry;
    callee.body_head = centry;
    let param_x = callee.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let zero = callee.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let ret = callee.add_value(
        ValueKind::FieldSet {
            base: param_x,
            field: "v".to_string(),
            value: zero,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    callee.blocks[centry].term = Terminator::Return(Some(ret));

    let mut caller = FnIR::new("caller".to_string(), vec!["arg".to_string()]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let arg = caller.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("arg".to_string()),
    );
    let call = caller.add_value(
        ValueKind::Call {
            callee: "fieldset_expr_helper".to_string(),
            args: vec![arg],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(call));

    let inliner = MirInliner::new();
    let ret = inliner
        .can_inline_expr(&callee)
        .expect("pure fieldset helper should be expression-inlineable");
    let replacement = inliner
        .inline_call_value(&mut caller, call, &callee, ret, &[arg])
        .expect("fieldset helper should clone into caller value graph");
    inliner.replace_uses(&mut caller, call, replacement);

    let ret_id = match caller.blocks[entry].term {
        Terminator::Return(Some(v)) => v,
        _ => panic!("expected return"),
    };
    let ValueKind::FieldSet {
        base,
        ref field,
        value,
    } = caller.values[ret_id].kind
    else {
        panic!("expected inlined fieldset");
    };
    assert_eq!(base, arg, "fieldset base should remap to caller arg");
    assert_eq!(field, "v");
    assert!(matches!(
        caller.values[value].kind,
        ValueKind::Const(Lit::Int(0))
    ));
}

#[test]
pub(crate) fn inline_value_calls_supports_index3d_helpers() {
    let mut callee = FnIR::new("index3d_expr_helper".to_string(), vec!["arr".to_string()]);
    let centry = callee.add_block();
    callee.entry = centry;
    callee.body_head = centry;
    let arr = callee.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("arr".to_string()),
    );
    let one = callee.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let ret = callee.add_value(
        ValueKind::Index3D {
            base: arr,
            i: one,
            j: one,
            k: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    callee.blocks[centry].term = Terminator::Return(Some(ret));

    let mut caller = FnIR::new("caller".to_string(), vec!["arr".to_string()]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let arg = caller.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("arr".to_string()),
    );
    let call = caller.add_value(
        ValueKind::Call {
            callee: "index3d_expr_helper".to_string(),
            args: vec![arg],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(call));

    let inliner = MirInliner::new();
    let ret = inliner
        .can_inline_expr(&callee)
        .expect("pure index3d helper should be expression-inlineable");
    let replacement = inliner
        .inline_call_value(&mut caller, call, &callee, ret, &[arg])
        .expect("index3d helper should clone into caller value graph");
    inliner.replace_uses(&mut caller, call, replacement);

    let ret_id = match caller.blocks[entry].term {
        Terminator::Return(Some(v)) => v,
        _ => panic!("expected return"),
    };
    let ValueKind::Index3D { base, i, j, k } = caller.values[ret_id].kind else {
        panic!("expected inlined index3d");
    };
    assert_eq!(base, arg, "index3d base should remap to caller arg");
    assert_eq!(i, j);
    assert_eq!(j, k);
    assert!(matches!(
        caller.values[i].kind,
        ValueKind::Const(Lit::Int(1))
    ));
}
