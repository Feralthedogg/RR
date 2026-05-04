use crate::type_precision_regression_common::*;

#[test]
pub(crate) fn graphics_and_grdevices_package_calls_have_direct_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["xs".to_string(), "ys".to_string(), "outfile".to_string()],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[1] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[2] =
        rr::compiler::internal::typeck::TypeState::scalar(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[2] = TypeTerm::Char;

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
    let outfile = fn_ir.add_value(
        ValueKind::Param { index: 2 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let topright = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "topright".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let main_title = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "signal".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let axis_side = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let int_two = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let label = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "pt".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let from_nfc = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "nfc".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let to_user = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "user".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let srgb = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "sRGB".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lab = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "Lab".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let red = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "red".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let half = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(0.5)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one_f = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(1.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let zero_f = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(0.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let plot_false = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Bool(false)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bool_true = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Bool(true)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let int_three = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(3)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let surface_vec = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one_f, zero_f, zero_f, one_f],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let surface_mat = fn_ir.add_value(
        ValueKind::Call {
            callee: "matrix".to_string(),
            args: vec![surface_vec, int_two, int_two],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let png = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::png".to_string(),
            args: vec![outfile],
            names: vec![Some("filename".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let jpeg = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::jpeg".to_string(),
            args: vec![outfile],
            names: vec![Some("filename".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bmp = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::bmp".to_string(),
            args: vec![outfile],
            names: vec![Some("filename".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tiff = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::tiff".to_string(),
            args: vec![outfile],
            names: vec![Some("filename".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let plot = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::plot".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lines = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::lines".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let points = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::points".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let abline = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::abline".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let title = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::title".to_string(),
            args: vec![main_title],
            names: vec![Some("main".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let box_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::box".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let text_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::text".to_string(),
            args: vec![xs, ys, label],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let axis = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::axis".to_string(),
            args: vec![axis_side],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let segments = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::segments".to_string(),
            args: vec![xs, ys, xs, ys],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let arrows = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::arrows".to_string(),
            args: vec![xs, ys, xs, ys],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mtext = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::mtext".to_string(),
            args: vec![main_title],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rug = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::rug".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let polygon = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::polygon".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let matplot = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::matplot".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let matlines = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::matlines".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let matpoints = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::matpoints".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pairs = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::pairs".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stripchart = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::stripchart".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dotchart = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::dotchart".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ax_ticks = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::axTicks".to_string(),
            args: vec![axis_side],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let strwidth = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::strwidth".to_string(),
            args: vec![label],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let strheight = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::strheight".to_string(),
            args: vec![label],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let grconvert_x = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::grconvertX".to_string(),
            args: vec![xs, from_nfc, to_user],
            names: vec![None, Some("from".to_string()), Some("to".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let grconvert_y = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::grconvertY".to_string(),
            args: vec![ys, from_nfc, to_user],
            names: vec![None, Some("from".to_string()), Some("to".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let clip_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::clip".to_string(),
            args: vec![zero_f, one_f, zero_f, one_f],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xspline_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::xspline".to_string(),
            args: vec![xs, ys, bool_true],
            names: vec![None, None, Some("open".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pie = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::pie".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let symbols = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::symbols".to_string(),
            args: vec![xs, ys, xs],
            names: vec![None, None, Some("circles".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let smooth_scatter = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::smoothScatter".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stem = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::stem".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let contour = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::contour".to_string(),
            args: vec![surface_mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let image = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::image".to_string(),
            args: vec![surface_mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let persp = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::persp".to_string(),
            args: vec![surface_mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let assocplot = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::assocplot".to_string(),
            args: vec![surface_mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mosaicplot = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::mosaicplot".to_string(),
            args: vec![surface_mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fourfoldplot = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::fourfoldplot".to_string(),
            args: vec![surface_mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let hist = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::hist".to_string(),
            args: vec![xs, plot_false],
            names: vec![None, Some("plot".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let boxplot = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::boxplot".to_string(),
            args: vec![xs, plot_false],
            names: vec![None, Some("plot".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let par = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::par".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let layout_vec = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![axis_side, int_two],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let layout_mat = fn_ir.add_value(
        ValueKind::Call {
            callee: "matrix".to_string(),
            args: vec![layout_vec, axis_side, int_two],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let layout = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::layout".to_string(),
            args: vec![layout_mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let layout_show = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::layout.show".to_string(),
            args: vec![axis_side],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rgb = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::rgb".to_string(),
            args: vec![one_f, zero_f, zero_f],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let gray = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::gray".to_string(),
            args: vec![half],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let adjust = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::adjustcolor".to_string(),
            args: vec![red, half],
            names: vec![None, Some("alpha.f".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let col2rgb = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::col2rgb".to_string(),
            args: vec![red],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let palette = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::palette".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let n2mfrow = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::n2mfrow".to_string(),
            args: vec![int_two],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dens_cols = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::densCols".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rgb2hsv = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::rgb2hsv".to_string(),
            args: vec![col2rgb],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let convert_color_vec = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one_f, zero_f, zero_f, zero_f, one_f, zero_f],
            names: vec![None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let convert_color_mat = fn_ir.add_value(
        ValueKind::Call {
            callee: "matrix".to_string(),
            args: vec![convert_color_vec, int_two, int_three, bool_true],
            names: vec![
                None,
                Some("nrow".to_string()),
                Some("ncol".to_string()),
                Some("byrow".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let convert_color = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::convertColor".to_string(),
            args: vec![convert_color_mat, srgb, lab],
            names: vec![None, Some("from".to_string()), Some("to".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let legend = fn_ir.add_value(
        ValueKind::Call {
            callee: "graphics::legend".to_string(),
            args: vec![topright, xs],
            names: vec![None, Some("legend".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dev_off = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::dev.off".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dev_cur = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::dev.cur".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dev_next = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::dev.next".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dev_prev = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::dev.prev".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dev_size = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::dev.size".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let palette_colors = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::palette.colors".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let palette_pals = fn_ir.add_value(
        ValueKind::Call {
            callee: "grDevices::palette.pals".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(dev_off));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [
        png,
        jpeg,
        bmp,
        tiff,
        plot,
        lines,
        points,
        abline,
        title,
        box_v,
        text_v,
        segments,
        arrows,
        mtext,
        polygon,
        matplot,
        matlines,
        matpoints,
        pairs,
        stripchart,
        dotchart,
        layout_show,
        clip_v,
        xspline_v,
        pie,
        symbols,
        smooth_scatter,
        stem,
        contour,
        image,
        assocplot,
        mosaicplot,
        fourfoldplot,
    ] {
        assert_eq!(
            out.values[vid].value_ty,
            rr::compiler::internal::typeck::TypeState::null()
        );
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }
    assert_eq!(out.values[persp].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[persp].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[persp].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );

    for vid in [
        axis,
        rug,
        ax_ticks,
        strwidth,
        strheight,
        grconvert_x,
        grconvert_y,
        dev_size,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    for vid in [hist, boxplot, par] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }
    assert_eq!(out.values[layout].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[layout].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[layout].value_term, TypeTerm::Int);
    for vid in [
        rgb,
        gray,
        adjust,
        palette,
        dens_cols,
        palette_colors,
        palette_pals,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }
    assert_eq!(out.values[n2mfrow].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[n2mfrow].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[n2mfrow].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(out.values[col2rgb].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[col2rgb].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[col2rgb].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Int))
    );
    for vid in [rgb2hsv, convert_color] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Matrix(Box::new(TypeTerm::Double))
        );
    }
    for vid in [palette_colors, palette_pals] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    assert_eq!(out.values[legend].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[legend].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[legend].value_term,
        TypeTerm::List(Box::new(TypeTerm::Any))
    );

    for vid in [dev_off, dev_cur, dev_next, dev_prev] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(out.values[vid].value_term, TypeTerm::Int);
    }
}
