use crate::type_precision_regression_common::*;

#[test]
pub(crate) fn datasets_package_table_loads_refine_known_numeric_table_shapes() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let titanic = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::Titanic".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ucb_admissions = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::UCBAdmissions".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let hair_eye_color = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::HairEyeColor".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let titanic_dim = fn_ir.add_value(
        ValueKind::Call {
            callee: "dim".to_string(),
            args: vec![titanic],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let titanic_nrow = fn_ir.add_value(
        ValueKind::Call {
            callee: "nrow".to_string(),
            args: vec![titanic],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let titanic_ncol = fn_ir.add_value(
        ValueKind::Call {
            callee: "ncol".to_string(),
            args: vec![titanic],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ucb_dim = fn_ir.add_value(
        ValueKind::Call {
            callee: "dim".to_string(),
            args: vec![ucb_admissions],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ucb_nrow = fn_ir.add_value(
        ValueKind::Call {
            callee: "nrow".to_string(),
            args: vec![ucb_admissions],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ucb_ncol = fn_ir.add_value(
        ValueKind::Call {
            callee: "ncol".to_string(),
            args: vec![ucb_admissions],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let hair_dim = fn_ir.add_value(
        ValueKind::Call {
            callee: "dim".to_string(),
            args: vec![hair_eye_color],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let hair_nrow = fn_ir.add_value(
        ValueKind::Call {
            callee: "nrow".to_string(),
            args: vec![hair_eye_color],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let hair_ncol = fn_ir.add_value(
        ValueKind::Call {
            callee: "ncol".to_string(),
            args: vec![hair_eye_color],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(titanic_nrow));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [titanic, ucb_admissions, hair_eye_color] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
    }

    assert_eq!(
        out.values[titanic].value_term,
        TypeTerm::ArrayDim(
            Box::new(TypeTerm::Double),
            vec![Some(4), Some(2), Some(2), Some(2)]
        )
    );
    assert_eq!(
        out.values[ucb_admissions].value_term,
        TypeTerm::ArrayDim(Box::new(TypeTerm::Double), vec![Some(2), Some(2), Some(6)])
    );
    assert_eq!(
        out.values[hair_eye_color].value_term,
        TypeTerm::ArrayDim(Box::new(TypeTerm::Double), vec![Some(4), Some(4), Some(2)])
    );

    for vid in [titanic_dim, ucb_dim, hair_dim] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
    }
    assert_eq!(
        out.values[titanic_dim].value_term,
        TypeTerm::VectorLen(Box::new(TypeTerm::Int), Some(4))
    );
    assert_eq!(
        out.values[ucb_dim].value_term,
        TypeTerm::VectorLen(Box::new(TypeTerm::Int), Some(3))
    );
    assert_eq!(
        out.values[hair_dim].value_term,
        TypeTerm::VectorLen(Box::new(TypeTerm::Int), Some(3))
    );

    for vid in [
        titanic_nrow,
        titanic_ncol,
        ucb_nrow,
        ucb_ncol,
        hair_nrow,
        hair_ncol,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(out.values[vid].value_term, TypeTerm::Int);
    }
}
