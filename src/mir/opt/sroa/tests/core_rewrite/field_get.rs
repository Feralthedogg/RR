use super::*;

#[test]
pub(crate) fn sroa_rewrites_direct_record_field_get() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let record = record_xy(&mut fn_ir, x, y);
    let get_x = fn_ir.add_value(
        ValueKind::FieldGet {
            base: record,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_x));

    assert!(optimize(&mut fn_ir), "expected SROA rewrite");
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].term,
        Terminator::Return(Some(ret)) if ret == x
    ));
}

#[test]
pub(crate) fn sroa_rewrites_nested_record_field_get_in_one_pass() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let mass = int_value(&mut fn_ir, 3);
    let pos = record_xy(&mut fn_ir, x, y);
    let body = record_pos_mass(&mut fn_ir, pos, mass);
    let get_pos = fn_ir.add_value(
        ValueKind::FieldGet {
            base: body,
            field: "pos".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let get_x = fn_ir.add_value(
        ValueKind::FieldGet {
            base: get_pos,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_x));

    assert!(optimize(&mut fn_ir), "expected nested SROA rewrite");
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].term,
        Terminator::Return(Some(ret)) if ret == x
    ));
}

#[test]
pub(crate) fn sroa_scalarizes_straight_line_record_chain_to_projected_scalar() {
    let mut fn_ir = test_fn();
    let ax = int_value(&mut fn_ir, 10);
    let ay = int_value(&mut fn_ir, 15);
    let vx = int_value(&mut fn_ir, 2);
    let vy = int_value(&mut fn_ir, -3);
    let dt = int_value(&mut fn_ir, 2);

    let moved_x = binary_value(&mut fn_ir, BinOp::Add, ax, vx);
    let moved_y = binary_value(&mut fn_ir, BinOp::Add, ay, vy);
    let moved = record_xy(&mut fn_ir, moved_x, moved_y);
    let rebound_x = fn_ir.add_value(
        ValueKind::Unary {
            op: UnaryOp::Neg,
            rhs: vx,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let rebound_y = fn_ir.add_value(
        ValueKind::Unary {
            op: UnaryOp::Neg,
            rhs: vy,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let rebound = record_xy(&mut fn_ir, rebound_x, rebound_y);
    let moved_get_x = fn_ir.add_value(
        ValueKind::FieldGet {
            base: moved,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let rebound_get_x = fn_ir.add_value(
        ValueKind::FieldGet {
            base: rebound,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let translated_x = binary_value(&mut fn_ir, BinOp::Add, moved_get_x, rebound_get_x);
    let moved_get_y = fn_ir.add_value(
        ValueKind::FieldGet {
            base: moved,
            field: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let rebound_get_y = fn_ir.add_value(
        ValueKind::FieldGet {
            base: rebound,
            field: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let translated_y = binary_value(&mut fn_ir, BinOp::Add, moved_get_y, rebound_get_y);
    let translated = record_xy(&mut fn_ir, translated_x, translated_y);
    let translated_get_x = fn_ir.add_value(
        ValueKind::FieldGet {
            base: translated,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let final_x = binary_value(&mut fn_ir, BinOp::Mul, translated_get_x, dt);
    let translated_get_y = fn_ir.add_value(
        ValueKind::FieldGet {
            base: translated,
            field: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let final_y = binary_value(&mut fn_ir, BinOp::Mul, translated_get_y, dt);
    let final_record = record_xy(&mut fn_ir, final_x, final_y);
    let projected = fn_ir.add_value(
        ValueKind::FieldGet {
            base: final_record,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(projected));

    assert!(optimize(&mut fn_ir), "expected chained SROA rewrite");
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].term,
        Terminator::Return(Some(ret)) if ret == final_x
    ));
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_rewrites_single_load_alias_field_get() {
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
    let get_y = fn_ir.add_value(
        ValueKind::FieldGet {
            base: load,
            field: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_y));

    assert!(optimize(&mut fn_ir), "expected SROA alias rewrite");
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].term,
        Terminator::Return(Some(ret)) if ret == y
    ));
    assert!(
        fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
        "dead scalarized aggregate assignment should be removed"
    );
    assert!(
        matches!(fn_ir.values[load].kind, ValueKind::Const(Lit::Null)),
        "dead aggregate load alias should be neutralized with its assignment"
    );
}

#[test]
pub(crate) fn sroa_scalarizes_record_in_function_with_unrelated_store_index() {
    let mut fn_ir = FnIR::new("sroa_store_test".to_string(), vec!["xs".to_string()]);
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
    let stored = int_value(&mut fn_ir, 99);
    fn_ir.blocks[entry].instrs.push(Instr::StoreIndex1D {
        base: xs,
        idx,
        val: stored,
        is_safe: false,
        is_na_safe: false,
        is_vector: false,
        span: Span::default(),
    });

    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let record = record_xy(&mut fn_ir, x, y);
    let get_x = fn_ir.add_value(
        ValueKind::FieldGet {
            base: record,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(get_x));

    assert!(
        optimize(&mut fn_ir),
        "unrelated StoreIndex should not block local record SROA"
    );
    assert!(matches!(
        fn_ir.blocks[entry].term,
        Terminator::Return(Some(ret)) if ret == x
    ));
    assert!(matches!(
        fn_ir.blocks[entry].instrs.as_slice(),
        [Instr::StoreIndex1D { base, idx: got_idx, val, .. }]
            if *base == xs && *got_idx == idx && *val == stored
    ));
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}
