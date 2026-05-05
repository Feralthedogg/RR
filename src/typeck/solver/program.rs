use super::*;
pub fn analyze_program(all_fns: &mut FxHashMap<String, FnIR>, cfg: TypeConfig) -> RR<()> {
    let scheduler = CompilerScheduler::new(CompilerParallelConfig::default());
    analyze_program_with_compiler_parallel(all_fns, cfg, &scheduler)
}

pub fn analyze_program_with_compiler_parallel(
    all_fns: &mut FxHashMap<String, FnIR>,
    cfg: TypeConfig,
    scheduler: &CompilerScheduler,
) -> RR<()> {
    let mut fn_ret: FxHashMap<String, TypeState> = FxHashMap::default();
    let mut fn_ret_term: FxHashMap<String, TypeTerm> = FxHashMap::default();
    let mut init_names: Vec<String> = all_fns.keys().cloned().collect();
    init_names.sort();
    for name in init_names {
        let Some(fn_ir) = all_fns.get(&name) else {
            continue;
        };
        fn_ret.insert(
            name.clone(),
            fn_ir.ret_ty_hint.unwrap_or(TypeState::unknown()),
        );
        fn_ret_term.insert(name, fn_ir.ret_term_hint.clone().unwrap_or(TypeTerm::Any));
    }

    let index_param_slots = collect_index_vector_param_slots_by_function(all_fns);
    let scalar_ret_demands = collect_scalar_index_return_demands(all_fns);
    let vector_ret_demands = collect_vector_index_return_demands(all_fns, &index_param_slots);
    let _ = apply_index_return_demands(
        all_fns,
        &mut fn_ret,
        &mut fn_ret_term,
        &scalar_ret_demands,
        &vector_ret_demands,
    );

    let scc_waves = type_solver_scc_waves(all_fns);
    for wave in scc_waves {
        let wave_total_ir = wave
            .iter()
            .flat_map(|names| names.iter())
            .filter_map(|name| all_fns.get(name))
            .map(type_fn_ir_work_size)
            .sum::<usize>();
        let mut wave_jobs = Vec::with_capacity(wave.len());
        for names in wave {
            let mut fns = Vec::with_capacity(names.len());
            for name in &names {
                let Some(fn_ir) = all_fns.remove(name) else {
                    return Err(InternalCompilerError::new(
                        Stage::Mir,
                        format!("type solver missing SCC member '{}'", name),
                    )
                    .into_exception());
                };
                fns.push((name.clone(), fn_ir));
            }
            wave_jobs.push(TypeSccJob { names, fns });
        }

        let solved_jobs = scheduler.map_try_stage(
            CompilerParallelStage::TypeAnalysis,
            wave_jobs,
            wave_total_ir,
            |job| {
                solve_type_scc_job(
                    job,
                    &fn_ret,
                    &fn_ret_term,
                    &scalar_ret_demands,
                    &vector_ret_demands,
                )
            },
        )?;

        for solved in solved_jobs {
            for (name, ret_ty, ret_term) in &solved.summaries {
                fn_ret.insert(name.clone(), *ret_ty);
                fn_ret_term.insert(name.clone(), ret_term.clone());
            }
            for (name, fn_ir) in solved.fns {
                all_fns.insert(name, fn_ir);
            }
        }
    }

    finish_type_analysis(all_fns, cfg)
}

pub(crate) struct TypeSccJob {
    pub(crate) names: Vec<String>,
    pub(crate) fns: Vec<(String, FnIR)>,
}

pub(crate) struct TypeSolvedScc {
    pub(crate) fns: Vec<(String, FnIR)>,
    pub(crate) summaries: Vec<(String, TypeState, TypeTerm)>,
}

pub(crate) fn type_fn_ir_work_size(fn_ir: &FnIR) -> usize {
    fn_ir.values.len()
        + fn_ir.blocks.len()
        + fn_ir.blocks.iter().map(|bb| bb.instrs.len()).sum::<usize>()
}

pub(crate) fn type_called_user_fns(fn_ir: &FnIR, all_fns: &FxHashMap<String, FnIR>) -> Vec<String> {
    let mut out = FxHashSet::default();
    for value in &fn_ir.values {
        let ValueKind::Call { callee, .. } = &value.kind else {
            continue;
        };
        if all_fns.contains_key(callee.as_str()) {
            out.insert(callee.clone());
        }
    }
    let mut out: Vec<String> = out.into_iter().collect();
    out.sort();
    out
}

pub(crate) fn type_solver_scc_waves(all_fns: &FxHashMap<String, FnIR>) -> Vec<Vec<Vec<String>>> {
    fn dfs_order(
        name: &str,
        graph: &FxHashMap<String, Vec<String>>,
        visited: &mut FxHashSet<String>,
        order: &mut Vec<String>,
    ) {
        if !visited.insert(name.to_string()) {
            return;
        }
        if let Some(nexts) = graph.get(name) {
            for next in nexts {
                dfs_order(next, graph, visited, order);
            }
        }
        order.push(name.to_string());
    }

    fn dfs_rev(
        name: &str,
        rev_graph: &FxHashMap<String, Vec<String>>,
        visited: &mut FxHashSet<String>,
        scc: &mut Vec<String>,
    ) {
        if !visited.insert(name.to_string()) {
            return;
        }
        scc.push(name.to_string());
        if let Some(nexts) = rev_graph.get(name) {
            for next in nexts {
                dfs_rev(next, rev_graph, visited, scc);
            }
        }
    }

    let mut names: Vec<String> = all_fns.keys().cloned().collect();
    names.sort();
    let mut graph: FxHashMap<String, Vec<String>> = FxHashMap::default();
    let mut rev_graph: FxHashMap<String, Vec<String>> = FxHashMap::default();
    for name in &names {
        let Some(fn_ir) = all_fns.get(name) else {
            continue;
        };
        let callees = type_called_user_fns(fn_ir, all_fns);
        graph.insert(name.clone(), callees.clone());
        for callee in callees {
            rev_graph.entry(callee).or_default().push(name.clone());
        }
    }
    for edges in rev_graph.values_mut() {
        edges.sort();
        edges.dedup();
    }

    let mut order = Vec::new();
    let mut visited = FxHashSet::default();
    for name in &names {
        dfs_order(name, &graph, &mut visited, &mut order);
    }

    let mut sccs = Vec::new();
    let mut assigned = FxHashSet::default();
    for name in order.into_iter().rev() {
        if assigned.contains(&name) {
            continue;
        }
        let mut scc = Vec::new();
        dfs_rev(&name, &rev_graph, &mut assigned, &mut scc);
        scc.sort();
        sccs.push(scc);
    }
    sccs.sort_by(|a, b| a[0].cmp(&b[0]));

    let mut scc_of = FxHashMap::default();
    for (idx, scc) in sccs.iter().enumerate() {
        for name in scc {
            scc_of.insert(name.clone(), idx);
        }
    }

    let mut outgoing: Vec<FxHashSet<usize>> = vec![FxHashSet::default(); sccs.len()];
    let mut incoming: Vec<FxHashSet<usize>> = vec![FxHashSet::default(); sccs.len()];
    for (idx, scc) in sccs.iter().enumerate() {
        for name in scc {
            let Some(fn_ir) = all_fns.get(name) else {
                continue;
            };
            for callee in type_called_user_fns(fn_ir, all_fns) {
                let Some(&callee_idx) = scc_of.get(&callee) else {
                    continue;
                };
                if callee_idx != idx {
                    outgoing[idx].insert(callee_idx);
                    incoming[callee_idx].insert(idx);
                }
            }
        }
    }

    let mut remaining_out: Vec<usize> = outgoing.iter().map(FxHashSet::len).collect();
    let mut ready: Vec<usize> = remaining_out
        .iter()
        .enumerate()
        .filter_map(|(idx, count)| (*count == 0).then_some(idx))
        .collect();
    ready.sort_by(|lhs, rhs| sccs[*lhs][0].cmp(&sccs[*rhs][0]));

    let mut waves = Vec::new();
    let mut scheduled = FxHashSet::default();
    while !ready.is_empty() {
        let current = ready.clone();
        let mut wave = Vec::with_capacity(current.len());
        let mut next_ready = Vec::new();
        for idx in current {
            if !scheduled.insert(idx) {
                continue;
            }
            wave.push(sccs[idx].clone());
            for parent in &incoming[idx] {
                remaining_out[*parent] = remaining_out[*parent].saturating_sub(1);
                if remaining_out[*parent] == 0 {
                    next_ready.push(*parent);
                }
            }
        }
        wave.sort_by(|lhs, rhs| lhs[0].cmp(&rhs[0]));
        waves.push(wave);
        next_ready.sort_by(|lhs, rhs| sccs[*lhs][0].cmp(&sccs[*rhs][0]));
        next_ready.dedup();
        ready = next_ready;
    }
    waves
}

pub(crate) fn can_apply_index_return_override_for_fn(
    fn_ir: &FnIR,
    demanded_shape: ShapeTy,
    demanded_term: &TypeTerm,
) -> bool {
    if let Some(hint) = fn_ir.ret_ty_hint
        && hint != TypeState::unknown()
    {
        if hint.shape != ShapeTy::Unknown && hint.shape != demanded_shape {
            return false;
        }
        if hint.prim != PrimTy::Any && hint.prim != PrimTy::Int {
            return false;
        }
    }
    if let Some(term_hint) = &fn_ir.ret_term_hint
        && !term_hint.is_any()
        && !term_hint.compatible_with(demanded_term)
    {
        return false;
    }
    true
}

pub(crate) fn solve_type_scc_job(
    mut job: TypeSccJob,
    global_ret: &FxHashMap<String, TypeState>,
    global_ret_term: &FxHashMap<String, TypeTerm>,
    scalar_ret_demands: &FxHashSet<String>,
    vector_ret_demands: &FxHashSet<String>,
) -> RR<TypeSolvedScc> {
    let mut summary_ret = global_ret.clone();
    let mut summary_ret_term = global_ret_term.clone();
    let vec_term = TypeTerm::Vector(Box::new(TypeTerm::Int));
    let mut changed = true;
    let mut guard = 0usize;

    while changed && guard < 16 {
        guard += 1;
        changed = false;
        for (name, fn_ir) in &mut job.fns {
            let enforce_vector_ret = vector_ret_demands.contains(name)
                && can_apply_index_return_override_for_fn(fn_ir, ShapeTy::Vector, &vec_term);
            let enforce_scalar_ret = scalar_ret_demands.contains(name)
                && can_apply_index_return_override_for_fn(fn_ir, ShapeTy::Scalar, &TypeTerm::Int);

            let mut ret = analyze_function(fn_ir, &summary_ret)?;
            let mut ret_term = analyze_function_terms(fn_ir, &summary_ret_term);
            if enforce_vector_ret {
                ret = coerce_index_vector_return(ret);
                ret_term = vec_term.clone();
            } else if enforce_scalar_ret {
                ret = coerce_index_scalar_return(ret);
                ret_term = TypeTerm::Int;
            }

            let prev = summary_ret
                .get(name)
                .copied()
                .unwrap_or(TypeState::unknown());
            let prev_term = summary_ret_term.get(name).cloned().unwrap_or(TypeTerm::Any);
            if ret != prev {
                summary_ret.insert(name.clone(), ret);
                changed = true;
            }
            if ret_term != prev_term {
                summary_ret_term.insert(name.clone(), ret_term.clone());
                changed = true;
            }
            fn_ir.inferred_ret_ty = ret;
            fn_ir.inferred_ret_term = ret_term;
        }
    }

    let summaries = job
        .names
        .iter()
        .map(|name| {
            (
                name.clone(),
                summary_ret
                    .get(name)
                    .copied()
                    .unwrap_or(TypeState::unknown()),
                summary_ret_term.get(name).cloned().unwrap_or(TypeTerm::Any),
            )
        })
        .collect();

    Ok(TypeSolvedScc {
        fns: job.fns,
        summaries,
    })
}
