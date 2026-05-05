use crate::type_precision_regression_common::*;

#[test]
pub(crate) fn methods_package_calls_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let class_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "MethodDefinition".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let generic_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "show".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let any_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "ANY".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let is_class = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::isClass".to_string(),
            args: vec![class_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_generic = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::isGeneric".to_string(),
            args: vec![generic_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let has_method = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::hasMethod".to_string(),
            args: vec![generic_name, any_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let exists_method = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::existsMethod".to_string(),
            args: vec![generic_name, any_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let class_def = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getClass".to_string(),
            args: vec![class_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let class_def_2 = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getClassDef".to_string(),
            args: vec![class_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let classes = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getClasses".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let global_env = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::globalenv".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let function_obj = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getFunction".to_string(),
            args: vec![generic_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let load_actions = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getLoadActions".to_string(),
            args: vec![global_env],
            names: vec![Some("where".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let package_name = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getPackageName".to_string(),
            args: vec![global_env],
            names: vec![Some("where".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let all_generics = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getGenerics".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let exists_function = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::existsFunction".to_string(),
            args: vec![generic_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let missing_action = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "missing-action".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let has_load_action = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::hasLoadAction".to_string(),
            args: vec![missing_action, global_env],
            names: vec![None, Some("where".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let has_arg = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::hasArg".to_string(),
            args: vec![generic_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let find_function = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::findFunction".to_string(),
            args: vec![generic_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let has_methods = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::hasMethods".to_string(),
            args: vec![generic_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let arith_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "Arith".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let group_ok = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::isGroup".to_string(),
            args: vec![arith_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let if_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "if".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let grammar_ok = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::isGrammarSymbol".to_string(),
            args: vec![if_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rematched_ok = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::isRematched".to_string(),
            args: vec![generic_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let old_class_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "oldClass".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let old_class_def = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getClass".to_string(),
            args: vec![old_class_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xs3_ok = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::isXS3Class".to_string(),
            args: vec![old_class_def],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let method_signatures = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::findMethodSignatures".to_string(),
            args: vec![generic_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let supers_all = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getAllSuperClasses".to_string(),
            args: vec![class_def],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sealed_class = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::isSealedClass".to_string(),
            args: vec![class_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sealed_method = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::isSealedMethod".to_string(),
            args: vec![generic_name, any_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let class_def_ok = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::isClassDef".to_string(),
            args: vec![class_def],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let class_adjacency = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::classesToAM".to_string(),
            args: vec![class_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cache_metadata = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::cacheMetaData".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let found_class = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::findClass".to_string(),
            args: vec![class_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let probe_message = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "rr probe".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let unique_function = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::findUnique".to_string(),
            args: vec![generic_name, probe_message, global_env],
            names: vec![None, None, Some("where".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let env_ref_class_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "envRefClass".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ref_class = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getRefClass".to_string(),
            args: vec![env_ref_class_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let validity_obj = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getValidity".to_string(),
            args: vec![class_def],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let null_value = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Null),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let test_virtual = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::testVirtual".to_string(),
            args: vec![classes, null_value, null_value, global_env],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let inherited_report = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::testInheritedMethods".to_string(),
            args: vec![generic_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let group = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getGroup".to_string(),
            args: vec![arith_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let group_members = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getGroupMembers".to_string(),
            args: vec![arith_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let slots_by_name = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getSlots".to_string(),
            args: vec![class_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let generic_obj = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getGeneric".to_string(),
            args: vec![generic_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formals = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::formalArgs".to_string(),
            args: vec![generic_obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_instance = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::is".to_string(),
            args: vec![class_def, class_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let new_obj = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::new".to_string(),
            args: vec![class_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let slot_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "x".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let slot_value = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::slot".to_string(),
            args: vec![new_obj, slot_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let valid_object = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::validObject".to_string(),
            args: vec![new_obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_virtual_class = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::isVirtualClass".to_string(),
            args: vec![class_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_class_union = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::isClassUnion".to_string(),
            args: vec![class_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let numeric_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "numeric".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let character_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "character".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let can_coerce = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::canCoerce".to_string(),
            args: vec![numeric_name, character_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let set_class = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::setClass".to_string(),
            args: vec![class_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let get_method = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getMethod".to_string(),
            args: vec![generic_name, any_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cache_method = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::cacheMethod".to_string(),
            args: vec![generic_name, any_name, get_method, generic_obj],
            names: vec![None, None, None, Some("fdef".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let find_method = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::findMethod".to_string(),
            args: vec![generic_name, any_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let methods_for_dispatch = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getMethodsForDispatch".to_string(),
            args: vec![generic_obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let standard_generic = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::standardGeneric".to_string(),
            args: vec![generic_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let data_part = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::getDataPart".to_string(),
            args: vec![new_obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let selected_method = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::selectMethod".to_string(),
            args: vec![generic_name, any_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let set_generic = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::setGeneric".to_string(),
            args: vec![generic_name, selected_method],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let set_method = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::setMethod".to_string(),
            args: vec![generic_name, any_name, selected_method],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let extends = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::extends".to_string(),
            args: vec![class_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let show_class = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::show".to_string(),
            args: vec![class_def],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let slots = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::slotNames".to_string(),
            args: vec![class_def],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let found_methods = fn_ir.add_value(
        ValueKind::Call {
            callee: "methods::findMethods".to_string(),
            args: vec![generic_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(is_class));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [
        is_class,
        is_generic,
        has_method,
        exists_method,
        exists_function,
        has_load_action,
        has_arg,
        has_methods,
        group_ok,
        grammar_ok,
        rematched_ok,
        xs3_ok,
        is_instance,
        valid_object,
        is_virtual_class,
        is_class_union,
        sealed_class,
        sealed_method,
        class_def_ok,
        test_virtual,
        can_coerce,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_term, TypeTerm::Logical);
    }

    for vid in [
        classes,
        slots,
        slots_by_name,
        group_members,
        formals,
        supers_all,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }
    assert_eq!(out.values[package_name].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[package_name].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[package_name].value_term, TypeTerm::Char);
    assert_eq!(out.values[extends].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[extends].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[extends].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    for vid in [
        class_def,
        class_def_2,
        all_generics,
        group,
        found_methods,
        found_class,
        unique_function,
        find_method,
        methods_for_dispatch,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }

    assert_eq!(out.values[class_adjacency].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[class_adjacency].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[class_adjacency].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[method_signatures].value_ty.shape,
        ShapeTy::Matrix
    );
    assert_eq!(out.values[method_signatures].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[method_signatures].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Char))
    );

    assert_eq!(
        out.values[cache_metadata].value_ty,
        rr::compiler::internal::typeck::TypeState::null()
    );
    assert_eq!(out.values[cache_metadata].value_term, TypeTerm::Null);
    assert_eq!(out.values[load_actions].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[load_actions].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[load_actions].value_term,
        TypeTerm::List(Box::new(TypeTerm::Any))
    );

    for vid in [
        function_obj,
        ref_class,
        generic_obj,
        cache_method,
        find_function,
        new_obj,
        data_part,
        slot_value,
        get_method,
        validity_obj,
        inherited_report,
        standard_generic,
        selected_method,
        set_class,
    ] {
        assert_eq!(
            out.values[vid].value_ty,
            rr::compiler::internal::typeck::TypeState::unknown()
        );
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }

    for vid in [set_generic, set_method] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(out.values[vid].value_term, TypeTerm::Char);
    }

    assert_eq!(
        out.values[show_class].value_ty,
        rr::compiler::internal::typeck::TypeState::null()
    );
    assert_eq!(out.values[show_class].value_term, TypeTerm::Null);
}
