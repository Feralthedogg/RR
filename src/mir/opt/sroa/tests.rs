use super::*;
use crate::utils::Span;

pub(crate) fn test_fn() -> FnIR {
    let mut fn_ir = FnIR::new("sroa_test".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;
    fn_ir
}

pub(crate) fn int_value(fn_ir: &mut FnIR, value: i64) -> ValueId {
    fn_ir.add_value(
        ValueKind::Const(Lit::Int(value)),
        Span::default(),
        Facts::empty(),
        None,
    )
}

pub(crate) fn record_xy(fn_ir: &mut FnIR, x: ValueId, y: ValueId) -> ValueId {
    fn_ir.add_value(
        ValueKind::RecordLit {
            fields: vec![("x".to_string(), x), ("y".to_string(), y)],
        },
        Span::default(),
        Facts::empty(),
        None,
    )
}

pub(crate) fn binary_value(fn_ir: &mut FnIR, op: BinOp, lhs: ValueId, rhs: ValueId) -> ValueId {
    fn_ir.add_value(
        ValueKind::Binary { op, lhs, rhs },
        Span::default(),
        Facts::empty(),
        None,
    )
}

pub(crate) fn record_pos_mass(fn_ir: &mut FnIR, pos: ValueId, mass: ValueId) -> ValueId {
    fn_ir.add_value(
        ValueKind::RecordLit {
            fields: vec![("pos".to_string(), pos), ("mass".to_string(), mass)],
        },
        Span::default(),
        Facts::empty(),
        None,
    )
}

pub(crate) fn sum_xy_fn() -> FnIR {
    let mut fn_ir = FnIR::new("sum_xy".to_string(), vec!["p".to_string()]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;
    let p = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("p".to_string()),
    );
    let x = fn_ir.add_value(
        ValueKind::FieldGet {
            base: p,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let y = fn_ir.add_value(
        ValueKind::FieldGet {
            base: p,
            field: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let sum = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: x,
            rhs: y,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(sum));
    fn_ir
}

pub(crate) fn make_xy_fn() -> FnIR {
    let mut fn_ir = FnIR::new("make_xy".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let record = record_xy(&mut fn_ir, x, y);
    fn_ir.blocks[entry].term = Terminator::Return(Some(record));
    fn_ir
}

pub(crate) fn make_xy_from_args_fn() -> FnIR {
    let mut fn_ir = FnIR::new(
        "make_xy_from_args".to_string(),
        vec!["x".to_string(), "y".to_string()],
    );
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;
    let x = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let y = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::default(),
        Facts::empty(),
        Some("y".to_string()),
    );
    let record = record_xy(&mut fn_ir, x, y);
    fn_ir.blocks[entry].term = Terminator::Return(Some(record));
    fn_ir
}

pub(crate) fn scale_xy_fn() -> FnIR {
    let mut fn_ir = FnIR::new(
        "scale_xy".to_string(),
        vec!["p".to_string(), "factor".to_string()],
    );
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;
    let p = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("p".to_string()),
    );
    let factor = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::default(),
        Facts::empty(),
        Some("factor".to_string()),
    );
    let x = fn_ir.add_value(
        ValueKind::FieldGet {
            base: p,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let y = fn_ir.add_value(
        ValueKind::FieldGet {
            base: p,
            field: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let scaled_x = binary_value(&mut fn_ir, BinOp::Mul, x, factor);
    let scaled_y = binary_value(&mut fn_ir, BinOp::Mul, y, factor);
    let record = record_xy(&mut fn_ir, scaled_x, scaled_y);
    fn_ir.blocks[entry].term = Terminator::Return(Some(record));
    fn_ir
}

pub(crate) fn forward_scale_xy_fn() -> FnIR {
    let mut fn_ir = FnIR::new(
        "forward_scale_xy".to_string(),
        vec!["p".to_string(), "factor".to_string()],
    );
    let entry = fn_ir.add_block();
    let body = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = body;
    let p = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("p".to_string()),
    );
    let factor = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::default(),
        Facts::empty(),
        Some("factor".to_string()),
    );
    let call = fn_ir.add_value(
        ValueKind::Call {
            callee: "scale_xy".to_string(),
            args: vec![p, factor],
            names: vec![None, None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.set_call_semantics(call, CallSemantics::UserDefined);
    fn_ir.blocks[entry].term = Terminator::Goto(body);
    fn_ir.blocks[body].term = Terminator::Return(Some(call));
    fn_ir
}

pub(crate) fn branch_make_xy_fn() -> FnIR {
    let mut fn_ir = FnIR::new("branch_make_xy".to_string(), vec![]);
    let entry = fn_ir.add_block();
    let then_bb = fn_ir.add_block();
    let else_bb = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let cond = fn_ir.add_value(
        ValueKind::Const(Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let then_x = int_value(&mut fn_ir, 1);
    let then_y = int_value(&mut fn_ir, 2);
    let then_record = record_xy(&mut fn_ir, then_x, then_y);
    let else_x = int_value(&mut fn_ir, 3);
    let else_y = int_value(&mut fn_ir, 4);
    let else_record = record_xy(&mut fn_ir, else_x, else_y);

    fn_ir.blocks[entry].term = Terminator::If {
        cond,
        then_bb,
        else_bb,
    };
    fn_ir.blocks[then_bb].term = Terminator::Return(Some(then_record));
    fn_ir.blocks[else_bb].term = Terminator::Return(Some(else_record));
    fn_ir
}

pub(crate) fn impure_make_xy_fn() -> FnIR {
    let mut fn_ir = FnIR::new("impure_make_xy".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;
    let side_effect = fn_ir.add_value(
        ValueKind::Call {
            callee: "print".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].instrs.push(Instr::Eval {
        val: side_effect,
        span: Span::default(),
    });
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let record = record_xy(&mut fn_ir, x, y);
    fn_ir.blocks[entry].term = Terminator::Return(Some(record));
    fn_ir
}

#[path = "tests/analysis.rs"]
pub(crate) mod analysis;
#[path = "tests/call_specialization.rs"]
pub(crate) mod call_specialization;
#[path = "tests/core_rewrite.rs"]
pub(crate) mod core_rewrite;
#[path = "tests/return_specialization.rs"]
pub(crate) mod return_specialization;
