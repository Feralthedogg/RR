use super::*;
use crate::utils::Span;

#[test]
fn outline_uses_deterministic_internal_name() {
    let mut all_fns = FxHashMap::default();
    let fn_ir = large_linear_function("main_fun", 400);
    all_fns.insert(fn_ir.name.clone(), fn_ir);

    let engine = TachyonEngine::with_phase_ordering_default_mode_compile_mode_and_opt_level(
        crate::mir::opt::types::PhaseOrderingMode::Off,
        crate::compiler::CompileMode::Standard,
        crate::compiler::OptLevel::O3,
    );
    let mut stats = TachyonPulseStats::default();
    let applied = optimize_program(&mut all_fns, &engine, &mut stats);

    assert_eq!(applied, 1);
    assert!(all_fns.contains_key("__rr_outline_main_fun_0"));
}

#[test]
fn outline_preserves_multi_live_out_with_record_return() {
    let mut all_fns = FxHashMap::default();
    let fn_ir = large_linear_function("pair", 400);
    all_fns.insert(fn_ir.name.clone(), fn_ir);

    let engine = TachyonEngine::with_phase_ordering_default_mode_compile_mode_and_opt_level(
        crate::mir::opt::types::PhaseOrderingMode::Off,
        crate::compiler::CompileMode::Standard,
        crate::compiler::OptLevel::O3,
    );
    let mut stats = TachyonPulseStats::default();
    let applied = optimize_program(&mut all_fns, &engine, &mut stats);

    assert_eq!(applied, 1);
    let helper = all_fns.get("__rr_outline_pair_0").unwrap();
    let ret = match helper.blocks[helper.body_head].term {
        Terminator::Return(Some(ret)) => ret,
        _ => panic!("outline helper should return a value"),
    };
    assert!(matches!(
        helper.values[ret].kind,
        ValueKind::RecordLit { .. }
    ));
}

#[test]
fn outline_preserves_live_out_used_by_successor_block() {
    let mut all_fns = FxHashMap::default();
    let fn_ir = large_successor_live_out_function("successor_live_out", 400);
    all_fns.insert(fn_ir.name.clone(), fn_ir);

    let engine = TachyonEngine::with_phase_ordering_default_mode_compile_mode_and_opt_level(
        crate::mir::opt::types::PhaseOrderingMode::Off,
        crate::compiler::CompileMode::Standard,
        crate::compiler::OptLevel::O3,
    );
    let mut stats = TachyonPulseStats::default();
    let applied = optimize_program(&mut all_fns, &engine, &mut stats);

    assert_eq!(applied, 1);
    let helper = &all_fns["__rr_outline_successor_live_out_0"];
    let ret = match helper.blocks[helper.body_head].term {
        Terminator::Return(Some(ret)) => ret,
        _ => panic!("outline helper should return a value"),
    };
    assert!(matches!(
        &helper.values[ret].kind,
        ValueKind::Load { var } if var == "tmp1"
    ));
}

fn large_linear_function(name: &str, repeated: usize) -> FnIR {
    let mut fn_ir = FnIR::new(name.to_string(), vec!["x".to_string()]);
    let entry = fn_ir.add_block();
    let body = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = body;
    fn_ir.blocks[entry].term = Terminator::Goto(body);

    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Float(1.0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    for idx in 0..repeated {
        let x = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let add = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: x,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[body].instrs.push(Instr::Assign {
            dst: format!("tmp{idx}"),
            src: add,
            span: Span::default(),
        });
    }
    let tmp1 = fn_ir.add_value(
        ValueKind::Load {
            var: "tmp1".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("tmp1".to_string()),
    );
    let tmp2 = fn_ir.add_value(
        ValueKind::Load {
            var: "tmp2".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("tmp2".to_string()),
    );
    let sum = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tmp1,
            rhs: tmp2,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[body].instrs.push(Instr::Assign {
        dst: "out".to_string(),
        src: sum,
        span: Span::default(),
    });
    let out = fn_ir.add_value(
        ValueKind::Load {
            var: "out".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("out".to_string()),
    );
    fn_ir.blocks[body].term = Terminator::Return(Some(out));
    fn_ir
}

fn large_successor_live_out_function(name: &str, repeated: usize) -> FnIR {
    let mut fn_ir = FnIR::new(name.to_string(), vec!["x".to_string()]);
    let entry = fn_ir.add_block();
    let body = fn_ir.add_block();
    let tail = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = body;
    fn_ir.blocks[entry].term = Terminator::Goto(body);

    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Float(1.0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    for idx in 0..repeated {
        let x = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let add = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: x,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[body].instrs.push(Instr::Assign {
            dst: format!("tmp{idx}"),
            src: add,
            span: Span::default(),
        });
    }
    fn_ir.blocks[body].term = Terminator::Goto(tail);

    let tmp1 = fn_ir.add_value(
        ValueKind::Load {
            var: "tmp1".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("tmp1".to_string()),
    );
    fn_ir.blocks[tail].term = Terminator::Return(Some(tmp1));
    fn_ir
}
