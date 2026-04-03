use super::type_precision_regression_common::*;

#[test]
fn base_direct_factor_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let a = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("a".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let b = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("b".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(1.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(2.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let three = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(3.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let chars = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![a, b, a],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nums = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let factor_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::factor".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cut_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::cut".to_string(),
            args: vec![nums, two_i],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let table_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::table".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(table_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [factor_v, cut_v, table_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Int))
        );
    }
}

#[test]
fn base_direct_env_file_helpers_have_builtin_types() {
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
    let fun = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("f".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bool_false = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Bool(false)),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let env_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::environment".to_string(),
            args: vec![fun],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let unlink_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::unlink".to_string(),
            args: vec![path, bool_false],
            names: vec![None, Some("recursive".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_path_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::file.path".to_string(),
            args: vec![path, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let basename_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::basename".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dirname_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::dirname".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let normalize_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::normalizePath".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dir_exists_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::dir.exists".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_exists_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::file.exists".to_string(),
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

    assert_eq!(out.values[env_v].value_ty.shape, ShapeTy::Unknown);
    assert_eq!(out.values[env_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[env_v].value_term, TypeTerm::Any);

    assert_eq!(out.values[unlink_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[unlink_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[unlink_v].value_term, TypeTerm::Int);

    for vid in [file_path_v, basename_v, dirname_v, normalize_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    for vid in [dir_exists_v, file_exists_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Logical))
        );
    }
}

#[test]
fn base_direct_eval_io_helpers_have_builtin_types() {
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
    let expr = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("x <- 1".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let width = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("width".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fun = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("f".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let eval_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::eval".to_string(),
            args: vec![expr],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let evalq_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::evalq".to_string(),
            args: vec![expr],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let do_call_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::do.call".to_string(),
            args: vec![fun, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let parse_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::parse".to_string(),
            args: vec![expr],
            names: vec![Some("text".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let get_option_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::getOption".to_string(),
            args: vec![width],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::file".to_string(),
            args: vec![path, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let list_files_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::list.files".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let path_expand_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::path.expand".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_rds_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::readRDS".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let save_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::save".to_string(),
            args: vec![path],
            names: vec![Some("file".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let get0_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::get0".to_string(),
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

    for vid in [
        eval_v,
        evalq_v,
        do_call_v,
        parse_v,
        get_option_v,
        file_v,
        read_rds_v,
        get0_v,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }

    assert_eq!(out.values[save_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[save_v].value_ty.prim, PrimTy::Null);
    assert_eq!(out.values[save_v].value_term, TypeTerm::Null);

    for vid in [list_files_v, path_expand_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }
}
