use super::type_precision_regression_common::*;

#[test]
fn stats_wrapper_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

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
    let a = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("a".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let b = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("b".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nums = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let groups = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![a, a, b, b],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let df = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![nums],
            names: vec![Some("x".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let by = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::list".to_string(),
            args: vec![groups],
            names: vec![Some("g".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mean_fn = fn_ir.add_value(
        ValueKind::Load {
            var: "base::mean".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ts_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ts".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let named = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::setNames".to_string(),
            args: vec![nums, groups],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let med = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::median.default".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let agg_df = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::aggregate.data.frame".to_string(),
            args: vec![df, by, mean_fn],
            names: vec![None, Some("by".to_string()), Some("FUN".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let agg_ts = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::aggregate.ts".to_string(),
            args: vec![ts_v, two, mean_fn],
            names: vec![
                None,
                Some("nfrequency".to_string()),
                Some("FUN".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(med));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[named].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[named].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[named].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[med].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[med].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[med].value_term, TypeTerm::Double);
    assert_eq!(out.values[agg_df].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[agg_df].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(
        out.values[agg_df].value_term,
        TypeTerm::DataFrame(Vec::new())
    );
    assert_eq!(out.values[agg_ts].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[agg_ts].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[agg_ts].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
}

#[test]
fn stats_termplot_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let formula_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("mpg ~ wt + hp".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mtcars = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::mtcars".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let false_v = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Bool(false)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![formula_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fit = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lm".to_string(),
            args: vec![formula, mtcars],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let termplot_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::termplot".to_string(),
            args: vec![fit, false_v],
            names: vec![None, Some("plot".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(termplot_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[termplot_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[termplot_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[termplot_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::DataFrameNamed(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
        ])))
    );
}

#[test]
fn stats_structure_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

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
    let one_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nums = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let matrix_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![nums, two_i],
            names: vec![None, Some("nrow".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let medpolish_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::medpolish".to_string(),
            args: vec![matrix_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let symnum_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::symnum".to_string(),
            args: vec![matrix_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str(
            "yield ~ block + N*P*K".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![formula_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let npk = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::npk".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let repl_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::replications".to_string(),
            args: vec![formula, npk],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let id_col = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one_i, one_i, two_i, two_i],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let time_col = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one_i, two_i, one_i, two_i],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let df = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![id_col, time_col, nums],
            names: vec![
                Some("id".to_string()),
                Some("time".to_string()),
                Some("y".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let id_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("id".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let time_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("time".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let direction = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("wide".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let reshape_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::reshape".to_string(),
            args: vec![df, id_name, time_name, direction],
            names: vec![
                None,
                Some("idvar".to_string()),
                Some("timevar".to_string()),
                Some("direction".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(medpolish_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[medpolish_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[medpolish_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[medpolish_v].value_term,
        TypeTerm::NamedList(vec![
            ("overall".to_string(), TypeTerm::Double),
            (
                "row".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "col".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "residuals".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            ("name".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(out.values[repl_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[repl_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[repl_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(out.values[reshape_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[reshape_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(
        out.values[reshape_v].value_term,
        TypeTerm::DataFrame(Vec::new())
    );
    assert_eq!(out.values[symnum_v].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[symnum_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(
        out.values[symnum_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Char))
    );
}
