use super::type_precision_regression_common::*;

#[test]
fn utils_hash_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["obj".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Any, false);
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
    let key = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("a".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let built = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str(
            "R 4.5.0; ; 2025-01-01; unix".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let hashtab = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::hashtab".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sethash = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::sethash".to_string(),
            args: vec![hashtab, key, obj],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let gethash = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::gethash".to_string(),
            args: vec![hashtab, key],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let remhash = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::remhash".to_string(),
            args: vec![hashtab, key],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let clrhash = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::clrhash".to_string(),
            args: vec![hashtab],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let numhash = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::numhash".to_string(),
            args: vec![hashtab],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let typhash = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::typhash".to_string(),
            args: vec![hashtab],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let maphash = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::maphash".to_string(),
            args: vec![hashtab, obj],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_hashtab = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::is.hashtab".to_string(),
            args: vec![hashtab],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_date_built = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::asDateBuilt".to_string(),
            args: vec![built],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let find_line = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::findLineNum".to_string(),
            args: vec![obj, obj],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(find_line));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [hashtab, gethash, find_line] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }

    assert_eq!(out.values[sethash].value_term, TypeTerm::Any);
    assert_eq!(out.values[sethash].value_ty.shape, ShapeTy::Unknown);

    for vid in [remhash, is_hashtab] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_term, TypeTerm::Logical);
    }

    for vid in [clrhash, maphash] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Null);
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }

    assert_eq!(out.values[numhash].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[numhash].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[numhash].value_term, TypeTerm::Int);

    assert_eq!(out.values[typhash].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[typhash].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[typhash].value_term, TypeTerm::Char);

    assert_eq!(out.values[as_date_built].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[as_date_built].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[as_date_built].value_term, TypeTerm::Double);
}

#[test]
fn utils_archive_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["path".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::scalar(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Char;

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let path = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bool_true = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Bool(true)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let web = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("web".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let mirrors = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::getCRANmirrors".to_string(),
            args: vec![bool_true],
            names: vec![Some("local.only".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mirror = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::findCRANmirror".to_string(),
            args: vec![web],
            names: vec![Some("type".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pkg_skel = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::package.skeleton".to_string(),
            args: vec![path],
            names: vec![Some("name".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let zip_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::zip".to_string(),
            args: vec![path, path],
            names: vec![None, Some("files".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let unzip_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::unzip".to_string(),
            args: vec![path, bool_true],
            names: vec![None, Some("list".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(unzip_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    assert_eq!(out.values[mirrors].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[mirrors].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[mirrors].value_term,
        TypeTerm::DataFrameNamed(vec![
            (
                "Name".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Country".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "City".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "URL".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Host".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Maintainer".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            ("OK".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Int))),
            (
                "CountryCode".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Comment".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])
    );

    assert_eq!(out.values[mirror].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[mirror].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[mirror].value_term, TypeTerm::Char);

    for vid in [pkg_skel, unzip_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }

    assert_eq!(out.values[zip_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[zip_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[zip_v].value_term, TypeTerm::Int);
}

#[test]
fn utils_interactive_helpers_have_direct_types() {
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
        add_call("utils::browseEnv"),
        add_call("utils::browseURL"),
        add_call("utils::browseVignettes"),
        add_call("utils::bug.report"),
        add_call("utils::checkCRAN"),
        add_call("utils::chooseBioCmirror"),
        add_call("utils::chooseCRANmirror"),
        add_call("utils::create.post"),
        add_call("utils::data.entry"),
        add_call("utils::dataentry"),
        add_call("utils::debugcall"),
        add_call("utils::debugger"),
        add_call("utils::demo"),
        add_call("utils::dump.frames"),
        add_call("utils::edit"),
        add_call("utils::emacs"),
        add_call("utils::example"),
        add_call("utils::file.edit"),
        add_call("utils::fix"),
        add_call("utils::fixInNamespace"),
        add_call("utils::flush.console"),
        add_call("utils::help.request"),
        add_call("utils::help.start"),
        add_call("utils::page"),
        add_call("utils::pico"),
        add_call("utils::process.events"),
        add_call("utils::prompt"),
        add_call("utils::promptData"),
        add_call("utils::promptImport"),
        add_call("utils::promptPackage"),
        add_call("utils::recover"),
        add_call("utils::removeSource"),
        add_call("utils::RShowDoc"),
        add_call("utils::RSiteSearch"),
        add_call("utils::rtags"),
        add_call("utils::setBreakpoint"),
        add_call("utils::suppressForeignCheck"),
        add_call("utils::undebugcall"),
        add_call("utils::url.show"),
        add_call("utils::vi"),
        add_call("utils::View"),
        add_call("utils::xedit"),
        add_call("utils::xemacs"),
    ];

    fn_ir.blocks[b0].term = Terminator::Return(Some(arg));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in vids {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Null);
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }
}
