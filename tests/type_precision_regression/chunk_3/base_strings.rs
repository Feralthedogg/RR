use crate::type_precision_regression_common::*;

#[test]
pub(crate) fn base_direct_string_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let alpha = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "Alpha".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let beta = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "beta".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let gamma = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "gamma".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let three = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(3)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let four = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(4)),
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
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Bool(true)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let false_v = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Bool(false)),
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
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(0.5)),
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
