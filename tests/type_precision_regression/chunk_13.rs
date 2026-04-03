use super::type_precision_regression_common::*;

#[test]
fn methods_plain_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["arg".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::scalar(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Char;

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let arg = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let mut add_call = |callee: &str, args: Vec<_>, names: Vec<Option<String>>| {
        fn_ir.add_value(
            ValueKind::Call {
                callee: callee.to_string(),
                args,
                names,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        )
    };

    let vids = vec![
        add_call("methods::showExtends", vec![arg], vec![None]),
        add_call("methods::setAs", vec![arg, arg], vec![None, None]),
        add_call("methods::signature", vec![arg], vec![None]),
        add_call("methods::implicitGeneric", vec![arg], vec![None]),
        add_call("methods::callGeneric", vec![arg], vec![None]),
        add_call("methods::Complex", vec![arg], vec![None]),
        add_call("methods::possibleExtends", vec![arg, arg], vec![None, None]),
        add_call("methods::Quote", vec![arg], vec![None]),
        add_call("methods::externalRefMethod", vec![arg], vec![None]),
        add_call(
            "methods::checkAtAssignment",
            vec![arg, arg],
            vec![None, None],
        ),
        add_call("methods::representation", vec![arg], vec![None]),
        add_call("methods::getMethods", vec![arg], vec![None]),
        add_call("methods::methodsPackageMetaName", vec![arg], vec![None]),
        add_call("methods::promptMethods", vec![arg], vec![None]),
        add_call("methods::coerce<-", vec![arg, arg], vec![None, None]),
        add_call("methods::S3Part<-", vec![arg, arg], vec![None, None]),
    ];

    fn_ir.blocks[b0].term = Terminator::Return(Some(arg));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in vids {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
}

#[test]
fn methods_meta_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["arg".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::scalar(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Char;

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let arg = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let dot_slot_names = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::.slotNames".to_string(),
            args: vec![arg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dot_has_slot = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::.hasSlot".to_string(),
            args: vec![arg, arg],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let broad_vids = vec![
        fn_ir.add_value(
            ValueKind::Call {
                callee: "methods::.EmptyPrimitiveSkeletons".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "methods::.S4methods".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "methods::.classEnv".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "methods::.__T__show:methods".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "methods::.__T__Math:base".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
    ];

    fn_ir.blocks[b0].term = Terminator::Return(Some(arg));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    assert_eq!(out.values[dot_slot_names].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[dot_slot_names].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[dot_slot_names].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    assert_eq!(out.values[dot_has_slot].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[dot_has_slot].value_ty.prim, PrimTy::Logical);
    assert_eq!(out.values[dot_has_slot].value_term, TypeTerm::Logical);

    for vid in broad_vids {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
}

#[test]
fn methods_remaining_plain_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["arg".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::scalar(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Char;

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let arg = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let valid_slot_names = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::validSlotNames".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let all_names = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::allNames".to_string(),
            args: vec![arg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let class_label = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::classLabel".to_string(),
            args: vec![arg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let set_package_name = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::setPackageName".to_string(),
            args: vec![arg, arg],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let broad_vids = vec![
        fn_ir.add_value(
            ValueKind::Call {
                callee: "methods::Math".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "methods::setIs".to_string(),
                args: vec![arg, arg],
                names: vec![None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "methods::loadMethod".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "methods::MethodsList".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "methods::functionBody<-".to_string(),
                args: vec![arg, arg],
                names: vec![None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
    ];

    fn_ir.blocks[b0].term = Terminator::Return(Some(arg));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [valid_slot_names, all_names] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    for vid in [class_label, set_package_name] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(out.values[vid].value_term, TypeTerm::Char);
    }

    for vid in broad_vids {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
}
