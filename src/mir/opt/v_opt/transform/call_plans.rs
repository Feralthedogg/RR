#[allow(clippy::too_many_arguments)]
pub(super) fn apply_recurrence_add_const_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: RecurrenceAddConstApplyPlan,
) -> bool {
    let Some(base_var) = resolve_base_var(fn_ir, plan.base) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let delta_val = if plan.negate_delta {
        fn_ir.add_value(
            ValueKind::Unary {
                op: crate::syntax::ast::UnaryOp::Neg,
                rhs: plan.delta,
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        )
    } else {
        plan.delta
    };
    let recur_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_recur_add_const".to_string(),
            args: vec![plan.base, plan.range.start, end, delta_val],
            names: vec![None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, base_var, recur_val)
}

pub(super) fn apply_recurrence_add_const_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: RecurrenceAddConst3DApplyPlan,
) -> bool {
    let Some(base_var) = resolve_base_var(fn_ir, plan.base) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let delta_val = if plan.negate_delta {
        fn_ir.add_value(
            ValueKind::Unary {
                op: crate::syntax::ast::UnaryOp::Neg,
                rhs: plan.delta,
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        )
    } else {
        plan.delta
    };
    let helper = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_recur_add_const",
        Axis3D::Dim2 => "rr_dim2_recur_add_const",
        Axis3D::Dim3 => "rr_dim3_recur_add_const",
    };
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let recur_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: vec![
                plan.base,
                fixed_a,
                fixed_b,
                plan.range.start,
                end,
                delta_val,
            ],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, base_var, recur_val)
}

pub(super) fn apply_shifted_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: ShiftedMapApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let src_start = add_int_offset(fn_ir, plan.range.start, plan.offset);
    let src_end = add_int_offset(fn_ir, end, plan.offset);
    let shifted_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_shift_assign".to_string(),
            args: vec![
                plan.dest,
                plan.src,
                plan.range.start,
                end,
                src_start,
                src_end,
            ],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, shifted_val)
}

pub(super) fn apply_shifted_map_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: ShiftedMap3DApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let src_start = add_int_offset(fn_ir, plan.range.start, plan.offset);
    let src_end = add_int_offset(fn_ir, end, plan.offset);
    let helper = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_shift_assign",
        Axis3D::Dim2 => "rr_dim2_shift_assign",
        Axis3D::Dim3 => "rr_dim3_shift_assign",
    };
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let shifted_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: vec![
                plan.dest,
                plan.src,
                fixed_a,
                fixed_b,
                plan.range.start,
                end,
                src_start,
                src_end,
            ],
            names: vec![None, None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, shifted_val)
}

pub(super) fn emit_call_map_argument_guards(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    dest: ValueId,
    whole_dest: bool,
    mapped_args: &[(ValueId, bool)],
    vector_args: &[ValueId],
) {
    for (arg, is_vec) in mapped_args {
        let check_val = if whole_dest && *is_vec && !same_length_proven(fn_ir, dest, *arg) {
            Some(fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_same_len".to_string(),
                    args: vec![dest, *arg],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ))
        } else if !*is_vec && !is_const_number(fn_ir, *arg) {
            Some(fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_same_or_scalar".to_string(),
                    args: vec![dest, *arg],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ))
        } else {
            None
        };
        if let Some(val) = check_val {
            fn_ir.blocks[preheader].instrs.push(Instr::Eval {
                val,
                span: crate::utils::Span::dummy(),
            });
        }
    }

    for i in 0..vector_args.len() {
        for j in (i + 1)..vector_args.len() {
            let a = vector_args[i];
            let b = vector_args[j];
            if same_length_proven(fn_ir, a, b) {
                continue;
            }
            let check_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_same_len".to_string(),
                    args: vec![a, b],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            fn_ir.blocks[preheader].instrs.push(Instr::Eval {
                val: check_val,
                span: crate::utils::Span::dummy(),
            });
        }
    }
}

pub(super) fn build_int_vector_literal(fn_ir: &mut FnIR, items: &[i64]) -> ValueId {
    let args: Vec<ValueId> = items
        .iter()
        .map(|item| {
            fn_ir.add_value(
                ValueKind::Const(Lit::Int(*item)),
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            )
        })
        .collect();
    fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args,
            names: vec![None; items.len()],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_call_map_auto_value(
    fn_ir: &mut FnIR,
    dest: ValueId,
    start: ValueId,
    end: ValueId,
    callee: &str,
    helper_cost: u32,
    mapped_args: &[(ValueId, bool)],
    whole_dest: bool,
) -> ValueId {
    let callee_val = fn_ir.add_value(
        ValueKind::Const(Lit::Str(callee.to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let helper_cost_val = fn_ir.add_value(
        ValueKind::Const(Lit::Int(helper_cost as i64)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let vector_slots: Vec<i64> = mapped_args
        .iter()
        .enumerate()
        .filter_map(|(index, (_, is_vec))| is_vec.then_some((index + 1) as i64))
        .collect();
    let vector_slots_val = build_int_vector_literal(fn_ir, &vector_slots);
    let mut args = if whole_dest {
        vec![dest, callee_val, helper_cost_val, vector_slots_val]
    } else {
        vec![
            dest,
            start,
            end,
            callee_val,
            helper_cost_val,
            vector_slots_val,
        ]
    };
    args.extend(mapped_args.iter().map(|(arg, _)| *arg));
    let callee_name = if whole_dest {
        "rr_call_map_whole_auto"
    } else {
        "rr_call_map_slice_auto"
    };
    fn_ir.add_value(
        ValueKind::Call {
            callee: callee_name.to_string(),
            args,
            names: vec![None; mapped_args.len() + if whole_dest { 4 } else { 6 }],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_call_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    dest: ValueId,
    callee: String,
    args: Vec<CallMapArg>,
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
    whole_dest: bool,
    shadow_vars: Vec<VarId>,
) -> bool {
    let whole_dest = whole_dest && lp.limit_adjust == 0;
    let trace_enabled = vectorize_trace_enabled();
    let lowering_mode = choose_call_map_lowering(fn_ir, &callee, &args, whole_dest, &shadow_vars);
    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
    let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
        return false;
    };
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let mut mapped_args = Vec::with_capacity(args.len());
    let mut vector_args = Vec::new();
    for (arg_i, arg) in args.into_iter().enumerate() {
        let out = if arg.vectorized {
            match materialize_vector_expr(
                fn_ir,
                arg.value,
                iv_phi,
                idx_vec,
                lp,
                &mut memo,
                &mut interner,
                true,
                false,
            ) {
                Some(v) => v,
                None => return false,
            }
        } else {
            resolve_materialized_value(fn_ir, arg.value)
        };
        let out = if arg.vectorized {
            maybe_hoist_callmap_arg_expr(fn_ir, site.preheader, out, arg_i)
        } else {
            out
        };
        if arg.vectorized {
            vector_args.push(out);
        }
        mapped_args.push((out, arg.vectorized));
    }

    emit_call_map_argument_guards(
        fn_ir,
        site.preheader,
        dest,
        whole_dest,
        &mapped_args,
        &vector_args,
    );

    let out_val = match lowering_mode {
        CallMapLoweringMode::RuntimeAuto { helper_cost } => {
            if trace_enabled {
                eprintln!(
                    "   [vec-profit] {} call_map runtime-auto callee={} helper_cost={} whole_dest={}",
                    fn_ir.name, callee, helper_cost, whole_dest
                );
            }
            build_call_map_auto_value(
                fn_ir,
                dest,
                start,
                end,
                &callee,
                helper_cost,
                &mapped_args,
                whole_dest,
            )
        }
        CallMapLoweringMode::DirectVector => {
            let mapped_args_vals: Vec<ValueId> = mapped_args.iter().map(|(arg, _)| *arg).collect();
            let mapped_val = fn_ir.add_value(
                if let Some(op) = intrinsic_for_call(&callee, mapped_args_vals.len()) {
                    ValueKind::Intrinsic {
                        op,
                        args: mapped_args_vals,
                    }
                } else {
                    ValueKind::Call {
                        callee,
                        args: mapped_args_vals,
                        names: vec![None; mapped_args.len()],
                    }
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            if whole_dest {
                mapped_val
            } else {
                let mapped_val = prepare_partial_slice_value(fn_ir, dest, mapped_val, start, end);
                fn_ir.add_value(
                    ValueKind::Call {
                        callee: "rr_assign_slice".to_string(),
                        args: vec![dest, start, end, mapped_val],
                        names: vec![None, None, None, None],
                    },
                    crate::utils::Span::dummy(),
                    crate::mir::def::Facts::empty(),
                    None,
                )
            }
        }
    };
    finish_vector_assignment_with_shadow_states(
        fn_ir,
        site,
        dest_var,
        out_val,
        &shadow_vars,
        Some(end),
    )
}

pub(super) fn apply_call_map_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: CallMap3DApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let Some(iv_phi) = lp.iv.as_ref().map(|iv| iv.phi_val) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let helper = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_call_assign",
        Axis3D::Dim2 => "rr_dim2_call_assign",
        Axis3D::Dim3 => "rr_dim3_call_assign",
    };
    let callee_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(plan.callee)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let mut mapped_args = vec![
        plan.dest,
        callee_lit,
        fixed_a,
        fixed_b,
        plan.range.start,
        end,
    ];
    for (arg_i, arg) in plan.args.into_iter().enumerate() {
        let out = if arg.vectorized {
            match materialize_vector_expr(
                fn_ir,
                arg.value,
                iv_phi,
                idx_vec,
                lp,
                &mut memo,
                &mut interner,
                true,
                false,
            ) {
                Some(v) => v,
                None => return false,
            }
        } else {
            resolve_materialized_value(fn_ir, arg.value)
        };
        let out = if arg.vectorized {
            maybe_hoist_callmap_arg_expr(fn_ir, site.preheader, out, arg_i)
        } else {
            out
        };
        mapped_args.push(out);
    }
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: mapped_args.clone(),
            names: vec![None; mapped_args.len()],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}

pub(super) fn apply_call_map_3d_general_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: CallMap3DGeneralApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let idx_seed = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let mut mapped_args = Vec::with_capacity(plan.args.len());
    for arg in plan.args {
        let out = match materialize_vector_or_scalar_expr(
            fn_ir,
            lp,
            arg.value,
            plan.iv_phi,
            idx_seed,
            &mut memo,
            &mut interner,
        ) {
            Some(v) => v,
            None => return false,
        };
        mapped_args.push(out);
    }
    let arg_len = mapped_args.len();
    let call_val = fn_ir.add_value(
        if let Some(op) = intrinsic_for_call(&plan.callee, arg_len) {
            ValueKind::Intrinsic {
                op,
                args: mapped_args.clone(),
            }
        } else {
            ValueKind::Call {
                callee: plan.callee,
                args: mapped_args,
                names: vec![None; arg_len],
            }
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let helper = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_assign_values",
        Axis3D::Dim2 => "rr_dim2_assign_values",
        Axis3D::Dim3 => "rr_dim3_assign_values",
    };
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: vec![plan.dest, call_val, fixed_a, fixed_b, plan.range.start, end],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}
