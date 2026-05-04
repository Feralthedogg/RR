use super::*;
use crate::mir::flow::Facts;
use crate::syntax::ast::Lit;
use crate::utils::Span;

pub(crate) fn one_block_fn(name: &str) -> FnIR {
    let mut f = FnIR::new(name.to_string(), vec![]);
    let entry = f.add_block();
    f.entry = entry;
    f.body_head = entry;
    f
}

#[test]
pub(crate) fn dce_preserves_eval_with_nested_side_effect_inside_pure_call() {
    let mut fn_ir = one_block_fn("dce_nested_pure_call_eval");
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let wrapped = fn_ir.add_value(
        ValueKind::Call {
            callee: "length".to_string(),
            args: vec![impure],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
        val: wrapped,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(
        !changed,
        "the eval should stay live rather than being deleted or rewritten"
    );
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
        [Instr::Eval { val, .. }] if *val == wrapped
    ));
}

#[test]
pub(crate) fn dce_demotes_dead_assign_with_nested_record_field_side_effect_to_eval() {
    let mut fn_ir = one_block_fn("dce_nested_record_field_assign");
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let record = fn_ir.add_value(
        ValueKind::RecordLit {
            fields: vec![("x".to_string(), impure)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let field = fn_ir.add_value(
        ValueKind::FieldGet {
            base: record,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: ".tachyon_dead".to_string(),
        src: field,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(
        changed,
        "dead assign should be rewritten to eval, not dropped"
    );
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
        [Instr::Eval { val, .. }] if *val == field
    ));
}

#[test]
pub(crate) fn dce_preserves_eval_with_nested_side_effect_inside_intrinsic() {
    let mut fn_ir = one_block_fn("dce_nested_intrinsic_eval");
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let wrapped = fn_ir.add_value(
        ValueKind::Intrinsic {
            op: IntrinsicOp::VecAbsF64,
            args: vec![impure],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
        val: wrapped,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(
        !changed,
        "the eval should stay live when an intrinsic argument has side effects"
    );
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
        [Instr::Eval { val, .. }] if *val == wrapped
    ));
}

#[test]
pub(crate) fn dce_demotes_dead_assign_with_nested_fieldset_side_effect_to_eval() {
    let mut fn_ir = one_block_fn("dce_nested_fieldset_assign");
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let record = fn_ir.add_value(
        ValueKind::RecordLit {
            fields: vec![("x".to_string(), one)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let updated = fn_ir.add_value(
        ValueKind::FieldSet {
            base: record,
            field: "x".to_string(),
            value: impure,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: ".tachyon_dead".to_string(),
        src: updated,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(
        changed,
        "dead assign should be rewritten to eval, not dropped"
    );
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
        [Instr::Eval { val, .. }] if *val == updated
    ));
}

#[test]
pub(crate) fn dce_preserves_eval_with_nested_side_effect_inside_index1d() {
    let mut fn_ir = one_block_fn("dce_nested_index1d_eval");
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let wrapped = fn_ir.add_value(
        ValueKind::Index1D {
            base: one,
            idx: impure,
            is_safe: false,
            is_na_safe: false,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
        val: wrapped,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(
        !changed,
        "the eval should stay live when an Index1D operand has side effects"
    );
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
        [Instr::Eval { val, .. }] if *val == wrapped
    ));
}

#[test]
pub(crate) fn dce_demotes_dead_assign_with_nested_index1d_side_effect_to_eval() {
    let mut fn_ir = one_block_fn("dce_nested_index1d_assign");
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let wrapped = fn_ir.add_value(
        ValueKind::Index1D {
            base: one,
            idx: impure,
            is_safe: false,
            is_na_safe: false,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: ".tachyon_dead".to_string(),
        src: wrapped,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(
        changed,
        "dead assign should be rewritten to eval, not dropped"
    );
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
        [Instr::Eval { val, .. }] if *val == wrapped
    ));
}

#[test]
pub(crate) fn dce_preserves_eval_with_nested_side_effect_inside_phi() {
    let mut fn_ir = FnIR::new("dce_nested_phi_eval".to_string(), vec![]);
    let entry = fn_ir.add_block();
    let left = fn_ir.add_block();
    let right = fn_ir.add_block();
    let merge = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let cond = fn_ir.add_value(
        ValueKind::Const(Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = fn_ir.add_value(
        ValueKind::Phi {
            args: vec![(impure, left), (one, right)],
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    fn_ir.values[phi].phi_block = Some(merge);

    fn_ir.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left,
        else_bb: right,
    };
    fn_ir.blocks[left].term = Terminator::Goto(merge);
    fn_ir.blocks[right].term = Terminator::Goto(merge);
    fn_ir.blocks[merge].instrs.push(Instr::Eval {
        val: phi,
        span: Span::default(),
    });
    fn_ir.blocks[merge].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(
        !changed,
        "the eval should stay live when a Phi arm has side effects"
    );
    assert!(matches!(
        fn_ir.blocks[merge].instrs.as_slice(),
        [Instr::Eval { val, .. }] if *val == phi
    ));
}

#[test]
pub(crate) fn dce_demotes_dead_assign_with_nested_phi_side_effect_to_eval() {
    let mut fn_ir = FnIR::new("dce_nested_phi_assign".to_string(), vec![]);
    let entry = fn_ir.add_block();
    let left = fn_ir.add_block();
    let right = fn_ir.add_block();
    let merge = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let cond = fn_ir.add_value(
        ValueKind::Const(Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = fn_ir.add_value(
        ValueKind::Phi {
            args: vec![(impure, left), (one, right)],
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    fn_ir.values[phi].phi_block = Some(merge);

    fn_ir.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left,
        else_bb: right,
    };
    fn_ir.blocks[left].term = Terminator::Goto(merge);
    fn_ir.blocks[right].term = Terminator::Goto(merge);
    fn_ir.blocks[merge].instrs.push(Instr::Assign {
        dst: ".tachyon_dead".to_string(),
        src: phi,
        span: Span::default(),
    });
    fn_ir.blocks[merge].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(
        changed,
        "dead assign should be rewritten to eval, not dropped"
    );
    assert!(matches!(
        fn_ir.blocks[merge].instrs.as_slice(),
        [Instr::Eval { val, .. }] if *val == phi
    ));
}

#[test]
pub(crate) fn dce_preserves_eval_with_nested_side_effect_inside_len() {
    let mut fn_ir = one_block_fn("dce_nested_len_eval");
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let wrapped = fn_ir.add_value(
        ValueKind::Len { base: impure },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
        val: wrapped,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(
        !changed,
        "the eval should stay live when a Len base has side effects"
    );
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
        [Instr::Eval { val, .. }] if *val == wrapped
    ));
}

#[test]
pub(crate) fn dce_demotes_dead_assign_with_nested_range_side_effect_to_eval() {
    let mut fn_ir = one_block_fn("dce_nested_range_assign");
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let wrapped = fn_ir.add_value(
        ValueKind::Range {
            start: impure,
            end: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: ".tachyon_dead".to_string(),
        src: wrapped,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(
        changed,
        "dead assign should be rewritten to eval, not dropped"
    );
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
        [Instr::Eval { val, .. }] if *val == wrapped
    ));
}

#[test]
pub(crate) fn dce_preserves_eval_with_nested_side_effect_inside_indices() {
    let mut fn_ir = one_block_fn("dce_nested_indices_eval");
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let wrapped = fn_ir.add_value(
        ValueKind::Indices { base: impure },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
        val: wrapped,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(
        !changed,
        "the eval should stay live when an Indices base has side effects"
    );
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
        [Instr::Eval { val, .. }] if *val == wrapped
    ));
}

#[test]
pub(crate) fn dce_demotes_dead_assign_with_nested_indices_side_effect_to_eval() {
    let mut fn_ir = one_block_fn("dce_nested_indices_assign");
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let wrapped = fn_ir.add_value(
        ValueKind::Indices { base: impure },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: ".tachyon_dead".to_string(),
        src: wrapped,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(
        changed,
        "dead assign should be rewritten to eval, not dropped"
    );
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
        [Instr::Eval { val, .. }] if *val == wrapped
    ));
}

#[test]
pub(crate) fn dce_preserves_eval_with_nested_side_effect_inside_index2d() {
    let mut fn_ir = one_block_fn("dce_nested_index2d_eval");
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let wrapped = fn_ir.add_value(
        ValueKind::Index2D {
            base: one,
            r: impure,
            c: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
        val: wrapped,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(
        !changed,
        "the eval should stay live when an Index2D operand has side effects"
    );
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
        [Instr::Eval { val, .. }] if *val == wrapped
    ));
}

#[test]
pub(crate) fn dce_demotes_dead_assign_with_nested_index2d_side_effect_to_eval() {
    let mut fn_ir = one_block_fn("dce_nested_index2d_assign");
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let wrapped = fn_ir.add_value(
        ValueKind::Index2D {
            base: one,
            r: impure,
            c: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: ".tachyon_dead".to_string(),
        src: wrapped,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(
        changed,
        "dead assign should be rewritten to eval, not dropped"
    );
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
        [Instr::Eval { val, .. }] if *val == wrapped
    ));
}

#[test]
pub(crate) fn dce_preserves_eval_with_nested_side_effect_inside_index3d() {
    let mut fn_ir = one_block_fn("dce_nested_index3d_eval");
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let wrapped = fn_ir.add_value(
        ValueKind::Index3D {
            base: one,
            i: impure,
            j: one,
            k: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
        val: wrapped,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(
        !changed,
        "the eval should stay live when an Index3D operand has side effects"
    );
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
        [Instr::Eval { val, .. }] if *val == wrapped
    ));
}

#[test]
pub(crate) fn dce_demotes_dead_assign_with_nested_index3d_side_effect_to_eval() {
    let mut fn_ir = one_block_fn("dce_nested_index3d_assign");
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let wrapped = fn_ir.add_value(
        ValueKind::Index3D {
            base: one,
            i: impure,
            j: one,
            k: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: ".tachyon_dead".to_string(),
        src: wrapped,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(
        changed,
        "dead assign should be rewritten to eval, not dropped"
    );
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
        [Instr::Eval { val, .. }] if *val == wrapped
    ));
}

#[test]
pub(crate) fn side_effect_scan_handles_cyclic_pure_values() {
    let mut fn_ir = one_block_fn("side_effect_scan_cyclic_pure");
    let pure_call = fn_ir.add_value(
        ValueKind::Call {
            callee: "length".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = fn_ir.add_value(
        ValueKind::Phi {
            args: vec![(pure_call, fn_ir.entry)],
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    fn_ir.values[pure_call].kind = ValueKind::Call {
        callee: "length".to_string(),
        args: vec![phi],
        names: vec![None],
    };

    assert!(
        !TachyonEngine::new().has_side_effect_val(phi, &fn_ir.values),
        "pure cyclic value graph should terminate and remain side-effect free"
    );
}

#[test]
pub(crate) fn side_effect_scan_finds_impure_call_inside_cycle() {
    let mut fn_ir = one_block_fn("side_effect_scan_cyclic_impure");
    let pure_call = fn_ir.add_value(
        ValueKind::Call {
            callee: "length".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = fn_ir.add_value(
        ValueKind::Phi {
            args: vec![(pure_call, fn_ir.entry), (impure, fn_ir.entry)],
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    fn_ir.values[pure_call].kind = ValueKind::Call {
        callee: "length".to_string(),
        args: vec![phi],
        names: vec![None],
    };

    assert!(
        TachyonEngine::new().has_side_effect_val(phi, &fn_ir.values),
        "cycle detection must not hide impure dependencies"
    );
}

#[test]
pub(crate) fn dce_keeps_dot_prefixed_temp_read_by_unsafe_r() {
    let mut fn_ir = one_block_fn("dce_unsafe_r_dot_temp_read");
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: ".tachyon_keep".to_string(),
        src: one,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::UnsafeRBlock {
        code: "print(.tachyon_keep)".to_string(),
        read_only: true,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

    let changed = TachyonEngine::new().dce(&mut fn_ir);
    assert!(!changed, "unsafe R should keep dot-prefixed RR temps live");
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
        [Instr::Assign { dst, .. }, Instr::UnsafeRBlock { .. }] if dst == ".tachyon_keep"
    ));
}

#[test]
pub(crate) fn unsafe_r_name_scan_collects_backtick_identifiers_but_ignores_strings_and_comments() {
    let mut live = FxHashSet::default();
    TachyonEngine::collect_unsafe_r_named_vars(
        r#"
          print(`.tachyon backtick`)
          print(".tachyon_string")
          # .tachyon_comment
        "#,
        &mut live,
    );

    assert!(live.contains(".tachyon backtick"));
    assert!(
        !live.contains(".tachyon_string"),
        "string contents must not be treated as variable reads"
    );
    assert!(
        !live.contains(".tachyon_comment"),
        "comment contents must not be treated as variable reads"
    );

    let mut unclosed_live = FxHashSet::default();
    TachyonEngine::collect_unsafe_r_named_vars("print(`.tachyon_unclosed)", &mut unclosed_live);
    assert!(unclosed_live.contains(".tachyon_unclosed)"));
}
