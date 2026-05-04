use super::*;

pub(crate) struct CubeSliceIndexInfo {
    pub(crate) face: ValueId,
    pub(crate) row: ValueId,
    pub(crate) size: ValueId,
    pub(crate) ctx: Option<ValueId>,
}

pub(crate) fn match_cube_slice_index_info(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
    idx: ValueId,
    iv_phi: ValueId,
) -> Option<CubeSliceIndexInfo> {
    let trace_enabled = vectorize_trace_enabled();
    let idx_root = resolve_match_alias_value(fn_ir, idx);
    let ValueKind::Call {
        callee,
        args,
        names,
    } = &fn_ir.values[idx_root].kind
    else {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: store index is not call ({:?})",
                fn_ir.name, fn_ir.values[idx_root].kind
            );
        }
        return None;
    };
    if callee != "rr_idx_cube_vec_i" || (args.len() != 4 && args.len() != 5) {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: index call shape mismatch (callee={}, arity={})",
                fn_ir.name,
                callee,
                args.len()
            );
        }
        return None;
    }
    if names.iter().any(|n| n.is_some()) {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: named args in rr_idx_cube_vec_i",
                fn_ir.name
            );
        }
        return None;
    }

    let y_arg = args[2];
    let y_ok = is_iv_equivalent(fn_ir, y_arg, iv_phi)
        || is_origin_var_iv_alias_in_loop(fn_ir, lp, y_arg, iv_phi)
        || is_floor_like_iv_expr(
            fn_ir,
            y_arg,
            iv_phi,
            &mut FxHashSet::default(),
            &mut FxHashSet::default(),
            0,
        );
    if !y_ok {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: y arg is not IV-equivalent (iv={:?}, iv_origin={:?}, y={:?}, y_origin={:?})",
                fn_ir.name,
                fn_ir.values[iv_phi].kind,
                induction_origin_var(fn_ir, iv_phi),
                fn_ir.values[y_arg].kind,
                fn_ir.values[canonical_value(fn_ir, y_arg)].origin_var
            );
        }
        return None;
    }

    for arg in [args[0], args[1], args[3]] {
        if expr_has_iv_dependency(fn_ir, arg, iv_phi)
            || !is_loop_invariant_scalar_expr(fn_ir, arg, iv_phi, user_call_whitelist)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cube-slice] {} reject: non-invariant cube arg ({:?}) dep={}",
                    fn_ir.name,
                    fn_ir.values[arg].kind,
                    expr_has_iv_dependency(fn_ir, arg, iv_phi)
                );
            }
            return None;
        }
    }
    let ctx = if args.len() == 5 {
        let arg = args[4];
        if expr_has_iv_dependency(fn_ir, arg, iv_phi)
            || !is_loop_invariant_scalar_expr(fn_ir, arg, iv_phi, user_call_whitelist)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cube-slice] {} reject: non-invariant cube ctx arg ({:?}) dep={}",
                    fn_ir.name,
                    fn_ir.values[arg].kind,
                    expr_has_iv_dependency(fn_ir, arg, iv_phi)
                );
            }
            return None;
        }
        Some(arg)
    } else {
        None
    };

    Some(CubeSliceIndexInfo {
        face: args[0],
        row: args[1],
        size: args[3],
        ctx,
    })
}
