use super::*;

pub(crate) fn sroa_specializes_known_record_field_call_argument() {
    let mut caller = FnIR::new("caller".to_string(), vec![]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let x = int_value(&mut caller, 1);
    let y = int_value(&mut caller, 2);
    let record = record_xy(&mut caller, x, y);
    caller.blocks[entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: record,
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
    let call = caller.add_value(
        ValueKind::Call {
            callee: "sum_xy".to_string(),
            args: vec![load],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(call));

    let mut all_fns = FxHashMap::default();
    all_fns.insert("caller".to_string(), caller);
    all_fns.insert("sum_xy".to_string(), sum_xy_fn());

    assert!(specialize_record_field_calls(&mut all_fns));
    let caller = all_fns.get("caller").expect("caller");
    let ValueKind::Call {
        callee,
        args,
        names,
    } = &caller.values[call].kind
    else {
        panic!("call should remain a direct call");
    };
    assert_ne!(callee, "sum_xy");
    assert_eq!(args, &vec![x, y]);
    assert_eq!(names, &vec![None, None]);
    assert!(
        caller.blocks[entry].instrs.is_empty(),
        "record alias should become dead once call args are scalarized"
    );

    let specialized = all_fns.get(callee).expect("specialized callee");
    assert_eq!(specialized.params.len(), 2);
    assert!(
        specialized
            .params
            .iter()
            .all(|param| param.contains("__rr_sroa_"))
    );
    assert!(
        specialized
            .values
            .iter()
            .any(|value| matches!(value.kind, ValueKind::Param { index: 0 }))
    );
    assert!(
        specialized
            .values
            .iter()
            .any(|value| matches!(value.kind, ValueKind::Param { index: 1 }))
    );
    assert!(crate::mir::verify::verify_ir(caller).is_ok());
    assert!(crate::mir::verify::verify_ir(specialized).is_ok());
}
pub(crate) fn sroa_specializes_param_order_named_record_field_call_argument() {
    let mut caller = FnIR::new("caller".to_string(), vec![]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let x = int_value(&mut caller, 1);
    let y = int_value(&mut caller, 2);
    let record = record_xy(&mut caller, x, y);
    caller.blocks[entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: record,
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
    let call = caller.add_value(
        ValueKind::Call {
            callee: "sum_xy".to_string(),
            args: vec![load],
            names: vec![Some("p".to_string())],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(call));

    let mut all_fns = FxHashMap::default();
    all_fns.insert("caller".to_string(), caller);
    all_fns.insert("sum_xy".to_string(), sum_xy_fn());

    assert!(specialize_record_field_calls(&mut all_fns));
    let caller = all_fns.get("caller").expect("caller");
    let ValueKind::Call {
        callee,
        args,
        names,
    } = &caller.values[call].kind
    else {
        panic!("named call should remain a direct call");
    };
    assert_ne!(callee, "sum_xy");
    assert_eq!(args, &vec![x, y]);
    assert_eq!(
        names,
        &vec![None, None],
        "param-order-compatible names can be erased after scalarizing args"
    );
    assert!(
        caller.blocks[entry].instrs.is_empty(),
        "record alias should become dead once named call args are scalarized"
    );
    assert!(crate::mir::verify::verify_ir(caller).is_ok());
}

#[test]
pub(crate) fn sroa_keeps_reordered_named_record_field_call_argument() {
    let mut caller = FnIR::new("caller".to_string(), vec![]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let x = int_value(&mut caller, 1);
    let y = int_value(&mut caller, 2);
    let record = record_xy(&mut caller, x, y);
    let call = caller.add_value(
        ValueKind::Call {
            callee: "sum_xy".to_string(),
            args: vec![record],
            names: vec![Some("other".to_string())],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(call));

    let mut all_fns = FxHashMap::default();
    all_fns.insert("caller".to_string(), caller);
    all_fns.insert("sum_xy".to_string(), sum_xy_fn());

    assert!(
        !specialize_record_field_calls(&mut all_fns),
        "non-param-order named calls must not be scalarized positionally"
    );
    let caller = all_fns.get("caller").expect("caller");
    assert!(matches!(
        &caller.values[call].kind,
        ValueKind::Call { callee, args, names }
            if callee == "sum_xy"
                && args == &vec![record]
                && names == &vec![Some("other".to_string())]
    ));
}

#[test]
pub(crate) fn sroa_does_not_specialize_record_call_when_param_escapes() {
    let mut callee = FnIR::new("escape_record".to_string(), vec!["p".to_string()]);
    let entry = callee.add_block();
    callee.entry = entry;
    callee.body_head = entry;
    let p = callee.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("p".to_string()),
    );
    callee.blocks[entry].term = Terminator::Return(Some(p));

    let mut caller = FnIR::new("caller".to_string(), vec![]);
    let entry = caller.add_block();
    caller.entry = entry;
    caller.body_head = entry;
    let x = int_value(&mut caller, 1);
    let y = int_value(&mut caller, 2);
    let record = record_xy(&mut caller, x, y);
    let call = caller.add_value(
        ValueKind::Call {
            callee: "escape_record".to_string(),
            args: vec![record],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    caller.blocks[entry].term = Terminator::Return(Some(call));

    let mut all_fns = FxHashMap::default();
    all_fns.insert("caller".to_string(), caller);
    all_fns.insert("escape_record".to_string(), callee);

    assert!(!specialize_record_field_calls(&mut all_fns));
    let caller = all_fns.get("caller").expect("caller");
    assert!(matches!(
        &caller.values[call].kind,
        ValueKind::Call { callee, args, .. } if callee == "escape_record" && args == &vec![record]
    ));
    assert!(!all_fns.keys().any(|name| name.contains("__rr_sroa_")));
}
