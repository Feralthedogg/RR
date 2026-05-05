use super::type_precision_regression_common::*;

#[test]
pub(crate) fn field_write_updates_dataframe_schema_term() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["df".to_string()]);
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::DataFrameNamed(vec![
        (
            "left".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Int)),
        ),
        (
            "right".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Double)),
        ),
    ]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let df = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let right_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "right".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(1.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(2.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let three = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(3.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let four = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(4.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let doubles = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let updated = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_set".to_string(),
            args: vec![df, right_name, doubles],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read_back = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![updated, right_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(read_back));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(
        out.values[updated].value_term,
        TypeTerm::DataFrameNamed(vec![
            (
                "left".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            (
                "right".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(
        out.values[read_back].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[read_back].value_ty.prim, PrimTy::Double);
}

#[test]
pub(crate) fn matrix_builtins_preserve_matrix_and_vector_shapes() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);
    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let six = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(6)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "seq_len".to_string(),
            args: vec![six],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rows = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cols = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(3)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat = fn_ir.add_value(
        ValueKind::Call {
            callee: "matrix".to_string(),
            args: vec![vals, rows, cols],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let row_sums = fn_ir.add_value(
        ValueKind::Call {
            callee: "rowSums".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let col_sums = fn_ir.add_value(
        ValueKind::Call {
            callee: "colSums".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cross = fn_ir.add_value(
        ValueKind::Call {
            callee: "crossprod".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tcross = fn_ir.add_value(
        ValueKind::Call {
            callee: "tcrossprod".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(cross));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[mat].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[row_sums].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[col_sums].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[cross].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[tcross].value_ty.shape, ShapeTy::Matrix);

    assert_eq!(
        out.values[row_sums].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[cross].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(3), Some(3))
    );
}

#[test]
pub(crate) fn matrix_shape_builtins_are_known_to_type_layer() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);
    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let six = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(6)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rows = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cols = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(3)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "seq_len".to_string(),
            args: vec![six],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat = fn_ir.add_value(
        ValueKind::Call {
            callee: "matrix".to_string(),
            args: vec![vals, rows, cols],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "dim".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nrow_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "nrow".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ncol_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "ncol".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dimnames_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "dimnames".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(dim_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[dim_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[dim_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[dim_v].value_term,
        TypeTerm::VectorLen(Box::new(TypeTerm::Int), Some(2))
    );
    assert_eq!(out.values[nrow_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[nrow_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[ncol_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[ncol_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[dimnames_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::Vector(Box::new(TypeTerm::Char))))
    );
}
