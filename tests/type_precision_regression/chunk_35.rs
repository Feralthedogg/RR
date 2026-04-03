use super::type_precision_regression_common::*;

#[test]
fn stats_ts_analysis_helpers_have_direct_types() {
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
    let four = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(4.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let eight = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(8.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sixteen = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(16.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let thirty_two = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(32.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sixty_four = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(64.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one_twenty_eight = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(128.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let four_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(4)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let plot_false = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Bool(false)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ar_param = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(0.7)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let model = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::list".to_string(),
            args: vec![ar_param],
            names: vec![Some("ar".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let twelve_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(12)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let series_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![
                one,
                two,
                four,
                eight,
                sixteen,
                thirty_two,
                sixty_four,
                one_twenty_eight,
                sixty_four,
                thirty_two,
                sixteen,
                eight,
            ],
            names: vec![
                None, None, None, None, None, None, None, None, None, None, None, None,
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let x = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ts".to_string(),
            args: vec![series_vals, four_i],
            names: vec![None, Some("frequency".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ar_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ar".to_string(),
            args: vec![x],
            names: vec![None],
        },
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
    let ar_yw_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ar.yw".to_string(),
            args: vec![x, one_i],
            names: vec![None, Some("order.max".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ar_mle_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ar.mle".to_string(),
            args: vec![x, one_i],
            names: vec![None, Some("order.max".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ar_burg_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ar.burg".to_string(),
            args: vec![x, one_i],
            names: vec![None, Some("order.max".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ar_ols_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ar.ols".to_string(),
            args: vec![x, one_i],
            names: vec![None, Some("order.max".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let arima_sim_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::arima.sim".to_string(),
            args: vec![model, twelve_i],
            names: vec![Some("model".to_string()), Some("n".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let neg_two_tenths = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(-0.2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let three_tenths = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(0.3)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let six_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(6)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ar_params = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![ar_param, neg_two_tenths],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ma_params = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![three_tenths],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let arma_acf_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ARMAacf".to_string(),
            args: vec![ar_params, ma_params, six_i],
            names: vec![
                Some("ar".to_string()),
                Some("ma".to_string()),
                Some("lag.max".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let arma_to_ma_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ARMAtoMA".to_string(),
            args: vec![ar_params, ma_params, six_i],
            names: vec![
                Some("ar".to_string()),
                Some("ma".to_string()),
                Some("lag.max".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let spec_ar_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::spec.ar".to_string(),
            args: vec![x, plot_false],
            names: vec![None, Some("plot".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(ar_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [ar_v, ar_yw_v, ar_mle_v, ar_burg_v, ar_ols_v, spec_ar_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }
    for vid in [arima_sim_v, arma_acf_v, arma_to_ma_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }

    for vid in [ar_v, ar_yw_v, ar_mle_v, ar_burg_v] {
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::NamedList(vec![
                ("order".to_string(), TypeTerm::Int),
                (
                    "ar".to_string(),
                    TypeTerm::Vector(Box::new(TypeTerm::Double))
                ),
                ("var.pred".to_string(), TypeTerm::Double),
                ("x.mean".to_string(), TypeTerm::Double),
                (
                    "aic".to_string(),
                    TypeTerm::Vector(Box::new(TypeTerm::Double))
                ),
                ("n.used".to_string(), TypeTerm::Int),
                ("n.obs".to_string(), TypeTerm::Int),
                ("order.max".to_string(), TypeTerm::Double),
                (
                    "partialacf".to_string(),
                    TypeTerm::ArrayDim(Box::new(TypeTerm::Double), vec![None, Some(1), Some(1)])
                ),
                (
                    "resid".to_string(),
                    TypeTerm::Vector(Box::new(TypeTerm::Double))
                ),
                ("method".to_string(), TypeTerm::Char),
                ("series".to_string(), TypeTerm::Char),
                ("frequency".to_string(), TypeTerm::Double),
                ("call".to_string(), TypeTerm::Any),
                (
                    "asy.var.coef".to_string(),
                    TypeTerm::Matrix(Box::new(TypeTerm::Double))
                ),
            ])
        );
    }
    assert_eq!(
        out.values[ar_ols_v].value_term,
        TypeTerm::NamedList(vec![
            ("order".to_string(), TypeTerm::Int),
            (
                "ar".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("var.pred".to_string(), TypeTerm::Double),
            ("x.mean".to_string(), TypeTerm::Double),
            (
                "aic".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("n.used".to_string(), TypeTerm::Int),
            ("n.obs".to_string(), TypeTerm::Int),
            ("order.max".to_string(), TypeTerm::Double),
            (
                "partialacf".to_string(),
                TypeTerm::ArrayDim(Box::new(TypeTerm::Double), vec![None, Some(1), Some(1)])
            ),
            (
                "resid".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("method".to_string(), TypeTerm::Char),
            ("series".to_string(), TypeTerm::Char),
            ("frequency".to_string(), TypeTerm::Double),
            ("call".to_string(), TypeTerm::Any),
            ("x.intercept".to_string(), TypeTerm::Double),
            (
                "asy.se.coef".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Any))
            ),
        ])
    );
    for vid in [arima_sim_v, arma_acf_v, arma_to_ma_v] {
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    assert_eq!(
        out.values[spec_ar_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "freq".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "spec".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            ("coh".to_string(), TypeTerm::Any),
            ("phase".to_string(), TypeTerm::Any),
            ("n.used".to_string(), TypeTerm::Any),
            ("series".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("snames".to_string(), TypeTerm::Any),
        ])
    );
}

#[test]
fn stats_signal_math_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let daniell = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("daniell".to_string())),
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
    let five = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(5.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let six = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(6.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let seven = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(7.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let eight = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(8.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nine = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(9.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ten = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(10.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let eleven = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(11.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let twelve = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(12.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fifteen_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(15)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let _two_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let four_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(4)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let open = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("open".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let kernel_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::kernel".to_string(),
            args: vec![daniell, one_i],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let x = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four, five, six, seven, eight, nine, ten],
            names: vec![None, None, None, None, None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let short = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, one],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let matrix_data = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![
                one, two, three, four, five, six, seven, eight, nine, ten, eleven, twelve,
            ],
            names: vec![
                None, None, None, None, None, None, None, None, None, None, None, None,
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let m = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![matrix_data, four_i],
            names: vec![None, Some("nrow".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bandwidth_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::bandwidth.kernel".to_string(),
            args: vec![kernel_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_tskernel_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::is.tskernel".to_string(),
            args: vec![kernel_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let df_kernel_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::df.kernel".to_string(),
            args: vec![kernel_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let kernapply_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::kernapply".to_string(),
            args: vec![x, kernel_v],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let convolve_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::convolve".to_string(),
            args: vec![x, short, open],
            names: vec![None, None, Some("type".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fft_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::fft".to_string(),
            args: vec![x],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mvfft_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::mvfft".to_string(),
            args: vec![m],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nextn_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::nextn".to_string(),
            args: vec![fifteen_i],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(kernel_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[kernel_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[kernel_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[bandwidth_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[bandwidth_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[is_tskernel_v].value_ty.prim, PrimTy::Logical);
    assert_eq!(out.values[is_tskernel_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[df_kernel_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[df_kernel_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[kernapply_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[kernapply_v].value_ty.shape, ShapeTy::Vector);
    for vid in [convolve_v, fft_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }
    assert_eq!(out.values[mvfft_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[mvfft_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[nextn_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[nextn_v].value_ty.shape, ShapeTy::Scalar);

    assert_eq!(
        out.values[kernel_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "coef".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("m".to_string(), TypeTerm::Int),
        ])
    );
    assert_eq!(out.values[bandwidth_v].value_term, TypeTerm::Double);
    assert_eq!(out.values[is_tskernel_v].value_term, TypeTerm::Logical);
    assert_eq!(out.values[df_kernel_v].value_term, TypeTerm::Double);
    assert_eq!(
        out.values[kernapply_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    for vid in [convolve_v, fft_v] {
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Any))
        );
    }
    assert_eq!(
        out.values[mvfft_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Any))
    );
    assert_eq!(out.values[nextn_v].value_term, TypeTerm::Int);
}

#[test]
fn stats_density_bw_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let vals = [
        0.1, 0.3, 0.6, 1.0, 1.5, 2.1, 2.8, 3.6, 4.5, 5.5, 6.6, 7.8, 9.1, 10.5, 12.0, 13.6, 15.3,
        17.1, 19.0, 21.0,
    ]
    .into_iter()
    .map(|v| {
        fn_ir.add_value(
            ValueKind::Const(RR::syntax::ast::Lit::Float(v)),
            Span::dummy(),
            Facts::empty(),
            None,
        )
    })
    .collect::<Vec<_>>();

    let x = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vals,
            names: vec![None; 20],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let density_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::density.default".to_string(),
            args: vec![x],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bw_nrd_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::bw.nrd".to_string(),
            args: vec![x],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bw_nrd0_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::bw.nrd0".to_string(),
            args: vec![x],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bw_ucv_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::bw.ucv".to_string(),
            args: vec![x],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bw_bcv_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::bw.bcv".to_string(),
            args: vec![x],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bw_sj_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::bw.SJ".to_string(),
            args: vec![x],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(density_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[density_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[density_v].value_ty.shape, ShapeTy::Vector);
    for vid in [bw_nrd_v, bw_nrd0_v, bw_ucv_v, bw_bcv_v, bw_sj_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_term, TypeTerm::Double);
    }
    assert_eq!(
        out.values[density_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("bw".to_string(), TypeTerm::Double),
            ("n".to_string(), TypeTerm::Int),
            ("old.coords".to_string(), TypeTerm::Logical),
            ("call".to_string(), TypeTerm::Any),
            ("data.name".to_string(), TypeTerm::Char),
            ("has.na".to_string(), TypeTerm::Logical),
        ])
    );
}
