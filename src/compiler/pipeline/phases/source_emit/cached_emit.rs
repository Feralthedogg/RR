use super::raw_emit::*;
use super::*;
use crate::mir::FnIR;
use assembly_cache::store_optimized_assembly_cache_state;
use metrics::{EmitMetricContext, EmitStageTimings, OptimizedAssemblyHits};
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};
use std::time::Instant;

#[path = "cached_emit/assembly_cache.rs"]
pub(crate) mod assembly_cache;
#[path = "cached_emit/metrics.rs"]
pub(crate) mod metrics;

pub(crate) type EmitFunctionsResult = (String, Vec<MapEntry>, usize, usize, EmitMetrics);

pub(crate) struct EmitFunctionsRequest<'a> {
    pub(crate) ui: &'a CliLog,
    pub(crate) total_steps: usize,
    pub(crate) program: &'a ProgramIR,
    pub(crate) emit_order: &'a [FnSlot],
    pub(crate) top_level_calls: &'a [FnSlot],
    pub(crate) opt_level: OptLevel,
    pub(crate) type_cfg: TypeConfig,
    pub(crate) parallel_cfg: ParallelConfig,
    pub(crate) scheduler: &'a CompilerScheduler,
    pub(crate) output_opts: CompileOutputOptions,
    pub(crate) cache: Option<&'a dyn EmitFunctionCache>,
}

#[derive(Clone)]
pub(crate) struct EmitFragmentCounters {
    pub(crate) cache_load_elapsed_ns: Arc<AtomicU64>,
    pub(crate) emitter_elapsed_ns: Arc<AtomicU64>,
    pub(crate) cache_store_elapsed_ns: Arc<AtomicU64>,
    pub(crate) quote_wrap_elapsed_ns: Arc<AtomicU64>,
    pub(crate) quoted_wrapped_functions: Arc<AtomicUsize>,
}

impl Default for EmitFragmentCounters {
    fn default() -> Self {
        Self {
            cache_load_elapsed_ns: Arc::new(AtomicU64::new(0)),
            emitter_elapsed_ns: Arc::new(AtomicU64::new(0)),
            cache_store_elapsed_ns: Arc::new(AtomicU64::new(0)),
            quote_wrap_elapsed_ns: Arc::new(AtomicU64::new(0)),
            quoted_wrapped_functions: Arc::new(AtomicUsize::new(0)),
        }
    }
}

pub(crate) struct EmitFragmentBuildOutcome {
    pub(crate) fragments: Vec<EmittedFnFragment>,
    pub(crate) elapsed_ns: u128,
    pub(crate) counters: EmitFragmentCounters,
}

pub(crate) struct EmitFragmentBuildRequest<'a> {
    pub(crate) program: &'a ProgramIR,
    pub(crate) emit_order: &'a [FnSlot],
    pub(crate) opt_level: OptLevel,
    pub(crate) type_cfg: TypeConfig,
    pub(crate) parallel_cfg: ParallelConfig,
    pub(crate) scheduler: &'a CompilerScheduler,
    pub(crate) output_opts: CompileOutputOptions,
    pub(crate) cache: Option<&'a dyn EmitFunctionCache>,
    pub(crate) quoted_entry_targets: &'a FxHashSet<String>,
    pub(crate) pure_user_calls: &'a FxHashSet<String>,
    pub(crate) fresh_user_calls: &'a FxHashSet<String>,
    pub(crate) seq_len_param_end_slots_by_fn: &'a FxHashMap<String, FxHashMap<usize, usize>>,
    pub(crate) direct_builtin_call_map: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct OptimizedAssemblyOptions {
    pub(crate) opt_level: OptLevel,
    pub(crate) direct_builtin_call_map: bool,
    pub(crate) output_opts: CompileOutputOptions,
}

pub(crate) struct OptimizedAssemblyCalls<'a> {
    pub(crate) pure_user_calls: &'a FxHashSet<String>,
    pub(crate) fresh_user_calls: &'a FxHashSet<String>,
}

#[derive(Clone, Copy)]
pub(crate) struct FragmentCacheCounts {
    pub(crate) hits: usize,
    pub(crate) misses: usize,
}

pub(crate) struct OptimizedAssemblyFastPath<'a> {
    pub(crate) program: &'a ProgramIR,
    pub(crate) cache: Option<&'a dyn EmitFunctionCache>,
    pub(crate) emitted_fragments: &'a [EmittedFnFragment],
    pub(crate) final_output: &'a str,
    pub(crate) options: OptimizedAssemblyOptions,
    pub(crate) calls: OptimizedAssemblyCalls<'a>,
    pub(crate) cache_counts: FragmentCacheCounts,
    pub(crate) metric_context: &'a EmitMetricContext<'a>,
    pub(crate) elapsed_ns: u128,
    pub(crate) fragment_assembly_elapsed_ns: u128,
}

pub(crate) struct OptimizedAssemblyCachePaths<'a> {
    pub(crate) base: OptimizedAssemblyFastPath<'a>,
    pub(crate) optimized_assembly_key: Option<&'a str>,
    pub(crate) optimized_fragment_variants_ready: bool,
    pub(crate) cached_optimized_final_artifact: Option<(String, Vec<MapEntry>)>,
    pub(crate) cached_optimized_final_source_map: Option<&'a [MapEntry]>,
}

pub(crate) struct EmitFragmentWorker<'a> {
    pub(crate) program: &'a ProgramIR,
    pub(crate) opt_level: OptLevel,
    pub(crate) type_cfg: TypeConfig,
    pub(crate) parallel_cfg: ParallelConfig,
    pub(crate) output_opts: CompileOutputOptions,
    pub(crate) cache: Option<&'a dyn EmitFunctionCache>,
    pub(crate) quoted_entry_targets: &'a FxHashSet<String>,
    pub(crate) direct_builtin_call_map: bool,
    pub(crate) pure_user_calls: Arc<FxHashSet<String>>,
    pub(crate) fresh_user_calls: Arc<FxHashSet<String>>,
    pub(crate) seq_len_param_end_slots_by_fn: Arc<FxHashMap<String, FxHashMap<usize, usize>>>,
    pub(crate) counters: EmitFragmentCounters,
}

impl EmitFragmentWorker<'_> {
    pub(crate) fn emit_slot(&self, slot: FnSlot) -> crate::error::RR<EmittedFnFragment> {
        let Some(unit) = self.program.fns.get(slot) else {
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
            self.opt_level,
            self.type_cfg,
            self.parallel_cfg,
            self.output_opts.compile_mode,
            self.seq_len_param_end_slots_by_fn.get(unit.name.as_str()),
        );
        let (mut code, mut map, cache_hit) = self.load_or_emit_fragment(fn_ir, &key)?;
        self.wrap_quoted_entry_body(unit.name.as_str(), &mut code, &mut map);
        let (optimized_code, optimized_map, optimized_cache_hit) =
            self.load_or_optimize_fragment(&code, &map)?;
        Ok(EmittedFnFragment {
            code,
            map,
            cache_hit,
            optimized_code,
            optimized_map,
            optimized_cache_hit,
        })
    }

    pub(crate) fn load_or_emit_fragment(
        &self,
        fn_ir: &FnIR,
        key: &str,
    ) -> crate::error::RR<(String, Vec<MapEntry>, bool)> {
        if let Some(cached) = self.load_cached_fragment(key)? {
            return Ok((cached.0, cached.1, true));
        }
        let mut emitter = crate::codegen::mir_emit::MirEmitter::with_shared_analysis_options(
            Arc::clone(&self.fresh_user_calls),
            Arc::clone(&self.pure_user_calls),
            Arc::clone(&self.seq_len_param_end_slots_by_fn),
            self.direct_builtin_call_map,
        );
        let emit_started = Instant::now();
        let (code, map) = emitter.emit(fn_ir)?;
        self.counters
            .emitter_elapsed_ns
            .fetch_add(emit_started.elapsed().as_nanos() as u64, Ordering::Relaxed);
        if let Some(cache) = self.cache {
            let store_started = Instant::now();
            cache.store(key, &code, &map)?;
            self.counters
                .cache_store_elapsed_ns
                .fetch_add(store_started.elapsed().as_nanos() as u64, Ordering::Relaxed);
        }
        Ok((code, map, false))
    }

    pub(crate) fn load_cached_fragment(
        &self,
        key: &str,
    ) -> crate::error::RR<Option<(String, Vec<MapEntry>)>> {
        let Some(cache) = self.cache else {
            return Ok(None);
        };
        let started = Instant::now();
        let loaded = cache.load(key)?;
        self.counters
            .cache_load_elapsed_ns
            .fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
        Ok(loaded)
    }

    pub(crate) fn wrap_quoted_entry_body(
        &self,
        fn_name: &str,
        code: &mut String,
        map: &mut [MapEntry],
    ) {
        let wrap_started = Instant::now();
        let maybe_wrapped = if self.quoted_entry_targets.contains(fn_name) {
            wrap_zero_arg_function_body_in_quote(code, fn_name)
        } else {
            None
        };
        if let Some(wrapped) = maybe_wrapped {
            *code = wrapped;
            for entry in map {
                entry.r_line = entry.r_line.saturating_sub(1);
            }
            self.counters
                .quote_wrap_elapsed_ns
                .fetch_add(wrap_started.elapsed().as_nanos() as u64, Ordering::Relaxed);
            self.counters
                .quoted_wrapped_functions
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(crate) fn load_or_optimize_fragment(
        &self,
        code: &str,
        map: &[MapEntry],
    ) -> crate::error::RR<(Option<String>, Option<Vec<MapEntry>>, bool)> {
        let Some(cache) = self.cache else {
            return Ok((None, None, false));
        };
        let key = crate::compiler::pipeline::optimized_fragment_output_cache_key(
            code,
            self.pure_user_calls.as_ref(),
            self.fresh_user_calls.as_ref(),
            crate::compiler::pipeline::OutputCacheKeyOptions {
                opt_level: self.opt_level,
                direct_builtin_call_map: self.direct_builtin_call_map,
                preserve_all_defs: self.output_opts.preserve_all_defs,
                compile_mode: self.output_opts.compile_mode,
            },
        );
        if let Some((optimized_code, optimized_map)) = cache.load_optimized_fragment(&key)? {
            return Ok((Some(optimized_code), Some(optimized_map), true));
        }
        let (optimized_code, optimized_map) = optimize_emitted_fragment(
            code,
            map,
            self.opt_level,
            self.direct_builtin_call_map,
            self.pure_user_calls.as_ref(),
            self.fresh_user_calls.as_ref(),
            self.output_opts,
        );
        cache.store_optimized_fragment(&key, &optimized_code, &optimized_map)?;
        Ok((Some(optimized_code), Some(optimized_map), false))
    }
}

pub(crate) fn build_emitted_fragments(
    request: EmitFragmentBuildRequest<'_>,
) -> crate::error::RR<EmitFragmentBuildOutcome> {
    let shared_pure_user_calls = Arc::new(request.pure_user_calls.clone());
    let shared_fresh_user_calls = Arc::new(request.fresh_user_calls.clone());
    let shared_seq_len_param_end_slots_by_fn =
        Arc::new(request.seq_len_param_end_slots_by_fn.clone());
    let counters = EmitFragmentCounters::default();
    let worker = EmitFragmentWorker {
        program: request.program,
        opt_level: request.opt_level,
        type_cfg: request.type_cfg,
        parallel_cfg: request.parallel_cfg,
        output_opts: request.output_opts,
        cache: request.cache,
        quoted_entry_targets: request.quoted_entry_targets,
        direct_builtin_call_map: request.direct_builtin_call_map,
        pure_user_calls: shared_pure_user_calls,
        fresh_user_calls: shared_fresh_user_calls,
        seq_len_param_end_slots_by_fn: shared_seq_len_param_end_slots_by_fn,
        counters: counters.clone(),
    };
    let emit_total_ir = request
        .emit_order
        .iter()
        .filter_map(|slot| request.program.get_slot(*slot))
        .map(fn_ir_work_size)
        .sum::<usize>();
    let fragment_build_started = Instant::now();
    let fragments = request.scheduler.map_try_stage(
        crate::compiler::scheduler::CompilerParallelStage::Emit,
        request.emit_order.to_vec(),
        emit_total_ir,
        |slot| worker.emit_slot(slot),
    )?;
    Ok(EmitFragmentBuildOutcome {
        fragments,
        elapsed_ns: fragment_build_started.elapsed().as_nanos(),
        counters,
    })
}

pub(crate) fn count_fragment_cache_hits(fragments: &[EmittedFnFragment]) -> (usize, usize) {
    let cache_hits = fragments
        .iter()
        .filter(|fragment| fragment.cache_hit)
        .count();
    (cache_hits, fragments.len().saturating_sub(cache_hits))
}

pub(crate) fn count_optimized_fragment_cache_hits(
    fragments: &[EmittedFnFragment],
) -> (usize, usize) {
    let hits = fragments
        .iter()
        .filter(|fragment| fragment.optimized_cache_hit)
        .count();
    let misses = fragments
        .iter()
        .filter(|fragment| fragment.optimized_code.is_some() && !fragment.optimized_cache_hit)
        .count();
    (hits, misses)
}

pub(crate) fn try_cached_optimized_assembly_paths(
    request: OptimizedAssemblyCachePaths<'_>,
) -> crate::error::RR<Option<EmitFunctionsResult>> {
    let base = &request.base;
    if request.optimized_fragment_variants_ready
        && let Some((cached_output, cached_source_map)) = request.cached_optimized_final_artifact
    {
        let raw_debug_elapsed_ns = maybe_emit_raw_debug_output(
            base.final_output,
            base.options.opt_level,
            base.calls.pure_user_calls,
            base.options.output_opts,
            base.cache,
        )?;
        let cached_output = preserve_source_names_in_output(base.program, cached_output);
        return Ok(Some((
            cached_output,
            cached_source_map,
            base.cache_counts.hits,
            base.cache_counts.misses,
            base.metric_context.build(
                base.elapsed_ns,
                OptimizedAssemblyHits {
                    final_artifact: 1,
                    ..OptimizedAssemblyHits::default()
                },
                EmitStageTimings {
                    fragment_assembly_elapsed_ns: base.fragment_assembly_elapsed_ns,
                    raw_rewrite_elapsed_ns: raw_debug_elapsed_ns,
                    ..EmitStageTimings::default()
                },
                None,
            ),
        )));
    }

    let Some(cached_source_map) = request.cached_optimized_final_source_map else {
        return Ok(None);
    };
    if !request.optimized_fragment_variants_ready {
        return Ok(None);
    }

    let direct_fast_path = base.cache.is_some_and(|c| {
        request
            .optimized_assembly_key
            .is_some_and(|key| c.has_optimized_assembly_safe(key).unwrap_or(false))
    });
    if direct_fast_path {
        return emit_direct_optimized_assembly_fast_path(base, cached_source_map);
    }

    let raw_fast_path = base.cache.is_some_and(|c| {
        request
            .optimized_assembly_key
            .is_some_and(|key| c.has_optimized_raw_assembly_safe(key).unwrap_or(false))
    });
    if raw_fast_path {
        return emit_raw_optimized_assembly_fast_path(base, cached_source_map);
    }

    let peephole_fast_path = base.cache.is_some_and(|c| {
        request
            .optimized_assembly_key
            .is_some_and(|key| c.has_optimized_peephole_assembly_safe(key).unwrap_or(false))
    });
    if peephole_fast_path {
        return emit_peephole_optimized_assembly_fast_path(base, cached_source_map);
    }

    Ok(None)
}

pub(crate) fn emit_direct_optimized_assembly_fast_path(
    request: &OptimizedAssemblyFastPath<'_>,
    cached_source_map: &[MapEntry],
) -> crate::error::RR<Option<EmitFunctionsResult>> {
    let fast_started = Instant::now();
    let raw_debug_elapsed_ns = maybe_emit_raw_debug_output(
        request.final_output,
        request.options.opt_level,
        request.calls.pure_user_calls,
        request.options.output_opts,
        request.cache,
    )?;
    let (optimized_output, _optimized_source_map) =
        assemble_emitted_fragments(request.emitted_fragments, true);
    let final_source_map = cached_source_map.to_vec();
    let optimized_output = preserve_source_names_in_output(request.program, optimized_output);
    let fragment_assembly_elapsed_ns =
        request.fragment_assembly_elapsed_ns + fast_started.elapsed().as_nanos();
    Ok(Some((
        optimized_output,
        final_source_map,
        request.cache_counts.hits,
        request.cache_counts.misses,
        request.metric_context.build(
            request.elapsed_ns,
            OptimizedAssemblyHits {
                direct: 1,
                ..OptimizedAssemblyHits::default()
            },
            EmitStageTimings {
                fragment_assembly_elapsed_ns,
                raw_rewrite_elapsed_ns: raw_debug_elapsed_ns,
                ..EmitStageTimings::default()
            },
            None,
        ),
    )))
}

pub(crate) fn emit_raw_optimized_assembly_fast_path(
    request: &OptimizedAssemblyFastPath<'_>,
    cached_source_map: &[MapEntry],
) -> crate::error::RR<Option<EmitFunctionsResult>> {
    let fast_started = Instant::now();
    let raw_debug_elapsed_ns = maybe_emit_raw_debug_output(
        request.final_output,
        request.options.opt_level,
        request.calls.pure_user_calls,
        request.options.output_opts,
        request.cache,
    )?;
    let (optimized_output, _optimized_source_map) =
        assemble_emitted_fragments(request.emitted_fragments, true);
    let final_source_map = cached_source_map.to_vec();
    let optimized_output = if contains_unsafe_r_escape(&optimized_output) {
        optimized_output
    } else {
        apply_post_assembly_finalize_rewrites(apply_full_raw_rewrites(
            optimized_output,
            request.calls.pure_user_calls,
            request.options.output_opts,
        ))
    };
    let optimized_output = preserve_source_names_in_output(request.program, optimized_output);
    let fragment_assembly_elapsed_ns =
        request.fragment_assembly_elapsed_ns + fast_started.elapsed().as_nanos();
    Ok(Some((
        optimized_output,
        final_source_map,
        request.cache_counts.hits,
        request.cache_counts.misses,
        request.metric_context.build(
            request.elapsed_ns,
            OptimizedAssemblyHits {
                raw: 1,
                ..OptimizedAssemblyHits::default()
            },
            EmitStageTimings {
                fragment_assembly_elapsed_ns,
                raw_rewrite_elapsed_ns: fast_started.elapsed().as_nanos() + raw_debug_elapsed_ns,
                ..EmitStageTimings::default()
            },
            None,
        ),
    )))
}

pub(crate) fn emit_peephole_optimized_assembly_fast_path(
    request: &OptimizedAssemblyFastPath<'_>,
    cached_source_map: &[MapEntry],
) -> crate::error::RR<Option<EmitFunctionsResult>> {
    let fast_started = Instant::now();
    let raw_debug_elapsed_ns = maybe_emit_raw_debug_output(
        request.final_output,
        request.options.opt_level,
        request.calls.pure_user_calls,
        request.options.output_opts,
        request.cache,
    )?;
    let (optimized_output, optimized_source_map) =
        assemble_emitted_fragments(request.emitted_fragments, true);
    let (optimized_output, _optimized_source_map) = apply_full_peephole_to_output(
        &optimized_output,
        &optimized_source_map,
        request.options.opt_level,
        request.options.direct_builtin_call_map,
        request.calls.pure_user_calls,
        request.calls.fresh_user_calls,
        request.options.output_opts,
    );
    let final_source_map = cached_source_map.to_vec();
    let optimized_output = preserve_source_names_in_output(request.program, optimized_output);
    let fragment_assembly_elapsed_ns =
        request.fragment_assembly_elapsed_ns + fast_started.elapsed().as_nanos();
    Ok(Some((
        optimized_output,
        final_source_map,
        request.cache_counts.hits,
        request.cache_counts.misses,
        request.metric_context.build(
            request.elapsed_ns,
            OptimizedAssemblyHits {
                peephole: 1,
                ..OptimizedAssemblyHits::default()
            },
            EmitStageTimings {
                fragment_assembly_elapsed_ns,
                raw_rewrite_elapsed_ns: raw_debug_elapsed_ns,
                peephole_elapsed_ns: fast_started.elapsed().as_nanos(),
                ..EmitStageTimings::default()
            },
            None,
        ),
    )))
}

pub(crate) fn emit_r_functions_cached(
    request: EmitFunctionsRequest<'_>,
) -> crate::error::RR<(String, Vec<MapEntry>, usize, usize, EmitMetrics)> {
    let EmitFunctionsRequest {
        ui,
        total_steps,
        program,
        emit_order,
        top_level_calls,
        opt_level,
        type_cfg,
        parallel_cfg,
        scheduler,
        output_opts,
        cache,
    } = request;
    let step_emit = ui.step_start(
        5,
        total_steps,
        "R Code Emission",
        "reconstruct control flow",
    );
    let quoted_entry_targets = quoted_body_entry_targets(program, top_level_calls);
    let pure_user_calls = collect_referentially_pure_user_functions(program);
    let fresh_user_calls = collect_fresh_returning_user_functions(program);
    let seq_len_param_end_slots_by_fn = collect_seq_len_param_end_slots_by_fn(program);
    let direct_builtin_call_map =
        matches!(type_cfg.native_backend, crate::typeck::NativeBackend::Off)
            && matches!(parallel_cfg.mode, ParallelMode::Off);
    let fragment_build = build_emitted_fragments(EmitFragmentBuildRequest {
        program,
        emit_order,
        opt_level,
        type_cfg,
        parallel_cfg,
        scheduler,
        output_opts,
        cache,
        quoted_entry_targets: &quoted_entry_targets,
        pure_user_calls: &pure_user_calls,
        fresh_user_calls: &fresh_user_calls,
        seq_len_param_end_slots_by_fn: &seq_len_param_end_slots_by_fn,
        direct_builtin_call_map,
    })?;
    let emitted_fragments = fragment_build.fragments;
    let fragment_build_elapsed_ns = fragment_build.elapsed_ns;

    let (cache_hits, cache_misses) = count_fragment_cache_hits(&emitted_fragments);
    let (optimized_fragment_cache_hits, optimized_fragment_cache_misses) =
        count_optimized_fragment_cache_hits(&emitted_fragments);
    let metric_context = EmitMetricContext {
        emitted_functions: emit_order.len(),
        cache_hits,
        cache_misses,
        fragment_build_elapsed_ns,
        cache_load_elapsed_ns: &fragment_build.counters.cache_load_elapsed_ns,
        emitter_elapsed_ns: &fragment_build.counters.emitter_elapsed_ns,
        cache_store_elapsed_ns: &fragment_build.counters.cache_store_elapsed_ns,
        optimized_fragment_cache_hits,
        optimized_fragment_cache_misses,
        quote_wrap_elapsed_ns: &fragment_build.counters.quote_wrap_elapsed_ns,
        quoted_wrapped_functions: &fragment_build.counters.quoted_wrapped_functions,
    };
    let fragment_assembly_started = Instant::now();
    let (mut final_output, final_source_map) =
        assemble_emitted_fragments(&emitted_fragments, false);
    let fragment_assembly_elapsed_ns = fragment_assembly_started.elapsed().as_nanos();
    ui.step_line_ok(&format!(
        "Emitted {} functions ({} debug maps) in {}",
        emit_order.len(),
        final_source_map.len(),
        format_duration(step_emit.elapsed())
    ));

    let optimized_fragment_variants_ready = emitted_fragments
        .iter()
        .all(|fragment| fragment.optimized_code.is_some() && fragment.optimized_map.is_some());
    let optimized_assembly_key = cache.map(|_| {
        crate::compiler::pipeline::optimized_assembly_cache_key(
            &final_output,
            &pure_user_calls,
            &fresh_user_calls,
            crate::compiler::pipeline::OutputCacheKeyOptions {
                opt_level,
                direct_builtin_call_map,
                preserve_all_defs: output_opts.preserve_all_defs,
                compile_mode: output_opts.compile_mode,
            },
        )
    });
    let cached_optimized_final_source_map =
        if let (Some(c), Some(key)) = (cache, optimized_assembly_key.as_deref()) {
            c.load_optimized_assembly_source_map(key)?
        } else {
            None
        };
    let cached_optimized_final_artifact =
        if let (Some(c), Some(key)) = (cache, optimized_assembly_key.as_deref()) {
            c.load_optimized_assembly_artifact(key)?
        } else {
            None
        };
    let optimized_options = OptimizedAssemblyOptions {
        opt_level,
        direct_builtin_call_map,
        output_opts,
    };
    let optimized_calls = OptimizedAssemblyCalls {
        pure_user_calls: &pure_user_calls,
        fresh_user_calls: &fresh_user_calls,
    };
    let cache_counts = FragmentCacheCounts {
        hits: cache_hits,
        misses: cache_misses,
    };
    if let Some(result) = try_cached_optimized_assembly_paths(OptimizedAssemblyCachePaths {
        base: OptimizedAssemblyFastPath {
            program,
            cache,
            emitted_fragments: &emitted_fragments,
            final_output: &final_output,
            options: optimized_options,
            calls: optimized_calls,
            cache_counts,
            metric_context: &metric_context,
            elapsed_ns: step_emit.elapsed().as_nanos(),
            fragment_assembly_elapsed_ns,
        },
        optimized_assembly_key: optimized_assembly_key.as_deref(),
        optimized_fragment_variants_ready,
        cached_optimized_final_artifact,
        cached_optimized_final_source_map: cached_optimized_final_source_map.as_deref(),
    })? {
        return Ok(result);
    }

    let skip_global_output_rewrites = contains_generated_poly_loop_controls(&final_output)
        || contains_unsafe_r_escape(&final_output);
    let raw_rewrite_cache_key = cache.map(|_| {
        crate::compiler::pipeline::raw_rewrite_output_cache_key(
            &final_output,
            opt_level,
            &pure_user_calls,
            output_opts.preserve_all_defs,
            output_opts.compile_mode,
        )
    });
    let raw_rewrite_started = Instant::now();
    if !skip_global_output_rewrites {
        if let (Some(cache), Some(cache_key)) = (cache, raw_rewrite_cache_key.as_deref()) {
            if let Some(cached_output) = cache.load_raw_rewrite(cache_key)? {
                final_output = cached_output;
            } else {
                final_output = apply_full_raw_rewrites(final_output, &pure_user_calls, output_opts);
                cache.store_raw_rewrite(cache_key, &final_output)?;
            }
        } else {
            final_output = apply_full_raw_rewrites(final_output, &pure_user_calls, output_opts);
        }
    }
    let raw_rewrite_elapsed_ns = raw_rewrite_started.elapsed().as_nanos();
    if let Some(path) = std::env::var_os("RR_DEBUG_RAW_R_PATH") {
        let _ = std::fs::write(path, &final_output);
    }
    let peephole_cache_key = cache.map(|_| {
        crate::compiler::pipeline::peephole_output_cache_key(
            &final_output,
            &pure_user_calls,
            &fresh_user_calls,
            crate::compiler::pipeline::OutputCacheKeyOptions {
                opt_level,
                direct_builtin_call_map,
                preserve_all_defs: output_opts.preserve_all_defs,
                compile_mode: output_opts.compile_mode,
            },
        )
    });
    let peephole_started = Instant::now();
    let ((final_output, line_map), peephole_profile) = if skip_global_output_rewrites {
        let line_map = (1..=final_output.lines().count() as u32).collect::<Vec<_>>();
        (
            (final_output, line_map),
            crate::compiler::peephole::PeepholeProfile::default(),
        )
    } else if let (Some(cache), Some(cache_key)) = (cache, peephole_cache_key.as_deref()) {
        if let Some((cached_output, cached_line_map)) = cache.load_peephole(cache_key)? {
            (
                (cached_output, cached_line_map),
                crate::compiler::peephole::PeepholeProfile::default(),
            )
        } else {
            let options = crate::compiler::peephole::PeepholeOptions::new(direct_builtin_call_map)
                .preserving_all_defs(output_opts.preserve_all_defs)
                .fast_dev(matches!(output_opts.compile_mode, CompileMode::FastDev))
                .opt_level(opt_level);
            let ((optimized_output, line_map), profile) =
                crate::compiler::peephole::optimize_emitted_r_with_context_and_fresh_with_profile(
                    &final_output,
                    &pure_user_calls,
                    &fresh_user_calls,
                    options,
                );
            cache.store_peephole(cache_key, &optimized_output, &line_map)?;
            ((optimized_output, line_map), profile)
        }
    } else {
        let options = crate::compiler::peephole::PeepholeOptions::new(direct_builtin_call_map)
            .preserving_all_defs(output_opts.preserve_all_defs)
            .fast_dev(matches!(output_opts.compile_mode, CompileMode::FastDev))
            .opt_level(opt_level);
        crate::compiler::peephole::optimize_emitted_r_with_context_and_fresh_with_profile(
            &final_output,
            &pure_user_calls,
            &fresh_user_calls,
            options,
        )
    };
    let peephole_elapsed_ns = peephole_started.elapsed().as_nanos();
    let remap_started = Instant::now();
    let final_source_map = remap_source_map_lines(final_source_map, &line_map);
    let source_map_remap_elapsed_ns = remap_started.elapsed().as_nanos();

    store_optimized_assembly_cache_state(assembly_cache::OptimizedAssemblyStoreRequest {
        cache,
        optimized_assembly_key: optimized_assembly_key.as_deref(),
        final_output: &final_output,
        final_source_map: &final_source_map,
        emitted_fragments: &emitted_fragments,
        options: optimized_options,
        calls: OptimizedAssemblyCalls {
            pure_user_calls: &pure_user_calls,
            fresh_user_calls: &fresh_user_calls,
        },
    })?;

    Ok((
        preserve_source_names_in_output(program, final_output),
        final_source_map,
        cache_hits,
        cache_misses,
        metric_context.build(
            step_emit.elapsed().as_nanos(),
            OptimizedAssemblyHits::default(),
            EmitStageTimings {
                fragment_assembly_elapsed_ns,
                raw_rewrite_elapsed_ns,
                peephole_elapsed_ns,
                source_map_remap_elapsed_ns,
            },
            Some(&peephole_profile),
        ),
    ))
}
