use super::*;

#[test]
pub(crate) fn sroa_splits_branch_record_phi_for_projected_field() {
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
    let get_x = fn_ir.add_value(
        ValueKind::FieldGet {
            base: record_phi,
            field: "x".to_string(),
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
    fn_ir.blocks[merge_bb].term = Terminator::Return(Some(get_x));

    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    assert!(optimize(&mut fn_ir), "expected branch phi SROA rewrite");
    let Terminator::Return(Some(ret)) = fn_ir.blocks[merge_bb].term else {
        panic!("merge block should still return a value");
    };
    assert_ne!(ret, get_x);
    assert!(matches!(
        &fn_ir.values[ret].kind,
        ValueKind::Phi { args } if *args == vec![(x1, left_bb), (x2, right_bb)]
    ));
    assert_eq!(fn_ir.values[ret].phi_block, Some(merge_bb));
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_splits_aliased_branch_record_phi_for_projected_field() {
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
    fn_ir.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left_bb,
        else_bb: right_bb,
    };
    fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[merge_bb].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: record_phi,
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
    fn_ir.blocks[merge_bb].term = Terminator::Return(Some(get_y));

    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    assert!(
        optimize(&mut fn_ir),
        "expected aliased branch phi SROA rewrite"
    );
    let Terminator::Return(Some(ret)) = fn_ir.blocks[merge_bb].term else {
        panic!("merge block should still return a value");
    };
    assert!(matches!(
        &fn_ir.values[ret].kind,
        ValueKind::Phi { args } if *args == vec![(y1, left_bb), (y2, right_bb)]
    ));
    assert_eq!(fn_ir.values[ret].phi_block, Some(merge_bb));
    assert!(
        fn_ir.blocks[merge_bb].instrs.is_empty(),
        "dead aliased aggregate phi assignment should be removed"
    );
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_splits_transitive_aliased_branch_record_phi_for_projected_field() {
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
    fn_ir.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left_bb,
        else_bb: right_bb,
    };
    fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[merge_bb].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: record_phi,
        span: Span::default(),
    });
    let load_point = fn_ir.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    fn_ir.blocks[merge_bb].instrs.push(Instr::Assign {
        dst: "alias".to_string(),
        src: load_point,
        span: Span::default(),
    });
    let load_alias = fn_ir.add_value(
        ValueKind::Load {
            var: "alias".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("alias".to_string()),
    );
    let get_x = fn_ir.add_value(
        ValueKind::FieldGet {
            base: load_alias,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[merge_bb].term = Terminator::Return(Some(get_x));

    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    assert!(
        optimize(&mut fn_ir),
        "expected transitive aliased branch phi SROA rewrite"
    );
    let Terminator::Return(Some(ret)) = fn_ir.blocks[merge_bb].term else {
        panic!("merge block should still return a value");
    };
    assert!(matches!(
        &fn_ir.values[ret].kind,
        ValueKind::Phi { args } if *args == vec![(x1, left_bb), (x2, right_bb)]
    ));
    assert_eq!(fn_ir.values[ret].phi_block, Some(merge_bb));
    assert!(
        fn_ir.blocks[merge_bb].instrs.is_empty(),
        "dead transitive aggregate aliases should be removed"
    );
    assert!(matches!(
        fn_ir.values[load_point].kind,
        ValueKind::Const(Lit::Null)
    ));
    assert!(matches!(
        fn_ir.values[load_alias].kind,
        ValueKind::Const(Lit::Null)
    ));
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_splits_nested_branch_record_phi_for_projected_field_in_one_pass() {
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
    let mass1 = int_value(&mut fn_ir, 10);
    let x2 = int_value(&mut fn_ir, 3);
    let y2 = int_value(&mut fn_ir, 4);
    let mass2 = int_value(&mut fn_ir, 20);
    let left_pos = record_xy(&mut fn_ir, x1, y1);
    let right_pos = record_xy(&mut fn_ir, x2, y2);
    let left_record = record_pos_mass(&mut fn_ir, left_pos, mass1);
    let right_record = record_pos_mass(&mut fn_ir, right_pos, mass2);
    let record_phi = fn_ir.add_value(
        ValueKind::Phi {
            args: vec![(left_record, left_bb), (right_record, right_bb)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.values[record_phi].phi_block = Some(merge_bb);
    let get_pos = fn_ir.add_value(
        ValueKind::FieldGet {
            base: record_phi,
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
    fn_ir.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left_bb,
        else_bb: right_bb,
    };
    fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[merge_bb].term = Terminator::Return(Some(get_x));

    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    assert!(
        optimize(&mut fn_ir),
        "expected nested branch phi SROA rewrite"
    );
    let Terminator::Return(Some(ret)) = fn_ir.blocks[merge_bb].term else {
        panic!("merge block should still return a value");
    };
    assert_ne!(ret, get_x);
    assert!(matches!(
        &fn_ir.values[ret].kind,
        ValueKind::Phi { args } if *args == vec![(x1, left_bb), (x2, right_bb)]
    ));
    assert_eq!(fn_ir.values[ret].phi_block, Some(merge_bb));
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());

    let value_count = fn_ir.values.len();
    assert!(
        !optimize(&mut fn_ir),
        "nested phi scalarization should be idempotent"
    );
    assert_eq!(fn_ir.values.len(), value_count);
}

#[test]
pub(crate) fn sroa_splits_and_rematerializes_branch_record_phi_return() {
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
    fn_ir.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left_bb,
        else_bb: right_bb,
    };
    fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[merge_bb].term = Terminator::Return(Some(record_phi));

    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    assert!(
        optimize(&mut fn_ir),
        "expected aggregate phi return rematerialization"
    );
    let Terminator::Return(Some(ret)) = fn_ir.blocks[merge_bb].term else {
        panic!("merge block should return a rematerialized record");
    };
    assert_ne!(ret, record_phi);
    let ValueKind::RecordLit { fields } = &fn_ir.values[ret].kind else {
        panic!("returned value should rematerialize as a record literal");
    };
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].0, "x");
    assert_eq!(fields[1].0, "y");
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

#[test]
pub(crate) fn sroa_splits_and_rematerializes_transitive_alias_branch_record_phi_return() {
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
    fn_ir.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left_bb,
        else_bb: right_bb,
    };
    fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[merge_bb].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: record_phi,
        span: Span::default(),
    });
    let load_point = fn_ir.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    fn_ir.blocks[merge_bb].instrs.push(Instr::Assign {
        dst: "alias".to_string(),
        src: load_point,
        span: Span::default(),
    });
    let load_alias = fn_ir.add_value(
        ValueKind::Load {
            var: "alias".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("alias".to_string()),
    );
    fn_ir.blocks[merge_bb].term = Terminator::Return(Some(load_alias));

    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    assert!(
        optimize(&mut fn_ir),
        "expected transitive alias aggregate phi return rematerialization"
    );
    let Terminator::Return(Some(ret)) = fn_ir.blocks[merge_bb].term else {
        panic!("merge block should return a rematerialized record");
    };
    assert_ne!(ret, load_alias);
    let ValueKind::RecordLit { fields } = &fn_ir.values[ret].kind else {
        panic!("returned value should rematerialize as a record literal");
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
    assert!(
        fn_ir.blocks[merge_bb].instrs.is_empty(),
        "dead transitive aggregate aliases should be removed"
    );
    assert!(matches!(
        fn_ir.values[load_point].kind,
        ValueKind::Const(Lit::Null)
    ));
    assert!(matches!(
        fn_ir.values[load_alias].kind,
        ValueKind::Const(Lit::Null)
    ));
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_splits_and_rematerializes_branch_record_phi_nested_record_field() {
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
    let mass = int_value(&mut fn_ir, 5);
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
    let body = record_pos_mass(&mut fn_ir, record_phi, mass);
    fn_ir.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left_bb,
        else_bb: right_bb,
    };
    fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[merge_bb].term = Terminator::Return(Some(body));

    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    assert!(
        optimize(&mut fn_ir),
        "expected nested record field aggregate phi rematerialization"
    );
    let ValueKind::RecordLit { fields } = &fn_ir.values[body].kind else {
        panic!("outer record should remain a record literal");
    };
    assert_eq!(fields.len(), 2);
    let ValueKind::RecordLit { fields: pos_fields } = &fn_ir.values[fields[0].1].kind else {
        panic!("nested phi field should rematerialize as a record literal");
    };
    assert_eq!(pos_fields.len(), 2);
    assert!(matches!(
        &fn_ir.values[pos_fields[0].1].kind,
        ValueKind::Phi { args } if *args == vec![(x1, left_bb), (x2, right_bb)]
    ));
    assert_eq!(fn_ir.values[pos_fields[0].1].phi_block, Some(merge_bb));
    assert!(matches!(
        &fn_ir.values[pos_fields[1].1].kind,
        ValueKind::Phi { args } if *args == vec![(y1, left_bb), (y2, right_bb)]
    ));
    assert_eq!(fn_ir.values[pos_fields[1].1].phi_block, Some(merge_bb));
    assert_eq!(fields[1], ("mass".to_string(), mass));
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_ignores_dead_nested_record_phi_materialization_demand() {
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
    let done = int_value(&mut fn_ir, 0);
    let x1 = int_value(&mut fn_ir, 1);
    let y1 = int_value(&mut fn_ir, 2);
    let x2 = int_value(&mut fn_ir, 3);
    let y2 = int_value(&mut fn_ir, 4);
    let mass = int_value(&mut fn_ir, 5);
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
    let _dead_body = record_pos_mass(&mut fn_ir, record_phi, mass);
    fn_ir.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left_bb,
        else_bb: right_bb,
    };
    fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[merge_bb].term = Terminator::Return(Some(done));

    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    let value_count = fn_ir.values.len();
    assert!(
        !optimize(&mut fn_ir),
        "dead nested aggregate values must not create SROA materialization demand"
    );
    assert_eq!(fn_ir.values.len(), value_count);
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_splits_and_rematerializes_branch_record_phi_index_base() {
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
    let idx = int_value(&mut fn_ir, 1);
    let x1 = int_value(&mut fn_ir, 1);
    let y1 = int_value(&mut fn_ir, 2);
    let x2 = int_value(&mut fn_ir, 3);
    let y2 = int_value(&mut fn_ir, 4);
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
    let indexed = fn_ir.add_value(
        ValueKind::Index1D {
            base: record_phi,
            idx,
            is_safe: false,
            is_na_safe: false,
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
    fn_ir.blocks[merge_bb].term = Terminator::Return(Some(indexed));

    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    assert!(
        optimize(&mut fn_ir),
        "expected index base aggregate phi rematerialization"
    );
    let ValueKind::Index1D {
        base, idx: got_idx, ..
    } = &fn_ir.values[indexed].kind
    else {
        panic!("indexed value should remain an Index1D");
    };
    assert_ne!(*base, record_phi);
    assert_eq!(*got_idx, idx);
    let ValueKind::RecordLit { fields } = &fn_ir.values[*base].kind else {
        panic!("index base should rematerialize as a record literal");
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

#[test]
pub(crate) fn sroa_splits_and_rematerializes_branch_record_phi_eval() {
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
    let done = int_value(&mut fn_ir, 0);
    let x1 = int_value(&mut fn_ir, 1);
    let y1 = int_value(&mut fn_ir, 2);
    let x2 = int_value(&mut fn_ir, 3);
    let y2 = int_value(&mut fn_ir, 4);
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
    fn_ir.blocks[entry].term = Terminator::If {
        cond,
        then_bb: left_bb,
        else_bb: right_bb,
    };
    fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
    fn_ir.blocks[merge_bb].instrs.push(Instr::Eval {
        val: record_phi,
        span: Span::default(),
    });
    fn_ir.blocks[merge_bb].term = Terminator::Return(Some(done));

    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    assert!(
        optimize(&mut fn_ir),
        "expected aggregate phi eval rematerialization"
    );
    let [Instr::Eval { val, .. }] = fn_ir.blocks[merge_bb].instrs.as_slice() else {
        panic!("merge block should keep one rematerialized eval");
    };
    assert_ne!(*val, record_phi);
    let ValueKind::RecordLit { fields } = &fn_ir.values[*val].kind else {
        panic!("eval value should rematerialize as a record literal");
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

#[test]
pub(crate) fn sroa_splits_loop_carried_record_phi_for_projected_field() {
    let mut fn_ir = test_fn();
    let entry = fn_ir.entry;
    let header_bb = fn_ir.add_block();
    let body_bb = fn_ir.add_block();
    let exit_bb = fn_ir.add_block();
    fn_ir.body_head = header_bb;

    let cond = fn_ir.add_value(
        ValueKind::Const(Lit::Bool(true)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let x0 = int_value(&mut fn_ir, 0);
    let y0 = int_value(&mut fn_ir, 10);
    let one = int_value(&mut fn_ir, 1);
    let seed = record_xy(&mut fn_ir, x0, y0);
    let record_phi = fn_ir.add_value(
        ValueKind::Phi {
            args: vec![(seed, entry), (seed, body_bb)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.values[record_phi].phi_block = Some(header_bb);
    let get_x = fn_ir.add_value(
        ValueKind::FieldGet {
            base: record_phi,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let next_x = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: get_x,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let get_y = fn_ir.add_value(
        ValueKind::FieldGet {
            base: record_phi,
            field: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let updated = record_xy(&mut fn_ir, next_x, get_y);
    if let ValueKind::Phi { args } = &mut fn_ir.values[record_phi].kind {
        args[1] = (updated, body_bb);
    }

    fn_ir.blocks[entry].term = Terminator::Goto(header_bb);
    fn_ir.blocks[header_bb].term = Terminator::If {
        cond,
        then_bb: body_bb,
        else_bb: exit_bb,
    };
    fn_ir.blocks[body_bb].term = Terminator::Goto(header_bb);
    fn_ir.blocks[exit_bb].term = Terminator::Return(Some(get_x));

    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    assert!(
        optimize(&mut fn_ir),
        "expected loop-carried record phi SROA rewrite"
    );
    let Terminator::Return(Some(ret)) = fn_ir.blocks[exit_bb].term else {
        panic!("exit block should still return a value");
    };
    assert_ne!(ret, get_x);
    assert!(matches!(
        &fn_ir.values[ret].kind,
        ValueKind::Phi { args } if *args == vec![(x0, entry), (next_x, body_bb)]
    ));
    assert_eq!(fn_ir.values[ret].phi_block, Some(header_bb));
    assert!(matches!(
        &fn_ir.values[next_x].kind,
        ValueKind::Binary { lhs, rhs, .. } if *lhs == ret && *rhs == one
    ));
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());

    let value_count = fn_ir.values.len();
    assert!(
        !optimize(&mut fn_ir),
        "dead projections must not keep growing scalar phi values"
    );
    assert_eq!(fn_ir.values.len(), value_count);
}
