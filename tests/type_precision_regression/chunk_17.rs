use super::type_precision_regression_common::*;

#[test]
fn base_direct_connection_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["text".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Char));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let text = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let stdin_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::stdin".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stdout_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::stdout".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stderr_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::stderr".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let text_conn_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::textConnection".to_string(),
            args: vec![text],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let text_val_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::textConnectionValue".to_string(),
            args: vec![text_conn_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let raw_conn_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::rawConnection".to_string(),
            args: vec![text],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let raw_val_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::rawConnectionValue".to_string(),
            args: vec![raw_conn_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let socket_conn_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::socketConnection".to_string(),
            args: vec![text],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let url_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::url".to_string(),
            args: vec![text],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pipe_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::pipe".to_string(),
            args: vec![text],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let open_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::open".to_string(),
            args: vec![stdin_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let close_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::close".to_string(),
            args: vec![stdin_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let close_all_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::closeAllConnections".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_open_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::isOpen".to_string(),
            args: vec![stdin_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_incomplete_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::isIncomplete".to_string(),
            args: vec![stdin_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::summary.connection".to_string(),
            args: vec![stdin_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let push_back_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::pushBack".to_string(),
            args: vec![text, stdin_v],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let push_back_len_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::pushBackLength".to_string(),
            args: vec![stdin_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let clear_push_back_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::clearPushBack".to_string(),
            args: vec![stdin_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let socket_select_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::socketSelect".to_string(),
            args: vec![socket_conn_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(text));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [
        stdin_v,
        stdout_v,
        stderr_v,
        text_conn_v,
        raw_conn_v,
        socket_conn_v,
        url_v,
        pipe_v,
        open_v,
        summary_v,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }

    assert_eq!(out.values[text_val_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[text_val_v].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[text_val_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    assert_eq!(out.values[raw_val_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[raw_val_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[raw_val_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Any))
    );

    for vid in [close_v, close_all_v, push_back_v, clear_push_back_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Null);
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }

    for vid in [is_open_v, is_incomplete_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_term, TypeTerm::Logical);
    }

    assert_eq!(out.values[push_back_len_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[push_back_len_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[push_back_len_v].value_term, TypeTerm::Int);

    assert_eq!(out.values[socket_select_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[socket_select_v].value_ty.prim, PrimTy::Logical);
    assert_eq!(
        out.values[socket_select_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Logical))
    );
}

#[test]
fn base_direct_table_io_helpers_have_builtin_types() {
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

    let scan_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::scan".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_table_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::read.table".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_csv_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::read.csv".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_csv2_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::read.csv2".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_delim_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::read.delim".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_delim2_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::read.delim2".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let write_table_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::write.table".to_string(),
            args: vec![read_table_v, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let write_csv_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::write.csv".to_string(),
            args: vec![read_table_v, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let write_csv2_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::write.csv2".to_string(),
            args: vec![read_table_v, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let save_rds_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::saveRDS".to_string(),
            args: vec![read_table_v, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dput_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::dput".to_string(),
            args: vec![read_table_v, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dump_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::dump".to_string(),
            args: vec![path, path],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let count_fields_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::count.fields".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sink_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sink".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sink_number_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sink.number".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let capture_output_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::capture.output".to_string(),
            args: vec![read_table_v],
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

    assert_eq!(out.values[scan_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[scan_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[scan_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Any))
    );

    for vid in [
        read_table_v,
        read_csv_v,
        read_csv2_v,
        read_delim_v,
        read_delim2_v,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::DataFrame(Vec::new()));
    }

    for vid in [
        write_table_v,
        write_csv_v,
        write_csv2_v,
        save_rds_v,
        dput_v,
        dump_v,
        sink_v,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Null);
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }

    assert_eq!(out.values[count_fields_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[count_fields_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[count_fields_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );

    assert_eq!(out.values[sink_number_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[sink_number_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[sink_number_v].value_term, TypeTerm::Int);

    assert_eq!(out.values[capture_output_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[capture_output_v].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[capture_output_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );
}

#[test]
fn base_direct_apply_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["xs".to_string(), "df".to_string()],
    );
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_ty_hints[1] = RR::typeck::TypeState::matrix(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::List(Box::new(TypeTerm::Any));
    fn_ir.param_term_hints[1] = TypeTerm::DataFrame(Vec::new());

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

    let lapply_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::lapply".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sapply_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sapply".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let reduce_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Reduce".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let filter_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Filter".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let position_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Position".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let split_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::split".to_string(),
            args: vec![xs, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let unsplit_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::unsplit".to_string(),
            args: vec![xs, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let within_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::within".to_string(),
            args: vec![df],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let transform_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::transform".to_string(),
            args: vec![df],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let expand_grid_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::expand.grid".to_string(),
            args: vec![xs, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let merge_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::merge".to_string(),
            args: vec![df, df],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(df));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [lapply_v, split_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }

    assert_eq!(out.values[sapply_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[sapply_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[sapply_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Any))
    );

    assert_eq!(out.values[reduce_v].value_ty.shape, ShapeTy::Unknown);
    assert_eq!(out.values[reduce_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[reduce_v].value_term, TypeTerm::Any);

    for vid in [filter_v, unsplit_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }

    assert_eq!(out.values[position_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[position_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[position_v].value_term, TypeTerm::Int);

    for vid in [within_v, transform_v, expand_grid_v, merge_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::DataFrame(Vec::new()));
    }
}
