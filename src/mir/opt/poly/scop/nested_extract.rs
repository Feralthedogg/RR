use super::*;
pub(crate) fn extract_triply_nested_scop_region(
    fn_ir: &FnIR,
    outer: &LoopInfo,
    middle: &LoopInfo,
    inner: &LoopInfo,
    all_loops: &[LoopInfo],
) -> Result<ScopRegion, ScopExtractionFailure> {
    if !direct_nested_loops(inner, all_loops).is_empty() {
        return Err(ScopExtractionFailure::UnsupportedNestedLoop);
    }
    let outer_iv = outer
        .iv
        .as_ref()
        .ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let middle_iv = middle
        .iv
        .as_ref()
        .ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let inner_iv = inner
        .iv
        .as_ref()
        .ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let outer_step = signed_step(outer).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let middle_step = signed_step(middle).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let inner_step = signed_step(inner).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    if outer_step == 0 || middle_step == 0 || inner_step == 0 {
        return Err(ScopExtractionFailure::MissingInductionVar);
    }

    for bid in outer
        .body
        .iter()
        .copied()
        .filter(|bid| !middle.body.contains(bid))
    {
        let block = &fn_ir.blocks[bid];
        for instr in &block.instrs {
            match instr {
                Instr::Assign { src, .. } => {
                    if try_lift_affine_expr(fn_ir, *src, outer).is_none()
                        && try_lift_affine_expr(fn_ir, *src, middle).is_none()
                        && try_lift_affine_expr(fn_ir, *src, inner).is_none()
                    {
                        return Err(ScopExtractionFailure::UnsupportedNestedLoop);
                    }
                }
                Instr::Eval { .. }
                | Instr::StoreIndex1D { .. }
                | Instr::StoreIndex2D { .. }
                | Instr::StoreIndex3D { .. }
                | Instr::UnsafeRBlock { .. } => {
                    return Err(ScopExtractionFailure::UnsupportedNestedLoop);
                }
            }
        }
    }
    for bid in middle
        .body
        .iter()
        .copied()
        .filter(|bid| !inner.body.contains(bid))
    {
        let block = &fn_ir.blocks[bid];
        for instr in &block.instrs {
            match instr {
                Instr::Assign { src, .. } => {
                    if try_lift_affine_expr(fn_ir, *src, outer).is_none()
                        && try_lift_affine_expr(fn_ir, *src, middle).is_none()
                        && try_lift_affine_expr(fn_ir, *src, inner).is_none()
                    {
                        return Err(ScopExtractionFailure::UnsupportedNestedLoop);
                    }
                }
                Instr::Eval { .. }
                | Instr::StoreIndex1D { .. }
                | Instr::StoreIndex2D { .. }
                | Instr::StoreIndex3D { .. }
                | Instr::UnsafeRBlock { .. } => {
                    return Err(ScopExtractionFailure::UnsupportedNestedLoop);
                }
            }
        }
    }

    let outer_lower = try_lift_affine_expr(fn_ir, outer_iv.init_val, outer)
        .ok_or(ScopExtractionFailure::NonAffineLoopBound)?;
    let outer_limit =
        choose_affine_loop_limit(fn_ir, outer).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let mut outer_upper = try_lift_affine_expr(fn_ir, outer_limit, outer)
        .ok_or(ScopExtractionFailure::NonAffineLoopBound)?;
    outer_upper.constant += outer.limit_adjust;

    let middle_lower = try_lift_affine_expr(fn_ir, middle_iv.init_val, middle)
        .ok_or(ScopExtractionFailure::NonAffineLoopBound)?;
    let middle_limit = choose_affine_loop_limit(fn_ir, middle)
        .ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let mut middle_upper = try_lift_affine_expr(fn_ir, middle_limit, middle)
        .ok_or(ScopExtractionFailure::NonAffineLoopBound)?;
    middle_upper.constant += middle.limit_adjust;

    let inner_lower = try_lift_affine_expr(fn_ir, inner_iv.init_val, inner)
        .ok_or(ScopExtractionFailure::NonAffineLoopBound)?;
    let inner_limit =
        choose_affine_loop_limit(fn_ir, inner).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let mut inner_upper = try_lift_affine_expr(fn_ir, inner_limit, inner)
        .ok_or(ScopExtractionFailure::NonAffineLoopBound)?;
    inner_upper.constant += inner.limit_adjust;

    let mut statements = Vec::new();
    let mut body_blocks: Vec<BlockId> = inner.body.iter().copied().collect();
    body_blocks.sort_unstable();
    let preds = build_pred_map(fn_ir);
    for bid in body_blocks {
        if bid != inner.header
            && matches!(fn_ir.blocks[bid].term, Terminator::If { .. })
            && !is_ignorable_loop_if_block(fn_ir, inner, bid, &inner.body)
        {
            if super::poly_trace_enabled() {
                eprintln!(
                    "   [poly-scop] {} triple header={} reject cfg block={} term={:?} instrs={:#?}",
                    fn_ir.name, inner.header, bid, fn_ir.blocks[bid].term, fn_ir.blocks[bid].instrs
                );
            }
            return Err(ScopExtractionFailure::UnsupportedCfgShape);
        }
        if bid == inner.header && preds.get(&bid).is_some_and(|incoming| incoming.len() > 2) {
            if super::poly_trace_enabled() {
                eprintln!(
                    "   [poly-scop] {} triple header={} reject preds={:?}",
                    fn_ir.name,
                    inner.header,
                    preds.get(&bid)
                );
            }
            return Err(ScopExtractionFailure::UnsupportedCfgShape);
        }
        for instr in &fn_ir.blocks[bid].instrs {
            let stmt = extract_stmt(fn_ir, inner, statements.len(), bid, instr)?;
            statements.push(stmt);
        }
    }

    let outer_name =
        loop_iv_name(fn_ir, outer).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let middle_name =
        loop_iv_name(fn_ir, middle).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let inner_name =
        loop_iv_name(fn_ir, inner).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    normalize_nested_accesses(
        &mut statements,
        &[outer_name.clone(), middle_name.clone(), inner_name.clone()],
    );

    let mut parameters = BTreeSet::new();
    for expr in [
        &outer_lower,
        &outer_upper,
        &middle_lower,
        &middle_upper,
        &inner_lower,
        &inner_upper,
    ] {
        collect_affine_symbols(expr, &mut parameters);
    }
    for stmt in &statements {
        for access in &stmt.accesses {
            for expr in &access.subscripts {
                collect_affine_symbols(expr, &mut parameters);
            }
        }
    }

    let constraints = vec![
        AffineConstraint {
            expr: outer_lower.clone(),
            kind: AffineConstraintKind::LowerBound,
        },
        AffineConstraint {
            expr: outer_upper.clone(),
            kind: AffineConstraintKind::UpperBound,
        },
        AffineConstraint {
            expr: middle_lower.clone(),
            kind: AffineConstraintKind::LowerBound,
        },
        AffineConstraint {
            expr: middle_upper.clone(),
            kind: AffineConstraintKind::UpperBound,
        },
        AffineConstraint {
            expr: inner_lower.clone(),
            kind: AffineConstraintKind::LowerBound,
        },
        AffineConstraint {
            expr: inner_upper.clone(),
            kind: AffineConstraintKind::UpperBound,
        },
    ];

    Ok(ScopRegion {
        header: outer.header,
        latch: outer.latch,
        exits: outer.exits.clone(),
        dimensions: vec![
            LoopDimension {
                iv_name: outer_name.clone(),
                lower_bound: outer_lower,
                upper_bound: outer_upper,
                step: outer_step,
            },
            LoopDimension {
                iv_name: middle_name.clone(),
                lower_bound: middle_lower,
                upper_bound: middle_upper,
                step: middle_step,
            },
            LoopDimension {
                iv_name: inner_name.clone(),
                lower_bound: inner_lower,
                upper_bound: inner_upper,
                step: inner_step,
            },
        ],
        iteration_space: PresburgerSet::new(vec![outer_name, middle_name, inner_name], constraints),
        parameters,
        statements,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::mir::ValueKind;
    use crate::mir::flow::Facts;
    use crate::mir::opt::loop_analysis::LoopAnalyzer;
    use crate::syntax::ast::BinOp;
    use crate::utils::Span;

    fn build_simple_loop(non_affine_idx: bool) -> FnIR {
        let mut fn_ir = FnIR::new(
            "poly_scop".to_string(),
            vec!["x".to_string(), "y".to_string(), "ind".to_string()],
        );
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let body = fn_ir.add_block();
        let exit = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let x = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            None,
        );
        let y = fn_ir.add_value(
            ValueKind::Param { index: 1 },
            Span::default(),
            Facts::empty(),
            None,
        );
        let ind = fn_ir.add_value(
            ValueKind::Param { index: 2 },
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let limit = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(8)),
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
                rhs: limit,
            },
            Span::default(),
            Facts::empty(),
            None,
        );

        let idx = if non_affine_idx {
            fn_ir.add_value(
                ValueKind::Index1D {
                    base: ind,
                    idx: phi,
                    is_safe: true,
                    is_na_safe: true,
                },
                Span::default(),
                Facts::empty(),
                None,
            )
        } else {
            phi
        };

        let read = fn_ir.add_value(
            ValueKind::Index1D {
                base: x,
                idx,
                is_safe: true,
                is_na_safe: true,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let plus_one = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: read,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[body].instrs.push(Instr::StoreIndex1D {
            base: y,
            idx,
            val: plus_one,
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

        fn_ir.blocks[entry].term = Terminator::Goto(header);
        fn_ir.blocks[header].term = Terminator::If {
            cond,
            then_bb: body,
            else_bb: exit,
        };
        fn_ir.blocks[body].term = Terminator::Goto(header);
        fn_ir.blocks[exit].term = Terminator::Return(None);
        fn_ir
    }

    fn build_simple_loop_with_empty_guard_if() -> FnIR {
        let mut fn_ir = FnIR::new(
            "poly_scop_guard".to_string(),
            vec!["x".to_string(), "y".to_string()],
        );
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let guard = fn_ir.add_block();
        let then_bb = fn_ir.add_block();
        let else_bb = fn_ir.add_block();
        let body = fn_ir.add_block();
        let exit = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let x = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            None,
        );
        let y = fn_ir.add_value(
            ValueKind::Param { index: 1 },
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let limit = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(8)),
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
                rhs: limit,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let guard_cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Le,
                lhs: one,
                rhs: limit,
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
        let plus_one = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: read,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[body].instrs.push(Instr::StoreIndex1D {
            base: y,
            idx: phi,
            val: plus_one,
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

        fn_ir.blocks[entry].term = Terminator::Goto(header);
        fn_ir.blocks[header].term = Terminator::If {
            cond,
            then_bb: guard,
            else_bb: exit,
        };
        fn_ir.blocks[guard].term = Terminator::If {
            cond: guard_cond,
            then_bb,
            else_bb,
        };
        fn_ir.blocks[then_bb].term = Terminator::Goto(body);
        fn_ir.blocks[else_bb].term = Terminator::Goto(body);
        fn_ir.blocks[body].term = Terminator::Goto(header);
        fn_ir.blocks[exit].term = Terminator::Return(None);
        fn_ir
    }

    fn build_simple_loop_with_affine_guard_assigns() -> FnIR {
        let mut fn_ir = FnIR::new(
            "poly_scop_guard_assign".to_string(),
            vec!["x".to_string(), "y".to_string()],
        );
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let guard = fn_ir.add_block();
        let then_bb = fn_ir.add_block();
        let else_bb = fn_ir.add_block();
        let body = fn_ir.add_block();
        let exit = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let x = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            None,
        );
        let y = fn_ir.add_value(
            ValueKind::Param { index: 1 },
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let limit = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(8)),
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
                rhs: limit,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let guard_cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Le,
                lhs: one,
                rhs: limit,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let affine_tmp = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: phi,
                rhs: one,
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
        let plus_one = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: read,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[then_bb].instrs.push(Instr::Assign {
            dst: "tmp".to_string(),
            src: affine_tmp,
            span: Span::default(),
        });
        fn_ir.blocks[else_bb].instrs.push(Instr::Assign {
            dst: "tmp".to_string(),
            src: affine_tmp,
            span: Span::default(),
        });
        fn_ir.blocks[body].instrs.push(Instr::StoreIndex1D {
            base: y,
            idx: phi,
            val: plus_one,
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

        fn_ir.blocks[entry].term = Terminator::Goto(header);
        fn_ir.blocks[header].term = Terminator::If {
            cond,
            then_bb: guard,
            else_bb: exit,
        };
        fn_ir.blocks[guard].term = Terminator::If {
            cond: guard_cond,
            then_bb,
            else_bb,
        };
        fn_ir.blocks[then_bb].term = Terminator::Goto(body);
        fn_ir.blocks[else_bb].term = Terminator::Goto(body);
        fn_ir.blocks[body].term = Terminator::Goto(header);
        fn_ir.blocks[exit].term = Terminator::Return(None);
        fn_ir
    }

    fn build_simple_loop_with_affine_guard_preamble() -> FnIR {
        let mut fn_ir = FnIR::new(
            "poly_scop_guard_preamble".to_string(),
            vec!["x".to_string(), "y".to_string()],
        );
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let guard = fn_ir.add_block();
        let then_bb = fn_ir.add_block();
        let else_bb = fn_ir.add_block();
        let body = fn_ir.add_block();
        let exit = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let x = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            None,
        );
        let y = fn_ir.add_value(
            ValueKind::Param { index: 1 },
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let limit = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(8)),
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
                rhs: limit,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let guard_cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Le,
                lhs: one,
                rhs: limit,
            },
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
        let alias = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: phi,
                rhs: zero,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let affine_tmp = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: phi,
                rhs: one,
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
        let plus_one = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: read,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );

        fn_ir.blocks[guard].instrs.push(Instr::Assign {
            dst: "ii".to_string(),
            src: alias,
            span: Span::default(),
        });
        fn_ir.blocks[then_bb].instrs.push(Instr::Assign {
            dst: "tmp".to_string(),
            src: affine_tmp,
            span: Span::default(),
        });
        fn_ir.blocks[else_bb].instrs.push(Instr::Assign {
            dst: "tmp".to_string(),
            src: affine_tmp,
            span: Span::default(),
        });
        fn_ir.blocks[body].instrs.push(Instr::StoreIndex1D {
            base: y,
            idx: phi,
            val: plus_one,
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

        fn_ir.blocks[entry].term = Terminator::Goto(header);
        fn_ir.blocks[header].term = Terminator::If {
            cond,
            then_bb: guard,
            else_bb: exit,
        };
        fn_ir.blocks[guard].term = Terminator::If {
            cond: guard_cond,
            then_bb,
            else_bb,
        };
        fn_ir.blocks[then_bb].term = Terminator::Goto(body);
        fn_ir.blocks[else_bb].term = Terminator::Goto(body);
        fn_ir.blocks[body].term = Terminator::Goto(header);
        fn_ir.blocks[exit].term = Terminator::Return(None);
        fn_ir
    }

    #[test]
    fn extracts_simple_affine_scop() {
        let fn_ir = build_simple_loop(false);
        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        let scop = extract_scop_region(&fn_ir, &loops[0], &loops).expect("expected SCoP");
        assert_eq!(scop.dimensions.len(), 1);
        assert_eq!(scop.statements.len(), 1);
        assert_eq!(scop.statements[0].accesses.len(), 2);
    }

    #[test]
    fn rejects_non_affine_indirect_index() {
        let fn_ir = build_simple_loop(true);
        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        let err = extract_scop_region(&fn_ir, &loops[0], &loops).expect_err("expected reject");
        assert_eq!(err, ScopExtractionFailure::NonAffineAccess);
    }

    #[test]
    fn extracts_scop_through_empty_guard_if() {
        let fn_ir = build_simple_loop_with_empty_guard_if();
        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        let scop = extract_scop_region(&fn_ir, &loops[0], &loops).expect("expected SCoP");
        assert_eq!(scop.dimensions.len(), 1);
        assert_eq!(scop.statements.len(), 1);
        assert_eq!(scop.statements[0].accesses.len(), 2);
    }

    #[test]
    fn extracts_scop_through_affine_guard_assign_branches() {
        let fn_ir = build_simple_loop_with_affine_guard_assigns();
        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        let scop = extract_scop_region(&fn_ir, &loops[0], &loops).expect("expected SCoP");
        assert_eq!(scop.dimensions.len(), 1);
        assert_eq!(scop.statements.len(), 3);
        assert_eq!(scop.statements[2].accesses.len(), 2);
    }

    #[test]
    fn extracts_scop_through_affine_guard_preamble_and_branches() {
        let fn_ir = build_simple_loop_with_affine_guard_preamble();
        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        let scop = extract_scop_region(&fn_ir, &loops[0], &loops).expect("expected SCoP");
        assert_eq!(scop.dimensions.len(), 1);
        assert_eq!(scop.statements.len(), 4);
        assert_eq!(scop.statements[3].accesses.len(), 2);
    }
}
