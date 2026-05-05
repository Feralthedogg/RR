use super::*;
use crate::syntax::ast::{BinOp, UnaryOp};
use crate::utils::Span;

pub(crate) fn build_loop_phi_ir() -> (FnIR, ValueId, ValueId, BlockId) {
    let mut fn_ir = FnIR::new("phi_loop".to_string(), vec![]);
    let entry = fn_ir.add_block();
    let header = fn_ir.add_block();
    let latch = fn_ir.add_block();
    let exit = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = header;

    fn_ir.blocks[entry].term = Terminator::Goto(header);
    fn_ir.blocks[latch].term = Terminator::Goto(header);

    let c0 = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let c1 = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let c10 = fn_ir.add_value(
        ValueKind::Const(Lit::Int(10)),
        Span::default(),
        Facts::empty(),
        None,
    );

    let phi_i = fn_ir.add_value(
        ValueKind::Phi {
            args: vec![(c0, entry), (c0, latch)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.values[phi_i].phi_block = Some(header);

    let v_next = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: phi_i,
            rhs: c1,
        },
        Span::default(),
        Facts::empty(),
        None,
    );

    if let ValueKind::Phi { args } = &mut fn_ir.values[phi_i].kind {
        args[1] = (v_next, latch);
    }

    let cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Lt,
            lhs: phi_i,
            rhs: c10,
        },
        Span::default(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[header].term = Terminator::If {
        cond,
        then_bb: latch,
        else_bb: exit,
    };
    fn_ir.blocks[exit].term = Terminator::Return(Some(phi_i));
    (fn_ir, phi_i, cond, header)
}

#[test]
pub(crate) fn test_meet_rules() {
    let sccp = MirSCCP::new();
    let top = Lattice::Top;
    let bot = Lattice::Bottom;
    let c1 = Lattice::Constant(Lit::Int(1));
    let c2 = Lattice::Constant(Lit::Int(2));

    assert_eq!(sccp.meet(&top, &c1), c1);
    assert_eq!(sccp.meet(&top, &bot), bot);
    assert_eq!(sccp.meet(&bot, &c1), bot);
    assert_eq!(sccp.meet(&bot, &top), bot);
    assert_eq!(sccp.meet(&c1, &c1), c1);
    assert_eq!(sccp.meet(&c1, &c2), Lattice::Bottom);
}

#[test]
pub(crate) fn test_phi_lowering_in_loop() {
    let (fn_ir, phi_i, cond, header) = build_loop_phi_ir();
    let sccp = MirSCCP::new();
    let (lattice, executable_blocks) = sccp.solve_for_test(&fn_ir);

    assert_eq!(lattice.get(&phi_i), Some(&Lattice::Bottom));
    assert_eq!(lattice.get(&cond), Some(&Lattice::Bottom));
    assert!(executable_blocks.contains(&header));
}

#[test]
pub(crate) fn test_dead_branch_removal() {
    let mut fn_ir = FnIR::new("branch_prune".to_string(), vec![]);
    let entry = fn_ir.add_block();
    let header = fn_ir.add_block();
    let then_bb = fn_ir.add_block();
    let else_bb = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = header;

    fn_ir.blocks[entry].term = Terminator::Goto(header);

    let c1000 = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1000)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let c0 = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Gt,
            lhs: c1000,
            rhs: c0,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let alive = fn_ir.add_value(
        ValueKind::Const(Lit::Str("Alive".to_string())),
        Span::default(),
        Facts::empty(),
        None,
    );
    let dead = fn_ir.add_value(
        ValueKind::Const(Lit::Str("Dead".to_string())),
        Span::default(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[header].term = Terminator::If {
        cond,
        then_bb,
        else_bb,
    };
    fn_ir.blocks[then_bb].term = Terminator::Return(Some(alive));
    fn_ir.blocks[else_bb].term = Terminator::Return(Some(dead));

    let sccp = MirSCCP::new();
    let (_lattice, executable_blocks) = sccp.solve_for_test(&fn_ir);
    assert!(!executable_blocks.contains(&else_bb));

    let mut optimized = fn_ir.clone();
    let changed = sccp.optimize(&mut optimized);
    assert!(changed);
    assert!(matches!(optimized.blocks[header].term, Terminator::Goto(t) if t == then_bb));
}

#[test]
pub(crate) fn test_phi_with_executable_top_input_drops_to_bottom() {
    let mut fn_ir = FnIR::new("phi_top_input".to_string(), vec!["n".to_string()]);
    let entry = fn_ir.add_block();
    let header = fn_ir.add_block();
    let latch = fn_ir.add_block();
    let exit = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = header;

    fn_ir.blocks[entry].term = Terminator::Goto(header);
    fn_ir.blocks[latch].term = Terminator::Goto(header);

    let c0 = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let c1 = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let n = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("n".to_string()),
    );

    let phi_i = fn_ir.add_value(
        ValueKind::Phi {
            args: vec![(c0, entry), (c0, latch)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.values[phi_i].phi_block = Some(header);

    // Leave the backedge expression unresolved in early iterations (depends on phi itself).
    let next = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: phi_i,
            rhs: c1,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    if let ValueKind::Phi { args } = &mut fn_ir.values[phi_i].kind {
        args[1] = (next, latch);
    }

    let cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Lt,
            lhs: phi_i,
            rhs: n,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[header].term = Terminator::If {
        cond,
        then_bb: latch,
        else_bb: exit,
    };
    fn_ir.blocks[exit].term = Terminator::Return(Some(phi_i));

    let sccp = MirSCCP::new();
    let (lattice, _executable_blocks) = sccp.solve_for_test(&fn_ir);
    assert_eq!(lattice.get(&phi_i), Some(&Lattice::Bottom));
}

#[test]
pub(crate) fn test_len_seq_along_constant_base() {
    let mut fn_ir = FnIR::new("len_seq".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let n = fn_ir.add_value(
        ValueKind::Const(Lit::Int(5)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let seq = fn_ir.add_value(
        ValueKind::Call {
            callee: "seq_along".to_string(),
            args: vec![n],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let len = fn_ir.add_value(
        ValueKind::Len { base: seq },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(len));

    let sccp = MirSCCP::new();
    let mut opt = fn_ir.clone();
    assert!(sccp.optimize(&mut opt));
    assert!(matches!(
        opt.values[len].kind,
        ValueKind::Const(Lit::Int(1))
    ));
}

#[test]
pub(crate) fn test_index_range_constant_fold() {
    let mut fn_ir = FnIR::new("index_range".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let c1 = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let c3 = fn_ir.add_value(
        ValueKind::Const(Lit::Int(3)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let c2 = fn_ir.add_value(
        ValueKind::Const(Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let r = fn_ir.add_value(
        ValueKind::Range { start: c1, end: c3 },
        Span::default(),
        Facts::empty(),
        None,
    );
    let idx = fn_ir.add_value(
        ValueKind::Index1D {
            base: r,
            idx: c2,
            is_safe: false,
            is_na_safe: false,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(idx));

    let sccp = MirSCCP::new();
    let mut opt = fn_ir.clone();
    assert!(sccp.optimize(&mut opt));
    assert!(matches!(
        opt.values[idx].kind,
        ValueKind::Const(Lit::Int(2))
    ));
}

#[test]
pub(crate) fn test_call_sum_constant_fold() {
    let mut fn_ir = FnIR::new("sum_const".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let a = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let b = fn_ir.add_value(
        ValueKind::Const(Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let c = fn_ir.add_value(
        ValueKind::Const(Lit::Int(3)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let sum = fn_ir.add_value(
        ValueKind::Call {
            callee: "sum".to_string(),
            args: vec![a, b, c],
            names: vec![None, None, None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(sum));

    let sccp = MirSCCP::new();
    let mut opt = fn_ir.clone();
    assert!(sccp.optimize(&mut opt));
    assert!(matches!(
        opt.values[sum].kind,
        ValueKind::Const(Lit::Int(6))
    ));
}

#[test]
pub(crate) fn test_len_of_c_literal_constant_fold() {
    let mut fn_ir = FnIR::new("len_c_const".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let a = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let b = fn_ir.add_value(
        ValueKind::Const(Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let c = fn_ir.add_value(
        ValueKind::Const(Lit::Int(3)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let vecv = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![a, b, c],
            names: vec![None, None, None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let len = fn_ir.add_value(
        ValueKind::Len { base: vecv },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(len));

    let sccp = MirSCCP::new();
    let mut opt = fn_ir.clone();
    assert!(sccp.optimize(&mut opt));
    assert!(matches!(
        opt.values[len].kind,
        ValueKind::Const(Lit::Int(3))
    ));
}

#[test]
pub(crate) fn test_index_seq_len_constant_fold() {
    let mut fn_ir = FnIR::new("index_seq_len".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let n = fn_ir.add_value(
        ValueKind::Const(Lit::Int(5)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let idx = fn_ir.add_value(
        ValueKind::Const(Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let seq = fn_ir.add_value(
        ValueKind::Call {
            callee: "seq_len".to_string(),
            args: vec![n],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let at = fn_ir.add_value(
        ValueKind::Index1D {
            base: seq,
            idx,
            is_safe: false,
            is_na_safe: false,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(at));

    let sccp = MirSCCP::new();
    let mut opt = fn_ir.clone();
    assert!(sccp.optimize(&mut opt));
    assert!(matches!(opt.values[at].kind, ValueKind::Const(Lit::Int(2))));
}

#[test]
pub(crate) fn test_index_seq_len_does_not_constant_fold_after_store() {
    let mut fn_ir = FnIR::new("index_seq_len_store".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let n = fn_ir.add_value(
        ValueKind::Const(Lit::Int(5)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let idx1 = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let idx2 = fn_ir.add_value(
        ValueKind::Const(Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let seq = fn_ir.add_value(
        ValueKind::Call {
            callee: "seq_len".to_string(),
            args: vec![n],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        Some("acc".to_string()),
    );
    let forty_two = fn_ir.add_value(
        ValueKind::Const(Lit::Int(42)),
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].instrs.push(Instr::StoreIndex1D {
        base: seq,
        idx: idx1,
        val: forty_two,
        is_safe: false,
        is_na_safe: false,
        is_vector: false,
        span: Span::default(),
    });
    let at = fn_ir.add_value(
        ValueKind::Index1D {
            base: seq,
            idx: idx2,
            is_safe: false,
            is_na_safe: false,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(at));

    let sccp = MirSCCP::new();
    let mut opt = fn_ir.clone();
    sccp.optimize(&mut opt);
    assert!(matches!(opt.values[at].kind, ValueKind::Index1D { .. }));
}

#[test]
pub(crate) fn test_call_log10_constant_fold() {
    let mut fn_ir = FnIR::new("log10_const".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let ten = fn_ir.add_value(
        ValueKind::Const(Lit::Int(10)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let log = fn_ir.add_value(
        ValueKind::Call {
            callee: "log10".to_string(),
            args: vec![ten],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(log));

    let sccp = MirSCCP::new();
    let mut opt = fn_ir.clone();
    assert!(sccp.optimize(&mut opt));
    assert!(matches!(
        opt.values[log].kind,
        ValueKind::Const(Lit::Int(1))
    ));
}

#[test]
pub(crate) fn test_int_division_constant_folds_to_r_double() {
    let mut fn_ir = FnIR::new("int_div_double".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let five = fn_ir.add_value(
        ValueKind::Const(Lit::Int(5)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let div = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Div,
            lhs: five,
            rhs: two,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(div));

    let sccp = MirSCCP::new();
    let mut opt = fn_ir.clone();
    assert!(sccp.optimize(&mut opt));
    assert!(matches!(
        opt.values[div].kind,
        ValueKind::Const(Lit::Float(v)) if (v - 2.5).abs() < f64::EPSILON
    ));
}

#[test]
pub(crate) fn test_mixed_numeric_binary_and_compare_constant_fold() {
    let mut fn_ir = FnIR::new("mixed_numeric_fold".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let one_point_five = fn_ir.add_value(
        ValueKind::Const(Lit::Float(1.5)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let sum = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: one_point_five,
            rhs: two,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let cmp = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Lt,
            lhs: one_point_five,
            rhs: two,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(sum));

    let sccp = MirSCCP::new();
    let mut opt = fn_ir.clone();
    assert!(sccp.optimize(&mut opt));
    assert!(matches!(
        opt.values[sum].kind,
        ValueKind::Const(Lit::Float(v)) if (v - 3.5).abs() < f64::EPSILON
    ));
    assert!(matches!(
        opt.values[cmp].kind,
        ValueKind::Const(Lit::Bool(true))
    ));
}

#[test]
pub(crate) fn test_negative_mod_uses_r_remainder_semantics() {
    let mut fn_ir = FnIR::new("r_mod".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let neg_one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(-1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let three = fn_ir.add_value(
        ValueKind::Const(Lit::Int(3)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let rem = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Mod,
            lhs: neg_one,
            rhs: three,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(rem));

    let sccp = MirSCCP::new();
    let mut opt = fn_ir.clone();
    assert!(sccp.optimize(&mut opt));
    assert!(matches!(
        opt.values[rem].kind,
        ValueKind::Const(Lit::Int(2))
    ));
}

#[test]
pub(crate) fn test_unary_neg_constant_fold() {
    let mut fn_ir = FnIR::new("neg_fold".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let value = fn_ir.add_value(
        ValueKind::Const(Lit::Float(1.25)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let neg = fn_ir.add_value(
        ValueKind::Unary {
            op: UnaryOp::Neg,
            rhs: value,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(neg));

    let sccp = MirSCCP::new();
    let mut opt = fn_ir.clone();
    assert!(sccp.optimize(&mut opt));
    assert!(matches!(
        opt.values[neg].kind,
        ValueKind::Const(Lit::Float(v)) if (v + 1.25).abs() < f64::EPSILON
    ));
}

#[test]
pub(crate) fn test_non_finite_builtin_result_is_not_folded() {
    let mut fn_ir = FnIR::new("exp_overflow".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let huge = fn_ir.add_value(
        ValueKind::Const(Lit::Float(1000.0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let exp = fn_ir.add_value(
        ValueKind::Call {
            callee: "exp".to_string(),
            args: vec![huge],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(exp));

    let sccp = MirSCCP::new();
    let mut opt = fn_ir.clone();
    let _ = sccp.optimize(&mut opt);
    assert!(
        !matches!(opt.values[exp].kind, ValueKind::Const(_)),
        "non-finite folded floats would emit invalid R literals"
    );
}

#[test]
pub(crate) fn test_div_overflow_is_not_folded() {
    let mut fn_ir = FnIR::new("div_overflow".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let min_i64 = fn_ir.add_value(
        ValueKind::Const(Lit::Int(i64::MIN)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let neg_one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(-1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let div = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Div,
            lhs: min_i64,
            rhs: neg_one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(div));

    let sccp = MirSCCP::new();
    let mut opt = fn_ir.clone();
    let _ = sccp.optimize(&mut opt);
    assert!(
        !matches!(opt.values[div].kind, ValueKind::Const(_)),
        "overflowing i64::MIN / -1 must stay runtime-evaluated"
    );
}

#[test]
pub(crate) fn test_range_len_overflow_is_not_folded() {
    let mut fn_ir = FnIR::new("range_len_overflow".to_string(), vec![]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let start = fn_ir.add_value(
        ValueKind::Const(Lit::Int(i64::MIN)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let end = fn_ir.add_value(
        ValueKind::Const(Lit::Int(i64::MAX)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let range = fn_ir.add_value(
        ValueKind::Range { start, end },
        Span::default(),
        Facts::empty(),
        None,
    );
    let len = fn_ir.add_value(
        ValueKind::Len { base: range },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(len));

    let sccp = MirSCCP::new();
    let mut opt = fn_ir.clone();
    let _ = sccp.optimize(&mut opt);
    assert!(
        !matches!(opt.values[len].kind, ValueKind::Const(_)),
        "range length overflow must not fold to an invalid constant"
    );
}
