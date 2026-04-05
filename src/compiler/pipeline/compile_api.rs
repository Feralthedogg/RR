use super::*;

pub(crate) fn runtime_roots_for_output(
    final_output: &str,
    include_source_bootstrap: bool,
) -> FxHashSet<String> {
    let _ = include_source_bootstrap;
    crate::runtime::referenced_runtime_symbols(final_output)
}

pub(crate) fn roots_need_strict_index_config(roots: &FxHashSet<String>) -> bool {
    roots.iter().any(|name| {
        matches!(
            name.as_str(),
            "rr_index1_read"
                | "rr_index1_read_strict"
                | "rr_index1_read_vec"
                | "rr_index1_read_idx"
                | "rr_index_vec_floor"
                | "rr_gather"
        )
    })
}

pub(crate) fn roots_need_native_parallel_config(roots: &FxHashSet<String>) -> bool {
    roots.iter().any(|name| {
        name.starts_with("rr_parallel_")
            || name.starts_with("rr_native_")
            || name.starts_with("rr_intrinsic_")
            || name.starts_with("rr_call_map_")
            || matches!(
                name.as_str(),
                "rr_parallel_typed_vec_call" | "rr_vector_scalar_fallback_enabled"
            )
    })
}

pub(crate) fn append_runtime_configuration(
    out: &mut String,
    entry_path: &str,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    include_source_bootstrap: bool,
    runtime_roots: &FxHashSet<String>,
) {
    if include_source_bootstrap {
        let source_label = Path::new(entry_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(entry_path);
        out.push_str(&format!(
            ".rr_env$file <- \"{}\";\n",
            escape_r_string(source_label)
        ));
        out.push_str(".rr_env$native_anchor_roots <- character(0);\n");
    }
    if roots_need_strict_index_config(runtime_roots) {
        out.push_str(
            ".rr_env$strict_index_read <- identical(Sys.getenv(\"RR_STRICT_INDEX_READ\", \"0\"), \"1\");\n",
        );
    }
    if roots_need_native_parallel_config(runtime_roots) {
        out.push_str(&format!(
            ".rr_env$native_backend <- \"{}\";\n",
            type_cfg.native_backend.as_str()
        ));
        out.push_str(&format!(
            ".rr_env$parallel_mode <- \"{}\";\n",
            parallel_cfg.mode.as_str()
        ));
        out.push_str(&format!(
            ".rr_env$parallel_backend <- \"{}\";\n",
            parallel_cfg.backend.as_str()
        ));
        out.push_str(&format!(
            ".rr_env$parallel_threads <- as.integer({});\n",
            parallel_cfg.threads
        ));
        out.push_str(&format!(
            ".rr_env$parallel_min_trip <- as.integer({});\n",
            parallel_cfg.min_trip
        ));
        out.push_str(
            ".rr_env$vector_fallback_base_trip <- suppressWarnings(as.integer(Sys.getenv(\"RR_VECTOR_FALLBACK_BASE_TRIP\", \"12\")));\n",
        );
        out.push_str(
            "if (is.na(.rr_env$vector_fallback_base_trip) || .rr_env$vector_fallback_base_trip < 0L) .rr_env$vector_fallback_base_trip <- 12L;\n",
        );
        out.push_str(
            ".rr_env$vector_fallback_helper_scale <- suppressWarnings(as.integer(Sys.getenv(\"RR_VECTOR_FALLBACK_HELPER_SCALE\", \"4\")));\n",
        );
        out.push_str(
            "if (is.na(.rr_env$vector_fallback_helper_scale) || .rr_env$vector_fallback_helper_scale < 0L) .rr_env$vector_fallback_helper_scale <- 4L;\n",
        );
        out.push_str(
            ".rr_env$native_autobuild <- tolower(Sys.getenv(\"RR_NATIVE_AUTOBUILD\", \"1\")) %in% c(\"1\", \"true\", \"yes\", \"on\");\n",
        );
        out.push_str(".rr_env$native_lib <- \"\";\n");
        out.push_str(".rr_env$native_loaded <- FALSE;\n");
    }
}

pub fn compile(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
) -> crate::error::RR<(String, Vec<crate::codegen::mir_emit::MapEntry>)> {
    compile_with_configs(
        entry_path,
        entry_input,
        opt_level,
        TypeConfig::default(),
        ParallelConfig::default(),
    )
}

pub fn compile_with_config(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
) -> crate::error::RR<(String, Vec<crate::codegen::mir_emit::MapEntry>)> {
    compile_with_configs(
        entry_path,
        entry_input,
        opt_level,
        type_cfg,
        ParallelConfig::default(),
    )
}

pub fn compile_with_configs(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
) -> crate::error::RR<(String, Vec<crate::codegen::mir_emit::MapEntry>)> {
    compile_with_configs_with_options(
        entry_path,
        entry_input,
        opt_level,
        type_cfg,
        parallel_cfg,
        CompileOutputOptions::default(),
    )
}

pub fn compile_with_configs_with_options(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    output_opts: CompileOutputOptions,
) -> crate::error::RR<(String, Vec<crate::codegen::mir_emit::MapEntry>)> {
    compile_with_configs_with_options_and_compiler_parallel(
        entry_path,
        entry_input,
        opt_level,
        type_cfg,
        parallel_cfg,
        CompilerParallelConfig::default(),
        output_opts,
    )
}

pub fn compile_with_configs_with_options_and_compiler_parallel(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    compiler_parallel_cfg: CompilerParallelConfig,
    output_opts: CompileOutputOptions,
) -> crate::error::RR<(String, Vec<crate::codegen::mir_emit::MapEntry>)> {
    let (code, map, _, _) = compile_with_pipeline_request(CompilePipelineRequest {
        entry_path,
        entry_input,
        opt_level,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg,
        cache: None,
        output_opts,
    })?;
    Ok((code, map))
}

pub(crate) fn compile_with_configs_using_emit_cache(
    entry_path: &str,
    entry_input: &str,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    cache: Option<&dyn EmitFunctionCache>,
    output_opts: CompileOutputOptions,
) -> crate::error::RR<(String, Vec<MapEntry>, usize, usize)> {
    compile_with_pipeline_request(CompilePipelineRequest {
        entry_path,
        entry_input,
        opt_level,
        type_cfg,
        parallel_cfg,
        compiler_parallel_cfg: CompilerParallelConfig::default(),
        cache,
        output_opts,
    })
}

pub(crate) struct CompilePipelineRequest<'a> {
    pub(crate) entry_path: &'a str,
    pub(crate) entry_input: &'a str,
    pub(crate) opt_level: OptLevel,
    pub(crate) type_cfg: TypeConfig,
    pub(crate) parallel_cfg: ParallelConfig,
    pub(crate) compiler_parallel_cfg: CompilerParallelConfig,
    pub(crate) cache: Option<&'a dyn EmitFunctionCache>,
    pub(crate) output_opts: CompileOutputOptions,
}

pub(crate) fn compile_with_configs_using_emit_cache_and_compiler_parallel(
    request: CompilePipelineRequest<'_>,
) -> crate::error::RR<(String, Vec<MapEntry>, usize, usize)> {
    compile_with_pipeline_request(request)
}

fn compile_with_pipeline_request(
    request: CompilePipelineRequest<'_>,
) -> crate::error::RR<(String, Vec<MapEntry>, usize, usize)> {
    crate::pkg::with_project_root_hint(request.entry_path, || {
        let ui = CliLog::new();
        let scheduler = CompilerScheduler::new(request.compiler_parallel_cfg);
        let compile_started = Instant::now();
        let optimize = request.opt_level.is_optimized();
        const TOTAL_STEPS: usize = 6;
        let input_label = std::path::Path::new(request.entry_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(request.entry_path);
        ui.banner(input_label, request.opt_level);

        let SourceAnalysisOutput {
            desugared_hir,
            global_symbols,
        } = run_source_analysis_and_canonicalization(
            &ui,
            request.entry_path,
            request.entry_input,
            TOTAL_STEPS,
            request.output_opts,
        )?;

        let mut program = run_mir_synthesis(
            &ui,
            TOTAL_STEPS,
            desugared_hir,
            &global_symbols,
            request.type_cfg,
            &scheduler,
        )?;

        let mut all_fns = program.take_all_fns_map()?;
        run_tachyon_phase(
            &ui,
            TOTAL_STEPS,
            optimize,
            request.opt_level,
            &mut all_fns,
            &scheduler,
        )?;
        verify_emittable_program(&all_fns)?;
        program.restore_all_fns_map(all_fns)?;
        let top_level_call_names = program.top_level_call_names();
        let emit_order = if request.output_opts.preserve_all_defs {
            program.emit_order.clone()
        } else {
            reachable_emit_order_slots(&program)
        };

        let (final_output, final_source_map, emit_cache_hits, emit_cache_misses) =
            emit_r_functions_cached(
                &ui,
                TOTAL_STEPS,
                &program,
                &emit_order,
                &program.top_level_calls,
                request.opt_level,
                request.type_cfg,
                request.parallel_cfg,
                &scheduler,
                request.output_opts,
                request.cache,
            )?;

        let final_code = if request.output_opts.inject_runtime {
            let step_runtime = ui.step_start(
                6,
                TOTAL_STEPS,
                "Runtime Injection",
                "link static analysis guards",
            );
            let with_runtime = inject_runtime_prelude(
                request.entry_path,
                request.type_cfg,
                request.parallel_cfg,
                final_output,
                &top_level_call_names,
            );
            ui.step_line_ok(&format!("Output size: {}", human_size(with_runtime.len())));
            ui.trace(
                "runtime",
                &format!("linked in {}", format_duration(step_runtime.elapsed())),
            );
            with_runtime
        } else {
            let step_runtime = ui.step_start(
                6,
                TOTAL_STEPS,
                "Runtime Injection",
                "helper-only (--no-runtime)",
            );
            let mut without_runtime = String::new();
            let runtime_roots = runtime_roots_for_output(&final_output, false);
            without_runtime.push_str(&crate::runtime::render_runtime_subset(&runtime_roots));
            if !without_runtime.ends_with('\n') {
                without_runtime.push('\n');
            }
            append_runtime_configuration(
                &mut without_runtime,
                request.entry_path,
                request.type_cfg,
                request.parallel_cfg,
                false,
                &runtime_roots,
            );
            if !final_output.is_empty() {
                without_runtime.push_str("# --- RR generated code (from user RR source) ---\n");
            }
            without_runtime.push_str(&final_output);
            for call in &top_level_call_names {
                if !without_runtime.ends_with('\n') {
                    without_runtime.push('\n');
                }
                if !without_runtime
                    .contains("# --- RR synthesized entrypoints (auto-generated) ---\n")
                    && !top_level_call_names.is_empty()
                {
                    without_runtime
                        .push_str("# --- RR synthesized entrypoints (auto-generated) ---\n");
                }
                without_runtime.push_str(&format!("{}()\n", call));
            }
            ui.step_line_ok(&format!(
                "Output size: {}",
                human_size(without_runtime.len())
            ));
            ui.trace(
                "runtime",
                &format!("helper-only in {}", format_duration(step_runtime.elapsed())),
            );
            without_runtime
        };
        let final_source_map =
            shifted_source_map_for_final_output_prefix(final_source_map, &final_code);
        ui.pulse_success(compile_started.elapsed());

        Ok((
            final_code,
            final_source_map,
            emit_cache_hits,
            emit_cache_misses,
        ))
    })
}

fn verify_emittable_program(
    all_fns: &FxHashMap<String, crate::mir::def::FnIR>,
) -> crate::error::RR<()> {
    let mut names: Vec<&String> = all_fns.keys().collect();
    names.sort();
    for name in names {
        let Some(fn_ir) = all_fns.get(name) else {
            continue;
        };
        crate::mir::verify::verify_emittable_ir(fn_ir).map_err(|e| {
            crate::error::InternalCompilerError::new(
                crate::error::Stage::Codegen,
                format!(
                    "emittable MIR verification failed for function '{}': {}",
                    fn_ir.name, e
                ),
            )
            .into_exception()
        })?;
    }
    Ok(())
}
