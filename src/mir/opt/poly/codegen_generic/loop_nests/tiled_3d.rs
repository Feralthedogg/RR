use super::*;
pub(crate) fn rebuild_generic_tiled_3d_loop_nest(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    preheader: usize,
    exit_bb: usize,
    skip_accessless_assigns: bool,
) -> bool {
    let (Some(tile_depth), Some(tile_rows), Some(tile_cols)) = (
        schedule.tile_depth.filter(|size| *size > 0),
        schedule.tile_rows.filter(|size| *size > 0),
        schedule.tile_cols.filter(|size| *size > 0),
    ) else {
        return false;
    };

    let ordered_names = ordered_dimension_names(schedule, scop);
    let mut dims = Vec::with_capacity(ordered_names.len());
    for name in ordered_names {
        let Some(dim) = scop.dimensions.iter().find(|dim| dim.iv_name == name) else {
            return false;
        };
        dims.push(dim.clone());
    }
    if dims.len() != 3 {
        return false;
    }

    let dim0 = &dims[0];
    let dim1 = &dims[1];
    let dim2 = &dims[2];

    let var0 = generated_iv_name(lp.header, &dim0.iv_name);
    let var1 = generated_iv_name(lp.header, &dim1.iv_name);
    let var2 = generated_iv_name(lp.header, &dim2.iv_name);
    let tile_var0 = generated_tile_iv_name(lp.header, &dim0.iv_name);
    let tile_var1 = generated_tile_iv_name(lp.header, &dim1.iv_name);
    let tile_var2 = generated_tile_iv_name(lp.header, &dim2.iv_name);

    let mut loop_var_map = FxHashMap::default();
    loop_var_map.insert(dim0.iv_name.clone(), var0.clone());
    loop_var_map.insert(dim1.iv_name.clone(), var1.clone());
    loop_var_map.insert(dim2.iv_name.clone(), var2.clone());

    let Some(lower0) = materialize_affine_expr(fn_ir, &dim0.lower_bound, &loop_var_map) else {
        return false;
    };
    let Some(upper0) = materialize_affine_expr(fn_ir, &dim0.upper_bound, &loop_var_map) else {
        return false;
    };
    let Some(lower1) = materialize_affine_expr(fn_ir, &dim1.lower_bound, &loop_var_map) else {
        return false;
    };
    let Some(upper1) = materialize_affine_expr(fn_ir, &dim1.upper_bound, &loop_var_map) else {
        return false;
    };
    let Some(lower2) = materialize_affine_expr(fn_ir, &dim2.lower_bound, &loop_var_map) else {
        return false;
    };
    let Some(upper2) = materialize_affine_expr(fn_ir, &dim2.upper_bound, &loop_var_map) else {
        return false;
    };

    for (dst, src) in [
        (tile_var0.clone(), lower0),
        (tile_var1.clone(), lower1),
        (tile_var2.clone(), lower2),
        (var0.clone(), lower0),
        (var1.clone(), lower1),
        (var2.clone(), lower2),
    ] {
        fn_ir.blocks[preheader].instrs.push(Instr::Assign {
            dst,
            src,
            span: Span::dummy(),
        });
    }

    let outer0_header = fn_ir.add_block();
    let outer1_init = fn_ir.add_block();
    let outer1_header = fn_ir.add_block();
    let outer2_init = fn_ir.add_block();
    let outer2_header = fn_ir.add_block();
    let dim0_init = fn_ir.add_block();
    let dim0_header = fn_ir.add_block();
    let dim1_init = fn_ir.add_block();
    let dim1_header = fn_ir.add_block();
    let dim2_init = fn_ir.add_block();
    let dim2_header = fn_ir.add_block();
    let body_bb = fn_ir.add_block();
    let dim2_step = fn_ir.add_block();
    let dim1_step = fn_ir.add_block();
    let dim0_step = fn_ir.add_block();
    let outer2_step = fn_ir.add_block();
    let outer1_step = fn_ir.add_block();
    let outer0_step = fn_ir.add_block();

    fn_ir.blocks[preheader].term = Terminator::Goto(outer0_header);

    let outer0_load = build_load(fn_ir, tile_var0.clone());
    let outer0_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: outer0_load,
            rhs: upper0,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer0_header].term = Terminator::If {
        cond: outer0_cond,
        then_bb: outer1_init,
        else_bb: exit_bb,
    };

    let Some(lower1_reload) = materialize_affine_expr(fn_ir, &dim1.lower_bound, &loop_var_map)
    else {
        return false;
    };
    let Some(lower2_reload) = materialize_affine_expr(fn_ir, &dim2.lower_bound, &loop_var_map)
    else {
        return false;
    };
    fn_ir.blocks[outer1_init].instrs.push(Instr::Assign {
        dst: tile_var1.clone(),
        src: lower1_reload,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer1_init].instrs.push(Instr::Assign {
        dst: tile_var2.clone(),
        src: lower2_reload,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer1_init].term = Terminator::Goto(outer1_header);

    let outer1_load = build_load(fn_ir, tile_var1.clone());
    let outer1_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: outer1_load,
            rhs: upper1,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer1_header].term = Terminator::If {
        cond: outer1_cond,
        then_bb: outer2_init,
        else_bb: outer0_step,
    };

    let Some(lower2_reload_b) = materialize_affine_expr(fn_ir, &dim2.lower_bound, &loop_var_map)
    else {
        return false;
    };
    fn_ir.blocks[outer2_init].instrs.push(Instr::Assign {
        dst: tile_var2.clone(),
        src: lower2_reload_b,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer2_init].term = Terminator::Goto(outer2_header);

    let outer2_load = build_load(fn_ir, tile_var2.clone());
    let outer2_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: outer2_load,
            rhs: upper2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer2_header].term = Terminator::If {
        cond: outer2_cond,
        then_bb: dim0_init,
        else_bb: outer1_step,
    };

    let tile0_for_init = build_load(fn_ir, tile_var0.clone());
    fn_ir.blocks[dim0_init].instrs.push(Instr::Assign {
        dst: var0.clone(),
        src: tile0_for_init,
        span: Span::dummy(),
    });
    fn_ir.blocks[dim0_init].term = Terminator::Goto(dim0_header);

    let dim0_load_a = build_load(fn_ir, var0.clone());
    let dim0_cond_a = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: dim0_load_a,
            rhs: upper0,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tile0_for_limit = build_load(fn_ir, tile_var0.clone());
    let tile0_span = fn_ir.add_value(
        ValueKind::Const(Lit::Int((tile_depth - 1) as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim0_tile_limit = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile0_for_limit,
            rhs: tile0_span,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim0_load_b = build_load(fn_ir, var0.clone());
    let dim0_cond_b = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: dim0_load_b,
            rhs: dim0_tile_limit,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim0_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::And,
            lhs: dim0_cond_a,
            rhs: dim0_cond_b,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[dim0_header].term = Terminator::If {
        cond: dim0_cond,
        then_bb: dim1_init,
        else_bb: outer2_step,
    };

    let tile1_for_init = build_load(fn_ir, tile_var1.clone());
    fn_ir.blocks[dim1_init].instrs.push(Instr::Assign {
        dst: var1.clone(),
        src: tile1_for_init,
        span: Span::dummy(),
    });
    fn_ir.blocks[dim1_init].term = Terminator::Goto(dim1_header);

    let dim1_load_a = build_load(fn_ir, var1.clone());
    let dim1_cond_a = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: dim1_load_a,
            rhs: upper1,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tile1_for_limit = build_load(fn_ir, tile_var1.clone());
    let tile1_span = fn_ir.add_value(
        ValueKind::Const(Lit::Int((tile_rows - 1) as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim1_tile_limit = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile1_for_limit,
            rhs: tile1_span,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim1_load_b = build_load(fn_ir, var1.clone());
    let dim1_cond_b = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: dim1_load_b,
            rhs: dim1_tile_limit,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim1_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::And,
            lhs: dim1_cond_a,
            rhs: dim1_cond_b,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[dim1_header].term = Terminator::If {
        cond: dim1_cond,
        then_bb: dim2_init,
        else_bb: dim0_step,
    };

    let tile2_for_init = build_load(fn_ir, tile_var2.clone());
    fn_ir.blocks[dim2_init].instrs.push(Instr::Assign {
        dst: var2.clone(),
        src: tile2_for_init,
        span: Span::dummy(),
    });
    fn_ir.blocks[dim2_init].term = Terminator::Goto(dim2_header);

    let dim2_load_a = build_load(fn_ir, var2.clone());
    let dim2_cond_a = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: dim2_load_a,
            rhs: upper2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tile2_for_limit = build_load(fn_ir, tile_var2.clone());
    let tile2_span = fn_ir.add_value(
        ValueKind::Const(Lit::Int((tile_cols - 1) as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim2_tile_limit = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile2_for_limit,
            rhs: tile2_span,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim2_load_b = build_load(fn_ir, var2.clone());
    let dim2_cond_b = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: dim2_load_b,
            rhs: dim2_tile_limit,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim2_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::And,
            lhs: dim2_cond_a,
            rhs: dim2_cond_b,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[dim2_header].term = Terminator::If {
        cond: dim2_cond,
        then_bb: body_bb,
        else_bb: dim1_step,
    };

    if emit_loop_iv_aliases(fn_ir, body_bb, scop, &loop_var_map).is_none()
        || emit_generic_body(fn_ir, body_bb, scop, &loop_var_map, skip_accessless_assigns).is_none()
    {
        return false;
    }
    fn_ir.blocks[body_bb].term = Terminator::Goto(dim2_step);

    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let dim2_load_for_step = build_load(fn_ir, var2.clone());
    let next_dim2 = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: dim2_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[dim2_step].instrs.push(Instr::Assign {
        dst: var2,
        src: next_dim2,
        span: Span::dummy(),
    });
    fn_ir.blocks[dim2_step].term = Terminator::Goto(dim2_header);

    let dim1_load_for_step = build_load(fn_ir, var1.clone());
    let next_dim1 = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: dim1_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[dim1_step].instrs.push(Instr::Assign {
        dst: var1,
        src: next_dim1,
        span: Span::dummy(),
    });
    fn_ir.blocks[dim1_step].term = Terminator::Goto(dim1_header);

    let dim0_load_for_step = build_load(fn_ir, var0.clone());
    let next_dim0 = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: dim0_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[dim0_step].instrs.push(Instr::Assign {
        dst: var0,
        src: next_dim0,
        span: Span::dummy(),
    });
    fn_ir.blocks[dim0_step].term = Terminator::Goto(dim0_header);

    let tile2_load_for_step = build_load(fn_ir, tile_var2.clone());
    let tile2_step_val = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_cols as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next_tile2 = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile2_load_for_step,
            rhs: tile2_step_val,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer2_step].instrs.push(Instr::Assign {
        dst: tile_var2,
        src: next_tile2,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer2_step].term = Terminator::Goto(outer2_header);

    let tile1_load_for_step = build_load(fn_ir, tile_var1.clone());
    let tile1_step_val = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_rows as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next_tile1 = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile1_load_for_step,
            rhs: tile1_step_val,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer1_step].instrs.push(Instr::Assign {
        dst: tile_var1,
        src: next_tile1,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer1_step].term = Terminator::Goto(outer1_header);

    let tile0_load_for_step = build_load(fn_ir, tile_var0.clone());
    let tile0_step_val = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_depth as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next_tile0 = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile0_load_for_step,
            rhs: tile0_step_val,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer0_step].instrs.push(Instr::Assign {
        dst: tile_var0,
        src: next_tile0,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer0_step].term = Terminator::Goto(outer0_header);

    for bid in &lp.body {
        fn_ir.blocks[*bid].instrs.clear();
        fn_ir.blocks[*bid].term = Terminator::Goto(exit_bb);
    }

    true
}
