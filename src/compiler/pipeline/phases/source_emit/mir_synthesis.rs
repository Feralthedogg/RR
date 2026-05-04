use super::*;

pub(crate) fn run_mir_synthesis(
    ui: &CliLog,
    total_steps: usize,
    mut desugared_hir: crate::hir::def::HirProgram,
    global_symbols: &FxHashMap<crate::hir::def::SymbolId, String>,
    type_cfg: TypeConfig,
    scheduler: &CompilerScheduler,
) -> crate::error::RR<(ProgramIR, MirSynthesisMetrics)> {
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

    crate::typeck::hm::apply_hm_hints(&mut desugared_hir, global_symbols);

    for module in &desugared_hir.modules {
        for item in &module.items {
            if let crate::hir::def::HirItem::Fn(f) = item
                && f.type_params.is_empty()
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
                crate::hir::def::HirItem::Fn(f) if f.type_params.is_empty() => {
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
                crate::hir::def::HirItem::Fn(_) => {}
                crate::hir::def::HirItem::Stmt(s) => top_level_stmts.push(s),
                _ => {}
            }
        }

        if !top_level_stmts.is_empty() {
            let top_fn_name = format!("Sym_top_{}", module.id.0);
            let top_fn = crate::hir::def::HirFn {
                id: crate::hir::def::FnId(1_000_000 + module.id.0),
                name: crate::hir::def::SymbolId(1_000_000 + module.id.0),
                type_params: Vec::new(),
                where_bounds: Vec::new(),
                params: Vec::new(),
                has_varargs: false,
                ret_ty: None,
                ret_ty_inferred: false,
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
    let lowered_fns = scheduler.map_try_stage(
        crate::compiler::scheduler::CompilerParallelStage::MirLowering,
        fn_jobs,
        total_hir_work,
        |job| {
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
        },
    )?;
    for (fn_name, _is_public, fn_ir) in lowered_fns {
        all_fns.insert(fn_name, fn_ir);
    }
    let lowered_tops = scheduler.map_try_stage(
        crate::compiler::scheduler::CompilerParallelStage::MirLowering,
        top_jobs,
        total_hir_work,
        |job| {
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
        },
    )?;
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

    let program = ProgramIR::from_parts(
        all_fns,
        emit_order,
        emit_roots,
        top_level_calls,
        meta_by_name,
    )?;
    let lowered_functions = program.fns.len();
    Ok((
        program,
        MirSynthesisMetrics {
            elapsed_ns: step_ssa.elapsed().as_nanos(),
            lowered_functions,
        },
    ))
}
