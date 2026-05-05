use super::type_precision_regression_common::*;

#[test]
pub(crate) fn base_direct_package_loading_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["pkg".to_string()]);
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::scalar(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Char;

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let pkg = fn_ir.add_value(
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

    let library_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::library".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let require_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::require".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let load_ns_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::loadNamespace".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pkg_has_ns_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::packageHasNamespace".to_string(),
            args: vec![pkg, pkg],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let searchpaths_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::searchpaths".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dlls_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::getLoadedDLLs".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_loaded_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::is.loaded".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dyn_load_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::dyn.load".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dyn_unload_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::dyn.unload".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let require_ns_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::requireNamespace".to_string(),
            args: vec![pkg, bool_true],
            names: vec![None, Some("quietly".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(pkg));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [library_v, searchpaths_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    for vid in [require_v, pkg_has_ns_v, is_loaded_v, require_ns_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_term, TypeTerm::Logical);
    }

    for vid in [load_ns_v, dlls_v, dyn_load_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }

    assert_eq!(out.values[dyn_unload_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[dyn_unload_v].value_ty.prim, PrimTy::Null);
    assert_eq!(out.values[dyn_unload_v].value_term, TypeTerm::Null);
}

#[test]
pub(crate) fn base_direct_connection_sys_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["path".to_string()]);
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Char, false);
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

    let read_lines_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::readLines".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let write_lines_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::writeLines".to_string(),
            args: vec![path, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let write_char_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::writeChar".to_string(),
            args: vec![path, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let write_bin_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::writeBin".to_string(),
            args: vec![path, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let flush_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::flush".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let seek_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::seek".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let trunc_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::truncate.connection".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_getenv_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.getenv".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_setenv_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.setenv".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_unsetenv_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.unsetenv".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_which_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.which".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_readlink_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.readlink".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_getpid_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.getpid".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_time_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.time".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_date_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.Date".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_info_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.info".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_getlocale_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.getlocale".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_glob_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Sys.glob".to_string(),
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
        read_lines_v,
        sys_getenv_v,
        sys_which_v,
        sys_readlink_v,
        sys_info_v,
        sys_glob_v,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    for vid in [write_lines_v, write_char_v, write_bin_v, flush_v, trunc_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Null);
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }

    assert_eq!(out.values[seek_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[seek_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[seek_v].value_term, TypeTerm::Double);

    for vid in [sys_setenv_v, sys_unsetenv_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_term, TypeTerm::Logical);
    }

    assert_eq!(out.values[sys_getpid_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[sys_getpid_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[sys_getpid_v].value_term, TypeTerm::Int);

    for vid in [sys_time_v, sys_date_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_term, TypeTerm::Double);
    }

    assert_eq!(out.values[sys_getlocale_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[sys_getlocale_v].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[sys_getlocale_v].value_term, TypeTerm::Char);
}

#[test]
pub(crate) fn base_direct_runtime_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["path".to_string()]);
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Char, false);
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

    let sys_call_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sys.call".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_calls_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sys.calls".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_parent_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sys.parent".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_parents_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sys.parents".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_nframe_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sys.nframe".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let search_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::search".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let geterr_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::geterrmessage".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let gettext_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::gettext".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let gettextf_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::gettextf".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ngettext_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::ngettext".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let message_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::message".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pkg_start_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::packageStartupMessage".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dot_pkg_start_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::.packageStartupMessage".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let source_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::source".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sys_source_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sys.source".to_string(),
            args: vec![path, path],
            names: vec![None, None],
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

    for vid in [sys_call_v, sys_calls_v, source_v, sys_source_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }

    for vid in [sys_parent_v, sys_nframe_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(out.values[vid].value_term, TypeTerm::Int);
    }

    assert_eq!(out.values[sys_parents_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[sys_parents_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[sys_parents_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );

    for vid in [search_v, gettext_v, gettextf_v, ngettext_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    assert_eq!(out.values[geterr_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[geterr_v].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[geterr_v].value_term, TypeTerm::Char);

    for vid in [message_v, pkg_start_v, dot_pkg_start_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Null);
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }
}
