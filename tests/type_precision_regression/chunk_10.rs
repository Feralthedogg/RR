use super::type_precision_regression_common::*;

#[test]
pub(crate) fn utils_tail_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["xs".to_string(), "mat".to_string(), "path".to_string()],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[1] = rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Int, false);
    fn_ir.param_ty_hints[2] =
        rr::compiler::internal::typeck::TypeState::scalar(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[1] = TypeTerm::Matrix(Box::new(TypeTerm::Int));
    fn_ir.param_term_hints[2] = TypeTerm::Char;

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let xs = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let path = fn_ir.add_value(
        ValueKind::Param { index: 2 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let head_matrix = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::head.matrix".to_string(),
            args: vec![mat, one],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tail_matrix = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::tail.matrix".to_string(),
            args: vec![mat, one],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stack_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::stack".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let unstack_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::unstack".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let opts = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::strOptions".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bib = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::toBibtex".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let latex = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::toLatex".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pb = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::txtProgressBar".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let get_pb = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::getTxtProgressBar".to_string(),
            args: vec![pb],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let set_pb = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::setTxtProgressBar".to_string(),
            args: vec![pb, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pkg_name = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::packageName".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let os_version = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::osVersion".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nsl = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::nsl".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_delim2 = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::read.delim2".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_dif = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::read.DIF".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_fortran = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::read.fortran".to_string(),
            args: vec![path, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let available = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::available.packages".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let menu_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::menu".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let select_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::select.list".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(select_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [head_matrix, tail_matrix] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Matrix(Box::new(TypeTerm::Int))
        );
    }

    for vid in [stack_v, unstack_v, read_delim2, read_dif, read_fortran] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::DataFrame(Vec::new()));
    }

    assert_eq!(out.values[opts].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[opts].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[opts].value_term,
        TypeTerm::NamedList(vec![
            ("strict.width".to_string(), TypeTerm::Char),
            ("digits.d".to_string(), TypeTerm::Int),
            ("vec.len".to_string(), TypeTerm::Int),
            ("list.len".to_string(), TypeTerm::Int),
            ("deparse.lines".to_string(), TypeTerm::Any),
            ("drop.deparse.attr".to_string(), TypeTerm::Logical),
            ("formatNum".to_string(), TypeTerm::Any),
        ])
    );

    for vid in [bib, latex] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    assert_eq!(out.values[pb].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[pb].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[pb].value_term, TypeTerm::Any);

    for vid in [get_pb, set_pb] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_term, TypeTerm::Double);
    }

    for vid in [pkg_name, os_version, nsl, select_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(out.values[vid].value_term, TypeTerm::Char);
    }

    assert_eq!(out.values[menu_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[menu_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[menu_v].value_term, TypeTerm::Int);

    assert_eq!(out.values[available].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[available].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[available].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Char))
    );
}

#[test]
pub(crate) fn utils_structure_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["obj".to_string()]);
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::List(Box::new(TypeTerm::Any));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let obj = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let logical = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Bool(true)),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let modify = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::modifyList".to_string(),
            args: vec![obj, obj],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let relist = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::relist".to_string(),
            args: vec![obj, obj],
            names: vec![None, Some("skeleton".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_relistable = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::as.relistable".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_relistable = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::is.relistable".to_string(),
            args: vec![as_relistable],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let person_list = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::personList".to_string(),
            args: vec![obj, obj],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let warn_err = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::warnErrList".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_cite = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::readCitationFile".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bibentry = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::bibentry".to_string(),
            args: vec![obj],
            names: vec![Some("title".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let citentry = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::citEntry".to_string(),
            args: vec![obj],
            names: vec![Some("title".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let citheader = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::citHeader".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let citfooter = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::citFooter".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(logical));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [
        modify,
        relist,
        as_relistable,
        person_list,
        warn_err,
        read_cite,
        bibentry,
        citentry,
        citheader,
        citfooter,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }

    assert_eq!(out.values[is_relistable].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[is_relistable].value_ty.prim, PrimTy::Logical);
    assert_eq!(out.values[is_relistable].value_term, TypeTerm::Logical);
}

#[test]
pub(crate) fn utils_parse_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["obj".to_string()]);
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::Any;

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let obj = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "lm".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ns = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "stats".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let parse_data = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::getParseData".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let parse_text = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::getParseText".to_string(),
            args: vec![obj, one],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let srcref = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::getSrcref".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let src_filename = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::getSrcFilename".to_string(),
            args: vec![srcref],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let src_directory = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::getSrcDirectory".to_string(),
            args: vec![srcref],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let src_location = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::getSrcLocation".to_string(),
            args: vec![srcref, one],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let globals = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::globalVariables".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let from_ns = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::getFromNamespace".to_string(),
            args: vec![name, ns],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let s3_method = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::getS3method".to_string(),
            args: vec![name, obj],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(s3_method));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    assert_eq!(out.values[parse_data].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[parse_data].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[parse_data].value_term,
        TypeTerm::DataFrame(Vec::new())
    );

    for vid in [parse_text, globals] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    for vid in [srcref, from_ns, s3_method] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }

    for vid in [src_filename, src_directory] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(out.values[vid].value_term, TypeTerm::Char);
    }

    assert_eq!(out.values[src_location].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[src_location].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[src_location].value_term, TypeTerm::Any);
}
