use super::*;
use crate::mir::flow::Facts;
use crate::mir::opt::loop_analysis::LoopAnalyzer;
use crate::utils::Span;

fn build_map_loop() -> (FnIR, LoopInfo, ScopRegion) {
    let mut fn_ir = FnIR::new(
        "poly_map".to_string(),
        vec!["x".to_string(), "y".to_string()],
    );
    let entry = fn_ir.add_block();
    let header = fn_ir.add_block();
    let body = fn_ir.add_block();
    let exit = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let x = fn_ir.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let y = fn_ir.add_value(
        ValueKind::Load {
            var: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("y".to_string()),
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let len = fn_ir.add_value(
        ValueKind::Len { base: y },
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = fn_ir.add_value(
        ValueKind::Phi { args: Vec::new() },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    fn_ir.values[phi].phi_block = Some(header);
    let cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: phi,
            rhs: len,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let read = fn_ir.add_value(
        ValueKind::Index1D {
            base: x,
            idx: phi,
            is_safe: true,
            is_na_safe: true,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let rhs = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: read,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[body]
        .instrs
        .push(crate::mir::Instr::StoreIndex1D {
            base: y,
            idx: phi,
            val: rhs,
            is_safe: true,
            is_na_safe: true,
            is_vector: false,
            span: Span::default(),
        });
    let next = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: phi,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.values[phi].kind = ValueKind::Phi {
        args: vec![(one, entry), (next, body)],
    };
    fn_ir.blocks[entry].term = crate::mir::Terminator::Goto(header);
    fn_ir.blocks[header].term = crate::mir::Terminator::If {
        cond,
        then_bb: body,
        else_bb: exit,
    };
    fn_ir.blocks[body].term = crate::mir::Terminator::Goto(header);
    fn_ir.blocks[exit].term = crate::mir::Terminator::Return(Some(y));

    let loops = LoopAnalyzer::new(&fn_ir).find_loops();
    let lp = loops[0].clone();
    let scop = crate::mir::opt::poly::scop::extract_scop_region(&fn_ir, &lp, &loops)
        .expect("expected scop");
    (fn_ir, lp, scop)
}

#[test]
fn identity_schedule_builds_map_plan() {
    let (fn_ir, lp, scop) = build_map_loop();
    let plan = build_identity_plan(&fn_ir, &lp, &scop).expect("expected plan");
    assert!(matches!(plan, VectorPlan::Map { .. }));
}
