use super::type_precision_regression_common::*;

#[test]
fn grdevices_tail_package_calls_have_direct_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["xs".to_string(), "ys".to_string(), "mat".to_string()],
    );
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[1] = RR::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[2] = RR::typeck::TypeState::matrix(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[2] = TypeTerm::Matrix(Box::new(TypeTerm::Char));

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
    let mat = fn_ir.add_value(
        ValueKind::Param { index: 2 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let zero = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(0.0)),
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
    let four = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(4)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pdf_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("pdf".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let axis_ticks = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::axisTicks".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_raster = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::as.raster".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let box_stats = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::boxplot.stats".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let hull = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::chull".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let contour = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::contourLines".to_string(),
            args: vec![xs],
            names: vec![Some("z".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let caps = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::dev.capabilities".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dev_list = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::dev.list".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dev_set = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::dev.set".to_string(),
            args: vec![four],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dev_interactive = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::dev.interactive".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let device_interactive = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::deviceIsInteractive".to_string(),
            args: vec![pdf_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let extend = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::extendrange".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let hcl = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::hcl".to_string(),
            args: vec![zero, one, one],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let hcl_pals = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::hcl.pals".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let gr_soft = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::grSoftVersion".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_raster = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::is.raster".to_string(),
            args: vec![as_raster],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nclass_fd = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::nclass.FD".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let trans = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::trans3d".to_string(),
            args: vec![xs, ys, xs, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xyc = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::xy.coords".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xyt = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::xyTable".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xyz = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::xyz.coords".to_string(),
            args: vec![xs, ys, xs],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dev_new = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::dev.new".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let record_plot = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::recordPlot".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(record_plot));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    assert_eq!(out.values[axis_ticks].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[axis_ticks].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[axis_ticks].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[as_raster].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[as_raster].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(
        out.values[as_raster].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Char))
    );

    assert_eq!(out.values[box_stats].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[box_stats].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[box_stats].value_term,
        TypeTerm::NamedList(vec![
            (
                "stats".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("n".to_string(), TypeTerm::Int),
            (
                "conf".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "out".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])
    );

    for vid in [hull, dev_list] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Int))
        );
    }

    for vid in [dev_set, nclass_fd] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_term, TypeTerm::Int);
    }

    for vid in [dev_interactive, device_interactive, is_raster] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_term, TypeTerm::Logical);
    }

    {
        let vid = extend;
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }

    for vid in [hcl, hcl_pals, gr_soft] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    for vid in [contour, caps, record_plot] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }

    assert_eq!(out.values[trans].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[trans].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[trans].value_term,
        TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])
    );

    assert_eq!(
        out.values[xyc].value_term,
        TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("xlab".to_string(), TypeTerm::Any),
            ("ylab".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(
        out.values[xyt].value_term,
        TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "number".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
        ])
    );
    assert_eq!(
        out.values[xyz].value_term,
        TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "z".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("xlab".to_string(), TypeTerm::Any),
            ("ylab".to_string(), TypeTerm::Any),
            ("zlab".to_string(), TypeTerm::Any),
        ])
    );

    assert_eq!(out.values[dev_new].value_ty.prim, PrimTy::Null);
    assert_eq!(out.values[dev_new].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[dev_new].value_term, TypeTerm::Null);
}

#[test]
fn tcltk_tail_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["obj".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::Any;

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let obj = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let mut add_call = |callee: &str, args: Vec<RR::mir::ValueId>| {
        let arg_len = args.len();
        fn_ir.add_value(
            ValueKind::Call {
                callee: callee.to_string(),
                args,
                names: vec![None; arg_len],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        )
    };

    let vids = vec![
        add_call("tcltk::.Tcl", vec![obj]),
        add_call("tcltk::.Tcl.args", vec![obj]),
        add_call("tcltk::.TkRoot", vec![]),
        add_call("tcltk::tclArray", vec![]),
        add_call("tcltk::tclServiceMode", vec![]),
        add_call("tcltk::tkgrid.configure", vec![obj]),
        add_call("tcltk::tkpack.configure", vec![obj]),
        add_call("tcltk::tkplace.info", vec![obj]),
        add_call("tcltk::tkfont.create", vec![]),
        add_call("tcltk::ttkbutton", vec![]),
    ];

    fn_ir.blocks[b0].term = Terminator::Return(Some(obj));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in vids {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
}

#[test]
fn tcltk_proxy_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["obj".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::Any;

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let obj = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let mut add_call = |callee: &str, args: Vec<RR::mir::ValueId>| {
        let arg_len = args.len();
        fn_ir.add_value(
            ValueKind::Call {
                callee: callee.to_string(),
                args,
                names: vec![None; arg_len],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        )
    };

    let vids = vec![
        add_call("tcltk::tclObj.tclVar", vec![obj]),
        add_call("tcltk::tclvalue.tclObj", vec![obj]),
        add_call("tcltk::tkcget", vec![obj]),
        add_call("tcltk::tkconfigure", vec![obj]),
        add_call("tcltk::tkevent.generate", vec![obj]),
        add_call("tcltk::tkgrab.status", vec![obj]),
        add_call("tcltk::tk_messageBox", vec![obj]),
        add_call("tcltk::tkitemconfigure", vec![obj]),
        add_call("tcltk::tktag.configure", vec![obj]),
        add_call("tcltk::tkwm.geometry", vec![obj]),
        add_call("tcltk::tkxview.moveto", vec![obj]),
        add_call("tcltk::tkyview.scroll", vec![obj]),
    ];

    fn_ir.blocks[b0].term = Terminator::Return(Some(obj));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in vids {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
}
