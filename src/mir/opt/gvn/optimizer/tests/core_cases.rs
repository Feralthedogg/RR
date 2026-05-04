use super::*;
fn one_block_fn(name: &str) -> FnIR {
    let mut f = FnIR::new(name.to_string(), vec![]);
    let b0 = f.add_block();
    f.entry = b0;
    f.body_head = b0;
    f
}

#[test]
fn gvn_cse_pure_calls() {
    let mut fn_ir = one_block_fn("gvn_call");
    let c1 = fn_ir.add_value(
        ValueKind::Const(Lit::Int(4)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let call1 = fn_ir.add_value(
        ValueKind::Call {
            callee: "abs".to_string(),
            args: vec![c1],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let call2 = fn_ir.add_value(
        ValueKind::Call {
            callee: "abs".to_string(),
            args: vec![c1],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sum = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: call1,
            rhs: call2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(sum));

    let changed = optimize(&mut fn_ir);
    assert!(changed, "expected pure-call CSE to fire");
    match fn_ir.values[sum].kind {
        ValueKind::Binary { lhs, rhs, .. } => {
            assert_eq!(lhs, rhs, "expected duplicated pure call to be CSE'd")
        }
        _ => panic!("sum value shape changed unexpectedly"),
    }
}

#[test]
fn gvn_does_not_cse_pure_call_when_argument_alias_is_mutated() {
    let mut fn_ir = one_block_fn("gvn_mutated_call_arg");
    let arr = fn_ir.add_value(
        ValueKind::Load {
            var: "xs".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        Some("xs".to_string()),
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let call1 = fn_ir.add_value(
        ValueKind::Call {
            callee: "sum".to_string(),
            args: vec![arr],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let call2 = fn_ir.add_value(
        ValueKind::Call {
            callee: "sum".to_string(),
            args: vec![arr],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "before".to_string(),
        src: call1,
        span: Span::dummy(),
    });
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::StoreIndex1D {
        base: arr,
        idx: one,
        val: zero,
        is_safe: true,
        is_na_safe: true,
        is_vector: false,
        span: Span::dummy(),
    });
    let sum = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: call1,
            rhs: call2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(sum));

    optimize(&mut fn_ir);
    match fn_ir.values[sum].kind {
        ValueKind::Binary { lhs, rhs, .. } => {
            assert_ne!(
                lhs, rhs,
                "pure calls that receive a mutated array alias must not be CSE'd"
            )
        }
        _ => panic!("sum value shape changed unexpectedly"),
    }
}

#[test]
fn gvn_does_not_cse_fresh_allocating_pure_calls() {
    let mut fn_ir = one_block_fn("gvn_fresh_call");
    let n = fn_ir.add_value(
        ValueKind::Const(Lit::Int(4)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let seq1 = fn_ir.add_value(
        ValueKind::Call {
            callee: "seq_len".to_string(),
            args: vec![n],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let seq2 = fn_ir.add_value(
        ValueKind::Call {
            callee: "seq_len".to_string(),
            args: vec![n],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sum = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: seq1,
            rhs: seq2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(sum));

    optimize(&mut fn_ir);
    match fn_ir.values[sum].kind {
        ValueKind::Binary { lhs, rhs, .. } => {
            assert_eq!(lhs, seq1);
            assert_eq!(rhs, seq2);
        }
        _ => panic!("sum value shape changed unexpectedly"),
    }
}

#[test]
fn gvn_cse_index2d_reads_when_unmutated() {
    let mut fn_ir = one_block_fn("gvn_index2d");
    let base = fn_ir.add_value(
        ValueKind::Load {
            var: "m".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        Some("m".to_string()),
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let idx1 = fn_ir.add_value(
        ValueKind::Index2D {
            base,
            r: one,
            c: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let idx2 = fn_ir.add_value(
        ValueKind::Index2D {
            base,
            r: one,
            c: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sum = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: idx1,
            rhs: idx2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(sum));

    let changed = optimize(&mut fn_ir);
    assert!(changed, "expected Index2D CSE to fire");
    match fn_ir.values[sum].kind {
        ValueKind::Binary { lhs, rhs, .. } => {
            assert_eq!(lhs, rhs, "expected duplicated Index2D read to be CSE'd")
        }
        _ => panic!("sum value shape changed unexpectedly"),
    }
}

#[test]
fn gvn_cse_runs_inside_loop_body() {
    let mut fn_ir = FnIR::new("gvn_loop".to_string(), vec!["keep_going".to_string()]);
    let entry = fn_ir.add_block();
    let header = fn_ir.add_block();
    let body = fn_ir.add_block();
    let exit = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = header;
    fn_ir.blocks[entry].term = Terminator::Goto(header);

    let cond = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        Some("keep_going".to_string()),
    );
    let x = fn_ir.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let add1 = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: x,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let add2 = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: x,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[header].term = Terminator::If {
        cond,
        then_bb: body,
        else_bb: exit,
    };
    fn_ir.blocks[body].instrs.push(Instr::Assign {
        dst: "a".to_string(),
        src: add1,
        span: Span::dummy(),
    });
    fn_ir.blocks[body].instrs.push(Instr::Assign {
        dst: "b".to_string(),
        src: add2,
        span: Span::dummy(),
    });
    fn_ir.blocks[body].term = Terminator::Goto(header);
    fn_ir.blocks[exit].term = Terminator::Return(None);

    let changed = optimize(&mut fn_ir);
    assert!(changed, "expected same-block loop-body CSE to fire");
    assert!(matches!(
        &fn_ir.blocks[body].instrs[1],
        Instr::Assign { src, .. } if *src == add1
    ));
}

#[test]
fn gvn_does_not_cse_across_loop_blocks() {
    let mut fn_ir = FnIR::new(
        "gvn_loop_cross_block".to_string(),
        vec!["keep_going".to_string()],
    );
    let entry = fn_ir.add_block();
    let header = fn_ir.add_block();
    let body = fn_ir.add_block();
    let exit = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = header;
    fn_ir.blocks[entry].term = Terminator::Goto(header);

    let cond = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        Some("keep_going".to_string()),
    );
    let x = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let header_add = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: x,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let body_add = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: x,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[header].instrs.push(Instr::Assign {
        dst: "header_add".to_string(),
        src: header_add,
        span: Span::dummy(),
    });
    fn_ir.blocks[header].term = Terminator::If {
        cond,
        then_bb: body,
        else_bb: exit,
    };
    fn_ir.blocks[body].instrs.push(Instr::Assign {
        dst: "body_add".to_string(),
        src: body_add,
        span: Span::dummy(),
    });
    fn_ir.blocks[body].term = Terminator::Goto(header);
    fn_ir.blocks[exit].term = Terminator::Return(None);

    let changed = optimize(&mut fn_ir);
    assert!(
        !changed,
        "looped functions should not CSE across block boundaries"
    );
    assert!(matches!(
        &fn_ir.blocks[body].instrs[0],
        Instr::Assign { src, .. } if *src == body_add
    ));
}

#[test]
fn gvn_cse_index3d_reads_when_unmutated() {
    let mut fn_ir = one_block_fn("gvn_index3d_unmutated");
    let base = fn_ir.add_value(
        ValueKind::Load {
            var: "cube".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        Some("cube".to_string()),
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let idx1 = fn_ir.add_value(
        ValueKind::Index3D {
            base,
            i: one,
            j: one,
            k: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let idx2 = fn_ir.add_value(
        ValueKind::Index3D {
            base,
            i: one,
            j: one,
            k: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sum = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: idx1,
            rhs: idx2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(sum));

    let changed = optimize(&mut fn_ir);
    assert!(changed, "expected unmutated Index3D CSE to fire");
    match fn_ir.values[sum].kind {
        ValueKind::Binary { lhs, rhs, .. } => {
            assert_eq!(lhs, rhs, "expected duplicated Index3D read to be CSE'd")
        }
        _ => panic!("sum value shape changed unexpectedly"),
    }
}

#[test]
fn gvn_does_not_cse_index3d_reads_when_base_is_mutated() {
    let mut fn_ir = one_block_fn("gvn_index3d_mutated");
    let base = fn_ir.add_value(
        ValueKind::Load {
            var: "cube".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        Some("cube".to_string()),
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let replacement = fn_ir.add_value(
        ValueKind::Const(Lit::Int(7)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let idx1 = fn_ir.add_value(
        ValueKind::Index3D {
            base,
            i: one,
            j: one,
            k: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let idx2 = fn_ir.add_value(
        ValueKind::Index3D {
            base,
            i: one,
            j: one,
            k: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "before".to_string(),
        src: idx1,
        span: Span::dummy(),
    });
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::StoreIndex3D {
        base,
        i: one,
        j: one,
        k: one,
        val: replacement,
        span: Span::dummy(),
    });
    let sum = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: idx1,
            rhs: idx2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(sum));

    optimize(&mut fn_ir);
    match fn_ir.values[sum].kind {
        ValueKind::Binary { lhs, rhs, .. } => {
            assert_ne!(
                lhs, rhs,
                "mutated Index3D reads must not be treated as equivalent"
            )
        }
        _ => panic!("sum value shape changed unexpectedly"),
    }
}

#[test]
fn gvn_canonicalizes_index3d_operands_after_inner_cse() {
    let mut fn_ir = one_block_fn("gvn_index3d");
    let base = fn_ir.add_value(
        ValueKind::Load {
            var: "cube".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        Some("cube".to_string()),
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let call1 = fn_ir.add_value(
        ValueKind::Call {
            callee: "abs".to_string(),
            args: vec![one],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let call2 = fn_ir.add_value(
        ValueKind::Call {
            callee: "abs".to_string(),
            args: vec![one],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let idx1 = fn_ir.add_value(
        ValueKind::Index3D {
            base,
            i: call1,
            j: one,
            k: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let idx2 = fn_ir.add_value(
        ValueKind::Index3D {
            base,
            i: call2,
            j: one,
            k: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sum = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: idx1,
            rhs: idx2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(sum));

    let changed = optimize(&mut fn_ir);
    assert!(changed, "expected pure call and Index3D CSE to fire");
    match fn_ir.values[sum].kind {
        ValueKind::Binary { lhs, rhs, .. } => {
            assert_eq!(
                lhs, rhs,
                "expected Index3D operands to canonicalize through inner CSE"
            );
        }
        _ => panic!("sum value shape changed unexpectedly"),
    }
}

#[test]
fn gvn_propagates_record_literal_cse_into_field_gets() {
    let mut fn_ir = one_block_fn("gvn_record_field");
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let record1 = fn_ir.add_value(
        ValueKind::RecordLit {
            fields: vec![("x".to_string(), one)],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let record2 = fn_ir.add_value(
        ValueKind::RecordLit {
            fields: vec![("x".to_string(), one)],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let field1 = fn_ir.add_value(
        ValueKind::FieldGet {
            base: record1,
            field: "x".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let field2 = fn_ir.add_value(
        ValueKind::FieldGet {
            base: record2,
            field: "x".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sum = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: field1,
            rhs: field2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(sum));

    let changed = optimize(&mut fn_ir);
    assert!(changed, "expected record/field CSE to fire");
    match fn_ir.values[sum].kind {
        ValueKind::Binary { lhs, rhs, .. } => {
            assert_eq!(lhs, rhs, "expected duplicate field gets to be CSE'd");
        }
        _ => panic!("sum value shape changed unexpectedly"),
    }
}

#[test]
fn gvn_canonicalizes_commutative_binary_operands() {
    let mut fn_ir = one_block_fn("gvn_commutative");
    let two = fn_ir.add_value(
        ValueKind::Const(Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let five = fn_ir.add_value(
        ValueKind::Const(Lit::Int(5)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let add1 = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: two,
            rhs: five,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let add2 = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: five,
            rhs: two,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sum = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: add1,
            rhs: add2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(sum));

    let changed = optimize(&mut fn_ir);
    assert!(changed, "expected commutative binary CSE to fire");
    match fn_ir.values[sum].kind {
        ValueKind::Binary { lhs, rhs, .. } => {
            assert_eq!(
                lhs, rhs,
                "expected swapped commutative operands to canonicalize"
            );
        }
        _ => panic!("sum value shape changed unexpectedly"),
    }
}

#[test]
fn gvn_cse_duplicate_intrinsics() {
    let mut fn_ir = one_block_fn("gvn_intrinsic");
    let x = fn_ir.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let intr1 = fn_ir.add_value(
        ValueKind::Intrinsic {
            op: IntrinsicOp::VecAbsF64,
            args: vec![x],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let intr2 = fn_ir.add_value(
        ValueKind::Intrinsic {
            op: IntrinsicOp::VecAbsF64,
            args: vec![x],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sum = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: intr1,
            rhs: intr2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(sum));

    let changed = optimize(&mut fn_ir);
    assert!(changed, "expected intrinsic CSE to fire");
    match fn_ir.values[sum].kind {
        ValueKind::Binary { lhs, rhs, .. } => {
            assert_eq!(lhs, rhs, "expected duplicated intrinsic values to be CSE'd");
        }
        _ => panic!("sum value shape changed unexpectedly"),
    }
}

#[test]
fn gvn_propagates_fieldset_cse_into_field_gets() {
    let mut fn_ir = one_block_fn("gvn_fieldset_field");
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rec = fn_ir.add_value(
        ValueKind::RecordLit {
            fields: vec![("x".to_string(), one)],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let set1 = fn_ir.add_value(
        ValueKind::FieldSet {
            base: rec,
            field: "x".to_string(),
            value: two,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let set2 = fn_ir.add_value(
        ValueKind::FieldSet {
            base: rec,
            field: "x".to_string(),
            value: two,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let get1 = fn_ir.add_value(
        ValueKind::FieldGet {
            base: set1,
            field: "x".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let get2 = fn_ir.add_value(
        ValueKind::FieldGet {
            base: set2,
            field: "x".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sum = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: get1,
            rhs: get2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(sum));

    let changed = optimize(&mut fn_ir);
    assert!(changed, "expected fieldset/field CSE to fire");
    match fn_ir.values[sum].kind {
        ValueKind::Binary { lhs, rhs, .. } => {
            assert_eq!(
                lhs, rhs,
                "expected duplicate field gets through identical fieldset to be CSE'd"
            );
        }
        _ => panic!("sum value shape changed unexpectedly"),
    }
}
