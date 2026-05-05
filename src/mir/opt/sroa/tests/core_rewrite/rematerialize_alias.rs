use super::*;

#[test]
pub(crate) fn sroa_rematerializes_returned_alias_and_removes_dead_assignment() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let record = record_xy(&mut fn_ir, x, y);
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: record,
        span: Span::default(),
    });
    let load = fn_ir.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(load));

    assert!(optimize(&mut fn_ir), "expected return rematerialization");
    let Terminator::Return(Some(ret)) = fn_ir.blocks[fn_ir.entry].term else {
        panic!("entry block should return a rematerialized record");
    };
    assert_ne!(ret, load);
    assert!(matches!(
        &fn_ir.values[ret].kind,
        ValueKind::RecordLit { fields }
            if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
    ));
    assert!(
        fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
        "dead aggregate alias assignment should be removed after rematerialization"
    );
    assert!(
        matches!(fn_ir.values[load].kind, ValueKind::Const(Lit::Null)),
        "dead return alias load should be neutralized with its assignment"
    );
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_rematerializes_store_index_alias_value_and_removes_dead_assignment() {
    let mut fn_ir = FnIR::new("sroa_store_value_test".to_string(), vec!["xs".to_string()]);
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let xs = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("xs".to_string()),
    );
    let idx = int_value(&mut fn_ir, 1);
    let done = int_value(&mut fn_ir, 0);
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let record = record_xy(&mut fn_ir, x, y);
    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: record,
        span: Span::default(),
    });
    let load = fn_ir.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    fn_ir.blocks[entry].instrs.push(Instr::StoreIndex1D {
        base: xs,
        idx,
        val: load,
        is_safe: false,
        is_na_safe: false,
        is_vector: false,
        span: Span::default(),
    });
    fn_ir.blocks[entry].term = Terminator::Return(Some(done));

    assert!(
        optimize(&mut fn_ir),
        "StoreIndex aggregate operands should rematerialize instead of blocking SROA"
    );
    assert_eq!(
        fn_ir.blocks[entry].instrs.len(),
        1,
        "dead aggregate alias assignment should be removed after store rematerialization"
    );
    let Instr::StoreIndex1D { val, .. } = &fn_ir.blocks[entry].instrs[0] else {
        panic!("store instruction should remain after aggregate rematerialization");
    };
    assert_ne!(*val, load);
    assert!(matches!(
        &fn_ir.values[*val].kind,
        ValueKind::RecordLit { fields }
            if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
    ));
    assert!(
        matches!(fn_ir.values[load].kind, ValueKind::Const(Lit::Null)),
        "dead store alias load should be neutralized with its assignment"
    );
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_rematerializes_unknown_call_alias_arg_and_removes_dead_assignment() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let record = record_xy(&mut fn_ir, x, y);
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: record,
        span: Span::default(),
    });
    let load = fn_ir.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    let call = fn_ir.add_value(
        ValueKind::Call {
            callee: "opaque_helper".to_string(),
            args: vec![load],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(call));

    assert!(
        optimize(&mut fn_ir),
        "expected call argument rematerialization"
    );
    let ValueKind::Call { args, .. } = &fn_ir.values[call].kind else {
        panic!("call value should remain a call");
    };
    assert_eq!(args.len(), 1);
    assert_ne!(args[0], load);
    assert!(matches!(
        &fn_ir.values[args[0]].kind,
        ValueKind::RecordLit { fields }
            if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
    ));
    assert!(
        fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
        "dead aggregate alias assignment should be removed after call rematerialization"
    );
    assert!(
        matches!(fn_ir.values[load].kind, ValueKind::Const(Lit::Null)),
        "dead call alias load should be neutralized with its assignment"
    );
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_rematerializes_intrinsic_alias_arg_and_removes_dead_assignment() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let record = record_xy(&mut fn_ir, x, y);
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: record,
        span: Span::default(),
    });
    let load = fn_ir.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    let intrinsic = fn_ir.add_value(
        ValueKind::Intrinsic {
            op: IntrinsicOp::VecMeanF64,
            args: vec![load],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(intrinsic));

    assert!(
        optimize(&mut fn_ir),
        "expected intrinsic argument rematerialization"
    );
    let ValueKind::Intrinsic { args, .. } = &fn_ir.values[intrinsic].kind else {
        panic!("intrinsic value should remain an intrinsic");
    };
    assert_eq!(args.len(), 1);
    assert_ne!(args[0], load);
    assert!(matches!(
        &fn_ir.values[args[0]].kind,
        ValueKind::RecordLit { fields }
            if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
    ));
    assert!(
        fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
        "dead aggregate alias assignment should be removed after intrinsic rematerialization"
    );
    assert!(matches!(
        fn_ir.values[load].kind,
        ValueKind::Const(Lit::Null)
    ));
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_rematerializes_eval_alias_and_removes_dead_assignment() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let done = int_value(&mut fn_ir, 0);
    let record = record_xy(&mut fn_ir, x, y);
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: record,
        span: Span::default(),
    });
    let load = fn_ir.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
        val: load,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(done));

    assert!(
        optimize(&mut fn_ir),
        "expected eval boundary rematerialization"
    );
    assert_eq!(fn_ir.blocks[fn_ir.entry].instrs.len(), 1);
    let Instr::Eval { val, .. } = fn_ir.blocks[fn_ir.entry].instrs[0] else {
        panic!("aggregate assignment should be removed and eval should remain");
    };
    assert_ne!(val, load);
    assert!(matches!(
        &fn_ir.values[val].kind,
        ValueKind::RecordLit { fields }
            if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
    ));
    assert!(
        matches!(fn_ir.values[load].kind, ValueKind::Const(Lit::Null)),
        "dead eval alias load should be neutralized with its assignment"
    );
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_rematerializes_nested_record_alias_field_and_removes_dead_assignment() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let mass = int_value(&mut fn_ir, 3);
    let record = record_xy(&mut fn_ir, x, y);
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "pos".to_string(),
        src: record,
        span: Span::default(),
    });
    let load = fn_ir.add_value(
        ValueKind::Load {
            var: "pos".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("pos".to_string()),
    );
    let body = record_pos_mass(&mut fn_ir, load, mass);
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(body));

    assert!(
        optimize(&mut fn_ir),
        "expected nested record field rematerialization"
    );
    let ValueKind::RecordLit { fields } = &fn_ir.values[body].kind else {
        panic!("outer record should remain a record literal");
    };
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].0, "pos");
    assert_ne!(fields[0].1, load);
    assert!(matches!(
        &fn_ir.values[fields[0].1].kind,
        ValueKind::RecordLit { fields }
            if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
    ));
    assert_eq!(fields[1], ("mass".to_string(), mass));
    assert!(
        fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
        "dead nested aggregate alias assignment should be removed"
    );
    assert!(
        matches!(fn_ir.values[load].kind, ValueKind::Const(Lit::Null)),
        "dead nested alias load should be neutralized with its assignment"
    );
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_rematerializes_index_base_alias_and_removes_dead_assignment() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let idx = int_value(&mut fn_ir, 1);
    let record = record_xy(&mut fn_ir, x, y);
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: record,
        span: Span::default(),
    });
    let load = fn_ir.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    let indexed = fn_ir.add_value(
        ValueKind::Index1D {
            base: load,
            idx,
            is_safe: false,
            is_na_safe: false,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(indexed));

    assert!(
        optimize(&mut fn_ir),
        "expected index base rematerialization"
    );
    let ValueKind::Index1D {
        base, idx: got_idx, ..
    } = &fn_ir.values[indexed].kind
    else {
        panic!("indexed value should remain an Index1D");
    };
    assert_ne!(*base, load);
    assert_eq!(*got_idx, idx);
    assert!(matches!(
        &fn_ir.values[*base].kind,
        ValueKind::RecordLit { fields }
            if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
    ));
    assert!(
        fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
        "dead index base alias assignment should be removed"
    );
    assert!(matches!(
        fn_ir.values[load].kind,
        ValueKind::Const(Lit::Null)
    ));
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_rematerializes_len_base_alias_and_removes_dead_assignment() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let record = record_xy(&mut fn_ir, x, y);
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: record,
        span: Span::default(),
    });
    let load = fn_ir.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    let len = fn_ir.add_value(
        ValueKind::Len { base: load },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(len));

    assert!(optimize(&mut fn_ir), "expected len base rematerialization");
    let ValueKind::Len { base } = &fn_ir.values[len].kind else {
        panic!("len value should remain a Len");
    };
    assert_ne!(*base, load);
    assert!(matches!(
        &fn_ir.values[*base].kind,
        ValueKind::RecordLit { fields }
            if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
    ));
    assert!(
        fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
        "dead len base alias assignment should be removed"
    );
    assert!(matches!(
        fn_ir.values[load].kind,
        ValueKind::Const(Lit::Null)
    ));
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}
