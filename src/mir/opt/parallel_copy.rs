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
            && i == index {
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
    let mut pending: Vec<Move> = moves;
    let mut cycle_idx: usize = 0;

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

fn value_reads_var(fn_ir: &FnIR, src: ValueId, var: &VarId) -> bool {
    let val = &fn_ir.values[src];
    if let Some(name) = &val.origin_var
        && name == var {
            return true;
        }
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
        ValueKind::Const(_) => return src,
    };

    fn_ir.add_value(new_kind, val.span, val.facts, None)
}
