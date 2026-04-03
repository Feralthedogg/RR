//! Source-loading, MIR synthesis, and emitted-R assembly helpers for the
//! compiler pipeline.
//!
//! The functions in this module prepare stable per-function jobs, lower HIR to
//! MIR, and concatenate emitted function fragments into the final artifact.

use super::super::*;

/// Owned lowering job for a user-declared function.
pub(crate) struct MirLowerJob {
    pub(crate) fn_name: String,
    pub(crate) is_public: bool,
    pub(crate) params: Vec<String>,
    pub(crate) var_names: FxHashMap<crate::hir::def::LocalId, String>,
    pub(crate) hir_fn: crate::hir::def::HirFn,
}

/// Synthetic lowering job used for module top-level entry shims.
pub(crate) struct TopLevelMirLowerJob {
    pub(crate) fn_name: String,
    pub(crate) hir_fn: crate::hir::def::HirFn,
}

/// One parallel emission result before the final artifact is concatenated.
pub(crate) struct EmittedFnFragment {
    pub(crate) code: String,
    pub(crate) map: Vec<MapEntry>,
    pub(crate) cache_hit: bool,
}

/// Cheap work estimate used by the scheduler for HIR-lowering jobs.
pub(crate) fn hir_fn_work_size(f: &crate::hir::def::HirFn) -> usize {
    let stmt_count = f.body.stmts.len();
    let local_count = f.local_names.len();
    (stmt_count * 8)
        .saturating_add(local_count)
        .saturating_add(f.params.len())
}

/// Collect user-defined callees that are reachable from a single MIR function.
pub(crate) fn called_user_fns(
    fn_ir: &crate::mir::def::FnIR,
    program: &ProgramIR,
) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    for value in &fn_ir.values {
        match &value.kind {
            crate::mir::def::ValueKind::Call { callee, .. } => {
                let canonical = callee.strip_suffix("_fresh").unwrap_or(callee);
                if program.contains_name(canonical) {
                    out.insert(canonical.to_string());
                }
            }
            crate::mir::def::ValueKind::Load { var } => {
                if program.contains_name(var) {
                    out.insert(var.clone());
                }
            }
            crate::mir::def::ValueKind::RSymbol { name } => {
                if program.contains_name(name) {
                    out.insert(name.clone());
                }
            }
            _ => {}
        }
    }
    out
}

pub(crate) fn collect_seq_len_param_end_slots_by_fn(
    program: &ProgramIR,
) -> FxHashMap<String, FxHashMap<usize, usize>> {
    fn unique_assign_source(
        fn_ir: &crate::mir::def::FnIR,
        var: &str,
    ) -> Option<crate::mir::def::ValueId> {
        let mut src: Option<crate::mir::def::ValueId> = None;
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                let crate::mir::def::Instr::Assign {
                    dst, src: value, ..
                } = instr
                else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                match src {
                    None => src = Some(*value),
                    Some(prev) if prev == *value => {}
                    Some(_) => return None,
                }
            }
        }
        src
    }

    fn resolve_load_alias_value(
        fn_ir: &crate::mir::def::FnIR,
        vid: crate::mir::def::ValueId,
    ) -> crate::mir::def::ValueId {
        let mut cur = vid;
        let mut seen = FxHashSet::default();
        while seen.insert(cur) {
            let crate::mir::def::ValueKind::Load { var } = &fn_ir.values[cur].kind else {
                break;
            };
            let Some(src) = unique_assign_source(fn_ir, var) else {
                break;
            };
            cur = src;
        }
        cur
    }

    fn seq_len_base_arg_slot(
        fn_ir: &crate::mir::def::FnIR,
        call_args: &[crate::mir::def::ValueId],
        arg: crate::mir::def::ValueId,
    ) -> Option<usize> {
        let resolved = resolve_load_alias_value(fn_ir, arg);
        let crate::mir::def::ValueKind::Call {
            callee,
            args,
            names,
        } = &fn_ir.values[resolved].kind
        else {
            return None;
        };
        if callee != "seq_len" || args.len() != 1 || names.iter().any(|name| name.is_some()) {
            return None;
        }
        let base = resolve_load_alias_value(fn_ir, args[0]);
        let mut matches = call_args
            .iter()
            .enumerate()
            .filter_map(|(slot, candidate)| {
                (resolve_load_alias_value(fn_ir, *candidate) == base).then_some(slot)
            });
        let slot = matches.next()?;
        matches.next().is_none().then_some(slot)
    }

    let mut summaries: FxHashMap<String, FxHashMap<usize, usize>> = FxHashMap::default();

    for slot in program.all_slots() {
        let Some(fn_ir) = program.get_slot(slot) else {
            continue;
        };
        for value in &fn_ir.values {
            let crate::mir::def::ValueKind::Call {
                callee,
                args,
                names,
            } = &value.kind
            else {
                continue;
            };
            if names.iter().any(|name| name.is_some()) {
                continue;
            }
            let canonical = callee.strip_suffix("_fresh").unwrap_or(callee.as_str());
            let Some(callee_ir) = program.get(canonical) else {
                continue;
            };
            if args.len() != callee_ir.params.len() {
                continue;
            }
            let local: FxHashMap<usize, usize> = args
                .iter()
                .enumerate()
                .filter_map(|(slot, arg)| {
                    seq_len_base_arg_slot(fn_ir, args, *arg).map(|end_slot| (slot, end_slot))
                })
                .collect();
            let entry = summaries
                .entry(canonical.to_string())
                .or_insert(local.clone());
            if entry != &local {
                entry.retain(|slot, end_slot| local.get(slot) == Some(end_slot));
            }
        }
    }

    summaries.retain(|_, slots| !slots.is_empty());
    summaries
}

pub(crate) fn reachable_emit_order_slots(program: &ProgramIR) -> Vec<FnSlot> {
    if program.emit_roots.is_empty() {
        return program.emit_order.clone();
    }

    let mut reachable = FxHashSet::default();
    let mut worklist: Vec<String> = program
        .emit_root_names()
        .iter()
        .filter(|name| program.contains_name(name.as_str()))
        .cloned()
        .collect();

    while let Some(name) = worklist.pop() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        let Some(fn_ir) = program.get(&name) else {
            continue;
        };
        for callee in called_user_fns(fn_ir, program) {
            if !reachable.contains(&callee) {
                worklist.push(callee);
            }
        }
    }

    if reachable.iter().all(|name| name.starts_with("Sym_top_")) {
        return program.emit_order.clone();
    }

    program
        .emit_order
        .iter()
        .copied()
        .filter(|slot| {
            program
                .fns
                .get(*slot)
                .is_some_and(|unit| reachable.contains(unit.name.as_str()))
        })
        .collect()
}

pub(crate) fn run_source_analysis_and_canonicalization(
    ui: &CliLog,
    entry_path: &str,
    entry_input: &str,
    total_steps: usize,
    output_opts: CompileOutputOptions,
) -> crate::error::RR<SourceAnalysisOutput> {
    let mut loaded_paths: FxHashSet<PathBuf> = FxHashSet::default();
    let mut queue = std::collections::VecDeque::new();

    let entry_abs = normalize_module_path(Path::new(entry_path));
    loaded_paths.insert(entry_abs.clone());
    queue.push_back((entry_abs, entry_input.to_string(), 0));

    let mut next_mod_id = 1;

    let step_load = ui.step_start(
        1,
        total_steps,
        "Source Analysis",
        "parse + scope resolution",
    );
    let mut hir_modules = Vec::new();
    let mut hir_lowerer = crate::hir::lower::Lowerer::with_policy(
        output_opts.strict_let,
        output_opts.warn_implicit_decl,
    );
    let mut load_errors: Vec<crate::error::RRException> = Vec::new();

    while let Some((curr_path, content, mod_id)) = queue.pop_front() {
        let curr_path_str = curr_path.to_string_lossy().to_string();
        ui.trace(&format!("module#{}", mod_id), &curr_path_str);

        let mut parser = Parser::new(&content);
        let ast_prog = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                load_errors.push(e);
                continue;
            }
        };

        let hir_mod =
            match hir_lowerer.lower_module(ast_prog, crate::hir::def::ModuleId(mod_id as u32)) {
                Ok(v) => v,
                Err(e) => {
                    load_errors.push(e);
                    continue;
                }
            };
        for w in hir_lowerer.take_warnings() {
            ui.warn(&format!("{}: {}", curr_path_str, w));
        }

        for item in &hir_mod.items {
            if let crate::hir::def::HirItem::Import(imp) = item {
                let import_path = &imp.module;
                let target = crate::pkg::resolve_import_path(&curr_path, import_path)?;

                if !loaded_paths.contains(&target) {
                    if !target.is_absolute() {
                        return Err(crate::error::RRException::new(
                            "RR.ParseError",
                            crate::error::RRCode::E0001,
                            crate::error::Stage::Parse,
                            format!(
                                "relative import resolution requires an absolute entry path; normalize '{}' before compiling",
                                curr_path_str
                            ),
                        ));
                    }
                    let target_lossy = target.to_string_lossy().to_string();
                    ui.trace("import", &target_lossy);
                    match fs::read_to_string(&target) {
                        Ok(content) => {
                            loaded_paths.insert(target.clone());
                            queue.push_back((target, content, next_mod_id));
                            next_mod_id += 1;
                        }
                        Err(e) => {
                            return Err(crate::error::RRException::new(
                                "RR.ParseError",
                                crate::error::RRCode::E0001,
                                crate::error::Stage::Parse,
                                format!("failed to load imported module '{}': {}", target_lossy, e),
                            ));
                        }
                    }
                }
            }
        }
        hir_modules.push(hir_mod);
    }
    if !load_errors.is_empty() {
        if load_errors.len() == 1 {
            return Err(load_errors.remove(0));
        }
        return Err(crate::error::RRException::aggregate(
            "RR.ParseError",
            crate::error::RRCode::E0001,
            crate::error::Stage::Parse,
            format!("source analysis failed: {} error(s)", load_errors.len()),
            load_errors,
        ));
    }
    ui.step_line_ok(&format!(
        "Loaded {} module(s) in {}",
        hir_modules.len(),
        format_duration(step_load.elapsed())
    ));

    let hir_prog = crate::hir::def::HirProgram {
        modules: hir_modules,
    };
    let global_symbols = hir_lowerer.into_symbols();

    let step_desugar = ui.step_start(
        2,
        total_steps,
        "Canonicalization",
        "normalize HIR structure",
    );
    let mut desugarer = crate::hir::desugar::Desugarer::new();
    let desugared_hir = desugarer.desugar_program(hir_prog)?;
    ui.step_line_ok(&format!(
        "Desugared {} module(s) in {}",
        desugared_hir.modules.len(),
        format_duration(step_desugar.elapsed())
    ));

    Ok(SourceAnalysisOutput {
        desugared_hir,
        global_symbols,
    })
}

fn emit_r_functions(
    ui: &CliLog,
    total_steps: usize,
    program: &ProgramIR,
    emit_order: &[FnSlot],
) -> crate::error::RR<(String, Vec<crate::codegen::mir_emit::MapEntry>)> {
    let scheduler = CompilerScheduler::new(CompilerParallelConfig::default());
    let (out, map, _, _) = emit_r_functions_cached(
        ui,
        total_steps,
        program,
        emit_order,
        &[],
        OptLevel::O0,
        TypeConfig::default(),
        ParallelConfig::default(),
        &scheduler,
        CompileOutputOptions::default(),
        None,
    )?;
    Ok((out, map))
}

fn trivial_zero_arg_entry_callee(
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

fn quoted_body_entry_targets(program: &ProgramIR, top_level_calls: &[FnSlot]) -> FxHashSet<String> {
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

fn wrap_zero_arg_function_body_in_quote(code: &str, fn_name: &str) -> Option<String> {
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_r_functions_cached(
    ui: &CliLog,
    total_steps: usize,
    program: &ProgramIR,
    emit_order: &[FnSlot],
    top_level_calls: &[FnSlot],
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    scheduler: &CompilerScheduler,
    output_opts: CompileOutputOptions,
    cache: Option<&dyn EmitFunctionCache>,
) -> crate::error::RR<(String, Vec<MapEntry>, usize, usize)> {
    let step_emit = ui.step_start(
        5,
        total_steps,
        "R Code Emission",
        "reconstruct control flow",
    );
    let quoted_entry_targets = quoted_body_entry_targets(program, top_level_calls);
    let fresh_user_calls = collect_fresh_returning_user_functions(program);
    let seq_len_param_end_slots_by_fn = collect_seq_len_param_end_slots_by_fn(program);
    let shared_fresh_user_calls = Arc::new(fresh_user_calls.clone());
    let shared_seq_len_param_end_slots_by_fn = Arc::new(seq_len_param_end_slots_by_fn.clone());
    let direct_builtin_call_map =
        matches!(type_cfg.native_backend, crate::typeck::NativeBackend::Off)
            && matches!(parallel_cfg.mode, ParallelMode::Off);
    let emit_total_ir = emit_order
        .iter()
        .filter_map(|slot| program.get_slot(*slot))
        .map(fn_ir_work_size)
        .sum::<usize>();
    let emitted_fragments = scheduler.map_try(emit_order.to_vec(), emit_total_ir, |slot| {
        let shared_fresh_user_calls = Arc::clone(&shared_fresh_user_calls);
        let shared_seq_len_param_end_slots_by_fn =
            Arc::clone(&shared_seq_len_param_end_slots_by_fn);
        let Some(unit) = program.fns.get(slot) else {
            return Err(crate::error::InternalCompilerError::new(
                crate::error::Stage::Codegen,
                format!("emit order references missing function slot '{}'", slot),
            )
            .into_exception());
        };
        let Some(fn_ir) = unit.ir.as_ref() else {
            return Err(crate::error::InternalCompilerError::new(
                crate::error::Stage::Codegen,
                format!(
                    "emit order references missing function '{}': MIR synthesis invariant violated",
                    unit.name
                ),
            )
            .into_exception());
        };

        let key = fn_emit_cache_key(
            fn_ir,
            opt_level,
            type_cfg,
            parallel_cfg,
            seq_len_param_end_slots_by_fn.get(unit.name.as_str()),
        );
        let maybe_hit = if let Some(c) = cache {
            c.load(&key)?
        } else {
            None
        };

        let (code, map, cache_hit) = if let Some((code, map)) = maybe_hit {
            (code, map, true)
        } else {
            let mut emitter = crate::codegen::mir_emit::MirEmitter::with_shared_analysis_options(
                shared_fresh_user_calls,
                shared_seq_len_param_end_slots_by_fn,
                direct_builtin_call_map,
            );
            let (code, map) = emitter.emit(fn_ir)?;
            if let Some(c) = cache {
                c.store(&key, &code, &map)?;
            }
            (code, map, false)
        };

        let mut code = code;
        let mut map = map;
        if quoted_entry_targets.contains(unit.name.as_str())
            && let Some(wrapped) = wrap_zero_arg_function_body_in_quote(&code, unit.name.as_str())
        {
            code = wrapped;
            for entry in &mut map {
                entry.r_line = entry.r_line.saturating_sub(1);
            }
        }
        Ok(EmittedFnFragment {
            code,
            map,
            cache_hit,
        })
    })?;

    let mut final_output = String::new();
    let mut final_source_map = Vec::new();
    let mut cache_hits = 0usize;
    let mut cache_misses = 0usize;
    let mut line_offset = 0u32;
    for mut fragment in emitted_fragments {
        if fragment.cache_hit {
            cache_hits += 1;
        } else {
            cache_misses += 1;
        }
        for entry in &mut fragment.map {
            entry.r_line = entry.r_line.saturating_add(line_offset);
        }
        line_offset = line_offset.saturating_add(emitted_segment_line_count(&fragment.code));
        final_output.push_str(&fragment.code);
        final_output.push('\n');
        final_source_map.extend(fragment.map);
    }
    ui.step_line_ok(&format!(
        "Emitted {} functions ({} debug maps) in {}",
        emit_order.len(),
        final_source_map.len(),
        format_duration(step_emit.elapsed())
    ));

    let pure_user_calls = collect_referentially_pure_user_functions(program);
    let skip_generated_poly_loop_rewrites = contains_generated_poly_loop_controls(&final_output);
    if !skip_generated_poly_loop_rewrites {
        final_output = rewrite_trivial_clamp_helper_calls_in_raw_emitted_r(&final_output);
        final_output = rewrite_branch_local_identical_alloc_rebinds_in_raw_emitted_r(&final_output);
        final_output = hoist_branch_local_pure_scalar_assigns_used_after_branch_in_raw_emitted_r(
            &final_output,
        );
        final_output = rewrite_single_use_scalar_index_aliases_in_raw_emitted_r(&final_output);
        final_output =
            rewrite_small_multiuse_scalar_index_aliases_in_adjacent_assignments_in_raw_emitted_r(
                &final_output,
            );
        final_output = strip_unused_raw_arg_aliases_in_raw_emitted_r(&final_output);
        final_output = rewrite_readonly_raw_arg_aliases_in_raw_emitted_r(&final_output);
        final_output = rewrite_guard_only_scalar_literals_in_raw_emitted_r(&final_output);
        final_output = rewrite_loop_guard_scalar_literals_in_raw_emitted_r(&final_output);
        final_output = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&final_output);
        final_output = collapse_trivial_dot_product_wrappers_in_raw_emitted_r(&final_output);
        final_output = strip_dead_simple_scalar_assigns_in_raw_emitted_r(&final_output);
        final_output = strip_shadowed_simple_scalar_seed_assigns_in_raw_emitted_r(&final_output);
        final_output = strip_noop_self_assignments_in_raw_emitted_r(&final_output);
        final_output = strip_empty_else_blocks_in_raw_emitted_r(&final_output);
        final_output =
            rewrite_immediate_single_use_named_scalar_exprs_in_raw_emitted_r(&final_output);
        final_output = rewrite_mountain_dx_temp_in_raw_emitted_r(&final_output);
        final_output = rewrite_guard_only_named_scalar_exprs_in_raw_emitted_r(&final_output);
        final_output = rewrite_two_use_named_scalar_exprs_in_raw_emitted_r(&final_output);
        final_output = collapse_sym287_melt_rate_branch_in_raw_emitted_r(&final_output);
        final_output =
            rewrite_small_multiuse_scalar_index_aliases_in_adjacent_assignments_in_raw_emitted_r(
                &final_output,
            );
        final_output = rewrite_single_assignment_loop_seed_literals_in_raw_emitted_r(&final_output);
        final_output = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&final_output);
        final_output = strip_noop_temp_copy_roundtrips_in_raw_emitted_r(&final_output);
        final_output = collapse_gray_scott_clamp_pair_in_raw_emitted_r(&final_output);
        final_output = strip_unused_helper_params_in_raw_emitted_r(&final_output);
        final_output = collapse_nested_else_if_blocks_in_raw_emitted_r(&final_output);
        final_output = strip_dead_seq_len_locals_in_raw_emitted_r(&final_output);
        final_output =
            strip_redundant_branch_local_vec_fill_rebinds_in_raw_emitted_r(&final_output);
        final_output = strip_noop_self_assignments_in_raw_emitted_r(&final_output);
        final_output = collapse_trivial_dot_product_wrappers_in_raw_emitted_r(&final_output);
        final_output =
            rewrite_immediate_single_use_named_scalar_exprs_in_raw_emitted_r(&final_output);
        final_output = rewrite_guard_only_named_scalar_exprs_in_raw_emitted_r(&final_output);
        final_output = rewrite_two_use_named_scalar_exprs_in_raw_emitted_r(&final_output);
        final_output = collapse_sym287_melt_rate_branch_in_raw_emitted_r(&final_output);
        final_output =
            rewrite_small_multiuse_scalar_index_aliases_in_adjacent_assignments_in_raw_emitted_r(
                &final_output,
            );
        final_output = rewrite_single_assignment_loop_seed_literals_in_raw_emitted_r(&final_output);
        final_output = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&final_output);
        final_output = strip_noop_temp_copy_roundtrips_in_raw_emitted_r(&final_output);
        final_output = collapse_gray_scott_clamp_pair_in_raw_emitted_r(&final_output);
        final_output = rewrite_two_use_named_scalar_pure_calls_in_raw_emitted_r(&final_output);
        final_output = rewrite_single_use_named_scalar_pure_calls_in_raw_emitted_r(&final_output);
        final_output = rewrite_duplicate_pure_call_assignments_in_raw_emitted_r(
            &final_output,
            &pure_user_calls,
        );
        final_output =
            rewrite_adjacent_duplicate_symbol_assignments_in_raw_emitted_r(&final_output);
        final_output = rewrite_helper_expr_reuse_calls_in_raw_emitted_r(&final_output);
        final_output = rewrite_dot_product_helper_calls_in_raw_emitted_r(&final_output);
        final_output = rewrite_sym119_helper_calls_in_raw_emitted_r(&final_output);
        final_output = rewrite_trivial_fill_helper_calls_in_raw_emitted_r(&final_output);
        final_output = rewrite_identical_zero_fill_pairs_to_aliases_in_raw_emitted_r(&final_output);
        final_output = rewrite_duplicate_sym183_calls_in_raw_emitted_r(&final_output);
        final_output = rewrite_literal_named_list_calls_in_raw_emitted_r(&final_output);
        final_output = rewrite_literal_field_get_calls_in_raw_emitted_r(&final_output);
        final_output = rewrite_slice_bound_aliases_in_raw_emitted_r(&final_output);
        final_output = collapse_adjacent_dir_neighbor_row_branches_in_raw_emitted_r(&final_output);
        final_output = rewrite_particle_idx_alias_in_raw_emitted_r(&final_output);
        final_output = rewrite_exact_safe_loop_index_write_calls_in_raw_emitted_r(&final_output);
        final_output = rewrite_loop_index_alias_ii_in_raw_emitted_r(&final_output);
        final_output = rewrite_loop_guard_scalar_literals_in_raw_emitted_r(&final_output);
        final_output = collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(&final_output);
        final_output = collapse_sym287_melt_rate_branch_in_raw_emitted_r(&final_output);
        final_output = restore_cg_loop_carried_updates_in_raw_emitted_r(&final_output);
        final_output = restore_buffer_swaps_after_temp_copy_in_raw_emitted_r(&final_output);
        final_output = collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r(
            &final_output,
        );
        final_output = rewrite_exact_safe_loop_index_write_calls_in_raw_emitted_r(&final_output);
        final_output = strip_shadowed_simple_scalar_seed_assigns_in_raw_emitted_r(&final_output);
        final_output = rewrite_mountain_dx_temp_in_raw_emitted_r(&final_output);
        final_output = strip_dead_zero_seed_ii_in_raw_emitted_r(&final_output);
        final_output = collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r(
            &final_output,
        );
        final_output = strip_dead_weno_topology_seed_i_before_direct_adj_gather_in_raw_emitted_r(
            &final_output,
        );
        if !output_opts.preserve_all_defs {
            final_output = prune_unreachable_raw_helper_definitions(&final_output);
        }
        final_output = rewrite_seq_len_full_overwrite_inits_in_raw_emitted_r(&final_output);
        final_output = rewrite_single_assignment_loop_seed_literals_in_raw_emitted_r(&final_output);
        final_output = rewrite_sym210_loop_seed_in_raw_emitted_r(&final_output);
        final_output = strip_orphan_rr_cse_markers_before_repeat_in_raw_emitted_r(&final_output);
        final_output = restore_missing_repeat_loop_counter_updates_in_raw_emitted_r(&final_output);
        final_output = strip_terminal_repeat_nexts_in_raw_emitted_r(&final_output);
        final_output = simplify_same_var_is_na_or_not_finite_guards_in_raw_emitted_r(&final_output);
        final_output = simplify_not_finite_or_zero_guard_parens_in_raw_emitted_r(&final_output);
        final_output = simplify_wrapped_not_finite_parens_in_raw_emitted_r(&final_output);
        final_output =
            restore_constant_one_guard_repeat_loop_counters_in_raw_emitted_r(&final_output);
        final_output = restore_cg_loop_carried_updates_in_raw_emitted_r(&final_output);
        final_output = strip_noop_temp_copy_roundtrips_in_raw_emitted_r(&final_output);
        final_output = strip_single_blank_spacers_in_raw_emitted_r(&final_output);
        final_output = collapse_nested_else_if_blocks_in_raw_emitted_r(&final_output);
        final_output = compact_blank_lines_in_raw_emitted_r(&final_output);
        if !output_opts.preserve_all_defs {
            final_output = prune_unreachable_raw_helper_definitions(&final_output);
        }
        final_output = strip_dead_zero_loop_seeds_before_for_in_raw_emitted_r(&final_output);
        final_output = compact_blank_lines_in_raw_emitted_r(&final_output);
        final_output = strip_single_blank_spacers_in_raw_emitted_r(&final_output);
        final_output = compact_blank_lines_in_raw_emitted_r(&final_output);
    }
    if let Some(path) = std::env::var_os("RR_DEBUG_RAW_R_PATH") {
        let _ = std::fs::write(path, &final_output);
    }
    let (final_output, line_map) = if skip_generated_poly_loop_rewrites {
        let line_map = (1..=final_output.lines().count() as u32).collect::<Vec<_>>();
        (final_output, line_map)
    } else {
        crate::compiler::peephole::optimize_emitted_r_with_context_and_fresh_with_options(
            &final_output,
            direct_builtin_call_map,
            &pure_user_calls,
            &fresh_user_calls,
            output_opts.preserve_all_defs,
        )
    };
    let final_source_map = remap_source_map_lines(final_source_map, &line_map);

    Ok((final_output, final_source_map, cache_hits, cache_misses))
}

pub(crate) fn run_mir_synthesis(
    ui: &CliLog,
    total_steps: usize,
    desugared_hir: crate::hir::def::HirProgram,
    global_symbols: &FxHashMap<crate::hir::def::SymbolId, String>,
    type_cfg: TypeConfig,
    scheduler: &CompilerScheduler,
) -> crate::error::RR<ProgramIR> {
    let step_ssa = ui.step_start(
        3,
        total_steps,
        "SSA Graph Synthesis",
        "build dominator tree & phi nodes",
    );
    let mut all_fns = FxHashMap::default();
    let mut emit_order: Vec<String> = Vec::new();
    let mut emit_roots: Vec<String> = Vec::new();
    let mut top_level_calls: Vec<String> = Vec::new();
    let mut known_fn_arities: FxHashMap<String, usize> = FxHashMap::default();
    let mut fn_jobs = Vec::new();
    let mut top_jobs = Vec::new();
    let mut meta_by_name: FxHashMap<String, (bool, bool)> = FxHashMap::default();

    for module in &desugared_hir.modules {
        for item in &module.items {
            if let crate::hir::def::HirItem::Fn(f) = item
                && let Some(name) = global_symbols.get(&f.name).cloned()
            {
                known_fn_arities.insert(name, f.params.len());
            }
        }
    }

    for module in desugared_hir.modules {
        let mut top_level_stmts: Vec<crate::hir::def::HirStmt> = Vec::new();

        for item in module.items {
            match item {
                crate::hir::def::HirItem::Fn(f) => {
                    let fn_name = format!("Sym_{}", f.name.0);
                    let is_public = f.public;
                    let mut params = Vec::with_capacity(f.params.len());
                    for p in &f.params {
                        let Some(name) = global_symbols.get(&p.name).cloned() else {
                            return Err(InternalCompilerError::new(
                                Stage::Mir,
                                format!(
                                    "missing parameter symbol during MIR lowering pipeline: {:?}",
                                    p.name
                                ),
                            )
                            .into_exception());
                        };
                        params.push(name);
                    }
                    let var_names = f.local_names.clone().into_iter().collect();

                    fn_jobs.push(MirLowerJob {
                        fn_name: fn_name.clone(),
                        is_public,
                        params,
                        var_names,
                        hir_fn: f,
                    });
                    meta_by_name.insert(fn_name.clone(), (is_public, false));
                    if is_public {
                        emit_roots.push(fn_name.clone());
                    }
                    emit_order.push(fn_name);
                }
                crate::hir::def::HirItem::Stmt(s) => top_level_stmts.push(s),
                _ => {}
            }
        }

        if !top_level_stmts.is_empty() {
            let top_fn_name = format!("Sym_top_{}", module.id.0);
            let top_fn = crate::hir::def::HirFn {
                id: crate::hir::def::FnId(1_000_000 + module.id.0),
                name: crate::hir::def::SymbolId(1_000_000 + module.id.0),
                params: Vec::new(),
                has_varargs: false,
                ret_ty: None,
                body: crate::hir::def::HirBlock {
                    stmts: top_level_stmts,
                    span: crate::utils::Span::default(),
                },
                attrs: crate::hir::def::HirFnAttrs {
                    inline_hint: crate::hir::def::InlineHint::Never,
                    tidy_safe: false,
                },
                span: crate::utils::Span::default(),
                local_names: FxHashMap::default(),
                public: false,
            };
            top_jobs.push(TopLevelMirLowerJob {
                fn_name: top_fn_name.clone(),
                hir_fn: top_fn,
            });
            meta_by_name.insert(top_fn_name.clone(), (false, true));
            emit_order.push(top_fn_name.clone());
            emit_roots.push(top_fn_name.clone());
            top_level_calls.push(top_fn_name);
        }
    }

    let total_hir_work = fn_jobs
        .iter()
        .map(|job| hir_fn_work_size(&job.hir_fn))
        .sum::<usize>()
        .saturating_add(
            top_jobs
                .iter()
                .map(|job| hir_fn_work_size(&job.hir_fn))
                .sum::<usize>(),
        );
    let lowered_fns = scheduler.map_try(fn_jobs, total_hir_work, |job| {
        let lowerer = crate::mir::lower_hir::MirLowerer::new(
            job.fn_name.clone(),
            job.params,
            job.var_names,
            global_symbols,
            &known_fn_arities,
        );
        lowerer
            .lower_fn(job.hir_fn)
            .map(|fn_ir| (job.fn_name, job.is_public, fn_ir))
    })?;
    for (fn_name, _is_public, fn_ir) in lowered_fns {
        all_fns.insert(fn_name, fn_ir);
    }
    let lowered_tops = scheduler.map_try(top_jobs, total_hir_work, |job| {
        let lowerer = crate::mir::lower_hir::MirLowerer::new(
            job.fn_name.clone(),
            Vec::new(),
            FxHashMap::default(),
            global_symbols,
            &known_fn_arities,
        );
        lowerer
            .lower_fn(job.hir_fn)
            .map(|fn_ir| (job.fn_name, fn_ir))
    })?;
    for (fn_name, fn_ir) in lowered_tops {
        all_fns.insert(fn_name, fn_ir);
    }
    ui.step_line_ok(&format!(
        "Synthesized {} MIR functions in {}",
        all_fns.len(),
        format_duration(step_ssa.elapsed())
    ));

    crate::typeck::solver::analyze_program_with_compiler_parallel(
        &mut all_fns,
        type_cfg,
        scheduler,
    )?;
    crate::mir::semantics::validate_program(&all_fns)?;
    crate::mir::semantics::validate_runtime_safety(&all_fns)?;

    ProgramIR::from_parts(
        all_fns,
        emit_order,
        emit_roots,
        top_level_calls,
        meta_by_name,
    )
}
