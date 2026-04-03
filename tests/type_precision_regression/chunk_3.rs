use super::type_precision_regression_common::*;

#[test]
fn base_direct_matrix_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let one = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
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
    let four = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(4)),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let seq_len_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::seq_len".to_string(),
            args: vec![four],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let seq_along_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::seq_along".to_string(),
            args: vec![seq_len_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let matrix_data = fn_ir.add_value(
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
            args: vec![matrix_data, two, two],
            names: vec![None, Some("nrow".to_string()), Some("ncol".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let t_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::t".to_string(),
            args: vec![matrix_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let diag_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::diag".to_string(),
            args: vec![matrix_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rbind_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::rbind".to_string(),
            args: vec![matrix_v, matrix_v],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cbind_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::cbind".to_string(),
            args: vec![matrix_v, matrix_v],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let row_sums_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::rowSums".to_string(),
            args: vec![matrix_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let col_sums_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::colSums".to_string(),
            args: vec![matrix_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let crossprod_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::crossprod".to_string(),
            args: vec![matrix_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tcrossprod_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::tcrossprod".to_string(),
            args: vec![matrix_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(tcrossprod_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[seq_len_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[seq_len_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[seq_along_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[seq_along_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[matrix_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[t_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[diag_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[row_sums_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[col_sums_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[crossprod_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[tcrossprod_v].value_ty.shape, ShapeTy::Matrix);

    assert_eq!(
        out.values[seq_len_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(
        out.values[seq_along_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(
        out.values[matrix_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[t_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[diag_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[rbind_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[cbind_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[row_sums_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[col_sums_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[crossprod_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[tcrossprod_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
}

#[test]
fn base_direct_numeric_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let neg_one = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(-1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
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
    let four = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(4)),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let pair = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![neg_one, two],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nums = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![three, one, two],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let abs_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::abs".to_string(),
            args: vec![pair],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let min_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::min".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let max_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::max".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let left = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, four],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let right = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![two, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pmax_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::pmax".to_string(),
            args: vec![left, right],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pmin_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::pmin".to_string(),
            args: vec![left, right],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sqrt_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sqrt".to_string(),
            args: vec![left],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let log_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::log".to_string(),
            args: vec![one],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let log10_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::log10".to_string(),
            args: vec![four],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let log2_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::log2".to_string(),
            args: vec![four],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let exp_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::exp".to_string(),
            args: vec![one],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let atan2_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::atan2".to_string(),
            args: vec![one, one],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sin_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sin".to_string(),
            args: vec![one],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cos_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::cos".to_string(),
            args: vec![one],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tan_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::tan".to_string(),
            args: vec![one],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let asin_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::asin".to_string(),
            args: vec![one],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let acos_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::acos".to_string(),
            args: vec![one],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let atan_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::atan".to_string(),
            args: vec![one],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sinh_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sinh".to_string(),
            args: vec![one],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cosh_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::cosh".to_string(),
            args: vec![one],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tanh_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::tanh".to_string(),
            args: vec![one],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sign_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sign".to_string(),
            args: vec![left],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let gamma_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::gamma".to_string(),
            args: vec![one],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lgamma_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::lgamma".to_string(),
            args: vec![one],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let floor_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::floor".to_string(),
            args: vec![left],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ceiling_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::ceiling".to_string(),
            args: vec![left],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let trunc_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::trunc".to_string(),
            args: vec![left],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let round_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::round".to_string(),
            args: vec![left],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_na_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::is.na".to_string(),
            args: vec![left],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_finite_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::is.finite".to_string(),
            args: vec![left],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rep_int_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::rep.int".to_string(),
            args: vec![two, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let print_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::print".to_string(),
            args: vec![left],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(round_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [abs_v, pmax_v, pmin_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Int))
        );
    }
    for vid in [sqrt_v, floor_v, ceiling_v, trunc_v, round_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    for vid in [min_v, max_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(out.values[vid].value_term, TypeTerm::Int);
    }
    for vid in [
        log_v, log10_v, log2_v, exp_v, atan2_v, sin_v, cos_v, tan_v, asin_v, acos_v, atan_v,
        sinh_v, cosh_v, tanh_v, gamma_v, lgamma_v,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_term, TypeTerm::Double);
    }
    assert_eq!(out.values[sign_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[sign_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[sign_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(out.values[rep_int_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[rep_int_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[rep_int_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    for vid in [is_na_v, is_finite_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Logical))
        );
    }
    assert_eq!(out.values[print_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[print_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[print_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
}

#[test]
fn base_direct_string_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let alpha = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("Alpha".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let beta = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("beta".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let gamma = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("gamma".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
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
    let four = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(4)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let txt = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![alpha, beta, gamma],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nums = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![three, one, two],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lower_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::tolower".to_string(),
            args: vec![txt],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let upper_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::toupper".to_string(),
            args: vec![txt],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nchar_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::nchar".to_string(),
            args: vec![txt],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nzchar_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::nzchar".to_string(),
            args: vec![txt],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let substr_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::substr".to_string(),
            args: vec![txt, one, two],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sub_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sub".to_string(),
            args: vec![alpha, beta, txt],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let gsub_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::gsub".to_string(),
            args: vec![alpha, beta, txt],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let grepl_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::grepl".to_string(),
            args: vec![alpha, txt],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let grep_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::grep".to_string(),
            args: vec![alpha, txt],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let starts_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::startsWith".to_string(),
            args: vec![txt, alpha],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ends_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::endsWith".to_string(),
            args: vec![txt, alpha],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let trimws_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::trimws".to_string(),
            args: vec![txt],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let chartr_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::chartr".to_string(),
            args: vec![alpha, beta, txt],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let strsplit_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::strsplit".to_string(),
            args: vec![txt, alpha],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let regexpr_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::regexpr".to_string(),
            args: vec![alpha, txt],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let gregexpr_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::gregexpr".to_string(),
            args: vec![alpha, txt],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let regexec_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::regexec".to_string(),
            args: vec![alpha, txt],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let agrep_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::agrep".to_string(),
            args: vec![alpha, txt],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let agrepl_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::agrepl".to_string(),
            args: vec![alpha, txt],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let which_min = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::which.min".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let which_max = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::which.max".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let true_v = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Bool(true)),
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
    let is_true = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::isTRUE".to_string(),
            args: vec![true_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_false = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::isFALSE".to_string(),
            args: vec![false_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one_list = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two_list = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let three_list = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lst = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::list".to_string(),
            args: vec![one_list, two_list, three_list],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lengths_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::lengths".to_string(),
            args: vec![lst],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let others = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![two, three, four],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let union_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::union".to_string(),
            args: vec![nums, others],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let intersect_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::intersect".to_string(),
            args: vec![nums, others],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let setdiff_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::setdiff".to_string(),
            args: vec![nums, others],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sample_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sample".to_string(),
            args: vec![nums, two],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sample_n_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sample".to_string(),
            args: vec![four, two],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sample_int_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sample.int".to_string(),
            args: vec![four, two],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let seq_i_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::seq".to_string(),
            args: vec![one, four],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let half = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(0.5)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let seq_d_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::seq".to_string(),
            args: vec![one, three, half],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ifelse_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::ifelse".to_string(),
            args: vec![nzchar_v, nums, others],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ifelse_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::ifelse".to_string(),
            args: vec![true_v, one, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rank_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::rank".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(which_max));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [
        lower_v, upper_v, substr_v, sub_v, gsub_v, trimws_v, chartr_v,
    ] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }
    for vid in [nchar_v, grep_v, regexpr_v, agrep_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Int))
        );
    }
    for vid in [nzchar_v, grepl_v, starts_v, ends_v, agrepl_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Logical))
        );
    }
    for vid in [gregexpr_v, regexec_v, strsplit_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }
    assert_eq!(
        out.values[gregexpr_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::Vector(Box::new(TypeTerm::Int))))
    );
    assert_eq!(
        out.values[regexec_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::Vector(Box::new(TypeTerm::Int))))
    );
    assert_eq!(
        out.values[strsplit_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::Vector(Box::new(TypeTerm::Char))))
    );
    for vid in [which_min, which_max] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_term, TypeTerm::Int);
    }
    for vid in [is_true, is_false] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_term, TypeTerm::Logical);
    }
    for vid in [
        lengths_v,
        union_v,
        intersect_v,
        setdiff_v,
        sample_v,
        sample_n_v,
        sample_int_v,
        seq_i_v,
        seq_d_v,
        ifelse_v,
        rank_v,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }
    assert_eq!(out.values[lengths_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[lengths_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    for vid in [union_v, intersect_v, setdiff_v, sample_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Int))
        );
    }
    for vid in [sample_n_v, sample_int_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Int))
        );
    }
    assert_eq!(out.values[seq_i_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[seq_d_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[seq_i_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(
        out.values[seq_d_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[ifelse_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[ifelse_s].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[ifelse_s].value_term, TypeTerm::Int);
    assert_eq!(out.values[ifelse_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[ifelse_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(out.values[rank_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[rank_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
}
