use super::type_precision_regression_common::*;

#[test]
fn methods_constructor_exports_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let ctor_method_def = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::.__C__MethodDefinition".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ctor_named_list = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::.__C__namedList".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ctor_null = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::.__C__.NULL".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ctor_weird = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::.__C__<-".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(None);

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [ctor_method_def, ctor_named_list, ctor_null, ctor_weird] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
}

#[test]
fn grid_remaining_helpers_have_direct_types() {
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

    let vids = vec![
        fn_ir.add_value(
            ValueKind::Call {
                callee: "grid::grob".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "grid::convertX".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "grid::grid.legend".to_string(),
                args: vec![arg],
                names: vec![Some("labels".to_string())],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "grid::vpTree".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "grid::grid.remove".to_string(),
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
    for vid in vids {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
}

#[test]
fn base_direct_namespace_object_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["xs".to_string(), "pkg".to_string()],
    );
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_ty_hints[1] = RR::typeck::TypeState::scalar(PrimTy::Char, false);
    fn_ir.param_term_hints[1] = TypeTerm::Char;

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let xs = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pkg = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let get_ns_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::getNamespace".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_ns_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::asNamespace".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_ns_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::isNamespace".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let find_pkg_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::find.package".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pkg_ver_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::package_version".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_name_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::is.name".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let get_element_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::getElement".to_string(),
            args: vec![xs, pkg],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let unname_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::unname".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(xs));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [get_ns_v, as_ns_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }

    for vid in [is_ns_v, is_name_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_term, TypeTerm::Logical);
    }

    assert_eq!(out.values[find_pkg_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[find_pkg_v].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[find_pkg_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    assert_eq!(out.values[pkg_ver_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[pkg_ver_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[pkg_ver_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::Any))
    );

    for vid in [get_element_v, unname_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
}
