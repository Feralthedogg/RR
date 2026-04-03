use super::type_precision_regression_common::*;

#[test]
fn base_direct_system_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["path".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Char));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let path = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let system_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::system".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let system2_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::system2".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let system_time_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::system.time".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_sleep_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.sleep".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_setlocale_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.setlocale".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_timezone_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.timezone".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_localeconv_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.localeconv".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_setfiletime_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.setFileTime".to_string(),
            args: vec![path, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_chmod_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.chmod".to_string(),
            args: vec![path, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_umask_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.umask".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(path));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [system_v, system2_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }

    assert_eq!(out.values[system_time_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[system_time_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[system_time_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[sys_sleep_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[sys_sleep_v].value_ty.prim, PrimTy::Null);
    assert_eq!(out.values[sys_sleep_v].value_term, TypeTerm::Null);

    for vid in [sys_setlocale_v, sys_timezone_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(out.values[vid].value_term, TypeTerm::Char);
    }

    assert_eq!(out.values[sys_localeconv_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[sys_localeconv_v].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[sys_localeconv_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    for vid in [sys_setfiletime_v, sys_chmod_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Logical))
        );
    }

    assert_eq!(out.values[sys_umask_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[sys_umask_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[sys_umask_v].value_term, TypeTerm::Int);
}

#[test]
fn readr_tidyr_tail_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["arg".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::Any;

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let arg = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let mut add_call = |callee: &str| {
        fn_ir.add_value(
            ValueKind::Call {
                callee: callee.to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        )
    };

    let vids = vec![
        add_call("readr::parse_integer"),
        add_call("readr::spec_csv"),
        add_call("readr::type_convert"),
        add_call("readr::read_lines"),
        add_call("readr::fwf_widths"),
        add_call("tidyr::gather"),
        add_call("tidyr::spread"),
        add_call("tidyr::replace_na"),
        add_call("tidyr::complete"),
        add_call("tidyr::hoist"),
        add_call("tidyr::separate_rows"),
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
fn dplyr_tail_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["arg".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::Any;

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let arg = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let mut add_call = |callee: &str| {
        fn_ir.add_value(
            ValueKind::Call {
                callee: callee.to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        )
    };

    let vids = vec![
        add_call("dplyr::slice"),
        add_call("dplyr::pull"),
        add_call("dplyr::count"),
        add_call("dplyr::distinct"),
        add_call("dplyr::relocate"),
        add_call("dplyr::across"),
        add_call("dplyr::rowwise"),
        add_call("dplyr::lag"),
        add_call("dplyr::lead"),
        add_call("dplyr::n_distinct"),
        add_call("dplyr::case_when"),
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
