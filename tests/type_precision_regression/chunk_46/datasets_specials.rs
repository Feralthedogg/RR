use crate::type_precision_regression_common::*;

#[test]
pub(crate) fn datasets_package_special_loads_refine_known_special_shapes() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let ability_cov = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::ability.cov".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let harman23 = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::Harman23.cor".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let harman74 = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::Harman74.cor".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let state_center = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::state.center".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bjsales = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::BJsales".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bjsales_lead = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::BJsales.lead".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let beaver1 = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::beaver1".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let beaver2 = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::beaver2".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let euro_cross = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::euro.cross".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let randu = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::randu".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let freeny = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::freeny".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stack_x = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::stack.x".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let freeny_x = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::freeny.x".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let freeny_y = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::freeny.y".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let iris3 = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::iris3".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let seatbelts = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::Seatbelts".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let orchard_sprays = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::OrchardSprays".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let theoph = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::Theoph".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let penguins = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::penguins".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let penguins_raw = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::penguins_raw".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let gait = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::gait".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let crimtab = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::crimtab".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let occupational_status = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::occupationalStatus".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let temp_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "temp".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let species_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "species".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let year_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "year".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let treatment_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "treatment".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let conc_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "conc".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let beaver1_temp = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![beaver1, temp_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let beaver2_temp = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![beaver2, temp_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let euro_cross_rows = fn_ir.add_value(
        ValueKind::Call {
            callee: "nrow".to_string(),
            args: vec![euro_cross],
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
    let cov_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "cov".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let center_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "center".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let n_obs_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "n.obs".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ability_cov_cov = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![ability_cov, cov_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let harman23_center = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![harman23, center_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let harman74_n_obs = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![harman74, n_obs_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let state_center_x = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![state_center, x_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let randu_x = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![randu, x_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let freeny_y_df = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![freeny, y_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stack_x_cols = fn_ir.add_value(
        ValueKind::Call {
            callee: "ncol".to_string(),
            args: vec![stack_x],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let freeny_x_cols = fn_ir.add_value(
        ValueKind::Call {
            callee: "ncol".to_string(),
            args: vec![freeny_x],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let iris3_dim = fn_ir.add_value(
        ValueKind::Call {
            callee: "dim".to_string(),
            args: vec![iris3],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let seatbelts_cols = fn_ir.add_value(
        ValueKind::Call {
            callee: "ncol".to_string(),
            args: vec![seatbelts],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let orchard_treatment = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![orchard_sprays, treatment_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let theoph_conc = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![theoph, conc_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let penguin_species = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![penguins, species_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let penguin_year = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![penguins, year_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let penguins_raw_cols = fn_ir.add_value(
        ValueKind::Call {
            callee: "colnames".to_string(),
            args: vec![penguins_raw],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let gait_dim = fn_ir.add_value(
        ValueKind::Call {
            callee: "dim".to_string(),
            args: vec![gait],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let crimtab_rows = fn_ir.add_value(
        ValueKind::Call {
            callee: "nrow".to_string(),
            args: vec![crimtab],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let occupational_cols = fn_ir.add_value(
        ValueKind::Call {
            callee: "ncol".to_string(),
            args: vec![occupational_status],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(bjsales));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [ability_cov, harman23, harman74, state_center] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
    }
    assert_eq!(
        out.values[ability_cov].value_term,
        TypeTerm::NamedList(vec![
            (
                "cov".to_string(),
                TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(6), Some(6)),
            ),
            (
                "center".to_string(),
                TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(6)),
            ),
            ("n.obs".to_string(), TypeTerm::Double),
        ])
    );
    assert_eq!(
        out.values[harman23].value_term,
        TypeTerm::NamedList(vec![
            (
                "cov".to_string(),
                TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(8), Some(8)),
            ),
            (
                "center".to_string(),
                TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(8)),
            ),
            ("n.obs".to_string(), TypeTerm::Double),
        ])
    );
    assert_eq!(
        out.values[harman74].value_term,
        TypeTerm::NamedList(vec![
            (
                "cov".to_string(),
                TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(24), Some(24)),
            ),
            (
                "center".to_string(),
                TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(24)),
            ),
            ("n.obs".to_string(), TypeTerm::Double),
        ])
    );
    assert_eq!(
        out.values[state_center].value_term,
        TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(50)),
            ),
            (
                "y".to_string(),
                TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(50)),
            ),
        ])
    );
    assert_eq!(
        out.values[ability_cov_cov].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(6), Some(6))
    );
    assert_eq!(
        out.values[harman23_center].value_term,
        TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(8))
    );
    assert_eq!(out.values[harman74_n_obs].value_term, TypeTerm::Double);
    assert_eq!(
        out.values[state_center_x].value_term,
        TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(50))
    );

    for vid in [bjsales, bjsales_lead, freeny_y] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }

    for vid in [randu, freeny] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
    }
    assert_eq!(
        out.values[randu].value_term,
        TypeTerm::DataFrameNamed(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "z".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(
        out.values[freeny].value_term,
        TypeTerm::DataFrameNamed(vec![
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "lag.quarterly.revenue".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "price.index".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "income.level".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "market.potential".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])
    );
    assert_eq!(
        out.values[randu_x].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[freeny_y_df].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    for vid in [beaver1, beaver2, stack_x] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
    }
    for vid in [seatbelts, orchard_sprays, theoph] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
    }
    for vid in [penguins, penguins_raw, gait, crimtab, occupational_status] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
    }
    assert_eq!(
        out.values[beaver1_temp].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[beaver2_temp].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    for vid in [euro_cross, freeny_x] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
    }
    assert_eq!(
        out.values[euro_cross].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(11), Some(11))
    );
    assert_eq!(
        out.values[freeny_x].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(39), Some(4))
    );
    assert_eq!(out.values[euro_cross_rows].value_term, TypeTerm::Int);
    assert_eq!(
        out.values[stack_x].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(21), Some(3))
    );
    assert_eq!(out.values[stack_x_cols].value_term, TypeTerm::Int);
    assert_eq!(out.values[freeny_x_cols].value_term, TypeTerm::Int);

    assert_eq!(out.values[iris3].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[iris3].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[iris3].value_term,
        TypeTerm::ArrayDim(Box::new(TypeTerm::Double), vec![Some(50), Some(4), Some(3)])
    );
    assert_eq!(
        out.values[iris3_dim].value_term,
        TypeTerm::VectorLen(Box::new(TypeTerm::Int), Some(3))
    );
    assert_eq!(
        out.values[seatbelts].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(192), Some(8))
    );
    assert_eq!(out.values[seatbelts_cols].value_term, TypeTerm::Int);
    assert_eq!(
        out.values[orchard_sprays].value_term,
        TypeTerm::DataFrameNamed(vec![
            (
                "decrease".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "rowpos".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "colpos".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "treatment".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])
    );
    assert_eq!(
        out.values[theoph].value_term,
        TypeTerm::DataFrameNamed(vec![
            (
                "Subject".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Wt".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "Dose".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Time".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "conc".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])
    );
    assert_eq!(
        out.values[orchard_treatment].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );
    assert_eq!(
        out.values[theoph_conc].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[penguins].value_term,
        TypeTerm::DataFrameNamed(vec![
            (
                "species".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "island".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "bill_len".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "bill_dep".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "flipper_len".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "body_mass".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "sex".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
            (
                "year".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
        ])
    );
    assert_eq!(
        out.values[penguin_species].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );
    assert_eq!(
        out.values[penguin_year].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(
        out.values[penguins_raw].value_term,
        TypeTerm::DataFrameNamed(vec![
            (
                "studyName".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Sample Number".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Species".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Region".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Island".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Stage".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Individual ID".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Clutch Completion".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Date Egg".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Culmen Length (mm)".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Culmen Depth (mm)".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Flipper Length (mm)".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Body Mass (g)".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Sex".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
            (
                "Delta 15 N (o/oo)".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Delta 13 C (o/oo)".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Comments".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])
    );
    assert_eq!(
        out.values[penguins_raw_cols].value_term,
        TypeTerm::VectorLen(Box::new(TypeTerm::Char), Some(17))
    );
    assert_eq!(
        out.values[gait].value_term,
        TypeTerm::ArrayDim(
            Box::new(TypeTerm::Double),
            vec![Some(20), Some(39), Some(2)]
        )
    );
    assert_eq!(
        out.values[gait_dim].value_term,
        TypeTerm::VectorLen(Box::new(TypeTerm::Int), Some(3))
    );
    assert_eq!(
        out.values[crimtab].value_term,
        TypeTerm::ArrayDim(Box::new(TypeTerm::Int), vec![Some(42), Some(22)])
    );
    assert_eq!(out.values[crimtab_rows].value_term, TypeTerm::Int);
    assert_eq!(
        out.values[occupational_status].value_term,
        TypeTerm::ArrayDim(Box::new(TypeTerm::Int), vec![Some(8), Some(8)])
    );
    assert_eq!(out.values[occupational_cols].value_term, TypeTerm::Int);
}
