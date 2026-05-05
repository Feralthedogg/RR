use super::*;

pub(in crate::mir::opt::v_opt) fn match_expr_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    if loop_has_inner_branch(fn_ir, lp) {
        return None;
    }
    let trace_enabled = vectorize_trace_enabled();
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let mut found: Vec<(ValueId, ValueId)> = Vec::new();
    let mut cube_candidate: Option<(ValueId, ValueId, CubeSliceIndexInfo)> = None;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { .. } => {
                    if trace_enabled {
                        eprintln!("   [vec-expr-map] reject: saw Eval in loop body");
                    }
                    return None;
                }
                Instr::StoreIndex2D { .. }
                | Instr::StoreIndex3D { .. }
                | Instr::UnsafeRBlock { .. } => return None,
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_vector,
                    ..
                } => {
                    let candidate = classify_expr_map_store_candidate(
                        fn_ir,
                        lp,
                        user_call_whitelist,
                        ExprMapStoreSpec {
                            base: *base,
                            idx: *idx,
                            expr: *val,
                            is_vector: *is_vector,
                        },
                        iv_phi,
                    )?;

                    match candidate {
                        ExprMapStoreCandidate::Standard { dest, expr } => {
                            if cube_candidate.is_some()
                                || found.iter().any(|(existing_dest, _)| {
                                    match (
                                        resolve_base_var(fn_ir, *existing_dest),
                                        resolve_base_var(fn_ir, dest),
                                    ) {
                                        (Some(a), Some(b)) => a == b,
                                        _ => same_base_value(fn_ir, *existing_dest, dest),
                                    }
                                })
                            {
                                trace_expr_map_duplicate_store_reject();
                                return None;
                            }
                            found.push((dest, expr));
                        }
                        ExprMapStoreCandidate::Cube { dest, expr, cube } => {
                            if !found.is_empty() || cube_candidate.is_some() {
                                trace_expr_map_duplicate_store_reject();
                                return None;
                            }
                            cube_candidate = Some((dest, expr, cube));
                        }
                    }
                }
            }
        }
    }

    finalize_expr_map_plan(fn_ir, lp, iv_phi, start, end, found, cube_candidate)
}

pub(crate) fn is_builtin_vector_safe_call(callee: &str, arity: usize) -> bool {
    let callee = callee.strip_prefix("base::").unwrap_or(callee);
    match callee {
        "abs" | "sqrt" | "exp" | "log" | "log10" | "log2" | "sin" | "cos" | "tan" | "asin"
        | "acos" | "atan" | "sinh" | "cosh" | "tanh" | "sign" | "floor" | "ceiling" | "trunc"
        | "gamma" | "lgamma" | "is.na" | "is.finite" | "seq_len" => arity == 1,
        "rr_wrap_index_vec" | "rr_wrap_index_vec_i" => arity == 4 || arity == 5,
        "rr_idx_cube_vec_i" => arity == 4 || arity == 5,
        "rr_index_vec_floor" => arity == 1 || arity == 2,
        "rr_index1_read_vec" | "rr_index1_read_vec_floor" | "rr_gather" => arity == 2 || arity == 3,
        "atan2" => arity == 2,
        "round" => arity == 1 || arity == 2,
        "pmax" | "pmin" => arity >= 2,
        _ => false,
    }
}
