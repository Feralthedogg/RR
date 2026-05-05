use super::type_precision_regression_common::*;

#[test]
pub(crate) fn stats_na_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let formula_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "mpg ~ wt + offset(hp)".to_string(),
        )),
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
    let na = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Na),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let weights_vec = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let na_input = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, na, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let matrix_input_vec = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let matrix_input = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![matrix_input_vec, two],
            names: vec![None, Some("ncol".to_string())],
        },
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
            args: vec![formula, mtcars, weights_vec],
            names: vec![None, Some("data".to_string()), Some("weights".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let model_frame = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.frame".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let weights_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::weights".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let model_weights_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.weights".to_string(),
            args: vec![model_frame],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let model_offset_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.offset".to_string(),
            args: vec![model_frame],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let offset_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::offset".to_string(),
            args: vec![weights_vec],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let na_omit_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::na.omit".to_string(),
            args: vec![weights_vec],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let na_exclude_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::na.exclude".to_string(),
            args: vec![na_input],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let na_pass_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::na.pass".to_string(),
            args: vec![weights_vec],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let na_fail_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::na.fail".to_string(),
            args: vec![weights_vec],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let na_action_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::na.action".to_string(),
            args: vec![na_exclude_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let napredict_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::napredict".to_string(),
            args: vec![na_action_v, weights_vec],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let naresid_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::naresid".to_string(),
            args: vec![na_action_v, weights_vec],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let naresid_matrix_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::naresid".to_string(),
            args: vec![na_action_v, matrix_input],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let naprint_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::naprint".to_string(),
            args: vec![na_action_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(weights_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [weights_v, model_weights_v, model_offset_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    for vid in [offset_v, na_omit_v, na_exclude_v, na_pass_v, na_fail_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    assert_eq!(out.values[na_action_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[na_action_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[na_action_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    for vid in [napredict_v, naresid_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    assert_eq!(out.values[naresid_matrix_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[naresid_matrix_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(
        out.values[naresid_matrix_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[naprint_v].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[naprint_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[naprint_v].value_term, TypeTerm::Char);
}

#[test]
pub(crate) fn stats_formula_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let formula_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "mpg ~ wt + hp".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let response_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "response".to_string(),
        )),
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
    let model_frame = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.frame".to_string(),
            args: vec![formula, mtcars],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let terms_formula_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::terms.formula".to_string(),
            args: vec![formula],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let delete_response_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::delete.response".to_string(),
            args: vec![terms_formula_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let get_all_vars_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::get_all_vars".to_string(),
            args: vec![formula, mtcars],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let model_response_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.response".to_string(),
            args: vec![model_frame],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let model_extract_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.extract".to_string(),
            args: vec![model_frame, response_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let case_names_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::case.names".to_string(),
            args: vec![model_frame],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let complete_cases_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::complete.cases".to_string(),
            args: vec![get_all_vars_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fivenum_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::fivenum".to_string(),
            args: vec![model_response_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(model_response_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    let formula_terms_term = TypeTerm::NamedList(vec![
        (
            "variables".to_string(),
            TypeTerm::List(Box::new(TypeTerm::Any)),
        ),
        (
            "factors".to_string(),
            TypeTerm::Matrix(Box::new(TypeTerm::Int)),
        ),
        (
            "term.labels".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Char)),
        ),
        (
            "order".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Int)),
        ),
        ("intercept".to_string(), TypeTerm::Int),
        ("response".to_string(), TypeTerm::Int),
        (
            "predvars".to_string(),
            TypeTerm::List(Box::new(TypeTerm::Any)),
        ),
        (
            "dataClasses".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Char)),
        ),
        (
            "class".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Char)),
        ),
        (".Environment".to_string(), TypeTerm::Any),
    ]);

    for vid in [terms_formula_v, delete_response_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_term, formula_terms_term.clone());
    }
    assert_eq!(out.values[get_all_vars_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[get_all_vars_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(
        out.values[get_all_vars_v].value_term,
        out.values[mtcars].value_term
    );
    for vid in [model_response_v, model_extract_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    assert_eq!(out.values[case_names_v].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[case_names_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[case_names_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );
    assert_eq!(out.values[complete_cases_v].value_ty.prim, PrimTy::Logical);
    assert_eq!(out.values[complete_cases_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[complete_cases_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Logical))
    );
    assert_eq!(out.values[fivenum_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[fivenum_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[fivenum_v].value_term,
        TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(5))
    );
}

#[test]
pub(crate) fn stats_matrix_formula_helpers_have_direct_types() {
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
    let x = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let y = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![three, two, one],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two_i = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let toeplitz_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::toeplitz".to_string(),
            args: vec![x],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let toeplitz2_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::toeplitz2".to_string(),
            args: vec![x, two_i],
            names: vec![None, Some("ncol".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let diffinv_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::diffinv".to_string(),
            args: vec![x],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let polym_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::polym".to_string(),
            args: vec![x, y],
            names: vec![None, None],
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
    let z_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "z".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let vars = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![x_name, z_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let onesided_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::asOneSidedFormula".to_string(),
            args: vec![vars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "y ~ x + z".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![formula_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let variable_names_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::variable.names".to_string(),
            args: vec![formula_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(toeplitz_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [toeplitz_v, toeplitz2_v, polym_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Matrix(Box::new(TypeTerm::Double))
        );
    }
    assert_eq!(out.values[diffinv_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[diffinv_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[diffinv_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[onesided_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[onesided_v].value_ty.shape, ShapeTy::Unknown);
    assert_eq!(out.values[onesided_v].value_term, TypeTerm::Any);
    assert_eq!(out.values[variable_names_v].value_ty.prim, PrimTy::Null);
    assert_eq!(out.values[variable_names_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[variable_names_v].value_term, TypeTerm::Null);
}
