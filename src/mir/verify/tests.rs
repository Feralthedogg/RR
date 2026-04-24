#[cfg(test)]
mod tests {
    use super::{VerifyError, verify_emittable_ir, verify_ir};
    use crate::mir::{Facts, FnIR, Instr, Terminator, ValueKind};
    use crate::syntax::ast::BinOp;
    use crate::utils::Span;

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

        let err =
            verify_ir(&f).expect_err("body_head with unreachable terminator must be rejected");
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

        let err =
            verify_ir(&f).expect_err("phi operand depending on same-block phi must be rejected");
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

        let err = verify_emittable_ir(&f)
            .expect_err("reachable phi nested in intrinsic must be rejected");
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

        let err =
            verify_ir(&f).expect_err("join load without all-path definition must be rejected");
        assert!(matches!(
            err,
            VerifyError::UseBeforeDef { block, value }
            if block == join && value == load_x
        ));
    }

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

        let err =
            verify_ir(&f).expect_err("separate body_head entry prologue must be param-copy-only");
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

        let err =
            verify_ir(&f).expect_err("loop header must not have multiple in-body predecessors");
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
}
