use crate::type_precision_regression_common::*;

#[test]
pub(crate) fn stats_model_frame_from_updated_model_preserves_named_dataframe_schema() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

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
    let six = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(6.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one, two, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ys = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![two, four, six],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let df = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![xs, ys],
            names: vec![Some("x".to_string()), Some("y".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "y ~ x".to_string(),
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
    let model = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lm".to_string(),
            args: vec![formula, df],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let updated = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::update".to_string(),
            args: vec![model, formula],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let frame = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.frame".to_string(),
            args: vec![updated],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let x_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "x".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let y_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "y".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let frame_x = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![frame, x_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let frame_y = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![frame, y_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(frame_y));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(
        out.values[frame].value_term,
        TypeTerm::DataFrameNamed(vec![
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
    assert_eq!(
        out.values[frame_x].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[frame_y].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
}

#[test]
pub(crate) fn stats_model_matrix_preserves_visible_row_count() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

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
    let six = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(6.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let zero = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(0.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xs3 = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one, two, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ys3 = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![two, four, six],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let zs3 = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![three, two, one],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let df3 = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![xs3, zs3, ys3],
            names: vec![
                Some("x".to_string()),
                Some("z".to_string()),
                Some("y".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xs4 = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ys4 = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![zero, zero, one, one],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let df4 = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![xs4, ys4],
            names: vec![Some("x".to_string()), Some("y".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "y ~ x".to_string(),
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
    let formula_add_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "y ~ x + z".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![formula_add_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add_no_intercept_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "y ~ 0 + x + z".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add_no_intercept = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![formula_add_no_intercept_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add_zero_mid_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "y ~ x + 0 + z".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add_zero_mid = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![formula_add_zero_mid_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add_minus_one_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "y ~ -1 + x + z".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add_minus_one = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![formula_add_minus_one_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add_minus_one_suffix_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "y ~ x + z - 1".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add_minus_one_suffix = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![formula_add_minus_one_suffix_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add_minus_zero_suffix_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "y ~ x + z - 0".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add_minus_zero_suffix = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![formula_add_minus_zero_suffix_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add_negzero_prefix_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "y ~ -0 + x + z".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add_negzero_prefix = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![formula_add_negzero_prefix_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add_restore_intercept_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "y ~ -1 + 1 + x + z".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add_restore_intercept = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![formula_add_restore_intercept_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add_remove_then_restore_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "y ~ x + z - 1 + 1".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_add_remove_then_restore = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![formula_add_remove_then_restore_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let explicit_matrix = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.matrix".to_string(),
            args: vec![formula, df3],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let explicit_add_matrix = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.matrix".to_string(),
            args: vec![formula_add, df3],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let explicit_add_no_intercept_matrix = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.matrix".to_string(),
            args: vec![formula_add_no_intercept, df3],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let explicit_add_zero_mid_matrix = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.matrix".to_string(),
            args: vec![formula_add_zero_mid, df3],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let explicit_add_minus_one_matrix = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.matrix".to_string(),
            args: vec![formula_add_minus_one, df3],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let explicit_add_minus_one_suffix_matrix = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.matrix".to_string(),
            args: vec![formula_add_minus_one_suffix, df3],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let explicit_add_minus_zero_suffix_matrix = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.matrix".to_string(),
            args: vec![formula_add_minus_zero_suffix, df3],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let explicit_add_negzero_prefix_matrix = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.matrix".to_string(),
            args: vec![formula_add_negzero_prefix, df3],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let explicit_add_restore_intercept_matrix = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.matrix".to_string(),
            args: vec![formula_add_restore_intercept, df3],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let explicit_add_remove_then_restore_matrix = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.matrix".to_string(),
            args: vec![formula_add_remove_then_restore, df3],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lm_model = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lm".to_string(),
            args: vec![formula, df3],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lm_matrix = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.matrix".to_string(),
            args: vec![lm_model],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let updated_model = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::update".to_string(),
            args: vec![lm_model, formula],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let updated_matrix = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.matrix".to_string(),
            args: vec![updated_model],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let family = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::binomial".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let glm_model = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::glm".to_string(),
            args: vec![formula, df4, family],
            names: vec![None, Some("data".to_string()), Some("family".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let glm_matrix = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.matrix".to_string(),
            args: vec![glm_model],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(explicit_matrix));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(
        out.values[explicit_matrix].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(3), Some(2))
    );
    assert_eq!(
        out.values[explicit_add_matrix].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(3), Some(3))
    );
    assert_eq!(
        out.values[explicit_add_no_intercept_matrix].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(3), Some(2))
    );
    assert_eq!(
        out.values[explicit_add_zero_mid_matrix].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(3), Some(2))
    );
    assert_eq!(
        out.values[explicit_add_minus_one_matrix].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(3), Some(2))
    );
    assert_eq!(
        out.values[explicit_add_minus_one_suffix_matrix].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(3), Some(2))
    );
    assert_eq!(
        out.values[explicit_add_minus_zero_suffix_matrix].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(3), Some(3))
    );
    assert_eq!(
        out.values[explicit_add_negzero_prefix_matrix].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(3), Some(3))
    );
    assert_eq!(
        out.values[explicit_add_restore_intercept_matrix].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(3), Some(3))
    );
    assert_eq!(
        out.values[explicit_add_remove_then_restore_matrix].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(3), Some(3))
    );
    assert_eq!(
        out.values[lm_matrix].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(3), Some(2))
    );
    assert_eq!(
        out.values[updated_matrix].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(3), Some(2))
    );
    assert_eq!(
        out.values[glm_matrix].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(4), Some(2))
    );
}
