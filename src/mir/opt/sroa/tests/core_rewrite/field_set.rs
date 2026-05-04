use super::*;

#[test]
pub(crate) fn sroa_rewrites_field_set_updated_field() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let replacement = int_value(&mut fn_ir, 3);
    let record = record_xy(&mut fn_ir, x, y);
    let updated = fn_ir.add_value(
        ValueKind::FieldSet {
            base: record,
            field: "x".to_string(),
            value: replacement,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let get_x = fn_ir.add_value(
        ValueKind::FieldGet {
            base: updated,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_x));

    assert!(optimize(&mut fn_ir), "expected FieldSet SROA rewrite");
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].term,
        Terminator::Return(Some(ret)) if ret == replacement
    ));
}

#[test]
pub(crate) fn sroa_rewrites_field_set_unchanged_field() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let replacement = int_value(&mut fn_ir, 3);
    let record = record_xy(&mut fn_ir, x, y);
    let updated = fn_ir.add_value(
        ValueKind::FieldSet {
            base: record,
            field: "x".to_string(),
            value: replacement,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let get_y = fn_ir.add_value(
        ValueKind::FieldGet {
            base: updated,
            field: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_y));

    assert!(optimize(&mut fn_ir), "expected FieldSet SROA rewrite");
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].term,
        Terminator::Return(Some(ret)) if ret == y
    ));
}

#[test]
pub(crate) fn sroa_rewrites_field_set_alias_and_removes_dead_assignment() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let replacement = int_value(&mut fn_ir, 4);
    let record = record_xy(&mut fn_ir, x, y);
    let updated = fn_ir.add_value(
        ValueKind::FieldSet {
            base: record,
            field: "y".to_string(),
            value: replacement,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: updated,
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

    assert!(optimize(&mut fn_ir), "expected FieldSet alias SROA rewrite");
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].term,
        Terminator::Return(Some(ret)) if ret == replacement
    ));
    assert!(
        fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
        "dead scalarized FieldSet assignment should be removed"
    );
}

#[test]
pub(crate) fn sroa_does_not_drop_impure_field_set_update() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
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
    let record = record_xy(&mut fn_ir, x, y);
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
    let get_y = fn_ir.add_value(
        ValueKind::FieldGet {
            base: updated,
            field: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_y));

    assert!(
        !optimize(&mut fn_ir),
        "SROA must not remove an impure FieldSet update even when reading another field"
    );
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].term,
        Terminator::Return(Some(ret)) if ret == get_y
    ));
}

#[test]
pub(crate) fn sroa_rematerializes_field_set_alias_base_with_impure_update() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
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
    let updated = fn_ir.add_value(
        ValueKind::FieldSet {
            base: load,
            field: "x".to_string(),
            value: impure,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(updated));

    assert!(
        optimize(&mut fn_ir),
        "expected FieldSet base rematerialization"
    );
    let ValueKind::FieldSet { base, value, .. } = &fn_ir.values[updated].kind else {
        panic!("updated value should remain a functional FieldSet");
    };
    assert_ne!(*base, load);
    assert_eq!(*value, impure);
    assert!(matches!(
        &fn_ir.values[*base].kind,
        ValueKind::RecordLit { fields }
            if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
    ));
    assert!(
        fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
        "dead FieldSet base alias assignment should be removed"
    );
    assert!(
        matches!(fn_ir.values[load].kind, ValueKind::Const(Lit::Null)),
        "dead FieldSet base alias load should be neutralized"
    );
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_splits_and_rematerializes_field_set_phi_base_with_impure_update() {
    let mut fn_ir = test_fn();
    let entry = fn_ir.entry;
    let left_bb = fn_ir.add_block();
    let right_bb = fn_ir.add_block();
    let merge_bb = fn_ir.add_block();
    let cond = fn_ir.add_value(
        ValueKind::Const(Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let x1 = int_value(&mut fn_ir, 1);
    let y1 = int_value(&mut fn_ir, 2);
    let x2 = int_value(&mut fn_ir, 3);
    let y2 = int_value(&mut fn_ir, 4);
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
    let left_record = record_xy(&mut fn_ir, x1, y1);
    let right_record = record_xy(&mut fn_ir, x2, y2);
    let record_phi = fn_ir.add_value(
        ValueKind::Phi {
            args: vec![(left_record, left_bb), (right_record, right_bb)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.values[record_phi].phi_block = Some(merge_bb);
    let updated = fn_ir.add_value(
        ValueKind::FieldSet {
            base: record_phi,
            field: "x".to_string(),
            value: impure,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left_bb,
        else_bb: right_bb,
    };
    fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[merge_bb].term = Terminator::Return(Some(updated));

    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    assert!(
        optimize(&mut fn_ir),
        "expected FieldSet phi base rematerialization"
    );
    let ValueKind::FieldSet { base, value, .. } = &fn_ir.values[updated].kind else {
        panic!("updated value should remain a functional FieldSet");
    };
    assert_ne!(*base, record_phi);
    assert_eq!(*value, impure);
    let ValueKind::RecordLit { fields } = &fn_ir.values[*base].kind else {
        panic!("FieldSet base should rematerialize as a record literal");
    };
    assert_eq!(fields.len(), 2);
    assert!(matches!(
        &fn_ir.values[fields[0].1].kind,
        ValueKind::Phi { args } if *args == vec![(x1, left_bb), (x2, right_bb)]
    ));
    assert_eq!(fn_ir.values[fields[0].1].phi_block, Some(merge_bb));
    assert!(matches!(
        &fn_ir.values[fields[1].1].kind,
        ValueKind::Phi { args } if *args == vec![(y1, left_bb), (y2, right_bb)]
    ));
    assert_eq!(fn_ir.values[fields[1].1].phi_block, Some(merge_bb));
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}
