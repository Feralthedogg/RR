use super::*;

#[test]
fn verify_ir_rejects_if_with_identical_branch_targets() {
    let mut f = FnIR::new("identical_if_targets".to_string(), Vec::new());
    let entry = f.add_block();
    let join = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let cond = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );

    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: join,
        else_bb: join,
    };
    f.blocks[join].term = Terminator::Return(None);

    let err = verify_ir(&f).expect_err("identical If targets must be rejected");
    assert!(matches!(
        err,
        VerifyError::InvalidBranchTargets {
            block,
            then_bb,
            else_bb
        } if block == entry && then_bb == join && else_bb == join
    ));
}

#[test]
fn verify_ir_rejects_body_head_without_direct_entry_goto() {
    let mut f = FnIR::new("body_head_entry_edge".to_string(), Vec::new());
    let entry = f.add_block();
    let body = f.add_block();
    let other = f.add_block();
    f.entry = entry;
    f.body_head = body;

    let cond = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let self_call = f.add_value(
        ValueKind::Call {
            callee: "body_head_entry_edge".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );

    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: body,
        else_bb: other,
    };
    f.blocks[body].term = Terminator::Return(Some(self_call));
    f.blocks[other].term = Terminator::Return(None);

    let err = verify_ir(&f).expect_err("body_head must be entered by a direct entry goto");
    assert!(matches!(
        err,
        VerifyError::InvalidBodyHeadEntryEdge { entry: e, body_head: h }
        if e == entry && h == body
    ));
}

#[test]
fn verify_ir_rejects_non_param_entry_prologue_when_body_head_is_separate() {
    let mut f = FnIR::new(
        "entry_prologue_not_param_copy".to_string(),
        vec!["p".to_string()],
    );
    let entry = f.add_block();
    let body = f.add_block();
    f.entry = entry;
    f.body_head = body;

    let c1 = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let self_call = f.add_value(
        ValueKind::Call {
            callee: "entry_prologue_not_param_copy".to_string(),
            args: vec![c1],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );

    f.blocks[entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: c1,
        span: Span::default(),
    });
    f.blocks[entry].term = Terminator::Goto(body);
    f.blocks[body].term = Terminator::Return(Some(self_call));

    let err = verify_ir(&f).expect_err("separate body_head entry prologue must be param-copy-only");
    assert!(matches!(
        err,
        VerifyError::InvalidEntryPrologue { block, value }
        if block == entry && value == c1
    ));
}

#[test]
fn verify_ir_rejects_entry_prologue_copy_into_non_runtime_param_target() {
    let mut f = FnIR::new(
        "entry_prologue_wrong_param_target".to_string(),
        vec!["p".to_string()],
    );
    let entry = f.add_block();
    let body = f.add_block();
    f.entry = entry;
    f.body_head = body;

    let p0 = f.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some(".arg_p".to_string()),
    );
    let self_call = f.add_value(
        ValueKind::Call {
            callee: "entry_prologue_wrong_param_target".to_string(),
            args: vec![p0],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );

    f.blocks[entry].instrs.push(Instr::Assign {
        dst: "tmp".to_string(),
        src: p0,
        span: Span::default(),
    });
    f.blocks[entry].term = Terminator::Goto(body);
    f.blocks[body].term = Terminator::Return(Some(self_call));

    let err = verify_ir(&f).expect_err(
        "separate body_head entry prologue must copy params only into runtime param targets",
    );
    assert!(matches!(
        err,
        VerifyError::InvalidEntryPrologue { block, value }
        if block == entry && value == p0
    ));
}

#[test]
fn verify_ir_rejects_loop_header_with_both_branches_in_body() {
    let mut f = FnIR::new("loop_header_split".to_string(), Vec::new());
    let entry = f.add_block();
    let header = f.add_block();
    let body1 = f.add_block();
    let body2 = f.add_block();
    let exit = f.add_block();
    f.entry = entry;
    f.body_head = header;

    let cond1 = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let cond2 = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );

    f.blocks[entry].term = Terminator::Goto(header);
    f.blocks[header].term = Terminator::If {
        cond: cond1,
        then_bb: body1,
        else_bb: body2,
    };
    f.blocks[body1].term = Terminator::Goto(body2);
    f.blocks[body2].term = Terminator::If {
        cond: cond2,
        then_bb: header,
        else_bb: exit,
    };
    f.blocks[exit].term = Terminator::Return(None);

    let err = verify_ir(&f)
        .expect_err("loop header must have exactly one body successor and one exit successor");
    assert!(matches!(
        err,
        VerifyError::InvalidLoopHeaderSplit {
            header: h,
            then_in_body: true,
            else_in_body: true
        } if h == header
    ));
}

#[test]
fn verify_ir_ignores_unreachable_phi_shape() {
    let mut f = FnIR::new("unreachable_phi_shape".to_string(), Vec::new());
    let entry = f.add_block();
    let exit = f.add_block();
    let dead_pred = f.add_block();
    let dead_header = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let zero = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = f.add_value(
        ValueKind::Phi {
            args: vec![(zero, dead_pred)],
        },
        Span::default(),
        Facts::empty(),
        Some("dead".to_string()),
    );
    f.values[phi].phi_block = Some(dead_header);

    f.blocks[entry].term = Terminator::Goto(exit);
    f.blocks[exit].term = Terminator::Return(Some(zero));
    f.blocks[dead_pred].term = Terminator::Goto(dead_header);
    f.blocks[dead_header].term = Terminator::Return(Some(phi));

    verify_ir(&f).expect("unreachable phi shape should not block verifier");
}

#[test]
fn verify_ir_ignores_dead_only_phi_arm_on_reachable_join() {
    let mut f = FnIR::new("dead_only_phi_arm".to_string(), Vec::new());
    let entry = f.add_block();
    let live_pred = f.add_block();
    let join = f.add_block();
    let dead_arm = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let dead_phi_seed = f.add_value(
        ValueKind::Phi { args: vec![] },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    f.values[dead_phi_seed].phi_block = Some(dead_arm);
    let join_phi = f.add_value(
        ValueKind::Phi {
            args: vec![(dead_phi_seed, dead_arm), (one, live_pred)],
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    f.values[join_phi].phi_block = Some(join);

    f.blocks[entry].term = Terminator::Goto(live_pred);
    f.blocks[live_pred].term = Terminator::Goto(join);
    f.blocks[join].term = Terminator::Return(Some(join_phi));
    f.blocks[dead_arm].term = Terminator::Unreachable;

    verify_ir(&f).expect("dead-only phi arm should not block reachable join verification");
}

#[test]
fn verify_ir_ignores_unused_unreachable_phi_without_owner_block() {
    let mut f = FnIR::new(
        "unused_unreachable_phi_without_owner".to_string(),
        Vec::new(),
    );
    let entry = f.add_block();
    let exit = f.add_block();
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
    let _dead_phi = f.add_value(
        ValueKind::Phi {
            args: vec![(zero, entry), (one, exit)],
        },
        Span::default(),
        Facts::empty(),
        Some("dead".to_string()),
    );

    f.blocks[entry].term = Terminator::Goto(exit);
    f.blocks[exit].term = Terminator::Return(Some(zero));

    verify_ir(&f).expect("unused unreachable phi without owner block should be ignored");
}

#[test]
fn verify_ir_accepts_ownerless_phi_when_join_block_is_uniquely_inferred() {
    let mut f = FnIR::new("ownerless_phi_inferred_join".to_string(), Vec::new());
    let entry = f.add_block();
    let left = f.add_block();
    let right = f.add_block();
    let join = f.add_block();
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
            args: vec![(one, left), (two, right)],
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );

    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left,
        else_bb: right,
    };
    f.blocks[left].term = Terminator::Goto(join);
    f.blocks[right].term = Terminator::Goto(join);
    f.blocks[join].term = Terminator::Return(Some(phi));

    verify_ir(&f).expect("ownerless phi with unique join block should be accepted");
}

#[test]
fn verify_ir_allows_loop_phi_backedge_value_depending_on_same_phi() {
    let mut f = FnIR::new("loop_phi_backedge_value".to_string(), Vec::new());
    let entry = f.add_block();
    let header = f.add_block();
    let body = f.add_block();
    let exit = f.add_block();
    f.entry = entry;
    f.body_head = header;

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
    let phi = f.add_value(
        ValueKind::Phi {
            args: vec![(zero, entry), (zero, body)],
        },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    f.values[phi].phi_block = Some(header);
    let next = f.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: phi,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    if let ValueKind::Phi { args } = &mut f.values[phi].kind {
        args[1] = (next, body);
    }
    let cond = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );

    f.blocks[entry].term = Terminator::Goto(header);
    f.blocks[header].term = Terminator::If {
        cond,
        then_bb: body,
        else_bb: exit,
    };
    f.blocks[body].term = Terminator::Goto(header);
    f.blocks[exit].term = Terminator::Return(Some(phi));

    verify_ir(&f).expect("loop-carried backedge value using the same phi should be allowed");
}

#[test]
fn verify_ir_allows_loop_phi_backedge_value_depending_on_other_header_phi() {
    let mut f = FnIR::new("loop_phi_cross_header_value".to_string(), Vec::new());
    let entry = f.add_block();
    let header = f.add_block();
    let body = f.add_block();
    let exit = f.add_block();
    f.entry = entry;
    f.body_head = header;

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
    let i_phi = f.add_value(
        ValueKind::Phi {
            args: vec![(zero, entry), (zero, body)],
        },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    f.values[i_phi].phi_block = Some(header);
    let sum_phi = f.add_value(
        ValueKind::Phi {
            args: vec![(zero, entry), (zero, body)],
        },
        Span::default(),
        Facts::empty(),
        Some("sum".to_string()),
    );
    f.values[sum_phi].phi_block = Some(header);
    let i_next = f.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: i_phi,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    let sum_next = f.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: sum_phi,
            rhs: i_phi,
        },
        Span::default(),
        Facts::empty(),
        Some("sum".to_string()),
    );
    if let ValueKind::Phi { args } = &mut f.values[i_phi].kind {
        args[1] = (i_next, body);
    }
    if let ValueKind::Phi { args } = &mut f.values[sum_phi].kind {
        args[1] = (sum_next, body);
    }
    let cond = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );

    f.blocks[entry].term = Terminator::Goto(header);
    f.blocks[header].term = Terminator::If {
        cond,
        then_bb: body,
        else_bb: exit,
    };
    f.blocks[body].term = Terminator::Goto(header);
    f.blocks[exit].term = Terminator::Return(Some(sum_phi));

    verify_ir(&f).expect("loop header backedge values may depend on other header phis");
}

#[test]
fn verify_ir_ignores_unreachable_loop_backedge_shape() {
    let mut f = FnIR::new("unreachable_loop_backedge".to_string(), Vec::new());
    let entry = f.add_block();
    let header = f.add_block();
    let live_exit = f.add_block();
    let live_else = f.add_block();
    let dead_latch = f.add_block();
    f.entry = entry;
    f.body_head = header;

    let cond = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
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

    f.blocks[entry].term = Terminator::Goto(header);
    f.blocks[header].term = Terminator::If {
        cond,
        then_bb: live_exit,
        else_bb: live_else,
    };
    f.blocks[live_exit].term = Terminator::Return(Some(zero));
    f.blocks[live_else].term = Terminator::Return(Some(zero));
    f.blocks[dead_latch].term = Terminator::Goto(header);

    verify_ir(&f).expect("unreachable backedge must not create a fake reachable loop");
}

#[test]
fn verify_ir_rejects_loop_header_with_multiple_body_predecessors() {
    let mut f = FnIR::new("loop_header_multi_latch".to_string(), Vec::new());
    let entry = f.add_block();
    let header = f.add_block();
    let branch = f.add_block();
    let body_a = f.add_block();
    let body_b = f.add_block();
    let exit = f.add_block();
    f.entry = entry;
    f.body_head = header;

    let cond1 = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let cond2 = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let cond3 = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );

    f.blocks[entry].term = Terminator::Goto(header);
    f.blocks[header].term = Terminator::If {
        cond: cond1,
        then_bb: branch,
        else_bb: exit,
    };
    f.blocks[branch].term = Terminator::If {
        cond: cond2,
        then_bb: body_a,
        else_bb: body_b,
    };
    // `body_a` is both a direct predecessor of the header and a predecessor
    // of the latch, so the natural loop discovered from `body_b -> header`
    // contains two distinct in-body header predecessors (`body_a`, `body_b`)
    // rather than a body-only forwarder.
    f.blocks[body_a].term = Terminator::If {
        cond: cond3,
        then_bb: header,
        else_bb: body_b,
    };
    f.blocks[body_b].term = Terminator::Goto(header);
    f.blocks[exit].term = Terminator::Return(None);

    let err = verify_ir(&f).expect_err("loop header must not have multiple in-body predecessors");
    assert!(matches!(
        err,
        VerifyError::InvalidLoopHeaderPredecessors {
            header: h,
            body_preds: _,
            outer_preds: _,
        } if h == header
    ));
}

#[test]
fn verify_ir_allows_loop_header_with_body_forwarder_backedge() {
    let mut f = FnIR::new("loop_header_body_forwarder".to_string(), Vec::new());
    let entry = f.add_block();
    let header = f.add_block();
    let branch = f.add_block();
    let continue_fwd = f.add_block();
    let latch = f.add_block();
    let exit = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let cond1 = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let cond2 = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );

    f.blocks[entry].term = Terminator::Goto(header);
    f.blocks[header].term = Terminator::If {
        cond: cond1,
        then_bb: branch,
        else_bb: exit,
    };
    f.blocks[branch].term = Terminator::If {
        cond: cond2,
        then_bb: continue_fwd,
        else_bb: latch,
    };
    f.blocks[continue_fwd].term = Terminator::Goto(header);
    f.blocks[latch].term = Terminator::Goto(header);
    f.blocks[exit].term = Terminator::Return(None);

    verify_ir(&f).expect("body-only forwarder backedge should be accepted");
}

#[test]
fn verify_ir_allows_loop_header_with_multiple_outer_predecessors() {
    let mut f = FnIR::new("loop_header_multi_outer".to_string(), Vec::new());
    let entry = f.add_block();
    let outer_left = f.add_block();
    let outer_right = f.add_block();
    let header = f.add_block();
    let body = f.add_block();
    let exit = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let cond = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let loop_cond = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );

    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: outer_left,
        else_bb: outer_right,
    };
    f.blocks[outer_left].term = Terminator::Goto(header);
    f.blocks[outer_right].term = Terminator::Goto(header);
    f.blocks[header].term = Terminator::If {
        cond: loop_cond,
        then_bb: body,
        else_bb: exit,
    };
    f.blocks[body].term = Terminator::Goto(header);
    f.blocks[exit].term = Terminator::Return(None);

    verify_ir(&f).expect("multiple direct outer predecessors should be accepted");
}

#[test]
fn verify_ir_rejects_loop_header_with_conditional_outer_predecessor() {
    let mut f = FnIR::new("loop_header_outer_if_pred".to_string(), Vec::new());
    let entry = f.add_block();
    let guard = f.add_block();
    let header = f.add_block();
    let body = f.add_block();
    let exit = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let cond1 = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let cond2 = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );

    f.blocks[entry].term = Terminator::Goto(guard);
    f.blocks[guard].term = Terminator::If {
        cond: cond1,
        then_bb: header,
        else_bb: exit,
    };
    f.blocks[header].term = Terminator::If {
        cond: cond2,
        then_bb: body,
        else_bb: exit,
    };
    f.blocks[body].term = Terminator::Goto(header);
    f.blocks[exit].term = Terminator::Return(None);

    let err =
        verify_ir(&f).expect_err("loop header outer predecessor must jump directly to header");
    assert!(matches!(
        err,
        VerifyError::InvalidLoopHeaderPredecessors {
            header: h,
            body_preds: 1,
            outer_preds: 1,
        } if h == header
    ));
}
