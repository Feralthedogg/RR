fn emit_matrix_map_guards(
    fn_ir: &mut FnIR,
    preheader: crate::mir::BlockId,
    operands: &[MatrixMapOperands],
) {
    for operand in operands {
        emit_same_matrix_shape_or_scalar_guard(fn_ir, preheader, operand.dest, operand.lhs_src);
        emit_same_matrix_shape_or_scalar_guard(fn_ir, preheader, operand.dest, operand.rhs_src);
    }
}

fn emit_array3_map_guards(
    fn_ir: &mut FnIR,
    preheader: crate::mir::BlockId,
    operands: &[Array3MapOperands],
) {
    for operand in operands {
        emit_same_array3_shape_or_scalar_guard(fn_ir, preheader, operand.dest, operand.lhs_src);
        emit_same_array3_shape_or_scalar_guard(fn_ir, preheader, operand.dest, operand.rhs_src);
    }
}

fn build_guard_bool(fn_ir: &mut FnIR, callee: &str, lhs: ValueId, rhs: ValueId) -> ValueId {
    fn_ir.add_value(
        ValueKind::Call {
            callee: callee.to_string(),
            args: vec![lhs, rhs],
            names: vec![None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

fn combine_guard_terms(fn_ir: &mut FnIR, terms: Vec<ValueId>) -> Option<ValueId> {
    let mut iter = terms.into_iter();
    let mut acc = iter.next()?;
    for term in iter {
        acc = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::And,
                lhs: acc,
                rhs: term,
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
    }
    Some(acc)
}

fn build_matrix_map_guard_cond(
    fn_ir: &mut FnIR,
    operands: &[MatrixMapOperands],
) -> Option<ValueId> {
    let mut terms = Vec::new();
    for operand in operands {
        terms.push(build_guard_bool(
            fn_ir,
            "rr_can_same_matrix_shape_or_scalar",
            operand.dest,
            operand.lhs_src,
        ));
        terms.push(build_guard_bool(
            fn_ir,
            "rr_can_same_matrix_shape_or_scalar",
            operand.dest,
            operand.rhs_src,
        ));
    }
    combine_guard_terms(fn_ir, terms)
}

fn build_vector_map_guard_cond(
    fn_ir: &mut FnIR,
    operands: &[VectorMapOperands],
) -> Option<ValueId> {
    let mut terms = Vec::new();
    for operand in operands {
        terms.push(build_guard_bool(
            fn_ir,
            "rr_can_same_or_scalar",
            operand.dest,
            operand.lhs_src,
        ));
        terms.push(build_guard_bool(
            fn_ir,
            "rr_can_same_or_scalar",
            operand.dest,
            operand.rhs_src,
        ));
    }
    combine_guard_terms(fn_ir, terms)
}

fn build_vector_reduce_guard_cond(
    fn_ir: &mut FnIR,
    operands: &[VectorReduceOperands],
) -> Option<ValueId> {
    let mut terms = Vec::new();
    for operand in operands {
        terms.push(fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_can_reduce_range".to_string(),
                args: vec![operand.base, operand.start, operand.end],
                names: vec![None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ));
    }
    combine_guard_terms(fn_ir, terms)
}

fn build_array3_map_guard_cond(
    fn_ir: &mut FnIR,
    operands: &[Array3MapOperands],
) -> Option<ValueId> {
    let mut terms = Vec::new();
    for operand in operands {
        terms.push(build_guard_bool(
            fn_ir,
            "rr_can_same_array3_shape_or_scalar",
            operand.dest,
            operand.lhs_src,
        ));
        terms.push(build_guard_bool(
            fn_ir,
            "rr_can_same_array3_shape_or_scalar",
            operand.dest,
            operand.rhs_src,
        ));
    }
    combine_guard_terms(fn_ir, terms)
}

fn build_matrix_rect_reduce_guard_cond(
    fn_ir: &mut FnIR,
    operands: &[MatrixRectReduceOperands],
) -> Option<ValueId> {
    let mut terms = Vec::new();
    for operand in operands {
        terms.push(fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_can_matrix_reduce_rect".to_string(),
                args: vec![
                    operand.base,
                    operand.r_start,
                    operand.r_end,
                    operand.c_start,
                    operand.c_end,
                ],
                names: vec![None, None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ));
    }
    combine_guard_terms(fn_ir, terms)
}

fn build_matrix_col_reduce_guard_cond(
    fn_ir: &mut FnIR,
    operands: &[MatrixColReduceOperands],
) -> Option<ValueId> {
    let mut terms = Vec::new();
    for operand in operands {
        terms.push(fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_can_col_reduce_range".to_string(),
                args: vec![operand.base, operand.col, operand.start, operand.end],
                names: vec![None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ));
    }
    combine_guard_terms(fn_ir, terms)
}

fn build_array3_dim1_reduce_guard_cond(
    fn_ir: &mut FnIR,
    operands: &[Array3Dim1ReduceOperands],
) -> Option<ValueId> {
    let mut terms = Vec::new();
    for operand in operands {
        terms.push(fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_can_dim1_reduce_range".to_string(),
                args: vec![
                    operand.base,
                    operand.fixed_a,
                    operand.fixed_b,
                    operand.start,
                    operand.end,
                ],
                names: vec![None, None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ));
    }
    combine_guard_terms(fn_ir, terms)
}

fn build_array3_cube_reduce_guard_cond(
    fn_ir: &mut FnIR,
    operands: &[Array3CubeReduceOperands],
) -> Option<ValueId> {
    let mut terms = Vec::new();
    for operand in operands {
        terms.push(fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_can_array3_reduce_cube".to_string(),
                args: vec![
                    operand.base,
                    operand.i_start,
                    operand.i_end,
                    operand.j_start,
                    operand.j_end,
                    operand.k_start,
                    operand.k_end,
                ],
                names: vec![None, None, None, None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ));
    }
    combine_guard_terms(fn_ir, terms)
}
