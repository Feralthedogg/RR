use super::*;

#[test]
pub(crate) fn sroa_does_not_drop_impure_unused_record_fields() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
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
    let record = record_xy(&mut fn_ir, x, impure);
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

    assert!(
        !optimize(&mut fn_ir),
        "SROA must not remove record construction when it would drop an impure field"
    );
    assert!(matches!(
        fn_ir.blocks[fn_ir.entry].term,
        Terminator::Return(Some(ret)) if ret == get_x
    ));
}

#[test]
pub(crate) fn sroa_snapshots_record_field_load_before_reassignment() {
    let mut fn_ir = test_fn();
    let initial = int_value(&mut fn_ir, 1);
    let replacement = int_value(&mut fn_ir, 2);
    let y = int_value(&mut fn_ir, 3);
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: initial,
        span: Span::default(),
    });
    let load_x = fn_ir.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let record = record_xy(&mut fn_ir, load_x, y);
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: record,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: replacement,
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
    let get_x = fn_ir.add_value(
        ValueKind::FieldGet {
            base: load_point,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_x));

    assert!(
        optimize(&mut fn_ir),
        "SROA should snapshot load fields at the aggregate alias assignment"
    );
    let Terminator::Return(Some(ret)) = fn_ir.blocks[fn_ir.entry].term else {
        panic!("entry block should still return a value");
    };
    let ValueKind::Load { var } = &fn_ir.values[ret].kind else {
        panic!("projected field should load the snapshot temp");
    };
    assert!(var.contains("__rr_sroa_snap_x"));
    assert!(matches!(
        &fn_ir.blocks[fn_ir.entry].instrs[..],
        [
            Instr::Assign {
                dst: initial_dst,
                src: initial_src,
                ..
            },
            Instr::Assign { dst, src, .. },
            Instr::Assign {
                dst: reassigned,
                src: reassigned_src,
                ..
            },
        ] if initial_dst == "x"
            && *initial_src == initial
            && dst == var
            && *src == load_x
            && reassigned == "x"
            && *reassigned_src == replacement
    ));
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_snapshots_record_field_expression_before_reassignment() {
    let mut fn_ir = test_fn();
    let initial = int_value(&mut fn_ir, 1);
    let replacement = int_value(&mut fn_ir, 2);
    let y = int_value(&mut fn_ir, 3);
    let one = int_value(&mut fn_ir, 1);
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: initial,
        span: Span::default(),
    });
    let load_x = fn_ir.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let computed_x = binary_value(&mut fn_ir, BinOp::Add, load_x, one);
    let record = record_xy(&mut fn_ir, computed_x, y);
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: record,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: replacement,
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
    let get_x = fn_ir.add_value(
        ValueKind::FieldGet {
            base: load_point,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_x));

    assert!(
        optimize(&mut fn_ir),
        "SROA should snapshot pure load-dependent field expressions"
    );
    let Terminator::Return(Some(ret)) = fn_ir.blocks[fn_ir.entry].term else {
        panic!("entry block should still return a value");
    };
    let ValueKind::Load { var } = &fn_ir.values[ret].kind else {
        panic!("projected field should load the expression snapshot temp");
    };
    assert!(var.contains("__rr_sroa_snap_x"));
    assert!(matches!(
        &fn_ir.blocks[fn_ir.entry].instrs[..],
        [
            Instr::Assign {
                dst: initial_dst,
                src: initial_src,
                ..
            },
            Instr::Assign { dst, src, .. },
            Instr::Assign {
                dst: reassigned,
                src: reassigned_src,
                ..
            },
        ] if initial_dst == "x"
            && *initial_src == initial
            && dst == var
            && *src == computed_x
            && reassigned == "x"
            && *reassigned_src == replacement
    ));
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}

#[test]
pub(crate) fn sroa_rematerializes_snapshot_record_alias_return_after_reassignment() {
    let mut fn_ir = test_fn();
    let initial = int_value(&mut fn_ir, 1);
    let replacement = int_value(&mut fn_ir, 2);
    let y = int_value(&mut fn_ir, 3);
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: initial,
        span: Span::default(),
    });
    let load_x = fn_ir.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let record = record_xy(&mut fn_ir, load_x, y);
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: record,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: replacement,
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
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(load_point));

    assert!(
        optimize(&mut fn_ir),
        "SROA should rematerialize a returned alias from snapshot fields"
    );
    let Terminator::Return(Some(ret)) = fn_ir.blocks[fn_ir.entry].term else {
        panic!("entry block should return a rematerialized record");
    };
    let ValueKind::RecordLit { fields } = &fn_ir.values[ret].kind else {
        panic!("returned value should rematerialize as a record literal");
    };
    assert_eq!(fields.len(), 2);
    let ValueKind::Load { var } = &fn_ir.values[fields[0].1].kind else {
        panic!("x field should be loaded from its snapshot temp");
    };
    assert!(var.contains("__rr_sroa_snap_x"));
    assert_eq!(fields[1], ("y".to_string(), y));
    assert!(fn_ir.blocks[fn_ir.entry].instrs.iter().any(
        |instr| matches!(instr, Instr::Assign { dst, src, .. } if dst == var && *src == load_x)
    ));
    assert!(matches!(
        fn_ir.values[load_point].kind,
        ValueKind::Const(Lit::Null)
    ));
    assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
}
