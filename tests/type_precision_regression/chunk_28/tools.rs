use crate::type_precision_regression_common::*;

#[test]
pub(crate) fn tools_package_calls_have_direct_char_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["path".to_string(), "paths".to_string()],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::scalar(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Char;
    fn_ir.param_ty_hints[1] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Char));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let path = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let paths = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let title = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::toTitleCase".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let abs_path = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::file_path_as_absolute".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let user_dir = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::R_user_dir".to_string(),
            args: vec![path, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let md5 = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::md5sum".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sha = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::sha256sum".to_string(),
            args: vec![paths],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ext = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::file_ext".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sans_ext = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::file_path_sans_ext".to_string(),
            args: vec![paths],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let listed_exts = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::list_files_with_exts".to_string(),
            args: vec![path, paths],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let code_type = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "code".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let listed_type = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::list_files_with_type".to_string(),
            args: vec![path, code_type],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stats_pkg = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "stats".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let depends = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::dependsOnPkgs".to_string(),
            args: vec![stats_pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let vignette_info = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::getVignetteInfo".to_string(),
            args: vec![stats_pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pkg_vignettes = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::pkgVignettes".to_string(),
            args: vec![stats_pkg],
            names: vec![Some("package".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let open_delim = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "[".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let close_delim = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "]".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let delim_chars = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![open_delim, close_delim],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nested_text = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "a[b[c]d]e".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let delim_match = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::delimMatch".to_string(),
            args: vec![nested_text, delim_chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let uri_text = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "https://example.com/path?a=1#frag".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let parsed_uri = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::parse_URI_reference".to_string(),
            args: vec![uri_text],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let parsed_rd = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::parse_Rd".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rd_text = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::Rd2txt".to_string(),
            args: vec![parsed_rd],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rd_html = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::Rd2HTML".to_string(),
            args: vec![parsed_rd],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rd_latex = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::Rd2latex".to_string(),
            args: vec![parsed_rd],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rd_ex = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::Rd2ex".to_string(),
            args: vec![parsed_rd],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rd_index = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::Rdindex".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_index = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::read.00Index".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let checked_rd = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::checkRd".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rd_filtered = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::RdTextFilter".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nonascii_shown = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::showNonASCII".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nonascii_file = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::showNonASCIIfile".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rd_options = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::Rd2txt_options".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let names_field_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "names".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dir_field_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "dir".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pkg_vig_names = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![pkg_vignettes, names_field_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pkg_vig_dir = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![pkg_vignettes, dir_field_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let width_field_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "width".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rd_width = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![rd_options, width_field_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let encoded_text = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::encoded_text_to_latex".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let latex_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "alpha_beta".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let parsed_latex = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::parseLatex".to_string(),
            args: vec![latex_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bibstyle = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::getBibstyle".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let deparsed_latex = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::deparseLatex".to_string(),
            args: vec![parsed_latex],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let escaped_latex_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "caf\\'e".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let parsed_latex_utf8 = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::parseLatex".to_string(),
            args: vec![escaped_latex_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let latex_utf8 = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::latexToUtf8".to_string(),
            args: vec![parsed_latex_utf8],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let standard_package_names = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::standard_package_names".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let base_aliases_db = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::base_aliases_db".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let base_rdxrefs_db = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::base_rdxrefs_db".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cran_aliases_db = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::CRAN_aliases_db".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cran_archive_db = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::CRAN_archive_db".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cran_package_db = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::CRAN_package_db".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cran_rdxrefs_db = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::CRAN_rdxrefs_db".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cran_authors_db = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::CRAN_authors_db".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cran_current_db = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::CRAN_current_db".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cran_check_results = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::CRAN_check_results".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cran_check_details = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::CRAN_check_details".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cran_check_issues = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::CRAN_check_issues".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pkg_a = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "a5R".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pkg_b = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "aae.pop".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_packages = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![pkg_a, pkg_b],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cran_summary = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::summarize_CRAN_check_status".to_string(),
            args: vec![summary_packages],
            names: vec![Some("packages".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let installed = fn_ir.add_value(
        ValueKind::Call {
            callee: "utils::installed.packages".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let package_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "stats".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let recursive = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Bool(false)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let deps = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::package_dependencies".to_string(),
            args: vec![package_name, installed, recursive],
            names: vec![None, Some("db".to_string()), Some("recursive".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rd_db = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::Rd_db".to_string(),
            args: vec![package_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(title));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [title, abs_path, user_dir, ext] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(out.values[vid].value_term, TypeTerm::Char);
    }

    assert_eq!(out.values[sans_ext].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[sans_ext].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[sans_ext].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    for vid in [md5, sha, listed_exts, listed_type, depends] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    assert_eq!(out.values[vignette_info].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[vignette_info].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[vignette_info].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Char))
    );

    assert_eq!(out.values[cran_package_db].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[cran_package_db].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[cran_package_db].value_term,
        TypeTerm::DataFrame(Vec::new())
    );

    assert_eq!(out.values[delim_match].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[delim_match].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[delim_match].value_term, TypeTerm::Int);

    assert_eq!(out.values[parsed_uri].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[parsed_uri].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[parsed_uri].value_term,
        TypeTerm::DataFrameNamed(vec![
            (
                "scheme".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "authority".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "path".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "query".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "fragment".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])
    );

    assert_eq!(out.values[bibstyle].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[bibstyle].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[bibstyle].value_term, TypeTerm::Char);

    {
        let vid = parsed_rd;
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }
    assert_eq!(
        out.values[pkg_vignettes].value_term,
        TypeTerm::NamedList(vec![
            (
                "docs".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
            (
                "names".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
            (
                "engines".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "patterns".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "encodings".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            ("dir".to_string(), TypeTerm::Char),
            ("pkgdir".to_string(), TypeTerm::Char),
            (
                "msg".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
        ])
    );
    assert_eq!(
        out.values[rd_options].value_term,
        TypeTerm::NamedList(vec![
            ("width".to_string(), TypeTerm::Int),
            ("minIndent".to_string(), TypeTerm::Int),
            ("extraIndent".to_string(), TypeTerm::Int),
            ("sectionIndent".to_string(), TypeTerm::Int),
            ("sectionExtra".to_string(), TypeTerm::Int),
            ("itemBullet".to_string(), TypeTerm::Char),
            ("enumFormat".to_string(), TypeTerm::Any),
            ("showURLs".to_string(), TypeTerm::Logical),
            ("code_quote".to_string(), TypeTerm::Logical),
            ("underline_titles".to_string(), TypeTerm::Logical),
        ])
    );
    assert_eq!(out.values[rd_width].value_term, TypeTerm::Int);
    assert_eq!(
        out.values[pkg_vig_names].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );
    assert_eq!(out.values[pkg_vig_dir].value_term, TypeTerm::Char);

    for vid in [rd_html, rd_latex, rd_ex] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(out.values[vid].value_term, TypeTerm::Char);
    }

    assert_eq!(
        out.values[rd_index].value_ty,
        rr::compiler::internal::typeck::TypeState::null()
    );
    assert_eq!(out.values[rd_index].value_term, TypeTerm::Null);

    assert_eq!(out.values[read_index].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[read_index].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[read_index].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Char))
    );

    assert_eq!(out.values[encoded_text].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[encoded_text].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[encoded_text].value_term, TypeTerm::Char);

    assert_eq!(out.values[deparsed_latex].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[deparsed_latex].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[deparsed_latex].value_term, TypeTerm::Char);

    assert_eq!(out.values[parsed_latex].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[parsed_latex].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[parsed_latex].value_term,
        TypeTerm::List(Box::new(TypeTerm::Any))
    );

    assert_eq!(out.values[latex_utf8].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[latex_utf8].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[latex_utf8].value_term,
        TypeTerm::List(Box::new(TypeTerm::Any))
    );

    for vid in [
        standard_package_names,
        base_aliases_db,
        base_rdxrefs_db,
        cran_aliases_db,
        cran_archive_db,
        cran_rdxrefs_db,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }

    for vid in [
        cran_authors_db,
        cran_current_db,
        cran_check_results,
        cran_check_details,
        cran_check_issues,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::DataFrame(Vec::new()));
    }

    assert_eq!(out.values[cran_summary].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[cran_summary].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[cran_summary].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    for vid in [
        rd_text,
        checked_rd,
        rd_filtered,
        nonascii_shown,
        nonascii_file,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    assert_eq!(out.values[installed].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[installed].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[installed].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Char))
    );

    for vid in [deps, rd_db] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }
}
