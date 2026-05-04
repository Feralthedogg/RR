use super::*;

pub(crate) fn emit_r_functions(
    ui: &CliLog,
    total_steps: usize,
    program: &ProgramIR,
    emit_order: &[FnSlot],
) -> crate::error::RR<(String, Vec<crate::codegen::mir_emit::MapEntry>)> {
    let scheduler = CompilerScheduler::new(CompilerParallelConfig::default());
    let (out, map, _, _, _) = emit_r_functions_cached(EmitFunctionsRequest {
        ui,
        total_steps,
        program,
        emit_order,
        top_level_calls: &[],
        opt_level: OptLevel::O0,
        type_cfg: TypeConfig::default(),
        parallel_cfg: ParallelConfig::default(),
        scheduler: &scheduler,
        output_opts: CompileOutputOptions::default(),
        cache: None,
    })?;
    Ok((out, map))
}
pub(crate) fn trivial_zero_arg_entry_callee(
    fn_ir: &crate::mir::def::FnIR,
    program: &ProgramIR,
) -> Option<String> {
    if !fn_ir.params.is_empty() {
        return None;
    }
    let mut returned = None;
    for block in &fn_ir.blocks {
        if let crate::mir::def::Terminator::Return(Some(val)) = block.term
            && returned.replace(val).is_some()
        {
            return None;
        }
    }
    let ret = returned?;
    match &fn_ir.values.get(ret)?.kind {
        crate::mir::def::ValueKind::Call {
            callee,
            args,
            names,
        } if args.is_empty() && names.is_empty() => {
            let target = program.get(callee)?;
            if target.params.is_empty() && !callee.starts_with("Sym_top_") {
                Some(callee.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}
pub(crate) fn quoted_body_entry_targets(
    program: &ProgramIR,
    top_level_calls: &[FnSlot],
) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    for top_slot in top_level_calls {
        let Some(top_fn) = program.get_slot(*top_slot) else {
            continue;
        };
        let Some(callee) = trivial_zero_arg_entry_callee(top_fn, program) else {
            continue;
        };
        out.insert(callee);
    }
    out
}
pub(crate) fn wrap_zero_arg_function_body_in_quote(code: &str, fn_name: &str) -> Option<String> {
    const MIN_LINES_FOR_ENTRY_QUOTE_WRAP: usize = 20;
    if code.lines().count() < MIN_LINES_FOR_ENTRY_QUOTE_WRAP {
        return None;
    }

    let header = format!("{fn_name} <- function() \n{{\n");
    let footer = "}\n";
    if !code.starts_with(&header) || !code.ends_with(footer) {
        return None;
    }

    let body = &code[header.len()..code.len() - footer.len()];
    let body_name = format!(".__rr_body_{}", fn_name);
    let mut wrapped = String::new();
    wrapped.push_str(&format!("{body_name} <- quote({{\n"));
    wrapped.push_str(body);
    if !body.ends_with('\n') {
        wrapped.push('\n');
    }
    wrapped.push_str("})\n");
    wrapped.push_str(&header);
    wrapped.push_str(&format!("  eval({body_name}, envir = environment())\n"));
    wrapped.push_str(footer);
    Some(wrapped)
}
