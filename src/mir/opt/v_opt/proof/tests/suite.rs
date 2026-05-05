use super::*;
use crate::mir::opt::loop_analysis::LoopInfo;
use crate::mir::{BinOp, Facts};
use crate::utils::Span;

pub(crate) fn dummy_loop() -> LoopInfo {
    LoopInfo {
        header: 0,
        latch: 0,
        exits: Vec::new(),
        body: FxHashSet::default(),
        is_seq_len: None,
        is_seq_along: None,
        iv: None,
        limit: None,
        limit_adjust: 0,
    }
}

#[test]
pub(crate) fn disabled_config_falls_back_with_disabled_reason() {
    let fn_ir = FnIR::new("proof_dummy".to_string(), vec![]);
    let loop_info = dummy_loop();
    let outcome = analyze_loop_with_config(
        &fn_ir,
        &loop_info,
        &FxHashSet::default(),
        ProofConfig { enabled: false },
    );
    assert!(matches!(
        outcome,
        ProofOutcome::FallbackToPattern {
            reason: ProofFallbackReason::Disabled
        }
    ));
}

#[test]
pub(crate) fn enabled_config_falls_back_with_missing_induction_var_reason() {
    let fn_ir = FnIR::new("proof_dummy".to_string(), vec![]);
    let loop_info = dummy_loop();
    let outcome = analyze_loop_with_config(
        &fn_ir,
        &loop_info,
        &FxHashSet::default(),
        ProofConfig { enabled: true },
    );
    assert!(matches!(
        outcome,
        ProofOutcome::NotApplicable {
            reason: ProofFallbackReason::StorelessPlainLoop
        }
    ));
}

pub(crate) fn simple_map_fn() -> FnIR {
    base_single_store_loop_fn("proof_map", |fn_ir, load_x, load_i, one| {
        let read_x = fn_ir.add_value(
            ValueKind::Index1D {
                base: load_x,
                idx: load_i,
                is_safe: true,
                is_na_safe: true,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: read_x,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        )
    })
}

pub(crate) fn simple_expr_map_fn() -> FnIR {
    base_single_store_loop_fn("proof_expr_map", |fn_ir, load_x, load_i, one| {
        let two = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let read_x = fn_ir.add_value(
            ValueKind::Index1D {
                base: load_x,
                idx: load_i,
                is_safe: true,
                is_na_safe: true,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let plus = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: read_x,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Mul,
                lhs: plus,
                rhs: two,
            },
            Span::default(),
            Facts::empty(),
            None,
        )
    })
}

pub(crate) fn simple_expr_map_with_eval_side_effect_fn() -> FnIR {
    let mut fn_ir = simple_expr_map_fn();
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let body = 2usize;
    fn_ir.blocks[body].instrs.insert(
        0,
        Instr::Eval {
            val: impure,
            span: Span::default(),
        },
    );
    fn_ir
}

pub(crate) fn simple_call_map_fn() -> FnIR {
    base_single_store_loop_fn("proof_call_map", |fn_ir, load_x, load_i, _one| {
        let read_x = fn_ir.add_value(
            ValueKind::Index1D {
                base: load_x,
                idx: load_i,
                is_safe: true,
                is_na_safe: true,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.add_value(
            ValueKind::Call {
                callee: "abs".to_string(),
                args: vec![read_x],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        )
    })
}

pub(crate) fn simple_scatter_fn() -> FnIR {
    let mut fn_ir = FnIR::new("proof_scatter".to_string(), vec!["x".to_string()]);
    let entry = fn_ir.add_block();
    let header = fn_ir.add_block();
    let body = fn_ir.add_block();
    let exit = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let param_x = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let one = fn_ir.add_value(
        ValueKind::Const(crate::mir::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let load_x = fn_ir.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let load_i = fn_ir.add_value(
        ValueKind::Load {
            var: "i".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    let len_x = fn_ir.add_value(
        ValueKind::Len { base: load_x },
        Span::default(),
        Facts::empty(),
        None,
    );
    let cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: load_i,
            rhs: len_x,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let idx = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: load_i,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let next_i = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: load_i,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: param_x,
        span: Span::default(),
    });
    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "i".to_string(),
        src: one,
        span: Span::default(),
    });
    fn_ir.blocks[entry].term = Terminator::Goto(header);

    fn_ir.blocks[header].term = Terminator::If {
        cond,
        then_bb: body,
        else_bb: exit,
    };

    fn_ir.blocks[body].instrs.push(Instr::StoreIndex1D {
        base: load_x,
        idx,
        val: load_i,
        is_safe: true,
        is_na_safe: true,
        is_vector: false,
        span: Span::default(),
    });
    fn_ir.blocks[body].instrs.push(Instr::Assign {
        dst: "i".to_string(),
        src: next_i,
        span: Span::default(),
    });
    fn_ir.blocks[body].term = Terminator::Goto(header);
    fn_ir.blocks[exit].term = Terminator::Return(Some(load_x));

    fn_ir
}

pub(crate) fn simple_scatter_with_eval_side_effect_fn() -> FnIR {
    let mut fn_ir = simple_scatter_fn();
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let body = 2usize;
    fn_ir.blocks[body].instrs.insert(
        0,
        Instr::Eval {
            val: impure,
            span: Span::default(),
        },
    );
    fn_ir
}

pub(crate) fn simple_shifted_map_fn() -> FnIR {
    let mut fn_ir = FnIR::new("proof_shifted_map".to_string(), vec!["x".to_string()]);
    let entry = fn_ir.add_block();
    let header = fn_ir.add_block();
    let body = fn_ir.add_block();
    let exit = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let param_x = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let one = fn_ir.add_value(
        ValueKind::Const(crate::mir::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    let load_x = fn_ir.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let load_i = fn_ir.add_value(
        ValueKind::Load {
            var: "i".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    let len_x = fn_ir.add_value(
        ValueKind::Len { base: load_x },
        Span::default(),
        Facts::empty(),
        None,
    );
    let loop_end = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Sub,
            lhs: len_x,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: load_i,
            rhs: loop_end,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let rhs_idx = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: load_i,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let rhs = fn_ir.add_value(
        ValueKind::Index1D {
            base: load_x,
            idx: rhs_idx,
            is_safe: true,
            is_na_safe: true,
        },
        Span::default(),
        Facts::empty(),
        None,
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

    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: param_x,
        span: Span::default(),
    });
    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "i".to_string(),
        src: one,
        span: Span::default(),
    });
    fn_ir.blocks[entry].term = Terminator::Goto(header);
    fn_ir.blocks[header].term = Terminator::If {
        cond,
        then_bb: body,
        else_bb: exit,
    };
    fn_ir.blocks[body].instrs.push(Instr::StoreIndex1D {
        base: load_x,
        idx: load_i,
        val: rhs,
        is_safe: true,
        is_na_safe: true,
        is_vector: false,
        span: Span::default(),
    });
    fn_ir.blocks[body].instrs.push(Instr::Assign {
        dst: "i".to_string(),
        src: next_i,
        span: Span::default(),
    });
    fn_ir.blocks[body].term = Terminator::Goto(header);
    fn_ir.blocks[exit].term = Terminator::Return(Some(load_x));

    fn_ir
}

pub(crate) fn simple_multi_expr_map_fn() -> FnIR {
    let mut fn_ir = FnIR::new(
        "proof_multi_expr_map".to_string(),
        vec!["x".to_string(), "y".to_string()],
    );
    let entry = fn_ir.add_block();
    let header = fn_ir.add_block();
    let body = fn_ir.add_block();
    let exit = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let param_x = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let param_y = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::default(),
        Facts::empty(),
        Some("y".to_string()),
    );
    let one = fn_ir.add_value(
        ValueKind::Const(crate::mir::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    let load_x = fn_ir.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let load_y = fn_ir.add_value(
        ValueKind::Load {
            var: "y".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("y".to_string()),
    );
    let load_i = fn_ir.add_value(
        ValueKind::Load {
            var: "i".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    let len_x = fn_ir.add_value(
        ValueKind::Len { base: load_x },
        Span::default(),
        Facts::empty(),
        None,
    );
    let cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: load_i,
            rhs: len_x,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let read_x = fn_ir.add_value(
        ValueKind::Index1D {
            base: load_x,
            idx: load_i,
            is_safe: true,
            is_na_safe: true,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let next_x = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: read_x,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let read_y = fn_ir.add_value(
        ValueKind::Index1D {
            base: load_y,
            idx: load_i,
            is_safe: true,
            is_na_safe: true,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let next_y = fn_ir.add_value(
        ValueKind::Call {
            callee: "abs".to_string(),
            args: vec![read_y],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
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

    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: param_x,
        span: Span::default(),
    });
    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "y".to_string(),
        src: param_y,
        span: Span::default(),
    });
    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "i".to_string(),
        src: one,
        span: Span::default(),
    });
    fn_ir.blocks[entry].term = Terminator::Goto(header);
    fn_ir.blocks[header].term = Terminator::If {
        cond,
        then_bb: body,
        else_bb: exit,
    };
    fn_ir.blocks[body].instrs.push(Instr::StoreIndex1D {
        base: load_x,
        idx: load_i,
        val: next_x,
        is_safe: true,
        is_na_safe: true,
        is_vector: false,
        span: Span::default(),
    });
    fn_ir.blocks[body].instrs.push(Instr::StoreIndex1D {
        base: load_y,
        idx: load_i,
        val: next_y,
        is_safe: true,
        is_na_safe: true,
        is_vector: false,
        span: Span::default(),
    });
    fn_ir.blocks[body].instrs.push(Instr::Assign {
        dst: "i".to_string(),
        src: next_i,
        span: Span::default(),
    });
    fn_ir.blocks[body].term = Terminator::Goto(header);
    fn_ir.blocks[exit].term = Terminator::Return(Some(load_x));

    fn_ir
}

pub(crate) fn partial_range_single_store_loop_fn<F>(name: &str, build_rhs: F) -> FnIR
where
    F: Fn(&mut FnIR, ValueId, ValueId, ValueId) -> ValueId,
{
    let mut fn_ir = FnIR::new(name.to_string(), vec!["x".to_string()]);
    let entry = fn_ir.add_block();
    let header = fn_ir.add_block();
    let body = fn_ir.add_block();
    let exit = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let param_x = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let one = fn_ir.add_value(
        ValueKind::Const(crate::mir::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(crate::mir::Lit::Int(2)),
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    let load_x = fn_ir.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let load_i = fn_ir.add_value(
        ValueKind::Load {
            var: "i".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    let len_x = fn_ir.add_value(
        ValueKind::Len { base: load_x },
        Span::default(),
        Facts::empty(),
        None,
    );
    let cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Lt,
            lhs: load_i,
            rhs: len_x,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let rhs = build_rhs(&mut fn_ir, load_x, load_i, one);
    let next_i = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: load_i,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: param_x,
        span: Span::default(),
    });
    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "i".to_string(),
        src: two,
        span: Span::default(),
    });
    fn_ir.blocks[entry].term = Terminator::Goto(header);
    fn_ir.blocks[header].term = Terminator::If {
        cond,
        then_bb: body,
        else_bb: exit,
    };
    fn_ir.blocks[body].instrs.push(Instr::StoreIndex1D {
        base: load_x,
        idx: load_i,
        val: rhs,
        is_safe: true,
        is_na_safe: true,
        is_vector: false,
        span: Span::default(),
    });
    fn_ir.blocks[body].instrs.push(Instr::Assign {
        dst: "i".to_string(),
        src: next_i,
        span: Span::default(),
    });
    fn_ir.blocks[body].term = Terminator::Goto(header);
    fn_ir.blocks[exit].term = Terminator::Return(Some(load_x));

    fn_ir
}

pub(crate) fn partial_expr_map_fn() -> FnIR {
    partial_range_single_store_loop_fn("proof_partial_expr_map", |fn_ir, load_x, load_i, one| {
        let read_x = fn_ir.add_value(
            ValueKind::Index1D {
                base: load_x,
                idx: load_i,
                is_safe: true,
                is_na_safe: true,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: read_x,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        )
    })
}

pub(crate) fn partial_call_map_fn() -> FnIR {
    partial_range_single_store_loop_fn("proof_partial_call_map", |fn_ir, load_x, load_i, _one| {
        let read_x = fn_ir.add_value(
            ValueKind::Index1D {
                base: load_x,
                idx: load_i,
                is_safe: true,
                is_na_safe: true,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.add_value(
            ValueKind::Call {
                callee: "abs".to_string(),
                args: vec![read_x],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        )
    })
}
pub(crate) fn simple_cond_map_fn() -> FnIR {
    let mut fn_ir = FnIR::new("proof_cond_map".to_string(), vec!["x".to_string()]);
    let entry = fn_ir.add_block();
    let header = fn_ir.add_block();
    let branch = fn_ir.add_block();
    let then_bb = fn_ir.add_block();
    let else_bb = fn_ir.add_block();
    let latch = fn_ir.add_block();
    let exit = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let param_x = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let one = fn_ir.add_value(
        ValueKind::Const(crate::mir::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let zero = fn_ir.add_value(
        ValueKind::Const(crate::mir::Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let load_x = fn_ir.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("x".to_string()),
    );
    let load_i = fn_ir.add_value(
        ValueKind::Load {
            var: "i".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    let len_x = fn_ir.add_value(
        ValueKind::Len { base: load_x },
        Span::default(),
        Facts::empty(),
        None,
    );
    let loop_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: load_i,
            rhs: len_x,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let read_x = fn_ir.add_value(
        ValueKind::Index1D {
            base: load_x,
            idx: load_i,
            is_safe: true,
            is_na_safe: true,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let branch_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Gt,
            lhs: read_x,
            rhs: zero,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let then_rhs = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: read_x,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let else_rhs = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Sub,
            lhs: read_x,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let next_i = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: load_i,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: param_x,
        span: Span::default(),
    });
    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "i".to_string(),
        src: one,
        span: Span::default(),
    });
    fn_ir.blocks[entry].term = Terminator::Goto(header);
    fn_ir.blocks[header].term = Terminator::If {
        cond: loop_cond,
        then_bb: branch,
        else_bb: exit,
    };
    fn_ir.blocks[branch].term = Terminator::If {
        cond: branch_cond,
        then_bb,
        else_bb,
    };
    fn_ir.blocks[then_bb].instrs.push(Instr::StoreIndex1D {
        base: load_x,
        idx: load_i,
        val: then_rhs,
        is_safe: true,
        is_na_safe: true,
        is_vector: false,
        span: Span::default(),
    });
    fn_ir.blocks[then_bb].term = Terminator::Goto(latch);
    fn_ir.blocks[else_bb].instrs.push(Instr::StoreIndex1D {
        base: load_x,
        idx: load_i,
        val: else_rhs,
        is_safe: true,
        is_na_safe: true,
        is_vector: false,
        span: Span::default(),
    });
    fn_ir.blocks[else_bb].term = Terminator::Goto(latch);
    fn_ir.blocks[latch].instrs.push(Instr::Assign {
        dst: "i".to_string(),
        src: next_i,
        span: Span::default(),
    });
    fn_ir.blocks[latch].term = Terminator::Goto(header);
    fn_ir.blocks[exit].term = Terminator::Return(Some(load_x));

    fn_ir
}

pub(crate) fn simple_cond_map_with_eval_side_effect_fn() -> FnIR {
    let mut fn_ir = simple_cond_map_fn();
    let impure = fn_ir.add_value(
        ValueKind::Call {
            callee: "impure_helper".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let then_bb = 3usize;
    fn_ir.blocks[then_bb].instrs.insert(
        0,
        Instr::Eval {
            val: impure,
            span: Span::default(),
        },
    );
    fn_ir
}

pub(crate) fn simple_cond_map_with_assign_side_effect_fn() -> FnIR {
    let mut fn_ir = simple_cond_map_fn();
    let zero = fn_ir.add_value(
        ValueKind::Const(crate::mir::Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let then_bb = 3usize;
    fn_ir.blocks[then_bb].instrs.insert(
        0,
        Instr::Assign {
            dst: "tmp".to_string(),
            src: zero,
            span: Span::default(),
        },
    );
    fn_ir
}
