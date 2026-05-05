use super::*;
use crate::mir::opt::types::PhaseOrderingMode;
use crate::utils::Span;

fn add_const(fn_ir: &mut FnIR, value: f64, name: Option<&str>) -> ValueId {
    fn_ir.add_value(
        ValueKind::Const(Lit::Float(value)),
        Span::default(),
        Facts::empty(),
        name.map(str::to_string),
    )
}

#[test]
fn constant_trip_loop_is_unrolled() {
    let mut fn_ir = FnIR::new("unroll_probe".to_string(), vec![]);
    let entry = fn_ir.add_block();
    let header = fn_ir.add_block();
    let body = fn_ir.add_block();
    let exit = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let zero = add_const(&mut fn_ir, 0.0, Some("acc"));
    let one = add_const(&mut fn_ir, 1.0, Some("i"));
    let load_i = fn_ir.add_value(
        ValueKind::Load {
            var: "i".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    let four = add_const(&mut fn_ir, 4.0, None);
    let cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: load_i,
            rhs: four,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let acc = fn_ir.add_value(
        ValueKind::Load {
            var: "acc".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("acc".to_string()),
    );
    let next_acc = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: acc,
            rhs: load_i,
        },
        Span::default(),
        Facts::empty(),
        Some("acc".to_string()),
    );
    let next_i = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: load_i,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    fn_ir.blocks[entry].instrs = vec![
        Instr::Assign {
            dst: "acc".to_string(),
            src: zero,
            span: Span::default(),
        },
        Instr::Assign {
            dst: "i".to_string(),
            src: one,
            span: Span::default(),
        },
    ];
    fn_ir.blocks[entry].term = Terminator::Goto(header);
    fn_ir.blocks[header].term = Terminator::If {
        cond,
        then_bb: body,
        else_bb: exit,
    };
    fn_ir.blocks[body].instrs = vec![
        Instr::Assign {
            dst: "acc".to_string(),
            src: next_acc,
            span: Span::default(),
        },
        Instr::Assign {
            dst: "i".to_string(),
            src: next_i,
            span: Span::default(),
        },
    ];
    fn_ir.blocks[body].term = Terminator::Goto(header);
    fn_ir.blocks[exit].term = Terminator::Return(Some(acc));

    let engine = TachyonEngine::with_phase_ordering_default_mode_compile_mode_and_opt_level(
        PhaseOrderingMode::Off,
        crate::compiler::CompileMode::Standard,
        crate::compiler::OptLevel::O2,
    );
    let mut stats = TachyonPulseStats::default();
    assert_eq!(optimize(&mut fn_ir, &engine, &mut stats), 1);
    assert_eq!(stats.unroll_candidates, 1);
    assert!(matches!(fn_ir.blocks[header].term, Terminator::Unreachable));
}
