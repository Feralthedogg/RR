use super::*;
// Semantic and runtime-safety validation for MIR.
//
// The checks here defend the boundary between lowering/optimization and later
// codegen/runtime execution by rejecting invalid user-visible MIR states.

use super::call_model::*;
use crate::diagnostic::finish_diagnostics;
use crate::error::{RR, RRCode, RRException, Stage};
use crate::utils::did_you_mean;
use rustc_hash::{FxHashMap, FxHashSet};

pub fn validate_program(all_fns: &FxHashMap<String, FnIR>) -> RR<()> {
    let mut fn_names: Vec<String> = all_fns.keys().cloned().collect();
    fn_names.sort();

    let mut user_signatures: FxHashMap<String, UserFnSignature> = FxHashMap::default();
    let mut errors = Vec::new();
    for name in &fn_names {
        if let Some(fn_ir) = all_fns.get(name) {
            user_signatures.insert(
                name.clone(),
                UserFnSignature {
                    display_name: fn_ir.user_name.clone().unwrap_or_else(|| name.clone()),
                    param_names: fn_ir.params.clone(),
                    has_default: fn_ir
                        .param_default_r_exprs
                        .iter()
                        .map(Option::is_some)
                        .collect(),
                },
            );
        }
    }

    for name in fn_names {
        if let Some(fn_ir) = all_fns.get(&name) {
            errors.extend(validate_function(fn_ir, &user_signatures));
        }
    }
    finish_diagnostics(
        "RR.SemanticError",
        RRCode::E1002,
        Stage::Mir,
        format!("semantic validation failed: {} error(s)", errors.len()),
        errors,
    )
}

pub fn validate_runtime_safety(all_fns: &FxHashMap<String, FnIR>) -> RR<()> {
    let mut fn_names: Vec<String> = all_fns.keys().cloned().collect();
    fn_names.sort();

    let mut errors = Vec::new();
    for name in fn_names {
        if let Some(fn_ir) = all_fns.get(&name) {
            errors.extend(validate_function_runtime(fn_ir));
        }
    }
    finish_diagnostics(
        "RR.RuntimeError",
        RRCode::E2001,
        Stage::Mir,
        format!(
            "runtime safety validation failed: {} error(s)",
            errors.len()
        ),
        errors,
    )
}

pub(crate) fn suggest_name<I>(name: &str, candidates: I) -> Option<String>
where
    I: IntoIterator<Item = String>,
{
    did_you_mean(name, candidates)
}

pub(crate) fn validate_function(
    fn_ir: &FnIR,
    user_signatures: &FxHashMap<String, UserFnSignature>,
) -> Vec<RRException> {
    let mut errors = Vec::new();
    let mut assigned_vars: FxHashSet<String> = fn_ir.params.iter().cloned().collect();
    for block in &fn_ir.blocks {
        for ins in &block.instrs {
            if let Instr::Assign { dst, .. } = ins {
                assigned_vars.insert(dst.clone());
            }
        }
    }

    // Dead blocks still originate from user-written statements after terminators
    // such as `return`, so semantic validation must not silently skip them.
    for v in &fn_ir.values {
        match &v.kind {
            ValueKind::Load { var }
                if !assigned_vars.contains(var)
                    && !is_runtime_reserved_symbol(var)
                    && !is_namespaced_r_call(var) =>
            {
                let mut err = RRException::new(
                    "RR.SemanticError",
                    RRCode::E1001,
                    Stage::Mir,
                    format!("undefined variable '{}' in function '{}'", var, fn_ir.name),
                )
                .at(v.span)
                .push_frame("mir::semantics::validate_function/2", Some(v.span))
                .note("Declare the variable with let before use.");
                if let Some(suggestion) = suggest_name(var, assigned_vars.iter().cloned()) {
                    err = err.help(suggestion);
                }
                errors.push(err);
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } => {
                if let Err(e) =
                    validate_call_target(callee, args.len(), names, v.span, user_signatures)
                {
                    errors.push(e);
                }
            }
            _ => {}
        }
    }

    errors
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::validate_function_runtime;
    use crate::mir::{Facts, FnIR, Lit, Terminator, ValueKind};
    use crate::utils::Span;

    #[test]
    fn runtime_safety_flags_negative_index_through_phi_merged_record_field() {
        let mut f = FnIR::new("runtime_phi_field".to_string(), Vec::new());
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let neg_one = f.add_value(
            ValueKind::Const(Lit::Int(-1)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let x1 = f.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let x2 = f.add_value(
            ValueKind::Const(Lit::Int(2)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let x3 = f.add_value(
            ValueKind::Const(Lit::Int(3)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let x = f.add_value(
            ValueKind::Call {
                callee: "c".to_string(),
                args: vec![x1, x2, x3],
                names: vec![None, None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let rec_a = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("i".to_string(), neg_one)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let rec_b = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("i".to_string(), neg_one)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let rec_phi = f.add_value(
            ValueKind::Phi {
                args: vec![(rec_a, left), (rec_b, right)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        f.values[rec_phi].phi_block = Some(merge);
        let idx = f.add_value(
            ValueKind::FieldGet {
                base: rec_phi,
                field: "i".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let read = f.add_value(
            ValueKind::Index1D {
                base: x,
                idx,
                is_safe: false,
                is_na_safe: false,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        f.blocks[left].term = Terminator::Goto(merge);
        f.blocks[right].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(read));

        let errors = validate_function_runtime(&f);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("must be >= 1") || e.message.contains("out of bounds")),
            "expected runtime negative-index diagnostic, got: {errors:#?}"
        );
    }

    #[test]
    fn runtime_safety_does_not_treat_unknown_index_as_proven_below_one() {
        let mut f = FnIR::new("runtime_unknown_index".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let one = f.add_value(
            ValueKind::Const(Lit::Float(1.0)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(Lit::Float(2.0)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let base = f.add_value(
            ValueKind::Call {
                callee: "c".to_string(),
                args: vec![one, two],
                names: vec![None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let idx = f.add_value(
            ValueKind::Load {
                var: "ii".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let read = f.add_value(
            ValueKind::Index1D {
                base,
                idx,
                is_safe: false,
                is_na_safe: false,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Return(Some(read));

        let errors = validate_function_runtime(&f);
        assert!(
            !errors
                .iter()
                .any(|e| e.message.contains("must be >= 1") || e.message.contains("out of bounds")),
            "unexpected proven-below-one diagnostic for unknown index: {errors:#?}"
        );
    }

    #[test]
    fn runtime_safety_does_not_treat_unknown_seq_len_arg_as_proven_negative() {
        let mut f = FnIR::new("runtime_unknown_seq_len".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let n = f.add_value(
            ValueKind::Load {
                var: "n".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let seq = f.add_value(
            ValueKind::Call {
                callee: "seq_len".to_string(),
                args: vec![n],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Return(Some(seq));

        let errors = validate_function_runtime(&f);
        assert!(
            !errors
                .iter()
                .any(|e| e.message.contains("seq_len() with negative length")),
            "unexpected proven-negative seq_len diagnostic for unknown arg: {errors:#?}"
        );
    }

    #[test]
    fn runtime_safety_flags_negative_seq_len_through_nested_record_field() {
        let mut f = FnIR::new("runtime_nested_seq_len".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let neg_one = f.add_value(
            ValueKind::Const(Lit::Int(-1)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let inner = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("n".to_string(), neg_one)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let outer = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("inner".to_string(), inner)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let inner_field = f.add_value(
            ValueKind::FieldGet {
                base: outer,
                field: "inner".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let n = f.add_value(
            ValueKind::FieldGet {
                base: inner_field,
                field: "n".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let seq = f.add_value(
            ValueKind::Call {
                callee: "seq_len".to_string(),
                args: vec![n],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Return(Some(seq));

        let errors = validate_function_runtime(&f);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("seq_len() with negative length")),
            "expected proven-negative seq_len diagnostic, got: {errors:#?}"
        );
    }

    #[test]
    fn runtime_safety_flags_negative_seq_len_through_fieldset_override() {
        let mut f = FnIR::new("runtime_fieldset_seq_len".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let five = f.add_value(
            ValueKind::Const(Lit::Int(5)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let neg_two = f.add_value(
            ValueKind::Const(Lit::Int(-2)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let record = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("n".to_string(), five)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let updated = f.add_value(
            ValueKind::FieldSet {
                base: record,
                field: "n".to_string(),
                value: neg_two,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let n = f.add_value(
            ValueKind::FieldGet {
                base: updated,
                field: "n".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let seq = f.add_value(
            ValueKind::Call {
                callee: "seq_len".to_string(),
                args: vec![n],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Return(Some(seq));

        let errors = validate_function_runtime(&f);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("seq_len() with negative length")),
            "expected proven-negative seq_len diagnostic after fieldset override, got: {errors:#?}"
        );
    }

    #[test]
    fn runtime_safety_does_not_flag_positive_seq_len_after_fieldset_override() {
        let mut f = FnIR::new("runtime_fieldset_seq_len_positive".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let neg_one = f.add_value(
            ValueKind::Const(Lit::Int(-1)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let five = f.add_value(
            ValueKind::Const(Lit::Int(5)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let record = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("n".to_string(), neg_one)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let updated = f.add_value(
            ValueKind::FieldSet {
                base: record,
                field: "n".to_string(),
                value: five,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let n = f.add_value(
            ValueKind::FieldGet {
                base: updated,
                field: "n".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let seq = f.add_value(
            ValueKind::Call {
                callee: "seq_len".to_string(),
                args: vec![n],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Return(Some(seq));

        let errors = validate_function_runtime(&f);
        assert!(
            !errors
                .iter()
                .any(|e| e.message.contains("seq_len() with negative length")),
            "unexpected proven-negative seq_len diagnostic after positive override: {errors:#?}"
        );
    }
}
