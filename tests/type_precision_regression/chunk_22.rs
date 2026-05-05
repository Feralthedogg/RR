use super::type_precision_regression_common::*;

#[test]
pub(crate) fn base_package_wide_fallback_keeps_remaining_exports_on_direct_surface() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["arg".to_string()]);
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Char));

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
                callee: "base::R.home".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "base::findRestart".to_string(),
                args: vec![arg],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "base::gcinfo".to_string(),
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
pub(crate) fn base_direct_runtime_helper_batch_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["chars".to_string(), "nums".to_string()],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_ty_hints[1] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
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

    let debugging_state_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::debuggingState".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dont_check_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::dontCheck".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let debug_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::debug".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let delayed_assign_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::delayedAssign".to_string(),
            args: vec![chars, nums],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let env_profile_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::env.profile".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let error_condition_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::errorCondition".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let exists_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::exists".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let expression_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::expression".to_string(),
            args: vec![chars],
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

    assert_eq!(
        out.values[debugging_state_v].value_ty.shape,
        ShapeTy::Scalar
    );
    assert_eq!(out.values[debugging_state_v].value_ty.prim, PrimTy::Logical);
    assert_eq!(out.values[debugging_state_v].value_term, TypeTerm::Logical);

    assert_eq!(out.values[dont_check_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[dont_check_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[dont_check_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[exists_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[exists_v].value_ty.prim, PrimTy::Logical);
    assert_eq!(out.values[exists_v].value_term, TypeTerm::Logical);

    for vid in [
        debug_v,
        delayed_assign_v,
        env_profile_v,
        error_condition_v,
        expression_v,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
}

#[test]
pub(crate) fn base_direct_bitwise_coercion_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec![
            "chars".to_string(),
            "ints".to_string(),
            "dbls".to_string(),
            "obj".to_string(),
            "mat".to_string(),
        ],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_ty_hints[1] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Int, false);
    fn_ir.param_ty_hints[2] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[3] = rr::compiler::internal::typeck::TypeState::unknown();
    fn_ir.param_ty_hints[4] = rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Int, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Char));
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Int));
    fn_ir.param_term_hints[2] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[3] = TypeTerm::Any;
    fn_ir.param_term_hints[4] = TypeTerm::Matrix(Box::new(TypeTerm::Int));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let chars = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ints = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dbls = fn_ir.add_value(
        ValueKind::Param { index: 2 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let obj = fn_ir.add_value(
        ValueKind::Param { index: 3 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat = fn_ir.add_value(
        ValueKind::Param { index: 4 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let as_complex_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.complex".to_string(),
            args: vec![dbls],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_hexmode_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.hexmode".to_string(),
            args: vec![ints],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_numeric_version_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.numeric_version".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_ordered_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.ordered".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_qr_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.qr".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let asplit_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::asplit".to_string(),
            args: vec![mat, ints],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ass3_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::asS3".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bitwand_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::bitwAnd".to_string(),
            args: vec![ints, ints],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bitwnot_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::bitwNot".to_string(),
            args: vec![ints],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let conj_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Conj".to_string(),
            args: vec![dbls],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let gl_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::gl".to_string(),
            args: vec![ints, ints],
            names: vec![None, None],
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

    assert_eq!(out.values[as_complex_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[as_complex_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[as_complex_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Any))
    );

    for vid in [as_hexmode_v, as_ordered_v, bitwand_v, bitwnot_v, gl_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Int))
        );
    }

    assert_eq!(
        out.values[as_numeric_version_v].value_ty.shape,
        ShapeTy::Vector
    );
    assert_eq!(out.values[as_numeric_version_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[as_numeric_version_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::Any))
    );

    for vid in [as_qr_v, ass3_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }

    assert_eq!(out.values[asplit_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[asplit_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[asplit_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::Any))
    );

    assert_eq!(out.values[conj_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[conj_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[conj_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
}
