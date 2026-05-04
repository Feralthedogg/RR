use super::*;

#[test]
fn emittable_verify_rejects_reachable_phi() {
    let mut f = FnIR::new("phi_live".to_string(), Vec::new());
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
    f.values[phi].phi_block = Some(merge);
    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left,
        else_bb: right,
    };
    f.blocks[left].term = Terminator::Goto(merge);
    f.blocks[right].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi));

    let err = verify_emittable_ir(&f).expect_err("reachable phi must be rejected");
    assert!(matches!(err, VerifyError::ReachablePhi { value } if value == phi));
}

#[test]
fn verify_ir_rejects_phi_with_wrong_predecessor_count() {
    let mut f = FnIR::new("phi_bad_arity".to_string(), Vec::new());
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
    let phi = f.add_value(
        ValueKind::Phi {
            args: vec![(one, left)],
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
    f.blocks[left].term = Terminator::Goto(merge);
    f.blocks[right].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi));

    let err = verify_ir(&f).expect_err("phi with missing predecessor arm must be rejected");
    assert!(matches!(
        err,
        VerifyError::InvalidPhiArgs {
            phi_val,
            expected: 2,
            got: 1
        } if phi_val == phi
    ));
}

#[test]
fn verify_ir_rejects_phi_with_non_predecessor_source() {
    let mut f = FnIR::new("phi_bad_source".to_string(), Vec::new());
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
    let two = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = f.add_value(
        ValueKind::Phi {
            args: vec![(one, left), (two, entry)],
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
    f.blocks[left].term = Terminator::Goto(merge);
    f.blocks[right].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi));

    let err = verify_ir(&f).expect_err("phi with non-predecessor source must be rejected");
    assert!(matches!(
        err,
        VerifyError::InvalidPhiSource { phi_val, block }
        if phi_val == phi && block == entry
    ));
}

#[test]
fn verify_ir_rejects_non_phi_with_phi_block_metadata() {
    let mut f = FnIR::new("non_phi_phi_owner".to_string(), Vec::new());
    let entry = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let value = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    f.values[value].phi_block = Some(entry);
    f.blocks[entry].term = Terminator::Return(Some(value));

    let err = verify_ir(&f).expect_err("non-phi values must not carry phi owner metadata");
    assert!(matches!(
        err,
        VerifyError::InvalidPhiOwner { value: bad_value, block }
        if bad_value == value && block == entry
    ));
}

#[test]
fn verify_ir_rejects_phi_with_invalid_owner_block() {
    let mut f = FnIR::new("phi_bad_owner_block".to_string(), Vec::new());
    let entry = f.add_block();
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
            args: vec![(one, entry)],
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    f.values[phi].phi_block = Some(999);
    f.blocks[entry].term = Terminator::Return(Some(one));

    let err = verify_ir(&f).expect_err("phi with invalid owner block must be rejected");
    assert!(matches!(
        err,
        VerifyError::InvalidPhiOwnerBlock { value: bad_value, block }
        if bad_value == phi && block == 999
    ));
}

#[test]
fn verify_ir_rejects_param_with_invalid_index() {
    let mut f = FnIR::new("bad_param_index".to_string(), vec!["x".to_string()]);
    let entry = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let bad_param = f.add_value(
        ValueKind::Param { index: 3 },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    f.blocks[entry].term = Terminator::Return(Some(bad_param));

    let err = verify_ir(&f).expect_err("param with invalid index must be rejected");
    assert!(matches!(
        err,
        VerifyError::InvalidParamIndex {
            value,
            index: 3,
            param_count: 1
        } if value == bad_param
    ));
}

#[test]
fn verify_ir_rejects_call_with_too_many_arg_names() {
    let mut f = FnIR::new("call_bad_names".to_string(), Vec::new());
    let entry = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let call = f.add_value(
        ValueKind::Call {
            callee: "foo".to_string(),
            args: vec![one],
            names: vec![Some("x".to_string()), Some("y".to_string())],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    f.blocks[entry].term = Terminator::Return(Some(call));

    let err = verify_ir(&f).expect_err("call with too many arg names must be rejected");
    assert!(matches!(
        err,
        VerifyError::InvalidCallArgNames {
            value,
            args: 1,
            names: 2
        } if value == call
    ));
}

#[test]
fn verify_ir_rejects_self_referential_binary_value() {
    let mut f = FnIR::new("self_ref_binary".to_string(), Vec::new());
    let entry = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let self_ref = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    f.values[self_ref].kind = ValueKind::Binary {
        op: crate::syntax::ast::BinOp::Add,
        lhs: self_ref,
        rhs: one,
    };
    f.blocks[entry].term = Terminator::Return(Some(self_ref));

    let err = verify_ir(&f).expect_err("self-referential binary value must be rejected");
    assert!(matches!(
        err,
        VerifyError::SelfReferentialValue { value } if value == self_ref
    ));
}

#[test]
fn verify_ir_rejects_self_referential_phi_value() {
    let mut f = FnIR::new("self_ref_phi".to_string(), Vec::new());
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
    let zero = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );

    let phi = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    f.values[phi].kind = ValueKind::Phi {
        args: vec![(zero, left), (phi, right)],
    };
    f.values[phi].phi_block = Some(merge);
    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left,
        else_bb: right,
    };
    f.blocks[left].term = Terminator::Goto(merge);
    f.blocks[right].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi));

    let err = verify_ir(&f).expect_err("self-referential phi value must be rejected");
    assert!(matches!(
        err,
        VerifyError::SelfReferentialValue { value } if value == phi
    ));
}

#[test]
fn verify_ir_allows_loop_header_self_passthrough_phi_arm() {
    let mut f = FnIR::new("loop_self_phi_passthrough".to_string(), Vec::new());
    let entry = f.add_block();
    let header = f.add_block();
    let next_bb = f.add_block();
    let body = f.add_block();
    let exit = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let zero = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        Some("s".to_string()),
    );
    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let cond_true = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let sum_next = f.add_value(
        ValueKind::Binary {
            op: crate::syntax::ast::BinOp::Add,
            lhs: phi,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        Some("s".to_string()),
    );
    f.values[phi].kind = ValueKind::Phi {
        args: vec![(zero, entry), (phi, next_bb), (sum_next, body)],
    };
    f.values[phi].phi_block = Some(header);

    f.blocks[entry].term = Terminator::Goto(header);
    f.blocks[header].term = Terminator::If {
        cond: cond_true,
        then_bb: next_bb,
        else_bb: exit,
    };
    f.blocks[next_bb].term = Terminator::Goto(header);
    f.blocks[body].term = Terminator::Goto(header);
    f.blocks[exit].term = Terminator::Return(Some(phi));

    verify_ir(&f).expect("loop-header phi may carry its previous value on a body backedge");
}

#[test]
fn verify_ir_rejects_non_phi_mutual_cycle() {
    let mut f = FnIR::new("non_phi_cycle".to_string(), Vec::new());
    let entry = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let a = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let b = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    f.values[a].kind = ValueKind::Binary {
        op: crate::syntax::ast::BinOp::Add,
        lhs: b,
        rhs: one,
    };
    f.values[b].kind = ValueKind::Binary {
        op: crate::syntax::ast::BinOp::Mul,
        lhs: a,
        rhs: one,
    };
    f.blocks[entry].term = Terminator::Return(Some(a));

    let err = verify_ir(&f).expect_err("non-phi mutual cycle must be rejected");
    assert!(matches!(err, VerifyError::NonPhiValueCycle { value } if value == a || value == b));
}

#[test]
fn verify_ir_rejects_unreachable_body_head() {
    let mut f = FnIR::new("bad_body_head".to_string(), Vec::new());
    let entry = f.add_block();
    let dead = f.add_block();
    f.entry = entry;
    f.body_head = dead;

    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    f.blocks[entry].term = Terminator::Return(Some(one));
    f.blocks[dead].term = Terminator::Return(Some(one));

    let err = verify_ir(&f).expect_err("unreachable body_head must be rejected");
    assert!(matches!(err, VerifyError::InvalidBodyHead { block } if block == dead));
}

#[test]
fn verify_ir_rejects_body_head_with_unreachable_terminator() {
    let mut f = FnIR::new("body_head_unreachable".to_string(), Vec::new());
    let entry = f.add_block();
    let body = f.add_block();
    f.entry = entry;
    f.body_head = body;

    f.blocks[entry].term = Terminator::Goto(body);
    f.blocks[body].term = Terminator::Unreachable;

    let err = verify_ir(&f).expect_err("body_head with unreachable terminator must be rejected");
    assert!(matches!(
        err,
        VerifyError::InvalidBodyHeadTerminator { block } if block == body
    ));
}

#[test]
fn verify_ir_rejects_entry_with_predecessor() {
    let mut f = FnIR::new("entry_has_pred".to_string(), Vec::new());
    let entry = f.add_block();
    let stray = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    f.blocks[entry].term = Terminator::Return(Some(one));
    f.blocks[stray].term = Terminator::Goto(entry);

    let err = verify_ir(&f).expect_err("entry predecessor must be rejected");
    assert!(matches!(
        err,
        VerifyError::InvalidEntryPredecessor { pred } if pred == stray
    ));
}

#[test]
fn verify_ir_rejects_unreachable_entry() {
    let mut f = FnIR::new("entry_unreachable".to_string(), Vec::new());
    let entry = f.add_block();
    f.entry = entry;
    f.body_head = entry;
    f.blocks[entry].term = Terminator::Unreachable;

    let err = verify_ir(&f).expect_err("unreachable entry must be rejected");
    assert!(matches!(err, VerifyError::InvalidEntryTerminator));
}

#[test]
fn verify_ir_rejects_phi_in_zero_predecessor_block() {
    let mut f = FnIR::new("phi_zero_pred".to_string(), Vec::new());
    let entry = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let phi = f.add_value(
        ValueKind::Phi { args: vec![] },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    f.values[phi].phi_block = Some(entry);
    f.blocks[entry].term = Terminator::Return(Some(phi));

    let err = verify_ir(&f).expect_err("phi in zero-predecessor block must be rejected");
    assert!(matches!(
        err,
        VerifyError::InvalidPhiPlacement { value, block }
        if value == phi && block == entry
    ));
}

#[test]
fn verify_ir_rejects_phi_in_single_predecessor_block() {
    let mut f = FnIR::new("phi_single_pred".to_string(), Vec::new());
    let entry = f.add_block();
    let merge = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let c1 = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = f.add_value(
        ValueKind::Phi {
            args: vec![(c1, entry)],
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    f.values[phi].phi_block = Some(merge);

    f.blocks[entry].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi));

    let err = verify_ir(&f).expect_err("phi in single-predecessor block must be rejected");
    assert!(matches!(
        err,
        VerifyError::InvalidPhiPredecessorAliases { phi_val, block }
        if phi_val == phi && block == merge
    ));
}

#[test]
fn verify_ir_rejects_phi_with_duplicate_predecessor_arm() {
    let mut f = FnIR::new("phi_duplicate_pred_arm".to_string(), Vec::new());
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
    let two = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = f.add_value(
        ValueKind::Phi {
            args: vec![(one, left), (two, left)],
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
    f.blocks[left].term = Terminator::Goto(merge);
    f.blocks[right].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi));

    let err = verify_ir(&f).expect_err("phi with duplicate predecessor arm must be rejected");
    assert!(matches!(
        err,
        VerifyError::InvalidPhiPredecessorAliases { phi_val, block }
        if phi_val == phi && block == merge
    ));
}

#[test]
fn verify_ir_rejects_phi_arg_from_same_phi_block() {
    let mut f = FnIR::new("phi_same_block_arg".to_string(), Vec::new());
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
    let two = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi_a = f.add_value(
        ValueKind::Phi {
            args: vec![(one, left), (two, right)],
        },
        Span::default(),
        Facts::empty(),
        Some("a".to_string()),
    );
    f.values[phi_a].phi_block = Some(merge);
    let phi_b = f.add_value(
        ValueKind::Phi {
            args: vec![(phi_a, left), (two, right)],
        },
        Span::default(),
        Facts::empty(),
        Some("b".to_string()),
    );
    f.values[phi_b].phi_block = Some(merge);

    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left,
        else_bb: right,
    };
    f.blocks[left].term = Terminator::Goto(merge);
    f.blocks[right].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi_b));

    let err = verify_ir(&f).expect_err("same-block phi operand must be rejected");
    assert!(matches!(
        err,
        VerifyError::InvalidPhiEdgeValue { phi_val, value }
        if phi_val == phi_b && value == phi_a
    ));
}

#[test]
fn verify_ir_rejects_phi_arg_from_same_phi_block_via_intrinsic_wrapper() {
    let mut f = FnIR::new("phi_same_block_wrapped_arg".to_string(), Vec::new());
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
        ValueKind::Const(crate::syntax::ast::Lit::Float(1.0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let two = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Float(2.0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi_a = f.add_value(
        ValueKind::Phi {
            args: vec![(one, left), (two, right)],
        },
        Span::default(),
        Facts::empty(),
        Some("a".to_string()),
    );
    f.values[phi_a].phi_block = Some(merge);
    let wrapped = f.add_value(
        ValueKind::Intrinsic {
            op: crate::mir::IntrinsicOp::VecAbsF64,
            args: vec![phi_a],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi_b = f.add_value(
        ValueKind::Phi {
            args: vec![(wrapped, left), (two, right)],
        },
        Span::default(),
        Facts::empty(),
        Some("b".to_string()),
    );
    f.values[phi_b].phi_block = Some(merge);

    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left,
        else_bb: right,
    };
    f.blocks[left].term = Terminator::Goto(merge);
    f.blocks[right].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(phi_b));

    let err = verify_ir(&f).expect_err("phi operand depending on same-block phi must be rejected");
    assert!(matches!(
        err,
        VerifyError::InvalidPhiEdgeValue { phi_val, value }
        if phi_val == phi_b && value == wrapped
    ));
}

#[test]
fn emittable_verify_rejects_reachable_phi_nested_in_intrinsic() {
    let mut f = FnIR::new("phi_live_nested_intrinsic".to_string(), Vec::new());
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
        ValueKind::Const(crate::syntax::ast::Lit::Float(1.0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let two = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Float(2.0)),
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
    f.values[phi].phi_block = Some(merge);
    let intrinsic = f.add_value(
        ValueKind::Intrinsic {
            op: crate::mir::IntrinsicOp::VecAbsF64,
            args: vec![phi],
        },
        Span::default(),
        Facts::empty(),
        None,
    );

    f.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left,
        else_bb: right,
    };
    f.blocks[left].term = Terminator::Goto(merge);
    f.blocks[right].term = Terminator::Goto(merge);
    f.blocks[merge].term = Terminator::Return(Some(intrinsic));

    let err =
        verify_emittable_ir(&f).expect_err("reachable phi nested in intrinsic must be rejected");
    assert!(matches!(err, VerifyError::ReachablePhi { value } if value == phi));
}

#[test]
fn verify_ir_rejects_record_literal_with_bad_field_operand() {
    let mut f = FnIR::new("record_bad_operand".to_string(), Vec::new());
    let entry = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let record = f.add_value(
        ValueKind::RecordLit {
            fields: vec![("x".to_string(), 999)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    f.blocks[entry].term = Terminator::Return(Some(record));

    let err =
        verify_ir(&f).expect_err("record literal with invalid field operand must be rejected");
    assert!(matches!(err, VerifyError::BadValue(999)));
}

#[test]
fn verify_ir_rejects_same_block_use_before_def() {
    let mut f = FnIR::new("same_block_use_before_def".to_string(), Vec::new());
    let entry = f.add_block();
    f.entry = entry;
    f.body_head = entry;

    let load_x = f.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let one = f.add_value(
        ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );

    f.blocks[entry].instrs.push(Instr::Eval {
        val: load_x,
        span: Span::default(),
    });
    f.blocks[entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: one,
        span: Span::default(),
    });
    f.blocks[entry].term = Terminator::Return(None);

    let err = verify_ir(&f).expect_err("same-block load before assignment must be rejected");
    assert!(matches!(
        err,
        VerifyError::UseBeforeDef { block, value }
        if block == entry && value == load_x
    ));
}

#[test]
fn verify_ir_rejects_join_use_without_def_on_all_paths() {
    let mut f = FnIR::new("join_use_before_def".to_string(), Vec::new());
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
    let load_x = f.add_value(
        ValueKind::Load {
            var: "x".to_string(),
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
    f.blocks[left].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: one,
        span: Span::default(),
    });
    f.blocks[left].term = Terminator::Goto(join);
    f.blocks[right].term = Terminator::Goto(join);
    f.blocks[join].term = Terminator::Return(Some(load_x));

    let err = verify_ir(&f).expect_err("join load without all-path definition must be rejected");
    assert!(matches!(
        err,
        VerifyError::UseBeforeDef { block, value }
        if block == join && value == load_x
    ));
}
