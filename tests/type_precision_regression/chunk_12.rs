use super::type_precision_regression_common::*;

#[test]
pub(crate) fn utils_search_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["arg".to_string()]);
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Double));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let arg = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let labels = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::limitedLabels".to_string(),
            args: vec![arg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let format_ol = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::formatOL".to_string(),
            args: vec![arg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let format_ul = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::formatUL".to_string(),
            args: vec![arg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ls_str = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::ls.str".to_string(),
            args: vec![arg],
            names: vec![Some("envir".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lsf_str = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::lsf.str".to_string(),
            args: vec![arg],
            names: vec![Some("envir".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let news = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::news".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let vignette = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::vignette".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let hdb = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::hsearch_db".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let hdb_concepts = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::hsearch_db_concepts".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let hdb_keywords = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::hsearch_db_keywords".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(hdb_keywords));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [labels, format_ol, format_ul, ls_str, lsf_str] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    assert_eq!(out.values[news].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[news].value_ty.prim, PrimTy::Null);
    assert_eq!(out.values[news].value_term, TypeTerm::Null);

    assert_eq!(out.values[vignette].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[vignette].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[vignette].value_term,
        TypeTerm::NamedList(vec![
            ("type".to_string(), TypeTerm::Char),
            ("title".to_string(), TypeTerm::Char),
            ("header".to_string(), TypeTerm::Any),
            ("results".to_string(), TypeTerm::DataFrame(Vec::new())),
            ("footer".to_string(), TypeTerm::Any),
        ])
    );

    assert_eq!(out.values[hdb].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[hdb].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[hdb].value_term,
        TypeTerm::List(Box::new(TypeTerm::Any))
    );

    for vid in [hdb_concepts, hdb_keywords] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::DataFrame(Vec::new()));
    }
}

#[test]
fn utils_profile_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["path".to_string()]);
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::scalar(PrimTy::Char, false);
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
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Bool(true)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let web = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "web".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let tar_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::tar".to_string(),
            args: vec![path, path],
            names: vec![None, Some("files".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let untar_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::untar".to_string(),
            args: vec![path, bool_true],
            names: vec![None, Some("list".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ts = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::timestamp".to_string(),
            args: vec![path],
            names: vec![Some("prefix".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rprof = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::Rprof".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rprofmem = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::Rprofmem".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::summaryRprof".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let repos = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::setRepositories".to_string(),
            args: vec![bool_true],
            names: vec![Some("ind".to_string())],
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

    fn_ir.blocks[b0].term = Terminator::Return(Some(repos));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    assert_eq!(out.values[tar_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[tar_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[tar_v].value_term, TypeTerm::Int);

    assert_eq!(out.values[untar_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[untar_v].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[untar_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    assert_eq!(out.values[ts].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[ts].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[ts].value_term, TypeTerm::Char);

    for vid in [rprof, rprofmem] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Null);
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }

    assert_eq!(out.values[summary].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[summary].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[summary].value_term,
        TypeTerm::NamedList(vec![
            ("by.self".to_string(), TypeTerm::Any),
            ("by.total".to_string(), TypeTerm::Any),
            ("sample.interval".to_string(), TypeTerm::Double),
            ("sampling.time".to_string(), TypeTerm::Double),
        ])
    );

    assert_eq!(out.values[repos].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[repos].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[repos].value_term,
        TypeTerm::NamedList(vec![(
            "repos".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Char)),
        )])
    );

    assert_eq!(out.values[mirror].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[mirror].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[mirror].value_term, TypeTerm::Char);
}

#[test]
pub(crate) fn utils_remaining_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["arg".to_string()]);
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::scalar(PrimTy::Char, false);
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
    let logical_arg = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Bool(true)),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let mut add_call = |callee: &str, args, names| {
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

    let char_vids = vec![
        add_call("utils::?", vec![arg], vec![None]),
        add_call("utils::.AtNames", vec![arg], vec![None]),
        add_call("utils::.DollarNames", vec![arg], vec![None]),
        add_call(
            "utils::cite",
            vec![arg, arg],
            vec![None, Some("bib".to_string())],
        ),
        add_call(
            "utils::citeNatbib",
            vec![arg, arg],
            vec![None, Some("bib".to_string())],
        ),
        add_call("utils::help", vec![arg], vec![None]),
        add_call("utils::read.socket", vec![arg], vec![None]),
        add_call("utils::RweaveChunkPrefix", vec![arg], vec![None]),
    ];

    let romans = add_call("utils::.romans", vec![], vec![]);

    let logical_vids = vec![
        add_call(
            "utils::askYesNo",
            vec![arg, logical_arg],
            vec![None, Some("default".to_string())],
        ),
        add_call("utils::isS3method", vec![arg], vec![None]),
        add_call("utils::isS3stdGeneric", vec![arg], vec![None]),
    ];

    let rc_settings = add_call("utils::rc.settings", vec![], vec![]);
    let download_file = add_call("utils::download.file", vec![arg, arg], vec![None, None]);
    let scalar_any_vids = vec![
        add_call("utils::alarm", vec![], vec![]),
        add_call("utils::rc.getOption", vec![arg], vec![None]),
    ];
    let null_vids = vec![
        add_call("utils::close.socket", vec![arg], vec![None]),
        add_call("utils::history", vec![], vec![]),
        add_call("utils::loadhistory", vec![arg], vec![None]),
        add_call("utils::savehistory", vec![arg], vec![None]),
        add_call("utils::write.socket", vec![arg, arg], vec![None, None]),
    ];
    let any_vids = vec![
        add_call("utils::.checkHT", vec![arg], vec![None]),
        add_call("utils::.RtangleCodeLabel", vec![arg], vec![None]),
        add_call("utils::.S3methods", vec![arg], vec![None]),
        add_call("utils::aregexec", vec![arg, arg], vec![None, None]),
        add_call("utils::aspell", vec![arg], vec![None]),
        add_call("utils::aspell_package_C_files", vec![arg], vec![None]),
        add_call("utils::aspell_package_R_files", vec![arg], vec![None]),
        add_call("utils::aspell_package_Rd_files", vec![arg], vec![None]),
        add_call("utils::aspell_package_vignettes", vec![arg], vec![None]),
        add_call(
            "utils::aspell_write_personal_dictionary_file",
            vec![arg],
            vec![None],
        ),
        add_call(
            "utils::assignInMyNamespace",
            vec![arg, arg],
            vec![None, None],
        ),
        add_call("utils::assignInNamespace", vec![arg, arg], vec![None, None]),
        add_call("utils::changedFiles", vec![arg, arg], vec![None, None]),
        add_call("utils::de", vec![arg], vec![None]),
        add_call("utils::de.ncols", vec![arg], vec![None]),
        add_call("utils::de.restore", vec![arg], vec![None]),
        add_call("utils::de.setup", vec![arg], vec![None]),
        add_call("utils::download.packages", vec![arg], vec![None]),
        add_call("utils::install.packages", vec![arg], vec![None]),
        add_call("utils::make.packages.html", vec![arg], vec![None]),
        add_call("utils::make.socket", vec![arg], vec![None]),
        add_call("utils::makeRweaveLatexCodeRunner", vec![arg], vec![None]),
        add_call("utils::mirror2html", vec![arg], vec![None]),
        add_call("utils::new.packages", vec![], vec![]),
        add_call("utils::old.packages", vec![], vec![]),
        add_call("utils::packageStatus", vec![], vec![]),
        add_call("utils::rc.options", vec![], vec![]),
        add_call("utils::rc.status", vec![], vec![]),
        add_call("utils::remove.packages", vec![arg], vec![None]),
        add_call("utils::Rtangle", vec![arg], vec![None]),
        add_call("utils::RtangleFinish", vec![arg], vec![None]),
        add_call("utils::RtangleRuncode", vec![arg], vec![None]),
        add_call("utils::RtangleSetup", vec![arg], vec![None]),
        add_call("utils::RtangleWritedoc", vec![arg], vec![None]),
        add_call("utils::RweaveEvalWithOpt", vec![arg], vec![None]),
        add_call("utils::RweaveLatex", vec![arg], vec![None]),
        add_call("utils::RweaveLatexFinish", vec![arg], vec![None]),
        add_call("utils::RweaveLatexOptions", vec![arg], vec![None]),
        add_call("utils::RweaveLatexSetup", vec![arg], vec![None]),
        add_call("utils::RweaveLatexWritedoc", vec![arg], vec![None]),
        add_call("utils::RweaveTryStop", vec![arg], vec![None]),
        add_call("utils::Stangle", vec![arg], vec![None]),
        add_call("utils::Sweave", vec![arg], vec![None]),
        add_call("utils::SweaveHooks", vec![], vec![]),
        add_call("utils::SweaveSyntaxLatex", vec![arg], vec![None]),
        add_call("utils::SweaveSyntaxNoweb", vec![arg], vec![None]),
        add_call("utils::SweaveSyntConv", vec![arg], vec![None]),
        add_call("utils::update.packages", vec![], vec![]),
        add_call("utils::upgrade", vec![arg], vec![None]),
    ];

    fn_ir.blocks[b0].term = Terminator::Return(Some(download_file));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in char_vids {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    assert_eq!(out.values[romans].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[romans].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[romans].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );

    for vid in logical_vids {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_term, TypeTerm::Logical);
    }

    assert_eq!(out.values[rc_settings].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[rc_settings].value_ty.prim, PrimTy::Logical);
    assert_eq!(
        out.values[rc_settings].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Logical))
    );

    assert_eq!(out.values[download_file].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[download_file].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[download_file].value_term, TypeTerm::Int);

    for vid in scalar_any_vids {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }

    for vid in null_vids {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Null);
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }

    for vid in any_vids {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
}
