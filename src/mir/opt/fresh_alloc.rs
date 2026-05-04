use crate::mir::def::EscapeStatus;
use crate::mir::opt::loop_analysis::{LoopAnalyzer, LoopInfo, build_pred_map};
use crate::mir::*;
use crate::syntax::ast::Lit;

pub fn optimize(fn_ir: &mut FnIR) -> bool {
    // 1. Run Analysis
    // This populates the `escape` field on all values.
    crate::mir::analyze::escape::EscapeAnalysis::analyze_function(fn_ir);

    let analyzer = LoopAnalyzer::new(fn_ir);
    let loops = analyzer.find_loops();
    optimize_with_analyzed_loops(fn_ir, &loops)
}

pub fn optimize_with_loop_info(fn_ir: &mut FnIR, loops: &[LoopInfo]) -> bool {
    // Keep the escape precondition local even when Chronos supplies loops.
    crate::mir::analyze::escape::EscapeAnalysis::analyze_function(fn_ir);
    optimize_with_analyzed_loops(fn_ir, loops)
}

fn optimize_with_analyzed_loops(fn_ir: &mut FnIR, loops: &[LoopInfo]) -> bool {
    let mut changed = false;

    // 2. Pre-allocation for simple map-style loops
    let preds = build_pred_map(fn_ir);

    for lp in loops {
        let preheaders: Vec<BlockId> = preds
            .get(&lp.header)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|b| !lp.body.contains(b))
            .collect();
        if preheaders.len() != 1 {
            continue;
        }
        let preheader = preheaders[0];

        // Skip loops with branches in body (we can't guarantee full coverage)
        if lp
            .body
            .iter()
            .any(|b| matches!(fn_ir.blocks[*b].term, Terminator::If { .. }))
        {
            continue;
        }

        let iv = match &lp.iv {
            Some(iv) => iv.phi_val,
            None => continue,
        };

        let len_source = if let Some(x) = lp.is_seq_along {
            Some(LenSource::Along(x))
        } else {
            lp.is_seq_len.map(LenSource::Len)
        };

        let len_source = match len_source {
            Some(s) => s,
            None => continue,
        };

        // Track which vars already preallocated in preheader
        let mut already_prealloc = std::collections::HashSet::new();
        for instr in &fn_ir.blocks[preheader].instrs {
            if let Instr::Assign { dst, .. } = instr {
                already_prealloc.insert(dst.clone());
            }
        }

        let mut plans: Vec<(VarId, ValueId, LenSource)> = Vec::new();
        let mut inserted = std::collections::HashSet::new();
        for &bid in &lp.body {
            for instr in &fn_ir.blocks[bid].instrs {
                let (base, idx, val) = match instr {
                    Instr::StoreIndex1D {
                        base,
                        idx,
                        val,
                        is_vector: false,
                        ..
                    } => (base, idx, val),
                    _ => continue,
                };
                if *idx != iv {
                    continue;
                }

                let dest_var = match resolve_base_var(fn_ir, *base) {
                    Some(v) => v,
                    None => continue,
                };
                if already_prealloc.contains(&dest_var) || inserted.contains(&dest_var) {
                    continue;
                }

                plans.push((dest_var.clone(), *val, len_source));
                inserted.insert(dest_var);
            }
        }

        for (dest_var, val, len_source) in plans {
            let len_val = match len_source {
                LenSource::Along(x) => fn_ir.add_value(
                    ValueKind::Len { base: x },
                    crate::utils::Span::dummy(),
                    Facts::empty(),
                    None,
                ),
                LenSource::Len(n) => n,
            };

            let alloc_fn = infer_alloc_fn(fn_ir, val);
            let alloc_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: alloc_fn.to_string(),
                    args: vec![len_val],
                    names: vec![None],
                },
                crate::utils::Span::dummy(),
                Facts::empty(),
                None,
            );

            fn_ir.blocks[preheader].instrs.push(Instr::Assign {
                dst: dest_var.clone(),
                src: alloc_val,
                span: crate::utils::Span::dummy(),
            });
            changed = true;
        }
    }

    // 3. Rename calls to *_fresh for explicitly allowed builtins (opt-in)
    // Only rewrite calls we explicitly support. Never touch user functions (Sym_*).
    const FRESH_BUILTINS: &[&str] = &[];
    let mut candidates = vec![];

    for (i, val) in fn_ir.values.iter().enumerate() {
        if val.escape == EscapeStatus::Local
            && let ValueKind::Call { callee, .. } = &val.kind
        {
            if callee.starts_with("Sym_") {
                continue;
            }
            if !FRESH_BUILTINS.iter().any(|name| name == callee) {
                continue;
            }
            if !callee.ends_with("_fresh") {
                candidates.push(i);
            }
        }
    }

    for vid in candidates {
        if let ValueKind::Call { callee, .. } = &mut fn_ir.values[vid].kind {
            *callee = format!("{}_fresh", callee);
            changed = true;
        }
    }

    changed
}

#[derive(Clone, Copy)]
enum LenSource {
    Along(ValueId),
    Len(ValueId),
}

fn resolve_base_var(fn_ir: &FnIR, base: ValueId) -> Option<VarId> {
    if let ValueKind::Load { var } = &fn_ir.values[base].kind {
        return Some(var.clone());
    }
    fn_ir.values[base].origin_var.clone()
}

fn infer_alloc_fn(fn_ir: &FnIR, val: ValueId) -> &'static str {
    let v = &fn_ir.values[val];
    match &v.kind {
        ValueKind::Const(Lit::Int(_)) => "integer",
        ValueKind::Const(Lit::Bool(_)) => "logical",
        ValueKind::Const(Lit::Str(_)) => "character",
        ValueKind::Const(Lit::Float(_)) => "numeric",
        _ => {
            if v.facts.has(Facts::BOOL_SCALAR) {
                "logical"
            } else if v.facts.has(Facts::INT_SCALAR) {
                "integer"
            } else {
                "numeric"
            }
        }
    }
}
