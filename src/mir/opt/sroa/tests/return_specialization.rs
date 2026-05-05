use super::*;

#[test]
pub(crate) fn sroa_specializes_direct_record_return_field_call() {
    let mut caller = FnIR::new("caller".to_string(), vec![]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let call = caller.add_value(
        ValueKind::Call {
            callee: "make_xy".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let get_x = caller.add_value(
        ValueKind::FieldGet {
            base: call,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(get_x));

    let mut all_fns = FxHashMap::default();
    all_fns.insert("caller".to_string(), caller);
    all_fns.insert("make_xy".to_string(), make_xy_fn());

    assert!(specialize_record_return_field_calls(&mut all_fns));
    let caller = all_fns.get("caller").expect("caller");
    assert!(matches!(
        caller.values[get_x].kind,
        ValueKind::Const(Lit::Null)
    ));
    assert!(matches!(
        caller.blocks[entry].term,
        Terminator::Return(Some(ret)) if matches!(caller.values[ret].kind, ValueKind::Const(Lit::Int(1)))
    ));
    assert!(!all_fns.keys().any(|name| name.contains("__rr_sroa_ret_")));
    assert!(crate::mir::verify::verify_ir(caller).is_ok());
}

#[test]
pub(crate) fn sroa_inlines_param_order_named_direct_record_return_field_call() {
    let mut caller = FnIR::new("caller".to_string(), vec![]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let x = int_value(&mut caller, 11);
    let y = int_value(&mut caller, 13);
    let call = caller.add_value(
        ValueKind::Call {
            callee: "make_xy_from_args".to_string(),
            args: vec![x, y],
            names: vec![Some("x".to_string()), Some("y".to_string())],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let get_y = caller.add_value(
        ValueKind::FieldGet {
            base: call,
            field: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(get_y));

    let mut all_fns = FxHashMap::default();
    all_fns.insert("caller".to_string(), caller);
    all_fns.insert("make_xy_from_args".to_string(), make_xy_from_args_fn());

    assert!(specialize_record_return_field_calls(&mut all_fns));
    let caller = all_fns.get("caller").expect("caller");
    assert!(matches!(
        caller.values[get_y].kind,
        ValueKind::Const(Lit::Null)
    ));
    assert!(matches!(
        caller.blocks[entry].term,
        Terminator::Return(Some(ret)) if ret == y
    ));
    assert!(!all_fns.keys().any(|name| name.contains("__rr_sroa_ret_")));
    assert!(crate::mir::verify::verify_ir(caller).is_ok());
}

#[test]
pub(crate) fn sroa_keeps_scalar_return_helper_for_direct_branch_record_return() {
    let mut caller = FnIR::new("caller".to_string(), vec![]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let call = caller.add_value(
        ValueKind::Call {
            callee: "branch_make_xy".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let get_x = caller.add_value(
        ValueKind::FieldGet {
            base: call,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(get_x));

    let mut all_fns = FxHashMap::default();
    all_fns.insert("caller".to_string(), caller);
    all_fns.insert("branch_make_xy".to_string(), branch_make_xy_fn());

    assert!(specialize_record_return_field_calls(&mut all_fns));
    let caller = all_fns.get("caller").expect("caller");
    let ValueKind::Call { callee, args, .. } = &caller.values[get_x].kind else {
        panic!("branching record-return projection should use the scalar helper fallback");
    };
    assert!(callee.contains("__rr_sroa_ret_x"));
    assert!(args.is_empty());
    let specialized = all_fns.get(callee).expect("specialized return callee");
    assert!(specialized.blocks.iter().any(|block| matches!(
            block.term,
            Terminator::Return(Some(ret)) if matches!(specialized.values[ret].kind, ValueKind::Const(Lit::Int(1)))
        )));
    assert!(specialized.blocks.iter().any(|block| matches!(
            block.term,
            Terminator::Return(Some(ret)) if matches!(specialized.values[ret].kind, ValueKind::Const(Lit::Int(3)))
        )));
    assert!(crate::mir::verify::verify_ir(caller).is_ok());
    assert!(crate::mir::verify::verify_ir(specialized).is_ok());
}

#[test]
pub(crate) fn sroa_inlines_aliased_record_return_field_call_and_removes_alias() {
    let mut caller = FnIR::new("caller".to_string(), vec![]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let call = caller.add_value(
        ValueKind::Call {
            callee: "make_xy".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: call,
        span: Span::default(),
    });
    let load = caller.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    let get_y = caller.add_value(
        ValueKind::FieldGet {
            base: load,
            field: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(get_y));

    let mut all_fns = FxHashMap::default();
    all_fns.insert("caller".to_string(), caller);
    all_fns.insert("make_xy".to_string(), make_xy_fn());

    assert!(specialize_record_return_field_calls(&mut all_fns));
    let caller = all_fns.get("caller").expect("caller");
    let ValueKind::Load { var: temp_var } = &caller.values[get_y].kind else {
        panic!("aliased field projection should become a scalar temp load");
    };
    assert!(temp_var.contains("__rr_sroa_ret_y"));
    assert!(
        matches!(
            &caller.blocks[entry].instrs[..],
            [Instr::Assign { dst, src, .. }]
                if dst == temp_var
                    && matches!(caller.values[*src].kind, ValueKind::Const(Lit::Int(2)))
        ),
        "pure record-return alias assignment should be replaced by one inlined scalar temp"
    );
    assert!(matches!(
        caller.values[load].kind,
        ValueKind::Const(Lit::Null)
    ));
    let Instr::Assign { src, .. } = &caller.blocks[entry].instrs[0] else {
        panic!("expected scalar temp assignment");
    };
    assert!(matches!(
        caller.values[*src].kind,
        ValueKind::Const(Lit::Int(2))
    ));
    assert!(!all_fns.keys().any(|name| name.contains("__rr_sroa_ret_")));
    assert!(crate::mir::verify::verify_ir(caller).is_ok());
}

#[test]
pub(crate) fn sroa_shares_aliased_record_return_inline_temp_for_repeated_projection() {
    let mut caller = FnIR::new("caller".to_string(), vec![]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let call = caller.add_value(
        ValueKind::Call {
            callee: "make_xy".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: call,
        span: Span::default(),
    });
    let load_a = caller.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    let get_x_a = caller.add_value(
        ValueKind::FieldGet {
            base: load_a,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let load_b = caller.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    let get_x_b = caller.add_value(
        ValueKind::FieldGet {
            base: load_b,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let sum = caller.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: get_x_a,
            rhs: get_x_b,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(sum));

    let mut all_fns = FxHashMap::default();
    all_fns.insert("caller".to_string(), caller);
    all_fns.insert("make_xy".to_string(), make_xy_fn());

    assert!(specialize_record_return_field_calls(&mut all_fns));
    let caller = all_fns.get("caller").expect("caller");
    let ValueKind::Load { var: first_temp } = &caller.values[get_x_a].kind else {
        panic!("first repeated projection should load the shared scalar temp");
    };
    let ValueKind::Load { var: second_temp } = &caller.values[get_x_b].kind else {
        panic!("second repeated projection should load the shared scalar temp");
    };
    assert_eq!(first_temp, second_temp);
    assert!(first_temp.contains("__rr_sroa_ret_x"));
    let ret_x_calls: Vec<_> = caller
        .values
        .iter()
        .filter_map(|value| match &value.kind {
            ValueKind::Call { callee, .. } if callee.contains("__rr_sroa_ret_x") => Some(value.id),
            _ => None,
        })
        .collect();
    assert_eq!(
        ret_x_calls.len(),
        0,
        "inlineable repeated field projection should not need scalar-return calls"
    );
    assert_eq!(
        caller.blocks[entry].instrs.len(),
        1,
        "record alias assignment should be replaced by one scalar temp assignment"
    );
    assert!(matches!(
        &caller.blocks[entry].instrs[0],
        Instr::Assign { dst, src, .. }
            if dst == first_temp && matches!(caller.values[*src].kind, ValueKind::Const(Lit::Int(1)))
    ));
    assert!(matches!(
        caller.values[load_a].kind,
        ValueKind::Const(Lit::Null)
    ));
    assert!(matches!(
        caller.values[load_b].kind,
        ValueKind::Const(Lit::Null)
    ));
    assert!(crate::mir::verify::verify_ir(caller).is_ok());
}

#[test]
pub(crate) fn sroa_inlines_aliased_record_return_different_fields_without_scalar_calls() {
    let mut caller = FnIR::new("caller".to_string(), vec![]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let call = caller.add_value(
        ValueKind::Call {
            callee: "make_xy".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: call,
        span: Span::default(),
    });
    let load_x = caller.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    let get_x = caller.add_value(
        ValueKind::FieldGet {
            base: load_x,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let load_y = caller.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    let get_y = caller.add_value(
        ValueKind::FieldGet {
            base: load_y,
            field: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let sum = caller.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: get_x,
            rhs: get_y,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(sum));

    let mut all_fns = FxHashMap::default();
    all_fns.insert("caller".to_string(), caller);
    all_fns.insert("make_xy".to_string(), make_xy_fn());

    assert!(specialize_record_return_field_calls(&mut all_fns));
    let caller = all_fns.get("caller").expect("caller");
    let ValueKind::Load { var: x_temp } = &caller.values[get_x].kind else {
        panic!("x projection should load an inlined scalar temp");
    };
    let ValueKind::Load { var: y_temp } = &caller.values[get_y].kind else {
        panic!("y projection should load an inlined scalar temp");
    };
    assert_ne!(x_temp, y_temp);
    assert!(x_temp.contains("__rr_sroa_ret_x"));
    assert!(y_temp.contains("__rr_sroa_ret_y"));
    assert_eq!(
        caller.blocks[entry].instrs.len(),
        2,
        "record alias assignment should be replaced by fieldwise scalar temps"
    );
    assert!(matches!(
        &caller.blocks[entry].instrs[0],
        Instr::Assign { dst, src, .. }
            if dst == x_temp && matches!(caller.values[*src].kind, ValueKind::Const(Lit::Int(1)))
    ));
    assert!(matches!(
        &caller.blocks[entry].instrs[1],
        Instr::Assign { dst, src, .. }
            if dst == y_temp && matches!(caller.values[*src].kind, ValueKind::Const(Lit::Int(2)))
    ));
    assert!(matches!(
        caller.values[load_x].kind,
        ValueKind::Const(Lit::Null)
    ));
    assert!(matches!(
        caller.values[load_y].kind,
        ValueKind::Const(Lit::Null)
    ));
    assert!(!caller.values.iter().any(|value| {
        matches!(
            &value.kind,
            ValueKind::Call { callee, .. } if callee.contains("__rr_sroa_ret_")
        )
    }));
    assert!(!all_fns.keys().any(|name| name.contains("__rr_sroa_ret_")));
    assert!(crate::mir::verify::verify_ir(caller).is_ok());
}

#[test]
pub(crate) fn sroa_inlines_direct_aliased_nested_record_return_fields() {
    let mut caller = FnIR::new("caller".to_string(), vec!["scratch".to_string()]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let scratch = caller.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("scratch".to_string()),
    );
    let idx = int_value(&mut caller, 1);
    let x = int_value(&mut caller, 10);
    let y = int_value(&mut caller, 15);
    let factor = int_value(&mut caller, 3);
    let point = record_xy(&mut caller, x, y);
    let call = caller.add_value(
        ValueKind::Call {
            callee: "forward_scale_xy".to_string(),
            args: vec![point, factor],
            names: vec![None, None],
        },
        Span::default(),
        Facts::empty(),
        Some("out".to_string()),
    );
    caller.set_call_semantics(call, CallSemantics::UserDefined);
    caller.blocks[entry].instrs.push(Instr::Assign {
        dst: "out".to_string(),
        src: call,
        span: Span::default(),
    });
    let get_x = caller.add_value(
        ValueKind::FieldGet {
            base: call,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].instrs.push(Instr::StoreIndex1D {
        base: scratch,
        idx,
        val: get_x,
        is_safe: false,
        is_na_safe: false,
        is_vector: false,
        span: Span::default(),
    });
    let get_y = caller.add_value(
        ValueKind::FieldGet {
            base: call,
            field: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(get_y));

    let mut all_fns = FxHashMap::default();
    all_fns.insert("caller".to_string(), caller);
    all_fns.insert("forward_scale_xy".to_string(), forward_scale_xy_fn());
    all_fns.insert("scale_xy".to_string(), scale_xy_fn());

    assert!(specialize_record_return_field_calls(&mut all_fns));
    let caller = all_fns.get("caller").expect("caller");
    assert!(
        caller.blocks[entry]
            .instrs
            .iter()
            .all(|instr| !matches!(instr, Instr::Assign { dst, .. } if dst == "out")),
        "record-return alias assignment should be removed once fields are scalar temps"
    );
    assert!(caller.blocks[entry].instrs.iter().any(|instr| {
        matches!(instr, Instr::Assign { dst, src, .. }
                if dst.contains("__rr_sroa_ret_x")
                    && matches!(caller.values[*src].kind, ValueKind::Binary { op: BinOp::Mul, .. }))
    }));
    assert!(caller.blocks[entry].instrs.iter().any(|instr| {
        matches!(instr, Instr::Assign { dst, src, .. }
                if dst.contains("__rr_sroa_ret_y")
                    && matches!(caller.values[*src].kind, ValueKind::Binary { op: BinOp::Mul, .. }))
    }));
    assert!(
        caller.blocks[entry].instrs.iter().any(|instr| {
            matches!(instr, Instr::StoreIndex1D { val, .. }
                    if matches!(caller.values[*val].kind, ValueKind::Load { ref var }
                        if var.contains("__rr_sroa_ret_x")))
        }),
        "StoreIndex should consume the scalarized x temp, not the record-return call"
    );
    assert!(matches!(
        caller.blocks[entry].term,
        Terminator::Return(Some(ret))
            if matches!(caller.values[ret].kind, ValueKind::Load { ref var }
                if var.contains("__rr_sroa_ret_y"))
    ));
    assert!(!caller.blocks[entry].instrs.iter().any(|instr| {
        matches!(instr, Instr::Assign { src, .. }
                if matches!(&caller.values[*src].kind, ValueKind::Call { callee, .. }
                    if callee == "forward_scale_xy"))
    }));
    assert!(crate::mir::verify::verify_ir(caller).is_ok());
}

#[test]
pub(crate) fn sroa_does_not_insert_alias_temp_after_early_direct_projection_use() {
    let mut caller = FnIR::new("caller".to_string(), vec![]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let call = caller.add_value(
        ValueKind::Call {
            callee: "make_xy".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        Some("out".to_string()),
    );
    caller.set_call_semantics(call, CallSemantics::UserDefined);
    let get_x = caller.add_value(
        ValueKind::FieldGet {
            base: call,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].instrs.push(Instr::Assign {
        dst: "first".to_string(),
        src: get_x,
        span: Span::default(),
    });
    caller.blocks[entry].instrs.push(Instr::Assign {
        dst: "out".to_string(),
        src: call,
        span: Span::default(),
    });
    let first = caller.add_value(
        ValueKind::Load {
            var: "first".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("first".to_string()),
    );
    caller.blocks[entry].term = Terminator::Return(Some(first));

    let mut all_fns = FxHashMap::default();
    all_fns.insert("caller".to_string(), caller);
    all_fns.insert("make_xy".to_string(), make_xy_fn());

    assert!(specialize_record_return_field_calls(&mut all_fns));
    let caller = all_fns.get("caller").expect("caller");
    assert!(
        crate::mir::verify::verify_ir(caller).is_ok(),
        "direct projection before the alias assignment must not be rewritten to a later temp"
    );
    assert!(matches!(
        &caller.blocks[entry].instrs[0],
        Instr::Assign { dst, src, .. }
            if dst == "first"
                && matches!(caller.values[*src].kind, ValueKind::Const(Lit::Int(1)))
    ));
    assert!(
        caller.blocks[entry].instrs.iter().all(|instr| {
            !matches!(instr, Instr::Assign { dst, .. } if dst.contains("__rr_sroa_ret_"))
        }),
        "early direct projection should inline directly instead of depending on an alias temp"
    );
}

#[test]
pub(crate) fn sroa_does_not_insert_alias_temp_after_early_alias_load_projection_use() {
    let mut caller = FnIR::new("caller".to_string(), vec![]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let call = caller.add_value(
        ValueKind::Call {
            callee: "make_xy".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        Some("out".to_string()),
    );
    caller.set_call_semantics(call, CallSemantics::UserDefined);
    let load_out = caller.add_value(
        ValueKind::Load {
            var: "out".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("out".to_string()),
    );
    let get_x = caller.add_value(
        ValueKind::FieldGet {
            base: load_out,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].instrs.push(Instr::Assign {
        dst: "first".to_string(),
        src: get_x,
        span: Span::default(),
    });
    caller.blocks[entry].instrs.push(Instr::Assign {
        dst: "out".to_string(),
        src: call,
        span: Span::default(),
    });
    let first = caller.add_value(
        ValueKind::Load {
            var: "first".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("first".to_string()),
    );
    caller.blocks[entry].term = Terminator::Return(Some(first));

    let mut all_fns = FxHashMap::default();
    all_fns.insert("caller".to_string(), caller);
    all_fns.insert("make_xy".to_string(), make_xy_fn());

    assert!(
        !specialize_record_return_field_calls(&mut all_fns),
        "early alias-load projection is not safe for alias-temp scalarization"
    );
    let caller = all_fns.get("caller").expect("caller");
    assert!(matches!(
        caller.values[get_x].kind,
        ValueKind::FieldGet { base, ref field }
            if base == load_out && field == "x"
    ));
    assert!(
        caller.blocks[entry].instrs.iter().all(|instr| {
            !matches!(instr, Instr::Assign { dst, .. } if dst.contains("__rr_sroa_ret_"))
        }),
        "early alias-load projection should not depend on a later scalar temp"
    );
}

#[test]
pub(crate) fn sroa_does_not_specialize_impure_record_return_call() {
    let mut caller = FnIR::new("caller".to_string(), vec![]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let call = caller.add_value(
        ValueKind::Call {
            callee: "impure_make_xy".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let get_x = caller.add_value(
        ValueKind::FieldGet {
            base: call,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(get_x));

    let mut all_fns = FxHashMap::default();
    all_fns.insert("caller".to_string(), caller);
    all_fns.insert("impure_make_xy".to_string(), impure_make_xy_fn());

    assert!(!specialize_record_return_field_calls(&mut all_fns));
    let caller = all_fns.get("caller").expect("caller");
    assert!(matches!(
        &caller.values[get_x].kind,
        ValueKind::FieldGet { base, field } if *base == call && field == "x"
    ));
    assert!(!all_fns.keys().any(|name| name.contains("__rr_sroa_ret_")));
}
