use super::*;
pub(crate) fn ordered_dimension_names(schedule: &SchedulePlan, scop: &ScopRegion) -> Vec<String> {
    let mut dims = Vec::with_capacity(schedule.relation.output_expressions.len());
    for expr in &schedule.relation.output_expressions {
        let Some((AffineSymbol::LoopIv(name), coeff)) = expr.terms.iter().next() else {
            return scop
                .dimensions
                .iter()
                .map(|dim| dim.iv_name.clone())
                .collect();
        };
        if *coeff != 1 || expr.terms.len() != 1 || expr.constant != 0 {
            return scop
                .dimensions
                .iter()
                .map(|dim| dim.iv_name.clone())
                .collect();
        }
        dims.push(name.clone());
    }
    if dims.is_empty() {
        scop.dimensions
            .iter()
            .map(|dim| dim.iv_name.clone())
            .collect()
    } else {
        dims
    }
}

pub(crate) fn generated_iv_name(header: usize, dim_name: &str) -> String {
    format!("{GENERATED_LOOP_IV_PREFIX}{header}_{dim_name}")
}

pub(crate) fn generated_tile_iv_name(header: usize, dim_name: &str) -> String {
    format!("{GENERATED_LOOP_IV_PREFIX}tile_{header}_{dim_name}")
}

pub(crate) fn rebuild_generic_loop_nest(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    preheader: usize,
    exit_bb: usize,
    skip_accessless_assigns: bool,
) -> bool {
    let ordered_names = ordered_dimension_names(schedule, scop);
    let mut dims = Vec::with_capacity(ordered_names.len());
    for name in ordered_names {
        let Some(dim) = scop.dimensions.iter().find(|dim| dim.iv_name == name) else {
            return false;
        };
        dims.push(dim.clone());
    }

    let mut loop_var_map = FxHashMap::default();
    for dim in &scop.dimensions {
        loop_var_map.insert(
            dim.iv_name.clone(),
            generated_iv_name(lp.header, &dim.iv_name),
        );
    }
    for dim in &dims {
        let Some(dst) = loop_var_map.get(&dim.iv_name).cloned() else {
            return false;
        };
        let Some(init_val) = materialize_affine_expr(fn_ir, &dim.lower_bound, &loop_var_map) else {
            return false;
        };
        fn_ir.blocks[preheader].instrs.push(Instr::Assign {
            dst,
            src: init_val,
            span: Span::dummy(),
        });
    }

    let Some(entry_init) = build_loop_level(
        fn_ir,
        &dims,
        0,
        scop,
        &loop_var_map,
        exit_bb,
        skip_accessless_assigns,
    ) else {
        return false;
    };
    fn_ir.blocks[preheader].term = Terminator::Goto(entry_init);

    for bid in &lp.body {
        fn_ir.blocks[*bid].instrs.clear();
        fn_ir.blocks[*bid].term = Terminator::Goto(exit_bb);
    }

    true
}

pub(crate) fn rebuild_generic_tiled_1d_loop_nest(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    preheader: usize,
    exit_bb: usize,
    skip_accessless_assigns: bool,
) -> bool {
    let Some(tile_size) = schedule.tile_size.filter(|size| *size > 0) else {
        return false;
    };
    let dim = &scop.dimensions[0];
    let loop_var = generated_iv_name(lp.header, &dim.iv_name);
    let tile_var = generated_tile_iv_name(lp.header, &dim.iv_name);
    let mut loop_var_map = FxHashMap::default();
    loop_var_map.insert(dim.iv_name.clone(), loop_var.clone());

    let Some(lower) = materialize_affine_expr(fn_ir, &dim.lower_bound, &loop_var_map) else {
        return false;
    };
    let Some(upper) = materialize_affine_expr(fn_ir, &dim.upper_bound, &loop_var_map) else {
        return false;
    };
    fn_ir.blocks[preheader].instrs.push(Instr::Assign {
        dst: tile_var.clone(),
        src: lower,
        span: Span::dummy(),
    });
    fn_ir.blocks[preheader].instrs.push(Instr::Assign {
        dst: loop_var.clone(),
        src: lower,
        span: Span::dummy(),
    });

    let outer_header = fn_ir.add_block();
    let inner_init = fn_ir.add_block();
    let inner_header = fn_ir.add_block();
    let inner_step = fn_ir.add_block();
    let outer_step = fn_ir.add_block();
    let body_bb = fn_ir.add_block();
    fn_ir.blocks[preheader].term = Terminator::Goto(outer_header);

    let tile_load = build_load(fn_ir, tile_var.clone());
    let outer_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: tile_load,
            rhs: upper,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer_header].term = Terminator::If {
        cond: outer_cond,
        then_bb: inner_init,
        else_bb: exit_bb,
    };

    let tile_load_for_init = build_load(fn_ir, tile_var.clone());
    fn_ir.blocks[inner_init].instrs.push(Instr::Assign {
        dst: loop_var.clone(),
        src: tile_load_for_init,
        span: Span::dummy(),
    });
    fn_ir.blocks[inner_init].term = Terminator::Goto(inner_header);

    let loop_load_a = build_load(fn_ir, loop_var.clone());
    let inner_cond_a = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: loop_load_a,
            rhs: upper,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tile_load_for_limit = build_load(fn_ir, tile_var.clone());
    let tile_span = fn_ir.add_value(
        ValueKind::Const(Lit::Int((tile_size - 1) as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tile_limit = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile_load_for_limit,
            rhs: tile_span,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let loop_load_b = build_load(fn_ir, loop_var.clone());
    let inner_cond_b = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: loop_load_b,
            rhs: tile_limit,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let inner_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::And,
            lhs: inner_cond_a,
            rhs: inner_cond_b,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[inner_header].term = Terminator::If {
        cond: inner_cond,
        then_bb: body_bb,
        else_bb: outer_step,
    };

    if emit_loop_iv_aliases(fn_ir, body_bb, scop, &loop_var_map).is_none()
        || emit_generic_body(fn_ir, body_bb, scop, &loop_var_map, skip_accessless_assigns).is_none()
    {
        return false;
    }
    fn_ir.blocks[body_bb].term = Terminator::Goto(inner_step);

    let loop_load_for_step = build_load(fn_ir, loop_var.clone());
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next_loop = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: loop_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[inner_step].instrs.push(Instr::Assign {
        dst: loop_var,
        src: next_loop,
        span: Span::dummy(),
    });
    fn_ir.blocks[inner_step].term = Terminator::Goto(inner_header);

    let tile_load_for_step = build_load(fn_ir, tile_var.clone());
    let tile_step = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_size as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next_tile = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile_load_for_step,
            rhs: tile_step,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer_step].instrs.push(Instr::Assign {
        dst: tile_var,
        src: next_tile,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer_step].term = Terminator::Goto(outer_header);

    for bid in &lp.body {
        fn_ir.blocks[*bid].instrs.clear();
        fn_ir.blocks[*bid].term = Terminator::Goto(exit_bb);
    }

    true
}

pub(crate) fn generated_skew_iv_name(header: usize, dim_name: &str) -> String {
    format!("{GENERATED_LOOP_IV_PREFIX}skew_{header}_{dim_name}")
}

pub(crate) fn rebuild_generic_skewed_2d_loop_nest(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    _schedule: &SchedulePlan,
    preheader: usize,
    exit_bb: usize,
    skip_accessless_assigns: bool,
) -> bool {
    if scop.dimensions.len() != 2 || scop.dimensions.iter().any(|dim| dim.step != 1) {
        return false;
    }
    let outer_dim = &scop.dimensions[0];
    let inner_dim = &scop.dimensions[1];
    let outer_var = generated_iv_name(lp.header, &outer_dim.iv_name);
    let inner_var = generated_iv_name(lp.header, &inner_dim.iv_name);
    let skew_var = generated_skew_iv_name(lp.header, &inner_dim.iv_name);

    let mut loop_var_map = FxHashMap::default();
    loop_var_map.insert(outer_dim.iv_name.clone(), outer_var.clone());
    loop_var_map.insert(inner_dim.iv_name.clone(), inner_var.clone());

    let Some(outer_lower) = materialize_affine_expr(fn_ir, &outer_dim.lower_bound, &loop_var_map)
    else {
        return false;
    };
    let Some(outer_upper) = materialize_affine_expr(fn_ir, &outer_dim.upper_bound, &loop_var_map)
    else {
        return false;
    };
    let Some(inner_lower) = materialize_affine_expr(fn_ir, &inner_dim.lower_bound, &loop_var_map)
    else {
        return false;
    };
    let Some(inner_upper) = materialize_affine_expr(fn_ir, &inner_dim.upper_bound, &loop_var_map)
    else {
        return false;
    };

    fn_ir.blocks[preheader].instrs.push(Instr::Assign {
        dst: outer_var.clone(),
        src: outer_lower,
        span: Span::dummy(),
    });

    let outer_header = fn_ir.add_block();
    let inner_init = fn_ir.add_block();
    let inner_header = fn_ir.add_block();
    let body_bb = fn_ir.add_block();
    let inner_step = fn_ir.add_block();
    let outer_step = fn_ir.add_block();

    fn_ir.blocks[preheader].term = Terminator::Goto(outer_header);

    let outer_load = build_load(fn_ir, outer_var.clone());
    let outer_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: outer_load,
            rhs: outer_upper,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer_header].term = Terminator::If {
        cond: outer_cond,
        then_bb: inner_init,
        else_bb: exit_bb,
    };

    let outer_for_init = build_load(fn_ir, outer_var.clone());
    let skew_init = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: outer_for_init,
            rhs: inner_lower,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[inner_init].instrs.push(Instr::Assign {
        dst: skew_var.clone(),
        src: skew_init,
        span: Span::dummy(),
    });
    fn_ir.blocks[inner_init].instrs.push(Instr::Assign {
        dst: inner_var.clone(),
        src: inner_lower,
        span: Span::dummy(),
    });
    fn_ir.blocks[inner_init].term = Terminator::Goto(inner_header);

    let outer_for_limit = build_load(fn_ir, outer_var.clone());
    let skew_limit = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: outer_for_limit,
            rhs: inner_upper,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let skew_load = build_load(fn_ir, skew_var.clone());
    let inner_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: skew_load,
            rhs: skew_limit,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[inner_header].term = Terminator::If {
        cond: inner_cond,
        then_bb: body_bb,
        else_bb: outer_step,
    };

    if emit_loop_iv_aliases(fn_ir, body_bb, scop, &loop_var_map).is_none()
        || emit_generic_body(fn_ir, body_bb, scop, &loop_var_map, skip_accessless_assigns).is_none()
    {
        return false;
    }
    fn_ir.blocks[body_bb].term = Terminator::Goto(inner_step);

    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let skew_load_for_step = build_load(fn_ir, skew_var.clone());
    let next_skew = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: skew_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let inner_load_for_step = build_load(fn_ir, inner_var.clone());
    let next_inner = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: inner_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[inner_step].instrs.push(Instr::Assign {
        dst: skew_var,
        src: next_skew,
        span: Span::dummy(),
    });
    fn_ir.blocks[inner_step].instrs.push(Instr::Assign {
        dst: inner_var,
        src: next_inner,
        span: Span::dummy(),
    });
    fn_ir.blocks[inner_step].term = Terminator::Goto(inner_header);

    let outer_load_for_step = build_load(fn_ir, outer_var.clone());
    let next_outer = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: outer_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer_step].instrs.push(Instr::Assign {
        dst: outer_var,
        src: next_outer,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer_step].term = Terminator::Goto(outer_header);

    for bid in &lp.body {
        fn_ir.blocks[*bid].instrs.clear();
        fn_ir.blocks[*bid].term = Terminator::Goto(exit_bb);
    }

    true
}

pub(crate) fn rebuild_generic_tiled_2d_loop_nest(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    preheader: usize,
    exit_bb: usize,
    skip_accessless_assigns: bool,
) -> bool {
    let (Some(tile_rows), Some(tile_cols)) = (
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
    if dims.len() != 2 {
        return false;
    }
    let row_dim = &dims[0];
    let col_dim = &dims[1];

    let row_var = generated_iv_name(lp.header, &row_dim.iv_name);
    let col_var = generated_iv_name(lp.header, &col_dim.iv_name);
    let tile_row_var = generated_tile_iv_name(lp.header, &row_dim.iv_name);
    let tile_col_var = generated_tile_iv_name(lp.header, &col_dim.iv_name);

    let mut loop_var_map = FxHashMap::default();
    loop_var_map.insert(row_dim.iv_name.clone(), row_var.clone());
    loop_var_map.insert(col_dim.iv_name.clone(), col_var.clone());

    let Some(row_lower) = materialize_affine_expr(fn_ir, &row_dim.lower_bound, &loop_var_map)
    else {
        return false;
    };
    let Some(row_upper) = materialize_affine_expr(fn_ir, &row_dim.upper_bound, &loop_var_map)
    else {
        return false;
    };
    let Some(col_lower) = materialize_affine_expr(fn_ir, &col_dim.lower_bound, &loop_var_map)
    else {
        return false;
    };
    let Some(col_upper) = materialize_affine_expr(fn_ir, &col_dim.upper_bound, &loop_var_map)
    else {
        return false;
    };

    for (dst, src) in [
        (tile_row_var.clone(), row_lower),
        (tile_col_var.clone(), col_lower),
        (row_var.clone(), row_lower),
        (col_var.clone(), col_lower),
    ] {
        fn_ir.blocks[preheader].instrs.push(Instr::Assign {
            dst,
            src,
            span: Span::dummy(),
        });
    }

    let outer_row_header = fn_ir.add_block();
    let outer_col_init = fn_ir.add_block();
    let outer_col_header = fn_ir.add_block();
    let row_init = fn_ir.add_block();
    let row_header = fn_ir.add_block();
    let col_init = fn_ir.add_block();
    let col_header = fn_ir.add_block();
    let body_bb = fn_ir.add_block();
    let col_step = fn_ir.add_block();
    let row_step = fn_ir.add_block();
    let outer_col_step = fn_ir.add_block();
    let outer_row_step = fn_ir.add_block();

    fn_ir.blocks[preheader].term = Terminator::Goto(outer_row_header);

    let tile_row_load = build_load(fn_ir, tile_row_var.clone());
    let outer_row_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: tile_row_load,
            rhs: row_upper,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer_row_header].term = Terminator::If {
        cond: outer_row_cond,
        then_bb: outer_col_init,
        else_bb: exit_bb,
    };

    let Some(col_lower_reload) =
        materialize_affine_expr(fn_ir, &col_dim.lower_bound, &loop_var_map)
    else {
        return false;
    };
    fn_ir.blocks[outer_col_init].instrs.push(Instr::Assign {
        dst: tile_col_var.clone(),
        src: col_lower_reload,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer_col_init].term = Terminator::Goto(outer_col_header);

    let tile_col_load = build_load(fn_ir, tile_col_var.clone());
    let outer_col_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: tile_col_load,
            rhs: col_upper,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer_col_header].term = Terminator::If {
        cond: outer_col_cond,
        then_bb: row_init,
        else_bb: outer_row_step,
    };

    let tile_row_for_row_init = build_load(fn_ir, tile_row_var.clone());
    fn_ir.blocks[row_init].instrs.push(Instr::Assign {
        dst: row_var.clone(),
        src: tile_row_for_row_init,
        span: Span::dummy(),
    });
    fn_ir.blocks[row_init].term = Terminator::Goto(row_header);

    let row_load_a = build_load(fn_ir, row_var.clone());
    let row_cond_a = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: row_load_a,
            rhs: row_upper,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tile_row_for_limit = build_load(fn_ir, tile_row_var.clone());
    let tile_row_span = fn_ir.add_value(
        ValueKind::Const(Lit::Int((tile_rows - 1) as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let row_tile_limit = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile_row_for_limit,
            rhs: tile_row_span,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let row_load_b = build_load(fn_ir, row_var.clone());
    let row_cond_b = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: row_load_b,
            rhs: row_tile_limit,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let row_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::And,
            lhs: row_cond_a,
            rhs: row_cond_b,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[row_header].term = Terminator::If {
        cond: row_cond,
        then_bb: col_init,
        else_bb: outer_col_step,
    };

    let tile_col_for_col_init = build_load(fn_ir, tile_col_var.clone());
    fn_ir.blocks[col_init].instrs.push(Instr::Assign {
        dst: col_var.clone(),
        src: tile_col_for_col_init,
        span: Span::dummy(),
    });
    fn_ir.blocks[col_init].term = Terminator::Goto(col_header);

    let col_load_a = build_load(fn_ir, col_var.clone());
    let col_cond_a = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: col_load_a,
            rhs: col_upper,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tile_col_for_limit = build_load(fn_ir, tile_col_var.clone());
    let tile_col_span = fn_ir.add_value(
        ValueKind::Const(Lit::Int((tile_cols - 1) as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let col_tile_limit = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile_col_for_limit,
            rhs: tile_col_span,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let col_load_b = build_load(fn_ir, col_var.clone());
    let col_cond_b = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: col_load_b,
            rhs: col_tile_limit,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let col_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::And,
            lhs: col_cond_a,
            rhs: col_cond_b,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[col_header].term = Terminator::If {
        cond: col_cond,
        then_bb: body_bb,
        else_bb: row_step,
    };

    if emit_loop_iv_aliases(fn_ir, body_bb, scop, &loop_var_map).is_none()
        || emit_generic_body(fn_ir, body_bb, scop, &loop_var_map, skip_accessless_assigns).is_none()
    {
        return false;
    }
    fn_ir.blocks[body_bb].term = Terminator::Goto(col_step);

    let col_load_for_step = build_load(fn_ir, col_var.clone());
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next_col = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: col_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[col_step].instrs.push(Instr::Assign {
        dst: col_var,
        src: next_col,
        span: Span::dummy(),
    });
    fn_ir.blocks[col_step].term = Terminator::Goto(col_header);

    let row_load_for_step = build_load(fn_ir, row_var.clone());
    let next_row = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: row_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[row_step].instrs.push(Instr::Assign {
        dst: row_var,
        src: next_row,
        span: Span::dummy(),
    });
    fn_ir.blocks[row_step].term = Terminator::Goto(row_header);

    let tile_col_load_for_step = build_load(fn_ir, tile_col_var.clone());
    let tile_col_step_val = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_cols as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next_tile_col = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile_col_load_for_step,
            rhs: tile_col_step_val,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer_col_step].instrs.push(Instr::Assign {
        dst: tile_col_var,
        src: next_tile_col,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer_col_step].term = Terminator::Goto(outer_col_header);

    let tile_row_load_for_step = build_load(fn_ir, tile_row_var.clone());
    let tile_row_step_val = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_rows as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next_tile_row = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile_row_load_for_step,
            rhs: tile_row_step_val,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer_row_step].instrs.push(Instr::Assign {
        dst: tile_row_var,
        src: next_tile_row,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer_row_step].term = Terminator::Goto(outer_row_header);

    for bid in &lp.body {
        fn_ir.blocks[*bid].instrs.clear();
        fn_ir.blocks[*bid].term = Terminator::Goto(exit_bb);
    }

    true
}
