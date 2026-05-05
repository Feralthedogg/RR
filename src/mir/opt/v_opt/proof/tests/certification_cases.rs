use super::*;
pub(crate) fn simple_cond_reduction_fn() -> FnIR {
    let mut fn_ir = FnIR::new("proof_cond_reduce".to_string(), vec!["x".to_string()]);
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
    let phi_i = fn_ir.add_value(
        ValueKind::Phi { args: vec![] },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    fn_ir.values[phi_i].phi_block = Some(header);
    let phi_acc = fn_ir.add_value(
        ValueKind::Phi { args: vec![] },
        Span::default(),
        Facts::empty(),
        Some("acc".to_string()),
    );
    fn_ir.values[phi_acc].phi_block = Some(header);
    let len_x = fn_ir.add_value(
        ValueKind::Len { base: load_x },
        Span::default(),
        Facts::empty(),
        None,
    );
    let loop_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: phi_i,
            rhs: len_x,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let read_x = fn_ir.add_value(
        ValueKind::Index1D {
            base: load_x,
            idx: phi_i,
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
    let reduced_read = fn_ir.add_value(
        ValueKind::Index1D {
            base: load_x,
            idx: phi_i,
            is_safe: true,
            is_na_safe: true,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let next_acc = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: phi_acc,
            rhs: reduced_read,
        },
        Span::default(),
        Facts::empty(),
        Some("acc".to_string()),
    );
    let next_i = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: phi_i,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    if let ValueKind::Phi { args } = &mut fn_ir.values[phi_i].kind {
        args.push((one, entry));
        args.push((next_i, latch));
    }
    if let ValueKind::Phi { args } = &mut fn_ir.values[phi_acc].kind {
        args.push((zero, entry));
        args.push((next_acc, latch));
    }

    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: param_x,
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
        idx: phi_i,
        val: then_rhs,
        is_safe: true,
        is_na_safe: true,
        is_vector: false,
        span: Span::default(),
    });
    fn_ir.blocks[then_bb].term = Terminator::Goto(latch);
    fn_ir.blocks[else_bb].instrs.push(Instr::StoreIndex1D {
        base: load_x,
        idx: phi_i,
        val: else_rhs,
        is_safe: true,
        is_na_safe: true,
        is_vector: false,
        span: Span::default(),
    });
    fn_ir.blocks[else_bb].term = Terminator::Goto(latch);
    fn_ir.blocks[latch].instrs.push(Instr::Assign {
        dst: "acc".to_string(),
        src: next_acc,
        span: Span::default(),
    });
    fn_ir.blocks[latch].instrs.push(Instr::Assign {
        dst: "i".to_string(),
        src: next_i,
        span: Span::default(),
    });
    fn_ir.blocks[latch].term = Terminator::Goto(header);
    fn_ir.blocks[exit].term = Terminator::Return(Some(phi_acc));

    fn_ir
}

pub(crate) fn simple_cond_reduction_with_eval_side_effect_fn() -> FnIR {
    let mut fn_ir = simple_cond_reduction_fn();
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

pub(crate) fn simple_cond_reduction_with_assign_side_effect_fn() -> FnIR {
    let mut fn_ir = simple_cond_reduction_fn();
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

pub(crate) fn simple_branch_only_cond_reduction_fn() -> FnIR {
    let mut fn_ir = FnIR::new(
        "proof_branch_only_cond_reduce".to_string(),
        vec!["x".to_string()],
    );
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
    let zero = fn_ir.add_value(
        ValueKind::Const(crate::mir::Lit::Int(0)),
        Span::default(),
        Facts::empty(),
        Some("acc".to_string()),
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
    let phi_i = fn_ir.add_value(
        ValueKind::Phi { args: vec![] },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    fn_ir.values[phi_i].phi_block = Some(header);
    let phi_acc = fn_ir.add_value(
        ValueKind::Phi { args: vec![] },
        Span::default(),
        Facts::empty(),
        Some("acc".to_string()),
    );
    fn_ir.values[phi_acc].phi_block = Some(header);
    let len_x = fn_ir.add_value(
        ValueKind::Len { base: load_x },
        Span::default(),
        Facts::empty(),
        None,
    );
    let loop_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: phi_i,
            rhs: len_x,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let read_x = fn_ir.add_value(
        ValueKind::Index1D {
            base: load_x,
            idx: phi_i,
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
    let inc = fn_ir.add_value(
        ValueKind::Const(crate::mir::Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let next_acc_then = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: phi_acc,
            rhs: inc,
        },
        Span::default(),
        Facts::empty(),
        Some("acc".to_string()),
    );
    let merged_acc = fn_ir.add_value(
        ValueKind::Phi {
            args: vec![(next_acc_then, then_bb), (phi_acc, else_bb)],
        },
        Span::default(),
        Facts::empty(),
        Some("acc".to_string()),
    );
    fn_ir.values[merged_acc].phi_block = Some(latch);
    let next_i = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: phi_i,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );

    if let ValueKind::Phi { args } = &mut fn_ir.values[phi_i].kind {
        args.push((one, entry));
        args.push((next_i, latch));
    }
    if let ValueKind::Phi { args } = &mut fn_ir.values[phi_acc].kind {
        args.push((zero, entry));
        args.push((merged_acc, latch));
    }

    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: param_x,
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
    fn_ir.blocks[then_bb].instrs.push(Instr::Assign {
        dst: "acc".to_string(),
        src: next_acc_then,
        span: Span::default(),
    });
    fn_ir.blocks[then_bb].term = Terminator::Goto(latch);
    fn_ir.blocks[else_bb].term = Terminator::Goto(latch);
    fn_ir.blocks[latch].instrs.push(Instr::Assign {
        dst: "i".to_string(),
        src: next_i,
        span: Span::default(),
    });
    fn_ir.blocks[latch].term = Terminator::Goto(header);
    fn_ir.blocks[exit].term = Terminator::Return(Some(phi_acc));

    fn_ir
}

pub(crate) fn simple_sum_reduction_fn() -> FnIR {
    let mut fn_ir = FnIR::new("proof_reduce_sum".to_string(), vec!["x".to_string()]);
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
    let phi_i = fn_ir.add_value(
        ValueKind::Phi { args: vec![] },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );
    fn_ir.values[phi_i].phi_block = Some(header);
    let phi_acc = fn_ir.add_value(
        ValueKind::Phi { args: vec![] },
        Span::default(),
        Facts::empty(),
        Some("acc".to_string()),
    );
    fn_ir.values[phi_acc].phi_block = Some(header);
    let len_x = fn_ir.add_value(
        ValueKind::Len { base: load_x },
        Span::default(),
        Facts::empty(),
        None,
    );
    let cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: phi_i,
            rhs: len_x,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let read_x = fn_ir.add_value(
        ValueKind::Index1D {
            base: load_x,
            idx: phi_i,
            is_safe: true,
            is_na_safe: true,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let next_acc = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: phi_acc,
            rhs: read_x,
        },
        Span::default(),
        Facts::empty(),
        Some("acc".to_string()),
    );
    let next_i = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: phi_i,
            rhs: one,
        },
        Span::default(),
        Facts::empty(),
        Some("i".to_string()),
    );

    if let ValueKind::Phi { args } = &mut fn_ir.values[phi_i].kind {
        args.push((one, entry));
        args.push((next_i, body));
    }
    if let ValueKind::Phi { args } = &mut fn_ir.values[phi_acc].kind {
        args.push((zero, entry));
        args.push((next_acc, body));
    }

    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: param_x,
        span: Span::default(),
    });
    fn_ir.blocks[entry].term = Terminator::Goto(header);
    fn_ir.blocks[header].term = Terminator::If {
        cond,
        then_bb: body,
        else_bb: exit,
    };
    fn_ir.blocks[body].instrs.push(Instr::Assign {
        dst: "acc".to_string(),
        src: next_acc,
        span: Span::default(),
    });
    fn_ir.blocks[body].instrs.push(Instr::Assign {
        dst: "i".to_string(),
        src: next_i,
        span: Span::default(),
    });
    fn_ir.blocks[body].term = Terminator::Goto(header);
    fn_ir.blocks[exit].term = Terminator::Return(Some(phi_acc));

    fn_ir
}

pub(crate) fn base_single_store_loop_fn<F>(name: &str, build_rhs: F) -> FnIR
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

#[test]
pub(crate) fn enabled_config_certifies_simple_map_and_plan_applies_transactionally() {
    let fn_ir = simple_map_fn();
    let loops = LoopAnalyzer::new(&fn_ir).find_loops();
    assert_eq!(loops.len(), 1);

    let outcome = analyze_loop_with_config(
        &fn_ir,
        &loops[0],
        &FxHashSet::default(),
        ProofConfig { enabled: true },
    );
    let ProofOutcome::Certified(certified) = outcome else {
        panic!("expected certified proof outcome");
    };
    assert!(matches!(
        certified.plan,
        super::super::super::planning::VectorPlan::Map { op: BinOp::Add, .. }
    ));

    let mut applied_ir = fn_ir.clone();
    let applied =
        try_apply_vectorization_transactionally(&mut applied_ir, &loops[0], certified.plan.clone());
    assert!(applied, "expected certified map plan to apply cleanly");
}

#[test]
pub(crate) fn enabled_config_certifies_simple_expr_map_and_plan_applies_transactionally() {
    let fn_ir = simple_expr_map_fn();
    let loops = LoopAnalyzer::new(&fn_ir).find_loops();
    assert_eq!(loops.len(), 1);

    let outcome = analyze_loop_with_config(
        &fn_ir,
        &loops[0],
        &FxHashSet::default(),
        ProofConfig { enabled: true },
    );
    let ProofOutcome::Certified(certified) = outcome else {
        panic!("expected certified proof outcome");
    };
    assert!(matches!(
        certified.plan,
        super::super::super::planning::VectorPlan::ExprMap { .. }
    ));

    let mut applied_ir = fn_ir.clone();
    let applied =
        try_apply_vectorization_transactionally(&mut applied_ir, &loops[0], certified.plan.clone());
    assert!(applied, "expected certified expr-map plan to apply cleanly");
}

#[test]
pub(crate) fn expr_map_matcher_rejects_loop_with_eval_side_effect() {
    let fn_ir = simple_expr_map_with_eval_side_effect_fn();
    let loops = LoopAnalyzer::new(&fn_ir).find_loops();
    assert_eq!(loops.len(), 1);

    let plan =
        super::super::super::planning::match_expr_map(&fn_ir, &loops[0], &FxHashSet::default());
    assert!(
        plan.is_none(),
        "expr-map matcher must reject loops that contain Eval side effects"
    );
}

#[test]
pub(crate) fn enabled_config_certifies_simple_call_map_and_plan_applies_transactionally() {
    let fn_ir = simple_call_map_fn();
    let loops = LoopAnalyzer::new(&fn_ir).find_loops();
    assert_eq!(loops.len(), 1);

    let outcome = analyze_loop_with_config(
        &fn_ir,
        &loops[0],
        &FxHashSet::default(),
        ProofConfig { enabled: true },
    );
    let ProofOutcome::Certified(certified) = outcome else {
        panic!("expected certified proof outcome");
    };
    assert!(matches!(
        certified.plan,
        super::super::super::planning::VectorPlan::CallMap { .. }
    ));

    let mut applied_ir = fn_ir.clone();
    let applied =
        try_apply_vectorization_transactionally(&mut applied_ir, &loops[0], certified.plan.clone());
    assert!(applied, "expected certified call-map plan to apply cleanly");
}

#[test]
pub(crate) fn scatter_matcher_rejects_loop_with_eval_side_effect() {
    let fn_ir = simple_scatter_with_eval_side_effect_fn();
    let loops = LoopAnalyzer::new(&fn_ir).find_loops();
    assert_eq!(loops.len(), 1);

    let plan = super::super::super::planning::match_scatter_expr_map(
        &fn_ir,
        &loops[0],
        &FxHashSet::default(),
    );
    assert!(
        plan.is_none(),
        "scatter matcher must reject loops that contain Eval side effects"
    );
}

#[test]
pub(crate) fn enabled_config_certifies_simple_shifted_map_and_plan_applies_transactionally() {
    let fn_ir = simple_shifted_map_fn();
    let loops = LoopAnalyzer::new(&fn_ir).find_loops();
    assert_eq!(loops.len(), 1);

    let outcome = analyze_loop_with_config(
        &fn_ir,
        &loops[0],
        &FxHashSet::default(),
        ProofConfig { enabled: true },
    );
    let ProofOutcome::Certified(certified) = outcome else {
        panic!("expected certified proof outcome");
    };
    assert!(matches!(
        certified.plan,
        super::super::super::planning::VectorPlan::ShiftedMap { offset: 1, .. }
    ));

    let mut applied_ir = fn_ir.clone();
    let applied =
        try_apply_vectorization_transactionally(&mut applied_ir, &loops[0], certified.plan.clone());
    assert!(
        applied,
        "expected certified shifted-map plan to apply cleanly"
    );
}

#[test]
pub(crate) fn enabled_config_certifies_simple_multi_expr_map_and_plan_applies_transactionally() {
    let fn_ir = simple_multi_expr_map_fn();
    let loops = LoopAnalyzer::new(&fn_ir).find_loops();
    assert_eq!(loops.len(), 1);

    let outcome = analyze_loop_with_config(
        &fn_ir,
        &loops[0],
        &FxHashSet::default(),
        ProofConfig { enabled: true },
    );
    let ProofOutcome::Certified(certified) = outcome else {
        panic!("expected certified proof outcome");
    };
    assert!(matches!(
        certified.plan,
        super::super::super::planning::VectorPlan::MultiExprMap { .. }
    ));

    let mut applied_ir = fn_ir.clone();
    let applied =
        try_apply_vectorization_transactionally(&mut applied_ir, &loops[0], certified.plan.clone());
    assert!(
        applied,
        "expected certified multi-expr-map plan to apply cleanly"
    );
}

#[test]
pub(crate) fn enabled_config_certifies_partial_expr_map_and_plan_applies_transactionally() {
    let fn_ir = partial_expr_map_fn();
    let loops = LoopAnalyzer::new(&fn_ir).find_loops();
    assert_eq!(loops.len(), 1);

    let outcome = analyze_loop_with_config(
        &fn_ir,
        &loops[0],
        &FxHashSet::default(),
        ProofConfig { enabled: true },
    );
    let ProofOutcome::Certified(certified) = outcome else {
        panic!("expected certified proof outcome");
    };
    assert!(matches!(
        certified.plan,
        super::super::super::planning::VectorPlan::ExprMap {
            whole_dest: false,
            ..
        }
    ));

    let mut applied_ir = fn_ir.clone();
    let applied =
        try_apply_vectorization_transactionally(&mut applied_ir, &loops[0], certified.plan.clone());
    assert!(
        applied,
        "expected certified partial expr-map plan to apply cleanly"
    );
}

#[test]
pub(crate) fn enabled_config_certifies_partial_call_map_and_plan_applies_transactionally() {
    let fn_ir = partial_call_map_fn();
    let loops = LoopAnalyzer::new(&fn_ir).find_loops();
    assert_eq!(loops.len(), 1);

    let outcome = analyze_loop_with_config(
        &fn_ir,
        &loops[0],
        &FxHashSet::default(),
        ProofConfig { enabled: true },
    );
    let ProofOutcome::Certified(certified) = outcome else {
        panic!("expected certified proof outcome");
    };
    assert!(matches!(
        certified.plan,
        super::super::super::planning::VectorPlan::CallMap {
            whole_dest: false,
            ..
        }
    ));

    let mut applied_ir = fn_ir.clone();
    let applied =
        try_apply_vectorization_transactionally(&mut applied_ir, &loops[0], certified.plan.clone());
    assert!(
        applied,
        "expected certified partial call-map plan to apply cleanly"
    );
}

#[test]
pub(crate) fn enabled_config_certifies_simple_cond_map_and_plan_applies_transactionally() {
    let fn_ir = simple_cond_map_fn();
    let loops = LoopAnalyzer::new(&fn_ir).find_loops();
    assert_eq!(loops.len(), 1);

    let outcome = analyze_loop_with_config(
        &fn_ir,
        &loops[0],
        &FxHashSet::default(),
        ProofConfig { enabled: true },
    );
    let ProofOutcome::Certified(certified) = outcome else {
        panic!("expected certified proof outcome");
    };
    assert!(matches!(
        certified.plan,
        super::super::super::planning::VectorPlan::CondMap { .. }
    ));

    let mut applied_ir = fn_ir.clone();
    let applied =
        try_apply_vectorization_transactionally(&mut applied_ir, &loops[0], certified.plan.clone());
    assert!(applied, "expected certified cond-map plan to apply cleanly");
}

#[test]
pub(crate) fn cond_map_certification_rejects_branch_eval_side_effect() {
    let fn_ir = simple_cond_map_with_eval_side_effect_fn();
    let loops = LoopAnalyzer::new(&fn_ir).find_loops();
    assert_eq!(loops.len(), 1);

    let err = certify_simple_cond_map(&fn_ir, &loops[0], &FxHashSet::default())
        .expect_err("branch Eval side effect must reject cond-map certification");
    assert_eq!(err, ProofFallbackReason::BranchStoreShape);
}

#[test]
pub(crate) fn cond_map_certification_rejects_branch_assign_side_effect() {
    let fn_ir = simple_cond_map_with_assign_side_effect_fn();
    let loops = LoopAnalyzer::new(&fn_ir).find_loops();
    assert_eq!(loops.len(), 1);

    let err = certify_simple_cond_map(&fn_ir, &loops[0], &FxHashSet::default())
        .expect_err("branch Assign side effect must reject cond-map certification");
    assert_eq!(err, ProofFallbackReason::BranchStoreShape);
}

#[test]
pub(crate) fn enabled_config_certifies_simple_cond_reduction_and_plan_applies_transactionally() {
    let fn_ir = simple_cond_reduction_fn();
    let loops = LoopAnalyzer::new(&fn_ir).find_loops();
    assert_eq!(loops.len(), 1);

    let outcome = analyze_loop_with_config(
        &fn_ir,
        &loops[0],
        &FxHashSet::default(),
        ProofConfig { enabled: true },
    );
    let ProofOutcome::Certified(certified) = outcome else {
        panic!("expected certified proof outcome");
    };
    assert!(matches!(
        certified.plan,
        super::super::super::planning::VectorPlan::ReduceCond { .. }
    ));

    let mut applied_ir = fn_ir.clone();
    let applied =
        try_apply_vectorization_transactionally(&mut applied_ir, &loops[0], certified.plan.clone());
    assert!(
        applied,
        "expected certified conditional reduction plan to apply cleanly"
    );
}

#[test]
pub(crate) fn cond_reduction_certification_rejects_branch_eval_side_effect() {
    let fn_ir = simple_cond_reduction_with_eval_side_effect_fn();
    let loops = LoopAnalyzer::new(&fn_ir).find_loops();
    assert_eq!(loops.len(), 1);

    let err = certify_simple_cond_reduction(&fn_ir, &loops[0], &FxHashSet::default())
        .expect_err("branch Eval side effect must reject cond-reduction certification");
    assert_eq!(err, ProofFallbackReason::BranchStoreShape);
}

#[test]
pub(crate) fn cond_reduction_certification_rejects_branch_assign_side_effect() {
    let fn_ir = simple_cond_reduction_with_assign_side_effect_fn();
    let loops = LoopAnalyzer::new(&fn_ir).find_loops();
    assert_eq!(loops.len(), 1);

    let err = certify_simple_cond_reduction(&fn_ir, &loops[0], &FxHashSet::default())
        .expect_err("branch Assign side effect must reject cond-reduction certification");
    assert_eq!(err, ProofFallbackReason::BranchStoreShape);
}
