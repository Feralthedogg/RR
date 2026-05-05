use super::*;
pub(crate) fn scop_subset_with_stmt(scop: &ScopRegion, stmt: PolyStmt) -> ScopRegion {
    ScopRegion {
        header: scop.header,
        latch: scop.latch,
        exits: scop.exits.clone(),
        dimensions: scop.dimensions.clone(),
        iteration_space: scop.iteration_space.clone(),
        parameters: scop.parameters.clone(),
        statements: vec![stmt],
    }
}

pub(crate) type GenericLowerOne =
    fn(&mut FnIR, &LoopInfo, &ScopRegion, &SchedulePlan, usize, usize, bool) -> bool;

pub(crate) struct GenericFissionRequest<'a> {
    pub(crate) lp: &'a LoopInfo,
    pub(crate) scop: &'a ScopRegion,
    pub(crate) schedule: &'a SchedulePlan,
    pub(crate) preheader: usize,
    pub(crate) exit_bb: usize,
    pub(crate) skip_accessless_assigns: bool,
    pub(crate) lower_one: GenericLowerOne,
}

pub(crate) fn rebuild_generic_fissioned_sequence(
    fn_ir: &mut FnIR,
    request: GenericFissionRequest<'_>,
) -> bool {
    let stmts = request
        .scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if stmts.len() <= 1 {
        return false;
    }
    let mut next_preheader = request.preheader;
    for (idx, stmt) in stmts.into_iter().enumerate() {
        let next_exit = if idx + 1
            == request
                .scop
                .statements
                .iter()
                .filter(|stmt| !stmt.accesses.is_empty())
                .count()
        {
            request.exit_bb
        } else {
            fn_ir.add_block()
        };
        let subset = scop_subset_with_stmt(request.scop, stmt);
        if !(request.lower_one)(
            fn_ir,
            request.lp,
            &subset,
            request.schedule,
            next_preheader,
            next_exit,
            request.skip_accessless_assigns,
        ) {
            return false;
        }
        next_preheader = next_exit;
    }
    true
}

pub(crate) fn build_loop_level(
    fn_ir: &mut FnIR,
    dims: &[super::LoopDimension],
    level: usize,
    scop: &ScopRegion,
    loop_var_map: &FxHashMap<String, String>,
    loop_exit_bb: usize,
    skip_accessless_assigns: bool,
) -> Option<usize> {
    let dim = dims.get(level)?;
    let init_bb = fn_ir.add_block();
    let header_bb = fn_ir.add_block();
    let step_bb = fn_ir.add_block();

    let after_body_bb = if level + 1 == dims.len() {
        let body_bb = fn_ir.add_block();
        emit_loop_iv_aliases(fn_ir, body_bb, scop, loop_var_map)?;
        emit_generic_body(fn_ir, body_bb, scop, loop_var_map, skip_accessless_assigns)?;
        fn_ir.blocks[body_bb].term = Terminator::Goto(step_bb);
        body_bb
    } else {
        build_loop_level(
            fn_ir,
            dims,
            level + 1,
            scop,
            loop_var_map,
            step_bb,
            skip_accessless_assigns,
        )?
    };

    let init_val = materialize_affine_expr(fn_ir, &dim.lower_bound, loop_var_map)?;
    fn_ir.blocks[init_bb].instrs.push(Instr::Assign {
        dst: loop_var_map.get(&dim.iv_name)?.clone(),
        src: init_val,
        span: Span::dummy(),
    });
    fn_ir.blocks[init_bb].term = Terminator::Goto(header_bb);

    let iv_load = build_load(fn_ir, loop_var_map.get(&dim.iv_name)?.clone());
    let bound_val = materialize_affine_expr(fn_ir, &dim.upper_bound, loop_var_map)?;
    let cond = fn_ir.add_value(
        ValueKind::Binary {
            op: if dim.step >= 0 { BinOp::Le } else { BinOp::Ge },
            lhs: iv_load,
            rhs: bound_val,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[header_bb].term = Terminator::If {
        cond,
        then_bb: after_body_bb,
        else_bb: loop_exit_bb,
    };

    let step_load = build_load(fn_ir, loop_var_map.get(&dim.iv_name)?.clone());
    let step_mag = fn_ir.add_value(
        ValueKind::Const(Lit::Int(dim.step.unsigned_abs() as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next = fn_ir.add_value(
        ValueKind::Binary {
            op: if dim.step >= 0 {
                BinOp::Add
            } else {
                BinOp::Sub
            },
            lhs: step_load,
            rhs: step_mag,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[step_bb].instrs.push(Instr::Assign {
        dst: loop_var_map.get(&dim.iv_name)?.clone(),
        src: next,
        span: Span::dummy(),
    });
    fn_ir.blocks[step_bb].term = Terminator::Goto(header_bb);

    Some(init_bb)
}

pub(crate) fn emit_loop_iv_aliases(
    fn_ir: &mut FnIR,
    body_bb: usize,
    scop: &ScopRegion,
    loop_var_map: &FxHashMap<String, String>,
) -> Option<()> {
    for dim in &scop.dimensions {
        let generated = loop_var_map.get(&dim.iv_name)?;
        if generated == &dim.iv_name {
            continue;
        }
        let src = build_load(fn_ir, generated.clone());
        fn_ir.blocks[body_bb].instrs.push(Instr::Assign {
            dst: dim.iv_name.clone(),
            src,
            span: Span::dummy(),
        });
    }
    Some(())
}

pub(crate) fn emit_generic_body(
    fn_ir: &mut FnIR,
    body_bb: usize,
    scop: &ScopRegion,
    loop_var_map: &FxHashMap<String, String>,
    skip_accessless_assigns: bool,
) -> Option<()> {
    let loop_iv_names = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.as_str())
        .collect::<FxHashSet<_>>();
    let mut memo = FxHashMap::default();
    for (idx, stmt) in scop.statements.iter().enumerate() {
        if let PolyStmtKind::Assign { dst } = &stmt.kind
            && loop_iv_names.contains(dst.as_str())
        {
            continue;
        }
        if stmt_is_progress_assign(fn_ir, scop, stmt) {
            continue;
        }
        if skip_accessless_assigns
            && stmt.accesses.is_empty()
            && let PolyStmtKind::Assign { dst } = &stmt.kind
        {
            let needed_later = scop.statements[idx + 1..]
                .iter()
                .any(|later| stmt_mentions_var(fn_ir, later, dst));
            if !needed_later {
                continue;
            }
        }
        emit_generic_stmt(fn_ir, body_bb, stmt, loop_var_map, &mut memo)?;
    }
    Some(())
}

pub(crate) fn stmt_is_progress_assign(fn_ir: &FnIR, scop: &ScopRegion, stmt: &PolyStmt) -> bool {
    let (PolyStmtKind::Assign { dst }, Some(expr_root)) = (&stmt.kind, stmt.expr_root) else {
        return false;
    };
    if !stmt.accesses.is_empty() {
        return false;
    }
    let root = resolve_scop_local_source(fn_ir, scop, expr_root);
    match &fn_ir.values[root].kind {
        ValueKind::Binary {
            op: BinOp::Add | BinOp::Sub,
            lhs,
            rhs,
        } => {
            let lhs = resolve_scop_local_source(fn_ir, scop, *lhs);
            let rhs = resolve_scop_local_source(fn_ir, scop, *rhs);
            let lhs_self = is_same_named_value(fn_ir, lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, rhs, dst);
            let lhs_const = matches!(fn_ir.values[lhs].kind, ValueKind::Const(Lit::Int(_)));
            let rhs_const = matches!(fn_ir.values[rhs].kind, ValueKind::Const(Lit::Int(_)));
            (lhs_self && rhs_const) || (rhs_self && lhs_const)
        }
        _ => false,
    }
}

pub(crate) fn stmt_mentions_var(fn_ir: &FnIR, stmt: &PolyStmt, var: &str) -> bool {
    let mut seen = FxHashSet::default();
    if stmt
        .expr_root
        .is_some_and(|root| expr_mentions_var(fn_ir, root, var, &mut seen))
    {
        return true;
    }
    match &stmt.kind {
        PolyStmtKind::Assign { .. } | PolyStmtKind::Eval => false,
        PolyStmtKind::Store { base, subscripts } => {
            expr_mentions_var(fn_ir, *base, var, &mut seen)
                || subscripts
                    .iter()
                    .any(|sub| expr_mentions_var(fn_ir, *sub, var, &mut seen))
        }
    }
}

pub(crate) fn emit_generic_stmt(
    fn_ir: &mut FnIR,
    body_bb: usize,
    stmt: &PolyStmt,
    loop_var_map: &FxHashMap<String, String>,
    memo: &mut FxHashMap<ValueId, ValueId>,
) -> Option<()> {
    let span = stmt
        .expr_root
        .map(|root| fn_ir.values[root].span)
        .unwrap_or_else(Span::dummy);
    match (&stmt.kind, stmt.expr_root) {
        (PolyStmtKind::Assign { dst }, Some(root)) => {
            let src = clone_value_for_generic(fn_ir, root, loop_var_map, memo)?;
            fn_ir.blocks[body_bb].instrs.push(Instr::Assign {
                dst: dst.clone(),
                src,
                span,
            });
            Some(())
        }
        (PolyStmtKind::Eval, Some(root)) => {
            let val = clone_value_for_generic(fn_ir, root, loop_var_map, memo)?;
            fn_ir.blocks[body_bb].instrs.push(Instr::Eval { val, span });
            Some(())
        }
        (PolyStmtKind::Store { base, subscripts }, Some(root)) => {
            let base = clone_value_for_generic(fn_ir, *base, loop_var_map, memo)?;
            let value = clone_value_for_generic(fn_ir, root, loop_var_map, memo)?;
            let subscripts = subscripts
                .iter()
                .map(|sub| clone_value_for_generic(fn_ir, *sub, loop_var_map, memo))
                .collect::<Option<Vec<_>>>()?;
            match subscripts.as_slice() {
                [idx] => fn_ir.blocks[body_bb].instrs.push(Instr::StoreIndex1D {
                    base,
                    idx: *idx,
                    val: value,
                    is_safe: false,
                    is_na_safe: false,
                    is_vector: false,
                    span,
                }),
                [r, c] => fn_ir.blocks[body_bb].instrs.push(Instr::StoreIndex2D {
                    base,
                    r: *r,
                    c: *c,
                    val: value,
                    span,
                }),
                [i, j, k] => fn_ir.blocks[body_bb].instrs.push(Instr::StoreIndex3D {
                    base,
                    i: *i,
                    j: *j,
                    k: *k,
                    val: value,
                    span,
                }),
                _ => return None,
            }
            Some(())
        }
        _ => None,
    }
}

pub(crate) fn build_load(fn_ir: &mut FnIR, var: String) -> ValueId {
    fn_ir.add_value(
        ValueKind::Load { var: var.clone() },
        Span::dummy(),
        Facts::empty(),
        Some(var),
    )
}

pub(crate) fn materialize_symbol_value(
    fn_ir: &mut FnIR,
    symbol: &AffineSymbol,
    loop_var_map: &FxHashMap<String, String>,
) -> ValueId {
    match symbol {
        AffineSymbol::LoopIv(name) => build_load(
            fn_ir,
            loop_var_map
                .get(name)
                .cloned()
                .unwrap_or_else(|| name.clone()),
        ),
        AffineSymbol::Param(name) | AffineSymbol::Invariant(name) => {
            if let Some(index) = fn_ir.params.iter().position(|param| param == name) {
                fn_ir.add_value(
                    ValueKind::Param { index },
                    Span::dummy(),
                    Facts::empty(),
                    Some(name.clone()),
                )
            } else {
                build_load(fn_ir, name.clone())
            }
        }
        AffineSymbol::Length(name) => {
            let base = if let Some(index) = fn_ir.params.iter().position(|param| param == name) {
                fn_ir.add_value(
                    ValueKind::Param { index },
                    Span::dummy(),
                    Facts::empty(),
                    Some(name.clone()),
                )
            } else {
                build_load(fn_ir, name.clone())
            };
            fn_ir.add_value(ValueKind::Len { base }, Span::dummy(), Facts::empty(), None)
        }
    }
}

pub(crate) fn materialize_affine_expr(
    fn_ir: &mut FnIR,
    expr: &AffineExpr,
    loop_var_map: &FxHashMap<String, String>,
) -> Option<ValueId> {
    let mut acc: Option<ValueId> = None;
    if expr.constant != 0 || expr.terms.is_empty() {
        acc = Some(fn_ir.add_value(
            ValueKind::Const(Lit::Int(expr.constant)),
            Span::dummy(),
            Facts::empty(),
            None,
        ));
    }
    for (symbol, coeff) in &expr.terms {
        let base = materialize_symbol_value(fn_ir, symbol, loop_var_map);
        let term = if *coeff == 1 {
            base
        } else {
            let coeff_val = fn_ir.add_value(
                ValueKind::Const(Lit::Int(*coeff)),
                Span::dummy(),
                Facts::empty(),
                None,
            );
            fn_ir.add_value(
                ValueKind::Binary {
                    op: BinOp::Mul,
                    lhs: base,
                    rhs: coeff_val,
                },
                Span::dummy(),
                Facts::empty(),
                None,
            )
        };
        acc = Some(match acc {
            None => term,
            Some(lhs) => fn_ir.add_value(
                ValueKind::Binary {
                    op: BinOp::Add,
                    lhs,
                    rhs: term,
                },
                Span::dummy(),
                Facts::empty(),
                None,
            ),
        });
    }
    acc
}
