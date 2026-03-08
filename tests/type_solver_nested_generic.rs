use RR::compiler::{OptLevel, compile_with_config};
use RR::error::RRCode;
use RR::mir::{FnIR, Terminator, ValueKind};
use RR::typeck::solver::{TypeConfig, analyze_program};
use RR::typeck::{NativeBackend, TypeMode, TypeTerm};
use RR::{mir::Facts, utils::Span};
use rustc_hash::FxHashMap;

#[test]
fn mir_index_over_list_box_preserves_box_term() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["xs".to_string()]);
    fn_ir.param_term_hints[0] =
        TypeTerm::List(Box::new(TypeTerm::Boxed(Box::new(TypeTerm::Double))));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let p = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let idx = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let elem = fn_ir.add_value(
        ValueKind::Index1D {
            base: p,
            idx,
            is_safe: false,
            is_na_safe: false,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(elem));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(
        out.values[elem].value_term,
        TypeTerm::Boxed(Box::new(TypeTerm::Double))
    );
}

#[test]
fn strict_rejects_nested_generic_call_mismatch() {
    let mut callee = FnIR::new("Sym_inner".to_string(), vec!["x".to_string()]);
    callee.param_term_hints[0] =
        TypeTerm::List(Box::new(TypeTerm::Boxed(Box::new(TypeTerm::Double))));
    let cb = callee.add_block();
    callee.entry = cb;
    callee.body_head = cb;
    let cp = callee.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    callee.blocks[cb].term = Terminator::Return(Some(cp));

    let mut caller = FnIR::new("Sym_outer".to_string(), vec!["y".to_string()]);
    caller.param_term_hints[0] = TypeTerm::List(Box::new(TypeTerm::Boxed(Box::new(TypeTerm::Int))));
    let mb = caller.add_block();
    caller.entry = mb;
    caller.body_head = mb;
    let yp = caller.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let call = caller.add_value(
        ValueKind::Call {
            callee: "Sym_inner".to_string(),
            args: vec![yp],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    caller.blocks[mb].term = Terminator::Return(Some(call));

    let mut all = FxHashMap::default();
    all.insert("Sym_inner".to_string(), callee);
    all.insert("Sym_outer".to_string(), caller);

    let err = analyze_program(
        &mut all,
        TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    )
    .expect_err("strict analysis must fail");
    assert!(matches!(err.code, RRCode::E1011));
}

#[test]
fn strict_accepts_nested_generic_call_match() {
    let src = r#"
fn inner(x: list<box<float>>) -> int {
  return length(x)

}

fn outer(y: list<box<float>>) -> int {
  return inner(y)

}
"#;

    let res = compile_with_config(
        "nested_generic_match.rr",
        src,
        OptLevel::O1,
        RR::typeck::TypeConfig {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        },
    );
    res.expect("strict compile should pass");
}
