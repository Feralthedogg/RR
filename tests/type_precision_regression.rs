use RR::hir::def::Ty;
use RR::mir::{FnIR, Terminator, ValueKind};
use RR::typeck::solver::{TypeConfig, analyze_program, hir_ty_to_type_state, hir_ty_to_type_term};
use RR::typeck::{NaTy, PrimTy, ShapeTy, TypeTerm};
use RR::{mir::Facts, utils::Span};
use rustc_hash::FxHashMap;

#[test]
fn arithmetic_keeps_int_double_boundary_precise() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);
    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let i5 = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(5)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let i2 = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let div = fn_ir.add_value(
        ValueKind::Binary {
            op: RR::syntax::ast::BinOp::Div,
            lhs: i5,
            rhs: i2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let modu = fn_ir.add_value(
        ValueKind::Binary {
            op: RR::syntax::ast::BinOp::Mod,
            lhs: i5,
            rhs: i2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(div));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[div].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[modu].value_ty.prim, PrimTy::Int);
}

#[test]
fn hir_type_lowering_preserves_option_union_and_result_shape() {
    let option_int = hir_ty_to_type_state(&Ty::Option(Box::new(Ty::Int)));
    assert_eq!(option_int.prim, PrimTy::Int);
    assert_eq!(option_int.shape, ShapeTy::Scalar);
    assert_eq!(option_int.na, NaTy::Maybe);

    let union_num = hir_ty_to_type_state(&Ty::Union(vec![Ty::Int, Ty::Double]));
    assert_eq!(union_num.prim, PrimTy::Double);
    assert_eq!(union_num.shape, ShapeTy::Scalar);

    let result_num = hir_ty_to_type_state(&Ty::Result(Box::new(Ty::Int), Box::new(Ty::Double)));
    assert_eq!(result_num.prim, PrimTy::Double);
    assert_eq!(result_num.shape, ShapeTy::Scalar);

    let matrix_float = hir_ty_to_type_state(&Ty::Matrix(Box::new(Ty::Double)));
    assert_eq!(matrix_float.prim, PrimTy::Double);
    assert_eq!(matrix_float.shape, ShapeTy::Matrix);

    let df = hir_ty_to_type_state(&Ty::DataFrame(vec![]));
    assert_eq!(df.shape, ShapeTy::Matrix);

    let matrix_term = hir_ty_to_type_term(&Ty::Matrix(Box::new(Ty::Double)));
    assert_eq!(matrix_term, TypeTerm::Matrix(Box::new(TypeTerm::Double)));

    let df_term = hir_ty_to_type_term(&Ty::DataFrame(vec![
        (RR::hir::def::SymbolId(1), Ty::Int),
        (RR::hir::def::SymbolId(2), Ty::Double),
    ]));
    assert_eq!(
        df_term,
        TypeTerm::DataFrame(vec![TypeTerm::Int, TypeTerm::Double])
    );
}

#[test]
fn index_refines_scalar_type_from_structural_term() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["xs".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Int));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let p = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let idx = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let elem = fn_ir.add_value(
        ValueKind::Index1D {
            base: p,
            idx,
            is_safe: false,
            is_na_safe: false,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(elem));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[p].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[p].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[elem].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[elem].value_ty.shape, ShapeTy::Scalar);
}

#[test]
fn builtin_vector_calls_preserve_len_sym_and_numeric_precision() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["xs".to_string(), "ys".to_string()],
    );
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Int, false);
    fn_ir.param_ty_hints[1] = RR::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Int));
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
    let ys = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let three = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(3)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let abs_xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "abs".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pmax_scalar = fn_ir.add_value(
        ValueKind::Call {
            callee: "pmax".to_string(),
            args: vec![xs, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let log_xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "log10".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_na_xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "is.na".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sum_xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "sum".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pmax_zip = fn_ir.add_value(
        ValueKind::Call {
            callee: "pmax".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(sum_xs));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    let xs_len = out.param_ty_hints[0].len_sym;
    let ys_len = out.param_ty_hints[1].len_sym;
    assert!(xs_len.is_some());
    assert!(ys_len.is_some());
    assert_ne!(xs_len, ys_len);

    assert_eq!(out.values[abs_xs].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[abs_xs].value_ty.len_sym, xs_len);

    assert_eq!(out.values[pmax_scalar].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[pmax_scalar].value_ty.len_sym, xs_len);

    assert_eq!(out.values[log_xs].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[log_xs].value_ty.len_sym, xs_len);

    assert_eq!(out.values[is_na_xs].value_ty.prim, PrimTy::Logical);
    assert_eq!(out.values[is_na_xs].value_ty.len_sym, xs_len);

    assert_eq!(out.values[sum_xs].value_ty.prim, PrimTy::Int);

    assert_eq!(out.values[pmax_zip].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[pmax_zip].value_ty.len_sym, None);
}

#[test]
fn field_access_refines_from_dataframe_term() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["df".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::matrix(PrimTy::Any, false);
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
    let name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("right".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let field = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![df, name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(field));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(
        out.values[field].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[field].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[field].value_ty.prim, PrimTy::Double);
}

#[test]
fn strict_dataframe_schema_rejects_missing_and_mismatched_fields() {
    let mut callee = FnIR::new(
        "Sym_main".to_string(),
        vec!["df".to_string(), "bad".to_string()],
    );
    callee.param_ty_hints[0] = RR::typeck::TypeState::matrix(PrimTy::Any, false);
    callee.param_ty_hints[1] = RR::typeck::TypeState::scalar(PrimTy::Char, false);
    callee.param_term_hints[0] = TypeTerm::DataFrameNamed(vec![
        (
            "left".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Int)),
        ),
        (
            "right".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Double)),
        ),
    ]);
    callee.param_term_hints[1] = TypeTerm::Char;

    let b0 = callee.add_block();
    callee.entry = b0;
    callee.body_head = b0;

    let df = callee.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bad = callee.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let missing_name = callee.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("missing".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let right_name = callee.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("right".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let missing_get = callee.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![df, missing_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let _bad_set = callee.add_value(
        ValueKind::Call {
            callee: "rr_field_set".to_string(),
            args: vec![df, right_name, bad],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    callee.blocks[b0].term = Terminator::Return(Some(missing_get));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), callee);
    let err = analyze_program(
        &mut all,
        TypeConfig {
            mode: RR::typeck::TypeMode::Strict,
            native_backend: RR::typeck::NativeBackend::Off,
        },
    )
    .expect_err("strict analysis must fail");
    let text = format!("{err:?}");
    assert!(text.contains("visible dataframe schema"));
    assert!(text.contains("expects"));
}

#[test]
fn field_write_updates_dataframe_schema_term() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["df".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::matrix(PrimTy::Any, false);
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
        ValueKind::Const(RR::syntax::ast::Lit::Str("right".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(1.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(2.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let three = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(3.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let four = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(4.0)),
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
fn matrix_builtins_preserve_matrix_and_vector_shapes() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);
    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let six = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(6)),
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
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cols = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(3)),
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
fn matrix_shape_builtins_are_known_to_type_layer() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);
    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let six = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(6)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rows = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cols = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(3)),
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
    assert_eq!(out.values[nrow_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[nrow_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[ncol_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[ncol_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[dimnames_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::Vector(Box::new(TypeTerm::Char))))
    );
}

#[test]
fn matrix_shape_algebra_preserves_dimension_terms() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);
    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let six = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(6)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rows = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cols = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(3)),
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
    let trans = fn_ir.add_value(
        ValueKind::Call {
            callee: "t".to_string(),
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
    let diag_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "diag".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rb = fn_ir.add_value(
        ValueKind::Call {
            callee: "rbind".to_string(),
            args: vec![mat, mat],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cb = fn_ir.add_value(
        ValueKind::Call {
            callee: "cbind".to_string(),
            args: vec![mat, mat],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mm = fn_ir.add_value(
        ValueKind::Binary {
            op: RR::syntax::ast::BinOp::MatMul,
            lhs: mat,
            rhs: trans,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(mm));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(
        out.values[mat].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Int), Some(2), Some(3))
    );
    assert_eq!(
        out.values[trans].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Int), Some(3), Some(2))
    );
    assert_eq!(
        out.values[cross].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(3), Some(3))
    );
    assert_eq!(
        out.values[tcross].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(2), Some(2))
    );
    assert_eq!(
        out.values[diag_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(
        out.values[rb].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Int), Some(4), Some(3))
    );
    assert_eq!(
        out.values[cb].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Int), Some(2), Some(6))
    );
    assert_eq!(
        out.values[mm].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Int), Some(2), Some(2))
    );
    assert_eq!(out.values[mm].value_ty.shape, ShapeTy::Matrix);
}
