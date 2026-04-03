use super::type_precision_regression_common::*;

#[test]
fn utils_package_calls_have_direct_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["xs".to_string(), "df".to_string(), "path".to_string()],
    );
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[1] = RR::typeck::TypeState::matrix(PrimTy::Any, false);
    fn_ir.param_ty_hints[2] = RR::typeck::TypeState::scalar(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[1] = TypeTerm::DataFrameNamed(vec![
        (
            "x".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Double)),
        ),
        ("y".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Char))),
    ]);
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
    let df = fn_ir.add_value(
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

    let head = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::head".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tail = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::tail".to_string(),
            args: vec![df],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::read.csv".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_csv2 = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::read.csv2".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_table = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::read.table".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_fwf = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::read.fwf".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_delim = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::read.delim".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let package_version = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::packageVersion".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let maintainer = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::maintainer".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let package_date = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::packageDate".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let object_size = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::object.size".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let memory_size = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::memory.size".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let memory_limit = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::memory.limit".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let compare_left = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("1.2.0".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let compare_right = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("1.1.9".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let compare_version = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::compareVersion".to_string(),
            args: vec![compare_left, compare_right],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let capture = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::capture.output".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let package_description = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::packageDescription".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let session_info = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::sessionInfo".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let package_field_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("Package".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let base_pkgs_field_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("basePkgs".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let package_field = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![package_description, package_field_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let session_base_pkgs = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![session_info, base_pkgs_field_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let citation = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::citation".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let person = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::person".to_string(),
            args: vec![path, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_person = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::as.person".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_person_list = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::as.personList".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_roman = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::as.roman".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let has_name = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::hasName".to_string(),
            args: vec![df, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let capture_a = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("1-one".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let capture_b = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("2-two".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let capture_input = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![capture_a, capture_b],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let strcapture = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::strcapture".to_string(),
            args: vec![path, capture_input, df],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let apropos = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::apropos".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let find_pkg = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::find".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let get_anywhere = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::getAnywhere".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let args_anywhere = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::argsAnywhere".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let match_pat = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("me".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let match_values = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![path, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let find_matches = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::findMatches".to_string(),
            args: vec![match_pat, match_values],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let methods_fn = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::methods".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let help_search = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::help.search".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let data_iqr = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::data".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let contrib_url = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::contrib.url".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let locale_charset = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::localeToCharset".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let alpha_class = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("alpha".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let char_class = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::charClass".to_string(),
            args: vec![path, alpha_class],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_snapshot = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::fileSnapshot".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let url_encoded = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::URLencode".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let url_decoded = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::URLdecode".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let glob_rx = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::glob2rx".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_flag = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("-f".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_test_scalar = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::file_test".to_string(),
            args: vec![file_flag, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let other_path = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("other.txt".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_test_paths = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![path, other_path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_test_vector = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::file_test".to_string(),
            args: vec![file_flag, file_test_paths],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let write_table = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::write.table".to_string(),
            args: vec![df, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let write_csv2 = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::write.csv2".to_string(),
            args: vec![df, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let combn_n = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let combn = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::combn".to_string(),
            args: vec![xs, combn_n],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cat_word = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("cat".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dog_word = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("dog".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cot_word = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("cot".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dig_word = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("dig".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let words_a = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![cat_word, dog_word],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let words_b = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![cot_word, dig_word],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let adist = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::adist".to_string(),
            args: vec![words_a, words_b],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let count_fields = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::count.fields".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one_str = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("1".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two_str = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("2".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let chars = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one_str, two_str],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let converted = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::type.convert".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let write = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::write.csv".to_string(),
            args: vec![df, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let str_call = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::str".to_string(),
            args: vec![df],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(write));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    assert_eq!(out.values[head].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[head].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[head].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[tail].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[tail].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[tail].value_term, fn_ir_param_df_term());

    for vid in [read, read_csv2, read_table, read_delim, read_fwf] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::DataFrame(Vec::new()));
    }

    for vid in [package_version, citation, person, as_person, as_person_list] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }
    assert_eq!(
        out.values[package_description].value_term,
        TypeTerm::NamedList(vec![
            ("Package".to_string(), TypeTerm::Char),
            ("Version".to_string(), TypeTerm::Char),
            ("Priority".to_string(), TypeTerm::Char),
            ("Title".to_string(), TypeTerm::Char),
            ("Author".to_string(), TypeTerm::Char),
            ("Maintainer".to_string(), TypeTerm::Char),
            ("Contact".to_string(), TypeTerm::Char),
            ("Description".to_string(), TypeTerm::Char),
            ("License".to_string(), TypeTerm::Char),
            ("Imports".to_string(), TypeTerm::Char),
            ("Suggests".to_string(), TypeTerm::Char),
            ("NeedsCompilation".to_string(), TypeTerm::Char),
            ("Encoding".to_string(), TypeTerm::Char),
            ("Enhances".to_string(), TypeTerm::Char),
            ("Built".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[session_info].value_term,
        TypeTerm::NamedList(vec![
            ("R.version".to_string(), TypeTerm::Any),
            ("platform".to_string(), TypeTerm::Char),
            ("locale".to_string(), TypeTerm::Char),
            ("tzone".to_string(), TypeTerm::Char),
            ("tzcode_type".to_string(), TypeTerm::Char),
            ("running".to_string(), TypeTerm::Char),
            (
                "RNGkind".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "basePkgs".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            ("loadedOnly".to_string(), TypeTerm::Any),
            ("matprod".to_string(), TypeTerm::Char),
            ("BLAS".to_string(), TypeTerm::Char),
            ("LAPACK".to_string(), TypeTerm::Char),
            ("LA_version".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(out.values[package_field].value_term, TypeTerm::Char);
    assert_eq!(
        out.values[session_base_pkgs].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    assert_eq!(out.values[maintainer].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[maintainer].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[maintainer].value_term, TypeTerm::Char);

    for vid in [package_date, object_size, memory_size, memory_limit] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_term, TypeTerm::Double);
    }

    assert_eq!(out.values[as_roman].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[as_roman].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[as_roman].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );

    assert_eq!(out.values[has_name].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[has_name].value_ty.prim, PrimTy::Logical);
    assert_eq!(out.values[has_name].value_term, TypeTerm::Logical);

    assert_eq!(out.values[strcapture].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[strcapture].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[strcapture].value_term,
        TypeTerm::DataFrame(Vec::new())
    );

    for vid in [apropos, find_pkg, find_matches, methods_fn] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    for vid in [get_anywhere, help_search, data_iqr] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
    }
    assert_eq!(
        out.values[get_anywhere].value_term,
        TypeTerm::NamedList(vec![
            ("name".to_string(), TypeTerm::Char),
            ("objs".to_string(), TypeTerm::List(Box::new(TypeTerm::Any))),
            (
                "where".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
            (
                "visible".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Logical)),
            ),
            (
                "dups".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Logical)),
            ),
        ])
    );
    assert_eq!(
        out.values[help_search].value_term,
        TypeTerm::NamedList(vec![
            ("pattern".to_string(), TypeTerm::Char),
            (
                "fields".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
            ("type".to_string(), TypeTerm::Char),
            ("agrep".to_string(), TypeTerm::Any),
            ("ignore.case".to_string(), TypeTerm::Logical),
            (
                "types".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
            ("package".to_string(), TypeTerm::Any),
            ("lib.loc".to_string(), TypeTerm::Char),
            (
                "matches".to_string(),
                TypeTerm::DataFrameNamed(vec![
                    (
                        "Topic".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char))
                    ),
                    (
                        "Title".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char))
                    ),
                    (
                        "Name".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char))
                    ),
                    ("ID".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Char))),
                    (
                        "Package".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char))
                    ),
                    (
                        "LibPath".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char))
                    ),
                    (
                        "Type".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char))
                    ),
                    (
                        "Field".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char))
                    ),
                    (
                        "Entry".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char))
                    ),
                ]),
            ),
        ])
    );
    assert_eq!(
        out.values[data_iqr].value_term,
        TypeTerm::NamedList(vec![
            ("title".to_string(), TypeTerm::Char),
            ("header".to_string(), TypeTerm::Any),
            (
                "results".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Char))
            ),
            ("footer".to_string(), TypeTerm::Char),
        ])
    );

    assert_eq!(
        out.values[args_anywhere].value_ty,
        RR::typeck::TypeState::unknown()
    );
    assert_eq!(out.values[args_anywhere].value_term, TypeTerm::Any);

    assert_eq!(out.values[contrib_url].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[contrib_url].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[contrib_url].value_term, TypeTerm::Char);

    assert_eq!(out.values[locale_charset].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[locale_charset].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[locale_charset].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    assert_eq!(out.values[char_class].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[char_class].value_ty.prim, PrimTy::Logical);
    assert_eq!(
        out.values[char_class].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Logical))
    );

    assert_eq!(out.values[file_snapshot].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[file_snapshot].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[file_snapshot].value_term,
        TypeTerm::List(Box::new(TypeTerm::Any))
    );

    assert_eq!(out.values[compare_version].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[compare_version].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[compare_version].value_term, TypeTerm::Double);

    assert_eq!(out.values[capture].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[capture].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[capture].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    for vid in [url_encoded, url_decoded] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(out.values[vid].value_term, TypeTerm::Char);
    }

    assert_eq!(out.values[glob_rx].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[glob_rx].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[glob_rx].value_term, TypeTerm::Char);

    assert_eq!(out.values[file_test_scalar].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[file_test_scalar].value_ty.prim, PrimTy::Logical);
    assert_eq!(out.values[file_test_scalar].value_term, TypeTerm::Logical);

    assert_eq!(out.values[file_test_vector].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[file_test_vector].value_ty.prim, PrimTy::Logical);
    assert_eq!(
        out.values[file_test_vector].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Logical))
    );

    assert_eq!(out.values[combn].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[combn].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[combn].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[adist].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[adist].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[adist].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[count_fields].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[count_fields].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[count_fields].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );

    assert_eq!(out.values[converted].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[converted].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[converted].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    for vid in [write, write_table, write_csv2, str_call] {
        assert_eq!(out.values[vid].value_ty, RR::typeck::TypeState::null());
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }
}

#[test]
fn parallel_package_calls_have_direct_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec![
            "tasks".to_string(),
            "worker_count".to_string(),
            "cluster".to_string(),
        ],
    );
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[1] = RR::typeck::TypeState::scalar(PrimTy::Int, false);
    fn_ir.param_ty_hints[2] = RR::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[1] = TypeTerm::Int;
    fn_ir.param_term_hints[2] = TypeTerm::List(Box::new(TypeTerm::Any));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let tasks = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let worker_count = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cluster = fn_ir.add_value(
        ValueKind::Param { index: 2 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let cores = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::detectCores".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mk = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::makeCluster".to_string(),
            args: vec![worker_count],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let apply = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::parLapply".to_string(),
            args: vec![cluster, tasks],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let export_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("offset".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let export = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::clusterExport".to_string(),
            args: vec![cluster, export_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let evalq = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::clusterEvalQ".to_string(),
            args: vec![cluster, worker_count],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let map = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::clusterMap".to_string(),
            args: vec![cluster, tasks, tasks],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cluster_apply = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::clusterApply".to_string(),
            args: vec![cluster, tasks],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cluster_call = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::clusterCall".to_string(),
            args: vec![cluster, worker_count],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mc = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::mclapply".to_string(),
            args: vec![tasks],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cluster_split = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::clusterSplit".to_string(),
            args: vec![cluster, tasks],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let split_indices = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::splitIndices".to_string(),
            args: vec![worker_count, worker_count],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let apply_lb = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::clusterApplyLB".to_string(),
            args: vec![cluster, tasks],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let par_sapply = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::parSapply".to_string(),
            args: vec![cluster, tasks],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let par_sapply_lb = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::parSapplyLB".to_string(),
            args: vec![cluster, tasks],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let par_apply = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::parApply".to_string(),
            args: vec![cluster, tasks, worker_count, worker_count],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let job = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::mcparallel".to_string(),
            args: vec![tasks],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let collected = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::mccollect".to_string(),
            args: vec![job],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stop = fn_ir.add_value(
        ValueKind::Call {
            callee: "parallel::stopCluster".to_string(),
            args: vec![cluster],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(stop));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    assert_eq!(out.values[cores].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[cores].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[cores].value_term, TypeTerm::Int);

    for vid in [
        mk,
        apply,
        evalq,
        map,
        cluster_apply,
        cluster_call,
        mc,
        cluster_split,
        split_indices,
        apply_lb,
        par_sapply,
        par_sapply_lb,
        par_apply,
        job,
        collected,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
    }

    for vid in [
        mk,
        apply,
        evalq,
        map,
        cluster_apply,
        cluster_call,
        mc,
        cluster_split,
        split_indices,
        apply_lb,
        job,
        collected,
    ] {
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }

    for vid in [par_sapply, par_sapply_lb, par_apply] {
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Any))
        );
    }

    assert_eq!(out.values[export].value_ty, RR::typeck::TypeState::null());
    assert_eq!(out.values[export].value_term, TypeTerm::Null);
    assert_eq!(out.values[stop].value_ty, RR::typeck::TypeState::null());
    assert_eq!(out.values[stop].value_term, TypeTerm::Null);
}

#[test]
fn splines_package_calls_have_direct_matrix_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["xs".to_string(), "knots".to_string()],
    );
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[1] = RR::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Double));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let xs = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let knots = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let bs = fn_ir.add_value(
        ValueKind::Call {
            callee: "splines::bs".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ns = fn_ir.add_value(
        ValueKind::Call {
            callee: "splines::ns".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let spline_design = fn_ir.add_value(
        ValueKind::Call {
            callee: "splines::splineDesign".to_string(),
            args: vec![knots, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let interp = fn_ir.add_value(
        ValueKind::Call {
            callee: "splines::interpSpline".to_string(),
            args: vec![xs, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let periodic = fn_ir.add_value(
        ValueKind::Call {
            callee: "splines::periodicSpline".to_string(),
            args: vec![xs, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let back = fn_ir.add_value(
        ValueKind::Call {
            callee: "splines::backSpline".to_string(),
            args: vec![interp],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let spline_knots = fn_ir.add_value(
        ValueKind::Call {
            callee: "splines::splineKnots".to_string(),
            args: vec![interp],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let spline_order = fn_ir.add_value(
        ValueKind::Call {
            callee: "splines::splineOrder".to_string(),
            args: vec![interp],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xy = fn_ir.add_value(
        ValueKind::Call {
            callee: "splines::xyVector".to_string(),
            args: vec![xs, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let spline_des = fn_ir.add_value(
        ValueKind::Call {
            callee: "splines::spline.des".to_string(),
            args: vec![knots, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_poly = fn_ir.add_value(
        ValueKind::Call {
            callee: "splines::as.polySpline".to_string(),
            args: vec![interp],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let poly = fn_ir.add_value(
        ValueKind::Call {
            callee: "splines::polySpline".to_string(),
            args: vec![periodic],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_vec = fn_ir.add_value(
        ValueKind::Call {
            callee: "splines::asVector".to_string(),
            args: vec![xy],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(spline_design));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [bs, ns, spline_design] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Matrix(Box::new(TypeTerm::Double))
        );
    }
    for vid in [interp, periodic, back] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
    }
    assert_eq!(
        out.values[interp].value_term,
        TypeTerm::NamedList(vec![
            (
                "knots".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("coefficients".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(
        out.values[periodic].value_term,
        TypeTerm::NamedList(vec![
            (
                "knots".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("coefficients".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(
        out.values[back].value_term,
        TypeTerm::NamedList(vec![
            (
                "knots".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("coefficients".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(out.values[xy].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[xy].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[xy].value_term,
        TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(out.values[spline_des].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[spline_des].value_term,
        TypeTerm::NamedList(vec![
            (
                "knots".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("order".to_string(), TypeTerm::Double),
            (
                "derivs".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            (
                "design".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(out.values[as_poly].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[as_poly].value_term,
        TypeTerm::NamedList(vec![
            (
                "knots".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("coefficients".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(out.values[poly].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[poly].value_term,
        TypeTerm::NamedList(vec![
            (
                "knots".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("coefficients".to_string(), TypeTerm::Any),
            ("period".to_string(), TypeTerm::Double),
        ])
    );
    assert_eq!(out.values[spline_knots].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[spline_knots].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[spline_knots].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[spline_order].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[spline_order].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[spline_order].value_term, TypeTerm::Int);
    assert_eq!(out.values[as_vec].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[as_vec].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[as_vec].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
}
