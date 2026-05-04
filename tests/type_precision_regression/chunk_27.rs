use super::type_precision_regression_common::*;

#[test]
pub(crate) fn utils_package_calls_have_direct_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["xs".to_string(), "df".to_string(), "path".to_string()],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[1] = rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Any, false);
    fn_ir.param_ty_hints[2] =
        rr::compiler::internal::typeck::TypeState::scalar(PrimTy::Char, false);
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

    let head = add_call(&mut fn_ir, "utils::head", vec![xs]);
    let tail = add_call(&mut fn_ir, "utils::tail", vec![df]);
    let read = add_call(&mut fn_ir, "utils::read.csv", vec![path]);
    let read_csv2 = add_call(&mut fn_ir, "utils::read.csv2", vec![path]);
    let read_table = add_call(&mut fn_ir, "utils::read.table", vec![path]);
    let read_fwf = add_call(&mut fn_ir, "utils::read.fwf", vec![path]);
    let read_delim = add_call(&mut fn_ir, "utils::read.delim", vec![path]);
    let package_version = add_call(&mut fn_ir, "utils::packageVersion", vec![path]);
    let maintainer = add_call(&mut fn_ir, "utils::maintainer", vec![path]);
    let package_date = add_call(&mut fn_ir, "utils::packageDate", vec![path]);
    let object_size = add_call(&mut fn_ir, "utils::object.size", vec![xs]);
    let memory_size = add_call(&mut fn_ir, "utils::memory.size", vec![]);
    let memory_limit = add_call(&mut fn_ir, "utils::memory.limit", vec![]);
    let compare_left = add_str(&mut fn_ir, "1.2.0");
    let compare_right = add_str(&mut fn_ir, "1.1.9");
    let compare_version = add_call(
        &mut fn_ir,
        "utils::compareVersion",
        vec![compare_left, compare_right],
    );
    let capture = add_call(&mut fn_ir, "utils::capture.output", vec![xs]);
    let package_description = add_call(&mut fn_ir, "utils::packageDescription", vec![path]);
    let session_info = add_call(&mut fn_ir, "utils::sessionInfo", vec![]);
    let package_field_name = add_str(&mut fn_ir, "Package");
    let base_pkgs_field_name = add_str(&mut fn_ir, "basePkgs");
    let package_field = add_call(
        &mut fn_ir,
        "rr_field_get",
        vec![package_description, package_field_name],
    );
    let session_base_pkgs = add_call(
        &mut fn_ir,
        "rr_field_get",
        vec![session_info, base_pkgs_field_name],
    );
    let citation = add_call(&mut fn_ir, "utils::citation", vec![path]);
    let person = add_call(&mut fn_ir, "utils::person", vec![path, path]);
    let as_person = add_call(&mut fn_ir, "utils::as.person", vec![path]);
    let as_person_list = add_call(&mut fn_ir, "utils::as.personList", vec![path]);
    let as_roman = add_call(&mut fn_ir, "utils::as.roman", vec![xs]);
    let has_name = add_call(&mut fn_ir, "utils::hasName", vec![df, path]);
    let capture_a = add_str(&mut fn_ir, "1-one");
    let capture_b = add_str(&mut fn_ir, "2-two");
    let capture_input = add_call(&mut fn_ir, "c", vec![capture_a, capture_b]);
    let strcapture = add_call(
        &mut fn_ir,
        "utils::strcapture",
        vec![path, capture_input, df],
    );
    let apropos = add_call(&mut fn_ir, "utils::apropos", vec![path]);
    let find_pkg = add_call(&mut fn_ir, "utils::find", vec![path]);
    let get_anywhere = add_call(&mut fn_ir, "utils::getAnywhere", vec![path]);
    let args_anywhere = add_call(&mut fn_ir, "utils::argsAnywhere", vec![path]);
    let match_pat = add_str(&mut fn_ir, "me");
    let match_values = add_call(&mut fn_ir, "c", vec![path, path]);
    let find_matches = add_call(
        &mut fn_ir,
        "utils::findMatches",
        vec![match_pat, match_values],
    );
    let methods_fn = add_call(&mut fn_ir, "utils::methods", vec![path]);
    let help_search = add_call(&mut fn_ir, "utils::help.search", vec![path]);
    let data_iqr = add_call(&mut fn_ir, "utils::data", vec![]);
    let contrib_url = add_call(&mut fn_ir, "utils::contrib.url", vec![path]);
    let locale_charset = add_call(&mut fn_ir, "utils::localeToCharset", vec![]);
    let alpha_class = add_str(&mut fn_ir, "alpha");
    let char_class = add_call(&mut fn_ir, "utils::charClass", vec![path, alpha_class]);
    let file_snapshot = add_call(&mut fn_ir, "utils::fileSnapshot", vec![path]);
    let url_encoded = add_call(&mut fn_ir, "utils::URLencode", vec![path]);
    let url_decoded = add_call(&mut fn_ir, "utils::URLdecode", vec![path]);
    let glob_rx = add_call(&mut fn_ir, "utils::glob2rx", vec![path]);
    let file_flag = add_str(&mut fn_ir, "-f");
    let file_test_scalar = add_call(&mut fn_ir, "utils::file_test", vec![file_flag, path]);
    let other_path = add_str(&mut fn_ir, "other.txt");
    let file_test_paths = add_call(&mut fn_ir, "c", vec![path, other_path]);
    let file_test_vector = add_call(
        &mut fn_ir,
        "utils::file_test",
        vec![file_flag, file_test_paths],
    );
    let write_table = add_call(&mut fn_ir, "utils::write.table", vec![df, path]);
    let write_csv2 = add_call(&mut fn_ir, "utils::write.csv2", vec![df, path]);
    let combn_n = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let combn = add_call(&mut fn_ir, "utils::combn", vec![xs, combn_n]);
    let cat_word = add_str(&mut fn_ir, "cat");
    let dog_word = add_str(&mut fn_ir, "dog");
    let cot_word = add_str(&mut fn_ir, "cot");
    let dig_word = add_str(&mut fn_ir, "dig");
    let words_a = add_call(&mut fn_ir, "c", vec![cat_word, dog_word]);
    let words_b = add_call(&mut fn_ir, "c", vec![cot_word, dig_word]);
    let adist = add_call(&mut fn_ir, "utils::adist", vec![words_a, words_b]);
    let count_fields = add_call(&mut fn_ir, "utils::count.fields", vec![path]);
    let one_str = add_str(&mut fn_ir, "1");
    let two_str = add_str(&mut fn_ir, "2");
    let chars = add_call(&mut fn_ir, "c", vec![one_str, two_str]);
    let converted = add_call(&mut fn_ir, "utils::type.convert", vec![chars]);
    let write = add_call(&mut fn_ir, "utils::write.csv", vec![df, path]);
    let str_call = add_call(&mut fn_ir, "utils::str", vec![df]);
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
        rr::compiler::internal::typeck::TypeState::unknown()
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
        assert_eq!(
            out.values[vid].value_ty,
            rr::compiler::internal::typeck::TypeState::null()
        );
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
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[1] = rr::compiler::internal::typeck::TypeState::scalar(PrimTy::Int, false);
    fn_ir.param_ty_hints[2] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Any, false);
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

    let cores = add_call(&mut fn_ir, "parallel::detectCores", vec![]);
    let mk = add_call(&mut fn_ir, "parallel::makeCluster", vec![worker_count]);
    let apply = add_call(&mut fn_ir, "parallel::parLapply", vec![cluster, tasks]);
    let export_name = add_str(&mut fn_ir, "offset");
    let export = add_call(
        &mut fn_ir,
        "parallel::clusterExport",
        vec![cluster, export_name],
    );
    let evalq = add_call(
        &mut fn_ir,
        "parallel::clusterEvalQ",
        vec![cluster, worker_count],
    );
    let map = add_call(
        &mut fn_ir,
        "parallel::clusterMap",
        vec![cluster, tasks, tasks],
    );
    let cluster_apply = add_call(&mut fn_ir, "parallel::clusterApply", vec![cluster, tasks]);
    let cluster_call = add_call(
        &mut fn_ir,
        "parallel::clusterCall",
        vec![cluster, worker_count],
    );
    let mc = add_call(&mut fn_ir, "parallel::mclapply", vec![tasks]);
    let cluster_split = add_call(&mut fn_ir, "parallel::clusterSplit", vec![cluster, tasks]);
    let split_indices = add_call(
        &mut fn_ir,
        "parallel::splitIndices",
        vec![worker_count, worker_count],
    );
    let apply_lb = add_call(&mut fn_ir, "parallel::clusterApplyLB", vec![cluster, tasks]);
    let par_sapply = add_call(&mut fn_ir, "parallel::parSapply", vec![cluster, tasks]);
    let par_sapply_lb = add_call(&mut fn_ir, "parallel::parSapplyLB", vec![cluster, tasks]);
    let par_apply = add_call(
        &mut fn_ir,
        "parallel::parApply",
        vec![cluster, tasks, worker_count, worker_count],
    );
    let job = add_call(&mut fn_ir, "parallel::mcparallel", vec![tasks]);
    let collected = add_call(&mut fn_ir, "parallel::mccollect", vec![job]);
    let stop = add_call(&mut fn_ir, "parallel::stopCluster", vec![cluster]);
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

    assert_eq!(
        out.values[export].value_ty,
        rr::compiler::internal::typeck::TypeState::null()
    );
    assert_eq!(out.values[export].value_term, TypeTerm::Null);
    assert_eq!(
        out.values[stop].value_ty,
        rr::compiler::internal::typeck::TypeState::null()
    );
    assert_eq!(out.values[stop].value_term, TypeTerm::Null);
}

#[test]
pub(crate) fn splines_package_calls_have_direct_matrix_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["xs".to_string(), "knots".to_string()],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[1] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
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

    let bs = add_call(&mut fn_ir, "splines::bs", vec![xs]);
    let ns = add_call(&mut fn_ir, "splines::ns", vec![xs]);
    let spline_design = add_call(&mut fn_ir, "splines::splineDesign", vec![knots, xs]);
    let interp = add_call(&mut fn_ir, "splines::interpSpline", vec![xs, xs]);
    let periodic = add_call(&mut fn_ir, "splines::periodicSpline", vec![xs, xs]);
    let back = add_call(&mut fn_ir, "splines::backSpline", vec![interp]);
    let spline_knots = add_call(&mut fn_ir, "splines::splineKnots", vec![interp]);
    let spline_order = add_call(&mut fn_ir, "splines::splineOrder", vec![interp]);
    let xy = add_call(&mut fn_ir, "splines::xyVector", vec![xs, xs]);
    let spline_des = add_call(&mut fn_ir, "splines::spline.des", vec![knots, xs]);
    let as_poly = add_call(&mut fn_ir, "splines::as.polySpline", vec![interp]);
    let poly = add_call(&mut fn_ir, "splines::polySpline", vec![periodic]);
    let as_vec = add_call(&mut fn_ir, "splines::asVector", vec![xy]);
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
