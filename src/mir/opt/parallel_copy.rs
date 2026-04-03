use crate::mir::*;
use crate::utils::Span;

#[derive(Clone, Debug)]
pub struct Move {
    pub dst: VarId,
    pub src: ValueId,
}

fn param_runtime_var(fn_ir: &FnIR, index: usize) -> Option<&str> {
    // Parameters are usually rewritten to internal locals like `.arg_x`.
    // Prefer that runtime binding when present, and only fall back to the
    // public parameter name.
    for v in &fn_ir.values {
        if let ValueKind::Param { index: i } = v.kind
            && i == index
        {
            if let Some(name) = v.origin_var.as_deref() {
                return Some(name);
            }
            break;
        }
    }
    fn_ir.params.get(index).map(|s| s.as_str())
}

pub fn emit_parallel_copy(
    fn_ir: &mut FnIR,
    out_instrs: &mut Vec<Instr>,
    moves: Vec<Move>,
    span: Span,
) {
    let mut pending: Vec<Move> = moves
        .into_iter()
        .filter(|m| !move_is_noop(fn_ir, m))
        .collect();
    let mut cycle_idx: usize = 0;
    let mut capture_idx: usize = 0;

    pre_materialize_complex_pending_sources(
        fn_ir,
        out_instrs,
        &mut pending,
        span,
        &mut capture_idx,
    );

    while !pending.is_empty() {
        // Emit an acyclic move first: dst is not consumed by another pending source.
        let mut candidate_idx = None;

        for i in 0..pending.len() {
            let dst_var = &pending[i].dst;

            let mut captured = false;
            for (j, m) in pending.iter().enumerate() {
                if i == j {
                    continue;
                }
                if value_reads_var(fn_ir, m.src, dst_var) {
                    captured = true;
                    break;
                }
            }

            if !captured {
                candidate_idx = Some(i);
                break;
            }
        }

        if let Some(idx) = candidate_idx {
            let m = pending.remove(idx);
            out_instrs.push(Instr::Assign {
                dst: m.dst,
                src: m.src,
                span,
            });
        } else {
            // Cycle break: spill one victim variable into a temporary, rewrite users, then continue.

            let victim_var = pending[0].dst.clone();

            for m in &mut pending {
                if !value_reads_var(fn_ir, m.src, &victim_var)
                    || source_is_cheap_to_rewrite(fn_ir, m.src)
                {
                    continue;
                }

                let temp_var = format!("{}_src_tmp{}", m.dst, capture_idx);
                capture_idx += 1;
                out_instrs.push(Instr::Assign {
                    dst: temp_var.clone(),
                    src: m.src,
                    span,
                });
                m.src = fn_ir.add_value(
                    ValueKind::Load {
                        var: temp_var.clone(),
                    },
                    span,
                    Facts::empty(),
                    Some(temp_var),
                );
            }

            let temp_var = format!("{}_cycle_tmp{}", victim_var, cycle_idx);
            cycle_idx += 1;

            let save_val_id = fn_ir.add_value(
                ValueKind::Load {
                    var: victim_var.clone(),
                },
                span,
                Facts::empty(),
                None,
            );
            out_instrs.push(Instr::Assign {
                dst: temp_var.clone(),
                src: save_val_id,
                span,
            });

            for m in &mut pending {
                if value_reads_var(fn_ir, m.src, &victim_var) {
                    m.src = replace_var_read(fn_ir, m.src, &victim_var, &temp_var);
                }
            }

            let m = pending.remove(0);
            out_instrs.push(Instr::Assign {
                dst: m.dst,
                src: m.src,
                span,
            });
        }
    }
}

fn pre_materialize_complex_pending_sources(
    fn_ir: &mut FnIR,
    out_instrs: &mut Vec<Instr>,
    pending: &mut [Move],
    span: Span,
    capture_idx: &mut usize,
) {
    let pending_dsts: Vec<VarId> = pending.iter().map(|m| m.dst.clone()).collect();
    let mut materialized: std::collections::HashMap<ValueId, VarId> =
        std::collections::HashMap::new();

    for m in pending.iter_mut() {
        if source_is_cheap_to_rewrite(fn_ir, m.src)
            || !pending_dsts
                .iter()
                .any(|dst| value_reads_var(fn_ir, m.src, dst))
        {
            continue;
        }

        if let Some(temp_var) = materialized.get(&m.src).cloned() {
            m.src = fn_ir.add_value(
                ValueKind::Load {
                    var: temp_var.clone(),
                },
                span,
                Facts::empty(),
                Some(temp_var),
            );
            continue;
        }

        let temp_var = format!(".__pc_src_tmp{}", *capture_idx);
        *capture_idx += 1;
        out_instrs.push(Instr::Assign {
            dst: temp_var.clone(),
            src: m.src,
            span,
        });
        materialized.insert(m.src, temp_var.clone());
        m.src = fn_ir.add_value(
            ValueKind::Load {
                var: temp_var.clone(),
            },
            span,
            Facts::empty(),
            Some(temp_var),
        );
    }
}

fn source_is_cheap_to_rewrite(fn_ir: &FnIR, src: ValueId) -> bool {
    matches!(
        fn_ir.values[src].kind,
        ValueKind::Const(_)
            | ValueKind::Load { .. }
            | ValueKind::Param { .. }
            | ValueKind::RSymbol { .. }
    )
}

pub(crate) fn move_is_noop(fn_ir: &FnIR, m: &Move) -> bool {
    match &fn_ir.values[m.src].kind {
        ValueKind::Load { var } => var == &m.dst,
        ValueKind::Param { index } => {
            param_runtime_var(fn_ir, *index).is_some_and(|name| name == m.dst)
        }
        _ => false,
    }
}

fn value_reads_var(fn_ir: &FnIR, src: ValueId, var: &VarId) -> bool {
    let val = &fn_ir.values[src];
    match &val.kind {
        ValueKind::Load { var: v } => v == var,
        ValueKind::Param { index } => {
            if let Some(param_name) = param_runtime_var(fn_ir, *index) {
                return param_name == var;
            }
            false
        }
        ValueKind::Binary { lhs, rhs, .. } => {
            value_reads_var(fn_ir, *lhs, var) || value_reads_var(fn_ir, *rhs, var)
        }
        ValueKind::Unary { rhs, .. } => value_reads_var(fn_ir, *rhs, var),
        ValueKind::Call { args, .. } => args.iter().any(|a| value_reads_var(fn_ir, *a, var)),
        ValueKind::Intrinsic { args, .. } => args.iter().any(|a| value_reads_var(fn_ir, *a, var)),
        ValueKind::Phi { .. } => false,
        ValueKind::Index1D { base, idx, .. } => {
            value_reads_var(fn_ir, *base, var) || value_reads_var(fn_ir, *idx, var)
        }
        ValueKind::Index2D { base, r, c } => {
            value_reads_var(fn_ir, *base, var)
                || value_reads_var(fn_ir, *r, var)
                || value_reads_var(fn_ir, *c, var)
        }
        ValueKind::Index3D { base, i, j, k } => {
            value_reads_var(fn_ir, *base, var)
                || value_reads_var(fn_ir, *i, var)
                || value_reads_var(fn_ir, *j, var)
                || value_reads_var(fn_ir, *k, var)
        }
        ValueKind::Len { base } | ValueKind::Indices { base } => value_reads_var(fn_ir, *base, var),
        ValueKind::Range { start, end } => {
            value_reads_var(fn_ir, *start, var) || value_reads_var(fn_ir, *end, var)
        }
        _ => false,
    }
}

fn replace_var_read(fn_ir: &mut FnIR, src: ValueId, old_var: &VarId, new_var: &VarId) -> ValueId {
    let val = fn_ir.values[src].clone();

    if !value_reads_var(fn_ir, src, old_var) {
        return src;
    }

    let new_kind = match val.kind {
        ValueKind::Load { var } => {
            if &var == old_var {
                ValueKind::Load {
                    var: new_var.clone(),
                }
            } else {
                return src;
            }
        }
        ValueKind::Param { index } => {
            if let Some(param_name) = param_runtime_var(fn_ir, index) {
                if param_name == old_var {
                    ValueKind::Load {
                        var: new_var.clone(),
                    }
                } else {
                    return src;
                }
            } else {
                return src;
            }
        }
        ValueKind::Binary { op, lhs, rhs } => {
            let l = replace_var_read(fn_ir, lhs, old_var, new_var);
            let r = replace_var_read(fn_ir, rhs, old_var, new_var);
            ValueKind::Binary { op, lhs: l, rhs: r }
        }
        ValueKind::Unary { op, rhs } => {
            let r = replace_var_read(fn_ir, rhs, old_var, new_var);
            ValueKind::Unary { op, rhs: r }
        }
        ValueKind::Call {
            callee,
            args,
            names,
            ..
        } => {
            let new_args = args
                .iter()
                .map(|a| replace_var_read(fn_ir, *a, old_var, new_var))
                .collect();
            ValueKind::Call {
                callee,
                args: new_args,
                names,
            }
        }
        ValueKind::Intrinsic { op, args } => {
            let new_args = args
                .iter()
                .map(|a| replace_var_read(fn_ir, *a, old_var, new_var))
                .collect();
            ValueKind::Intrinsic { op, args: new_args }
        }
        ValueKind::Phi { .. } => {
            if let Some(name) = &val.origin_var {
                if name == old_var {
                    ValueKind::Load {
                        var: new_var.clone(),
                    }
                } else {
                    return src;
                }
            } else {
                return src;
            }
        }
        ValueKind::Index1D {
            base,
            idx,
            is_safe,
            is_na_safe,
        } => {
            let b = replace_var_read(fn_ir, base, old_var, new_var);
            let i = replace_var_read(fn_ir, idx, old_var, new_var);
            ValueKind::Index1D {
                base: b,
                idx: i,
                is_safe,
                is_na_safe,
            }
        }
        ValueKind::Index2D { base, r, c } => {
            let b = replace_var_read(fn_ir, base, old_var, new_var);
            let r = replace_var_read(fn_ir, r, old_var, new_var);
            let c = replace_var_read(fn_ir, c, old_var, new_var);
            ValueKind::Index2D { base: b, r, c }
        }
        ValueKind::Index3D { base, i, j, k } => {
            let base = replace_var_read(fn_ir, base, old_var, new_var);
            let i = replace_var_read(fn_ir, i, old_var, new_var);
            let j = replace_var_read(fn_ir, j, old_var, new_var);
            let k = replace_var_read(fn_ir, k, old_var, new_var);
            ValueKind::Index3D { base, i, j, k }
        }
        ValueKind::Len { base } => {
            let b = replace_var_read(fn_ir, base, old_var, new_var);
            ValueKind::Len { base: b }
        }
        ValueKind::Indices { base } => {
            let b = replace_var_read(fn_ir, base, old_var, new_var);
            ValueKind::Indices { base: b }
        }
        ValueKind::Range { start, end } => {
            let s = replace_var_read(fn_ir, start, old_var, new_var);
            let e = replace_var_read(fn_ir, end, old_var, new_var);
            ValueKind::Range { start: s, end: e }
        }
        ValueKind::RecordLit { fields } => {
            let fields = fields
                .iter()
                .map(|(name, value)| {
                    (
                        name.clone(),
                        replace_var_read(fn_ir, *value, old_var, new_var),
                    )
                })
                .collect();
            ValueKind::RecordLit { fields }
        }
        ValueKind::FieldGet { base, field } => {
            let base = replace_var_read(fn_ir, base, old_var, new_var);
            ValueKind::FieldGet { base, field }
        }
        ValueKind::FieldSet { base, field, value } => {
            let base = replace_var_read(fn_ir, base, old_var, new_var);
            let value = replace_var_read(fn_ir, value, old_var, new_var);
            ValueKind::FieldSet { base, field, value }
        }
        ValueKind::Const(_) | ValueKind::RSymbol { .. } => return src,
    };

    fn_ir.add_value(new_kind, val.span, val.facts, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_noop_load_move() {
        let mut f = FnIR::new("pc_noop".to_string(), vec![]);
        let b0 = f.add_block();
        f.entry = b0;
        f.body_head = b0;

        let load = f.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );

        let mut out = Vec::new();
        emit_parallel_copy(
            &mut f,
            &mut out,
            vec![Move {
                dst: "x".to_string(),
                src: load,
            }],
            Span::default(),
        );

        assert!(out.is_empty(), "no-op load move should be skipped");
    }

    #[test]
    fn cycle_break_materializes_complex_sources_before_rewrite() {
        let mut f = FnIR::new("pc_capture_complex".to_string(), vec![]);
        let b0 = f.add_block();
        f.entry = b0;
        f.body_head = b0;

        let load_x = f.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let load_y = f.add_value(
            ValueKind::Load {
                var: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("y".to_string()),
        );
        let complex = f.add_value(
            ValueKind::Call {
                callee: "foo".to_string(),
                args: vec![load_x, load_y],
                names: vec![None, None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );

        let mut out = Vec::new();
        emit_parallel_copy(
            &mut f,
            &mut out,
            vec![
                Move {
                    dst: "x".to_string(),
                    src: complex,
                },
                Move {
                    dst: "y".to_string(),
                    src: load_x,
                },
            ],
            Span::default(),
        );

        assert!(
            out.iter().any(|instr| matches!(
                instr,
                Instr::Assign { dst, src, .. }
                    if dst.starts_with(".__pc_src_tmp") && *src == complex
            )),
            "cycle break should capture complex victim-dependent source before rewriting"
        );
    }

    #[test]
    fn pre_materializes_shared_complex_sources_before_cycle_break() {
        let mut f = FnIR::new("pc_pre_materialize".to_string(), vec![]);
        let b0 = f.add_block();
        f.entry = b0;
        f.body_head = b0;

        let load_x = f.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let load_y = f.add_value(
            ValueKind::Load {
                var: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("y".to_string()),
        );
        let call = f.add_value(
            ValueKind::Call {
                callee: "foo".to_string(),
                args: vec![load_x, load_y],
                names: vec![None, None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let key_x = f.add_value(
            ValueKind::Const(Lit::Str("x".to_string())),
            Span::default(),
            Facts::empty(),
            None,
        );
        let field_x = f.add_value(
            ValueKind::Call {
                callee: "rr_field_get".to_string(),
                args: vec![call, key_x],
                names: vec![None, None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let key_y = f.add_value(
            ValueKind::Const(Lit::Str("y".to_string())),
            Span::default(),
            Facts::empty(),
            None,
        );
        let field_y = f.add_value(
            ValueKind::Call {
                callee: "rr_field_get".to_string(),
                args: vec![call, key_y],
                names: vec![None, None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );

        let mut out = Vec::new();
        emit_parallel_copy(
            &mut f,
            &mut out,
            vec![
                Move {
                    dst: "x".to_string(),
                    src: field_x,
                },
                Move {
                    dst: "y".to_string(),
                    src: field_y,
                },
            ],
            Span::default(),
        );

        let pre_materialized = out
            .iter()
            .filter(|instr| matches!(instr, Instr::Assign { dst, src, .. } if dst.starts_with(".__pc_src_tmp") && (*src == field_x || *src == field_y)))
            .count();
        assert_eq!(
            pre_materialized, 2,
            "complex pending sources should be materialized once before cycle-breaking rewrites"
        );
    }
}
