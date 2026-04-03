use super::type_precision_regression_common::*;

#[test]
fn base_direct_serialization_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["path".to_string(), "raws".to_string()],
    );
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::scalar(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Char;
    fn_ir.param_ty_hints[1] = RR::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Any));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let path = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let raws = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let read_bin_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::readBin".to_string(),
            args: vec![path, path],
            names: vec![None, Some("what".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_char_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::readChar".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let serialize_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::serialize".to_string(),
            args: vec![path, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let unserialize_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::unserialize".to_string(),
            args: vec![raws],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let load_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::load".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fifo_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::fifo".to_string(),
            args: vec![path, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let gzcon_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::gzcon".to_string(),
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

    for vid in [read_bin_v, serialize_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Any))
        );
    }

    for vid in [read_char_v, load_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    for vid in [unserialize_v, fifo_v, gzcon_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
}

#[test]
fn base_direct_path_file_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["paths".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Char));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let paths = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let getwd_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::getwd".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tempdir_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::tempdir".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tempfile_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::tempfile".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dir_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::dir".to_string(),
            args: vec![paths],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let list_dirs_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::list.dirs".to_string(),
            args: vec![paths],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dir_create_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::dir.create".to_string(),
            args: vec![paths],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_create_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::file.create".to_string(),
            args: vec![paths],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_remove_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::file.remove".to_string(),
            args: vec![paths],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_rename_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::file.rename".to_string(),
            args: vec![paths, paths],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_copy_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::file.copy".to_string(),
            args: vec![paths, paths],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_access_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::file.access".to_string(),
            args: vec![paths],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_info_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::file.info".to_string(),
            args: vec![paths],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_size_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::file.size".to_string(),
            args: vec![paths],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_mtime_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::file.mtime".to_string(),
            args: vec![paths],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_mode_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::file.mode".to_string(),
            args: vec![paths],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let system_file_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::system.file".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let path_package_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::path.package".to_string(),
            args: vec![paths],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let packages_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::.packages".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(paths));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [getwd_v, tempdir_v, tempfile_v, system_file_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(out.values[vid].value_term, TypeTerm::Char);
    }

    for vid in [dir_v, list_dirs_v, path_package_v, packages_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    for vid in [
        dir_create_v,
        file_create_v,
        file_remove_v,
        file_rename_v,
        file_copy_v,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Logical))
        );
    }

    for vid in [file_access_v, file_mode_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Int))
        );
    }

    assert_eq!(out.values[file_info_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[file_info_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[file_info_v].value_term,
        TypeTerm::DataFrame(Vec::new())
    );

    for vid in [file_size_v, file_mtime_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
}

#[test]
fn base_direct_environment_namespace_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["pkg".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::scalar(PrimTy::Char, false);
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

    let baseenv_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::baseenv".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let emptyenv_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::emptyenv".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let new_env_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::new.env".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let parent_env_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::parent.env".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_env_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.environment".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let list2env_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::list2env".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let topenv_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::topenv".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_env_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::is.environment".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let env_name_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::environmentName".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_list_env_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.list.environment".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let env_locked_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::environmentIsLocked".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let loaded_namespaces_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::loadedNamespaces".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_ns_loaded_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::isNamespaceLoaded".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ns_name_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::getNamespaceName".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ns_exports_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::getNamespaceExports".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ns_imports_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::getNamespaceImports".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ns_users_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::getNamespaceUsers".to_string(),
            args: vec![pkg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ns_ver_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::getNamespaceVersion".to_string(),
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
            args: vec![pkg],
            names: vec![None],
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

    for vid in [
        baseenv_v,
        emptyenv_v,
        new_env_v,
        parent_env_v,
        as_env_v,
        list2env_v,
        topenv_v,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }

    for vid in [is_env_v, env_locked_v, is_ns_loaded_v, require_ns_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_term, TypeTerm::Logical);
    }

    for vid in [env_name_v, ns_name_v, ns_ver_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(out.values[vid].value_term, TypeTerm::Char);
    }

    for vid in [loaded_namespaces_v, ns_exports_v, ns_users_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    assert_eq!(out.values[as_list_env_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[as_list_env_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[as_list_env_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::Any))
    );

    assert_eq!(out.values[ns_imports_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[ns_imports_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[ns_imports_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::Any))
    );
}
