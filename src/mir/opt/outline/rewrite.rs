use super::super::*;
use super::analysis::OutlineCandidate;
use crate::utils::Span;

pub(crate) fn replace_region_with_call(
    parent: &mut FnIR,
    candidate: &OutlineCandidate,
    helper_name: &str,
) {
    let call = add_helper_call(parent, helper_name, &candidate.live_ins);
    let mut replacement = Vec::new();
    append_live_out_assignments(parent, call, &candidate.live_outs, &mut replacement);

    let block = &mut parent.blocks[candidate.block];
    block
        .instrs
        .splice(candidate.start..candidate.end, replacement);
}

fn add_helper_call(parent: &mut FnIR, helper_name: &str, live_ins: &[VarId]) -> ValueId {
    let args = live_ins
        .iter()
        .map(|var| {
            parent.add_value(
                ValueKind::Load { var: var.clone() },
                Span::default(),
                Facts::empty(),
                Some(var.clone()),
            )
        })
        .collect();
    parent.add_value(
        ValueKind::Call {
            callee: helper_name.to_string(),
            args,
            names: vec![None; live_ins.len()],
        },
        Span::default(),
        Facts::empty(),
        None,
    )
}

fn append_live_out_assignments(
    parent: &mut FnIR,
    call: ValueId,
    live_outs: &[VarId],
    output: &mut Vec<Instr>,
) {
    match live_outs {
        [] => output.push(Instr::Eval {
            val: call,
            span: Span::default(),
        }),
        [var] => output.push(Instr::Assign {
            dst: var.clone(),
            src: call,
            span: Span::default(),
        }),
        vars => {
            for var in vars {
                let field = parent.add_value(
                    ValueKind::FieldGet {
                        base: call,
                        field: var.clone(),
                    },
                    Span::default(),
                    Facts::empty(),
                    Some(var.clone()),
                );
                output.push(Instr::Assign {
                    dst: var.clone(),
                    src: field,
                    span: Span::default(),
                });
            }
        }
    }
}
