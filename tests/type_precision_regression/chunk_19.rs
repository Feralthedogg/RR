use super::type_precision_regression_common::*;

#[test]
fn base_direct_constructor_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["chars".to_string(), "nums".to_string()],
    );
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_ty_hints[1] = RR::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Char));
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Double));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let chars = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nums = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let alist_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::alist".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_call_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.call".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_expr_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.expression".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_fun_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.function".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_name_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.name".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_null_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.null".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_pkg_ver_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.package_version".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_pairlist_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.pairlist".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_raw_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.raw".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_single_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.single".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_symbol_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.symbol".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_table_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.table".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(chars));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [alist_v, as_expr_v, as_pkg_ver_v, as_pairlist_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }

    for vid in [as_call_v, as_fun_v, as_name_v, as_symbol_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }

    assert_eq!(out.values[as_null_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[as_null_v].value_ty.prim, PrimTy::Null);
    assert_eq!(out.values[as_null_v].value_term, TypeTerm::Null);

    assert_eq!(out.values[as_raw_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[as_raw_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[as_raw_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Any))
    );

    assert_eq!(out.values[as_single_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[as_single_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[as_single_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[as_table_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[as_table_v].value_ty.prim, PrimTy::Double);
    assert!(matches!(
        out.values[as_table_v].value_term,
        TypeTerm::Matrix(_)
    ));
}

#[test]
fn base_direct_runtime_meta_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["chars".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Char));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let chars = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let active_binding_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::activeBindingFunction".to_string(),
            args: vec![chars, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let add_task_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::addTaskCallback".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let allow_interrupts_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::allowInterrupts".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let attach_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::attach".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let binding_active_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::bindingIsActive".to_string(),
            args: vec![chars, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let binding_locked_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::bindingIsLocked".to_string(),
            args: vec![chars, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bindtextdomain_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::bindtextdomain".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let browser_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::browser".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let browser_set_debug_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::browserSetDebug".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let builtins_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::builtins".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(chars));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    assert_eq!(out.values[add_task_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[add_task_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[add_task_v].value_term, TypeTerm::Int);

    for vid in [binding_active_v, binding_locked_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_term, TypeTerm::Logical);
    }

    assert_eq!(out.values[bindtextdomain_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[bindtextdomain_v].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[bindtextdomain_v].value_term, TypeTerm::Char);

    assert_eq!(out.values[builtins_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[builtins_v].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[builtins_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    for vid in [
        active_binding_v,
        allow_interrupts_v,
        attach_v,
        browser_v,
        browser_set_debug_v,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
}

#[test]
fn base_direct_numeric_misc_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["chars".to_string(), "nums".to_string(), "mat".to_string()],
    );
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_ty_hints[1] = RR::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[2] = RR::typeck::TypeState::matrix(PrimTy::Double, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Char));
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[2] = TypeTerm::Matrix(Box::new(TypeTerm::Double));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let chars = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nums = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat = fn_ir.add_value(
        ValueKind::Param { index: 2 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let backsolve_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::backsolve".to_string(),
            args: vec![mat, nums],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let balance_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::balancePOSIXlt".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bessel_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::besselI".to_string(),
            args: vec![nums, nums],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let beta_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::beta".to_string(),
            args: vec![nums, nums],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let casefold_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::casefold".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let char_expand_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::char.expand".to_string(),
            args: vec![chars, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let charmatch_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::charmatch".to_string(),
            args: vec![chars, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let char_to_raw_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::charToRaw".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let chkdots_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::chkDots".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let chol_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::chol".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let chol2inv_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::chol2inv".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let choose_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::choose".to_string(),
            args: vec![nums, nums],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let choose_ops_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::chooseOpsMethod".to_string(),
            args: vec![chars, chars, chars],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(chars));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    assert_eq!(out.values[backsolve_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[backsolve_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[backsolve_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[balance_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[balance_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[balance_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::Any))
    );

    for vid in [bessel_v, beta_v, choose_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }

    for vid in [casefold_v, char_expand_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    assert_eq!(out.values[charmatch_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[charmatch_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[charmatch_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );

    assert_eq!(out.values[char_to_raw_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[char_to_raw_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[char_to_raw_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Any))
    );

    assert_eq!(out.values[chkdots_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[chkdots_v].value_ty.prim, PrimTy::Null);
    assert_eq!(out.values[chkdots_v].value_term, TypeTerm::Null);

    for vid in [chol_v, chol2inv_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Matrix(Box::new(TypeTerm::Double))
        );
    }

    assert_eq!(out.values[choose_ops_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[choose_ops_v].value_ty.prim, PrimTy::Logical);
    assert_eq!(out.values[choose_ops_v].value_term, TypeTerm::Logical);
}
