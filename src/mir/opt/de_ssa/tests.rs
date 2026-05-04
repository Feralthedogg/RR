use super::run;
use crate::mir::{Facts, FnIR, Instr, IntrinsicOp, Terminator, ValueKind};
use crate::utils::Span;

#[test]
pub(crate) fn unreachable_phi_is_not_rewritten_to_null() {
    let mut f = FnIR::new("dead_phi".to_string(), Vec::new());
    let entry = f.add_block();
    let dead = f.add_block();
    f.entry = entry;
    f.body_head = entry;
    f.blocks[entry].term = Terminator::Return(None);
    f.blocks[dead].term = Terminator::Unreachable;

    let phi = f.add_value(
        ValueKind::Phi { args: Vec::new() },
        Span::default(),
        Facts::empty(),
        None,
    );
    f.values[phi].phi_block = Some(dead);

    let _ = run(&mut f);
    assert!(
        matches!(f.values[phi].kind, ValueKind::Phi { .. }),
        "dead phi should stay dead, not become NULL"
    );
}

#[test]
pub(crate) fn trivial_phi_is_eliminated_before_parallel_copy() {
    let mut f = FnIR::new("trivial_phi".to_string(), vec![]);
    let entry = f.add_block();
    let left = f.add_block();
    let right = f.add_block();
    let merge = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = f.add_value(
        ValueKind::Phi {
            args: vec![(one, left), (one, right)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    f.values[phi].phi_block = Some(merge);

    f.blocks[entry].term = Terminator::If {
        cond: one,
        then_bb: left,
        else_bb: right,
    };
    f.blocks[left].term = Terminator::Goto(merge);
    f.blocks[right].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi));

    let changed = run(&mut f);
    assert!(changed);
    assert!(
        !matches!(f.values[phi].kind, ValueKind::Phi { .. }),
        "trivial phi should be eliminated"
    );
}

#[test]
pub(crate) fn critical_edge_is_not_split_when_existing_assign_already_matches_phi_input() {
    let mut f = FnIR::new("critical_edge_no_split".to_string(), vec![]);
    let entry = f.add_block();
    let other = f.add_block();
    let merge = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let cond = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let two = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = f.add_value(
        ValueKind::Phi {
            args: vec![(one, entry), (two, other)],
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    f.values[phi].phi_block = Some(merge);

    f.blocks[entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: one,
        span: Span::default(),
    });
    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: merge,
        else_bb: other,
    };

    f.blocks[other].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: two,
        span: Span::default(),
    });
    f.blocks[other].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi));

    let before_blocks = f.blocks.len();
    let changed = run(&mut f);
    assert!(changed);
    assert_eq!(
        f.blocks.len(),
        before_blocks,
        "critical edge should not be split when no move is required"
    );
}

#[test]
pub(crate) fn trivial_phi_eliminates_load_alias_inputs_with_same_canonical_source() {
    let mut f = FnIR::new("trivial_phi_load_alias".to_string(), vec![]);
    let entry = f.add_block();
    let left = f.add_block();
    let right = f.add_block();
    let merge = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let cond = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let load_left = f.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let load_right = f.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = f.add_value(
        ValueKind::Phi {
            args: vec![(load_left, left), (load_right, right)],
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    f.values[phi].phi_block = Some(merge);

    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left,
        else_bb: right,
    };
    f.blocks[left].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: one,
        span: Span::default(),
    });
    f.blocks[left].term = Terminator::Goto(merge);
    f.blocks[right].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: one,
        span: Span::default(),
    });
    f.blocks[right].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi));

    let changed = run(&mut f);
    assert!(changed);
    assert!(
        !matches!(f.values[phi].kind, ValueKind::Phi { .. }),
        "phi with load-alias inputs from same canonical source should be eliminated"
    );
}

#[test]
pub(crate) fn critical_edge_is_not_split_when_phi_input_is_load_of_existing_assignment() {
    let mut f = FnIR::new("critical_edge_load_alias".to_string(), vec![]);
    let entry = f.add_block();
    let other = f.add_block();
    let merge = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let cond = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let two = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let load_x = f.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = f.add_value(
        ValueKind::Phi {
            args: vec![(load_x, entry), (two, other)],
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    f.values[phi].phi_block = Some(merge);

    f.blocks[entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: one,
        span: Span::default(),
    });
    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: merge,
        else_bb: other,
    };
    f.blocks[other].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: two,
        span: Span::default(),
    });
    f.blocks[other].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi));

    let before_blocks = f.blocks.len();
    let changed = run(&mut f);
    assert!(changed);
    assert_eq!(
        f.blocks.len(),
        before_blocks,
        "load-alias phi input should canonicalize to existing assignment without edge split"
    );
}

#[test]
pub(crate) fn critical_edge_is_not_split_for_noop_phi_edge_move() {
    let mut f = FnIR::new("critical_edge_noop_phi_move".to_string(), vec![]);
    let entry = f.add_block();
    let other = f.add_block();
    let merge = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let cond = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let load_x = f.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = f.add_value(
        ValueKind::Phi {
            args: vec![(load_x, entry), (one, other)],
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    f.values[phi].phi_block = Some(merge);

    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: merge,
        else_bb: other,
    };
    f.blocks[other].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: one,
        span: Span::default(),
    });
    f.blocks[other].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi));

    let before_blocks = f.blocks.len();
    let changed = run(&mut f);
    assert!(changed);
    assert_eq!(
        f.blocks.len(),
        before_blocks,
        "critical edge should not be split for phi input that lowers to a no-op move"
    );
}

#[test]
pub(crate) fn unique_predecessor_chain_existing_assign_avoids_redundant_phi_move() {
    let mut f = FnIR::new("unique_pred_chain_phi_move".to_string(), vec![]);
    let entry = f.add_block();
    let body = f.add_block();
    let then_bb = f.add_block();
    let else_bb = f.add_block();
    let merge = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let zero = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let cond = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let load_acc = f.add_value(
        ValueKind::Load {
            var: "acc".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("acc".to_string()),
    );
    let add = f.add_value(
        ValueKind::Binary {
            op: crate::syntax::ast::BinOp::Add,
            lhs: load_acc,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        Some("acc".to_string()),
    );
    let phi = f.add_value(
        ValueKind::Phi {
            args: vec![(add, then_bb), (add, else_bb)],
        },
        Span::default(),
        Facts::empty(),
        Some("acc".to_string()),
    );
    f.values[phi].phi_block = Some(merge);

    f.blocks[entry].instrs.push(Instr::Assign {
        dst: "acc".to_string(),
        src: zero,
        span: Span::default(),
    });
    f.blocks[entry].term = Terminator::Goto(body);
    f.blocks[body].instrs.push(Instr::Assign {
        dst: "acc".to_string(),
        src: add,
        span: Span::default(),
    });
    f.blocks[body].term = Terminator::If {
        cond,
        then_bb,
        else_bb,
    };
    f.blocks[then_bb].term = Terminator::Goto(merge);
    f.blocks[else_bb].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi));

    let before_blocks = f.blocks.len();
    let changed = run(&mut f);
    assert!(changed);
    assert_eq!(
        f.blocks.len(),
        before_blocks,
        "unique predecessor chain should let de-SSA see the existing carried assignment"
    );
}

#[test]
pub(crate) fn critical_edge_is_split_when_existing_alias_is_stale_after_later_source_write() {
    let mut f = FnIR::new("critical_edge_existing_stale_alias".to_string(), vec![]);
    let entry = f.add_block();
    let other = f.add_block();
    let merge = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let cond = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let zero = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let two = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let load_y_for_x = f.add_value(
        ValueKind::Load {
            var: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let load_y_at_end = f.add_value(
        ValueKind::Load {
            var: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = f.add_value(
        ValueKind::Phi {
            args: vec![(load_y_at_end, entry), (two, other)],
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    f.values[phi].phi_block = Some(merge);

    f.blocks[entry].instrs.push(Instr::Assign {
        dst: "y".to_string(),
        src: one,
        span: Span::default(),
    });
    f.blocks[entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: load_y_for_x,
        span: Span::default(),
    });
    f.blocks[entry].instrs.push(Instr::Assign {
        dst: "y".to_string(),
        src: zero,
        span: Span::default(),
    });
    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: merge,
        else_bb: other,
    };
    f.blocks[other].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: two,
        span: Span::default(),
    });
    f.blocks[other].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi));

    let before_blocks = f.blocks.len();
    let changed = run(&mut f);
    assert!(changed);
    assert!(
        f.blocks.len() > before_blocks,
        "critical edge must still split when the predecessor's existing alias is stale relative to a later source-variable write"
    );
}

#[test]
pub(crate) fn critical_edge_is_not_split_when_phi_input_matches_existing_field_get_shape() {
    let mut f = FnIR::new("critical_edge_field_get_shape".to_string(), vec![]);
    let entry = f.add_block();
    let other = f.add_block();
    let merge = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let cond = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let two = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let rec1 = f.add_value(
        ValueKind::RecordLit {
            fields: vec![("x".to_string(), one)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let rec2 = f.add_value(
        ValueKind::RecordLit {
            fields: vec![("x".to_string(), one)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let get_existing = f.add_value(
        ValueKind::FieldGet {
            base: rec1,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let get_phi = f.add_value(
        ValueKind::FieldGet {
            base: rec2,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = f.add_value(
        ValueKind::Phi {
            args: vec![(get_phi, entry), (two, other)],
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    f.values[phi].phi_block = Some(merge);

    f.blocks[entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: get_existing,
        span: Span::default(),
    });
    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: merge,
        else_bb: other,
    };
    f.blocks[other].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: two,
        span: Span::default(),
    });
    f.blocks[other].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi));

    let before_blocks = f.blocks.len();
    let changed = run(&mut f);
    assert!(changed);
    assert_eq!(
        f.blocks.len(),
        before_blocks,
        "critical edge should not split when predecessor already computes an equivalent field-get source"
    );
}

#[test]
pub(crate) fn critical_edge_is_not_split_when_phi_input_matches_existing_intrinsic_shape() {
    let mut f = FnIR::new("critical_edge_intrinsic_shape".to_string(), vec![]);
    let entry = f.add_block();
    let other = f.add_block();
    let merge = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let cond = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let two = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let intr_existing = f.add_value(
        ValueKind::Intrinsic {
            op: IntrinsicOp::VecAbsF64,
            args: vec![one],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let intr_phi = f.add_value(
        ValueKind::Intrinsic {
            op: IntrinsicOp::VecAbsF64,
            args: vec![one],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = f.add_value(
        ValueKind::Phi {
            args: vec![(intr_phi, entry), (two, other)],
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    f.values[phi].phi_block = Some(merge);

    f.blocks[entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: intr_existing,
        span: Span::default(),
    });
    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: merge,
        else_bb: other,
    };
    f.blocks[other].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: two,
        span: Span::default(),
    });
    f.blocks[other].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi));

    let before_blocks = f.blocks.len();
    let changed = run(&mut f);
    assert!(changed);
    assert_eq!(
        f.blocks.len(),
        before_blocks,
        "critical edge should not split when predecessor already computes an equivalent intrinsic source"
    );
}

#[test]
pub(crate) fn critical_edge_is_not_split_when_phi_input_matches_existing_fieldset_shape() {
    let mut f = FnIR::new("critical_edge_fieldset_shape".to_string(), vec![]);
    let entry = f.add_block();
    let other = f.add_block();
    let merge = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let cond = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let two = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let rec1 = f.add_value(
        ValueKind::RecordLit {
            fields: vec![("x".to_string(), one)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let rec2 = f.add_value(
        ValueKind::RecordLit {
            fields: vec![("x".to_string(), one)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let set_existing = f.add_value(
        ValueKind::FieldSet {
            base: rec1,
            field: "x".to_string(),
            value: two,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let set_phi = f.add_value(
        ValueKind::FieldSet {
            base: rec2,
            field: "x".to_string(),
            value: two,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = f.add_value(
        ValueKind::Phi {
            args: vec![(set_phi, entry), (two, other)],
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    f.values[phi].phi_block = Some(merge);

    f.blocks[entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: set_existing,
        span: Span::default(),
    });
    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: merge,
        else_bb: other,
    };
    f.blocks[other].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: two,
        span: Span::default(),
    });
    f.blocks[other].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi));

    let before_blocks = f.blocks.len();
    let changed = run(&mut f);
    assert!(changed);
    assert_eq!(
        f.blocks.len(),
        before_blocks,
        "critical edge should not split when predecessor already computes an equivalent field-set source"
    );
}
